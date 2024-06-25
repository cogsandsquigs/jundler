use crate::js_config::{PackageConfig, SEAConfig};
use anyhow::{anyhow, Context, Result};
use core::fmt;
use flate2::read::GzDecoder;
use log::{debug, warn};
use rand::distributions::{Alphanumeric, DistString};
use reqwest::blocking::get;
use semver::Version;
use std::env::consts::{ARCH, OS};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{default, fs};
use tar::Archive;
use tempdir::TempDir;

#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    #[clap(alias = "darwin")]
    MacOS,
    Linux,
    Windows,
}

impl Default for Os {
    fn default() -> Self {
        match OS {
            "macos" | "darwin" => Os::MacOS,
            "linux" => Os::Linux,
            "windows" => Os::Windows,
            _ => panic!("Building for unsupported os target!"),
        }
    }
}

impl fmt::Display for Os {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Os::MacOS => write!(f, "darwin"),
            Os::Linux => write!(f, "linux"),
            Os::Windows => write!(f, "win"),
        }
    }
}

#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    #[clap(alias = "x64")]
    #[clap(alias = "x86_64")]
    X64,

    #[clap(alias = "x86")]
    X86,

    Arm64,
}

impl default::Default for Arch {
    fn default() -> Self {
        match ARCH {
            "x86" => Arch::X86, // "x86" is not a valid value for ARCH, but we'll include it for completeness
            "x64" | "x86_64" => Arch::X64,
            "arm" | "aarch64" => Arch::Arm64,
            _ => panic!("Building for unsupported architecture target!"),
        }
    }
}

impl fmt::Display for Arch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Arch::X64 => write!(f, "x64"),
            Arch::X86 => write!(f, "x86"),
            Arch::Arm64 => write!(f, "arm64"),
        }
    }
}

pub struct Builder {
    /// The directory to build the project in.
    build_dir: PathBuf,

    /// The directory of the project to build.
    original_project_dir: PathBuf,

    /// The current OS.
    host_os: Os,

    /// The version of Node.js to build with.
    node_version: Version,

    /// The OS to build for.
    node_os: Os,

    /// The architecture to build for.
    node_arch: Arch,

    /// The SEA configuration for the project.
    sea_config: SEAConfig,

    /// The package configuration for the project.
    package_config: PackageConfig,

    /// Whether or not we are bundling the project.
    bundle: bool,
}

impl Builder {
    pub fn new(
        project_dir: PathBuf,
        node_version: Version,
        node_os: Os,
        node_arch: Arch,
        sea_config: SEAConfig,
        package_config: PackageConfig,
        bundle: bool,
    ) -> Result<Self> {
        // Create a temporary directory to store the build files.
        let temp_dir = TempDir::new(
            format!(
                "node-build-{}",
                Alphanumeric.sample_string(&mut rand::thread_rng(), 16)
            )
            .as_str(),
        )
        .context("Could not create a temporary directory to build in!")?;

        Ok(Self {
            build_dir: temp_dir.into_path(),
            original_project_dir: project_dir,
            host_os: Os::default(),
            node_version,
            node_os,
            node_arch,
            sea_config,
            package_config,
            bundle,
        })
    }

    /// Builds the Node.js binary with the SEA blob, outputting it in the current directory.
    pub fn build(&mut self) -> Result<()> {
        debug!("Build in directory: {}", self.build_dir.display());

        // Copy the project to the build directory
        self.copy_project()?;

        // Bundle the project if the user wants to
        if self.bundle {
            debug!("Bundling project with esbuild...");

            self.bundle_project()?;

            debug!("Bundled!");
        }

        debug!("Downloading Node.js binary...");

        // Download the archive
        let archive = self.download_node_archive()?;

        debug!("Downloaded!");
        debug!("Extracting Node.js binary...");

        // Extract the archive
        let node_bin = self.extract_node_archive(&archive)?;

        debug!("Extracted!");
        debug!("Generating SEA blob...");

        // Generate the SEA blob
        let sea_blob = self.gen_sea_blob()?;

        debug!("SEA blob generated!");
        debug!("Injecting app into Node.js binary...");

        // Inject the app into the node binary
        self.inject_app(&node_bin, &sea_blob)?;

        debug!("Injected!");

        // Move the binary to the current directory
        let app_name = if self.node_os == Os::Windows {
            self.package_config.name.clone() + ".exe"
        } else {
            self.package_config.name.clone()
        };

        let app_path = self.original_project_dir.join(&app_name);

        fs::copy(self.build_dir.join(&app_name), &app_path)
            .context("Error moving built binary to current working directory")?;

        debug!("Binary moved to: {}", app_path.display());

        // Codesign the binary if we're on MacOS
        match (self.host_os, self.node_os) {
            (Os::MacOS, Os::MacOS) => {
                debug!("Codesigning binary for MacOS...");
                self.macos_codesign(&app_path)?;
                debug!("Signed!");
            }

            (_, Os::MacOS) => {
                warn!("Warning: Not codesigning the binary because the host OS is not MacOS.");
                warn!("This will cause an error when running the binary on MacOS.");
                warn!("Please codesign the binary manually before distributing or running it.");
            }

            _ => {}
        }

        Ok(())
    }
}

// Private helper functions to do steps of the build process
impl Builder {
    /// Copy the project to the build directory, into a project folder.
    fn copy_project(&self) -> Result<()> {
        let project_dir = self.build_dir.join("project");

        // Create the project directory in the build directory
        fs::create_dir(self.build_dir.join("project")).context(format!(
            "Error creating temporary project directory at {}",
            project_dir.display()
        ))?;

        // Copy the project to the build directory
        fs_extra::dir::copy(
            &self.original_project_dir,
            &project_dir,
            &fs_extra::dir::CopyOptions::new()
                .content_only(true)
                .overwrite(true),
        )
        .context(format!(
            "Error copying project from {} to {}",
            self.original_project_dir.display(),
            project_dir.display()
        ))?;

        Ok(())
    }

    /// Bundle the project using `esbuild` if desired by the user.
    fn bundle_project(&mut self) -> Result<()> {
        // Install any and all packages required for the project
        let npm_install_cmd_output = Command::new("npm")
            .current_dir(&self.build_dir.join("project")) // Run the command in the project directory
            .arg("install")
            .output()
            .context("Error running npm install")?;

        if !npm_install_cmd_output.status.success() {
            return Err(anyhow!(
                "Error running npm install:\n{}\n{}",
                String::from_utf8_lossy(&npm_install_cmd_output.stdout),
                String::from_utf8_lossy(&npm_install_cmd_output.stderr)
            ));
        }

        // Run the esbuild command
        let esbuild_cmd_output = Command::new("npx")
            .current_dir(&self.build_dir.join("project")) // Run the command in the project directory
            .arg("esbuild")
            .arg(&self.sea_config.main)
            .arg("--bundle")
            .arg("--platform=node") // Bundle for Node.js
            .arg("--outfile=bundled.js") // Output to `bundled.js` in the build directory
            .output()
            .context("Error bundling project with esbuild")?;

        if !esbuild_cmd_output.status.success() {
            return Err(anyhow!(
                "Error bundling project with esbuild:\n{}\n{}",
                String::from_utf8_lossy(&esbuild_cmd_output.stdout),
                String::from_utf8_lossy(&esbuild_cmd_output.stderr)
            ));
        }

        // Rewrite `sea-config.json` to point to the bundled file
        let new_sea_config = SEAConfig {
            main: "bundled.js".to_string(),
            ..self.sea_config.clone()
        };

        self.sea_config = new_sea_config;

        // Write the new `sea-config.json` to the project directory
        let sea_config_path = self.build_dir.join("project").join("sea-config.json");

        let sea_config_file =
            File::create(&sea_config_path).context("Error creating new `sea-config.json` file")?;

        serde_json::to_writer_pretty(sea_config_file, &self.sea_config).context(format!(
            "Error writing new `sea-config.json` file to {}",
            sea_config_path.display()
        ))?;

        Ok(())
    }

    /// Download the Node.js archive from the official website, and returns the path to the downloaded file.
    fn download_node_archive(&self) -> Result<PathBuf> {
        let node_folder_name = format!(
            "node-v{}-{}-{}",
            self.node_version, self.node_os, self.node_arch
        );

        let mut url = format!(
            "https://nodejs.org/dist/v{}/{}",
            self.node_version, node_folder_name
        );

        if self.node_os == Os::Windows {
            // Download a zip file
            url += ".zip";
        } else {
            // Download a tarball
            url += ".tar.gz";
        }

        debug!("Downloading Node.js from: {}", url);

        // Download the file from the URL
        let response = get(&url).context(format!("Error downloading node from {}", url))?;

        let content = response
            .bytes()
            .context(format!("Error downloading node from {}", url))?;

        let file_name =
            self.build_dir
                .join("node")
                .with_extension(if self.node_os == Os::Windows {
                    "zip"
                } else {
                    "tar.gz"
                });

        let mut file = File::create(&file_name).context(format!(
            "Error creating file for node archive downloaded from {}",
            url
        ))?;

        // Writing the content to the file
        let mut pos = 0;
        while pos < content.len() {
            let bytes_written = file.write(&content[pos..]).context(format!(
                "Error writing to node archive with download from {}",
                url
            ))?;
            pos += bytes_written;
        }

        Ok(file_name)
    }

    /// Extract the Node.js archive, and returns the path to the extracted binary.
    fn extract_node_archive(&self, archive: &Path) -> Result<PathBuf> {
        // Extract the archive to `{build-dir}/node-v{version}-{os}-{arch}`
        let bin_path = match self.node_os {
            Os::MacOS | Os::Linux => {
                // Extract the tarball
                let tar_gz = File::open(archive).context("Error opening node archive file")?;
                let tar = GzDecoder::new(tar_gz);

                let mut archive = Archive::new(tar);

                archive
                    .unpack(&self.build_dir)
                    .context("Error extracting node archive file")?;

                self.build_dir
                    .join(format!(
                        "node-v{}-{}-{}",
                        self.node_version, self.node_os, self.node_arch
                    ))
                    .join("bin")
                    .join("node")
            }

            Os::Windows => {
                // Extract the zip file
                let file = File::open(archive).context("Error opening node archive file")?;
                let mut archive =
                    zip::ZipArchive::new(file).context("Error reading zip archive")?;

                archive
                    .extract(&self.build_dir)
                    .context("Error extracting node archive file")?;

                self.build_dir
                    .join(format!(
                        "node-v{}-{}-{}",
                        self.node_version, self.node_os, self.node_arch
                    ))
                    .join("node.exe")
            }
        };

        let new_bin_path = self.build_dir.join(if self.node_os == Os::Windows {
            self.package_config.name.clone() + ".exe"
        } else {
            self.package_config.name.clone()
        });

        // Move to the build directory and rename the binary
        fs::copy(bin_path, &new_bin_path)
            .context("Error moving node binary into build directory")?;

        Ok(new_bin_path)
    }

    /// Generate the SEA blob for the Node.js binary.
    fn gen_sea_blob(&self) -> Result<PathBuf> {
        // Get the path to `sea-config.json` from `{build-dir}/project/sea-config.json` because we want to use the
        // configuration that points to the bundled file IF the project was bundled (which is modified in the project
        // directory). Otherwise, this is the same as the original `sea-config.json` file.
        let sea_conf_path = self.build_dir.join("project").join("sea-config.json");
        // Generate the SEA blob
        let sea_blob_cmd_output = Command::new("node")
            .current_dir(&self.build_dir.join("project")) // Run the command in the project directory
            .arg("--experimental-sea-config")
            .arg(sea_conf_path)
            .output()
            .context("Error generating SEA blob file")?;

        if !sea_blob_cmd_output.status.success() {
            return Err(anyhow!(
                "Error generating SEA blob file:\n{}\n{}",
                String::from_utf8_lossy(&sea_blob_cmd_output.stdout),
                String::from_utf8_lossy(&sea_blob_cmd_output.stderr)
            ));
        }

        let sea_blob = self
            .build_dir
            .join("project") // Expect the sea blob to be in the project directory
            .join(&self.sea_config.output);

        let new_sea_blob_path = self.build_dir.join(&self.sea_config.output);

        // Move the sea blob to the build directory
        fs::rename(sea_blob, &new_sea_blob_path)
            .context("Error moving SEA blob file to build directory")?;

        Ok(new_sea_blob_path)
    }

    /// Injects the app into the node binary.
    fn inject_app(&self, node_bin: &Path, sea_blob: &Path) -> Result<()> {
        // Run the postject command
        let postject_cmd_output = Command::new("npx")
            .current_dir(&self.build_dir)
            .arg("--yes")
            .arg("postject")
            .arg(node_bin)
            .arg("NODE_SEA_BLOB")
            .arg(sea_blob)
            .arg("--sentinel-fuse")
            .arg("NODE_SEA_FUSE_fce680ab2cc467b6e072b8b5df1996b2")
            .arg(if self.node_os == Os::MacOS {
                "--macho-segment-name"
            } else {
                ""
            })
            .arg(if self.node_os == Os::MacOS {
                "NODE_SEA"
            } else {
                ""
            })
            .output()
            .context("Error injecting app into node binary")?;

        if !postject_cmd_output.status.success() {
            return Err(anyhow!(
                "Error generating SEA blob file:\n{}\n{}",
                String::from_utf8_lossy(&postject_cmd_output.stdout),
                String::from_utf8_lossy(&postject_cmd_output.stderr)
            ));
        }

        Ok(())
    }

    /// Codesign the binary for MacOS
    fn macos_codesign(&self, binary: &Path) -> Result<()> {
        let codesign_cmd_output = Command::new("codesign")
            .arg("--force")
            .arg("--sign")
            .arg("-")
            .arg(binary)
            .output()
            .context("Error codesigning the binary")?;

        if !codesign_cmd_output.status.success() {
            return Err(anyhow!(
                "Error codesigning the binary:\n{}\n{}",
                String::from_utf8_lossy(&codesign_cmd_output.stdout),
                String::from_utf8_lossy(&codesign_cmd_output.stderr)
            ));
        }

        Ok(())
    }
}
