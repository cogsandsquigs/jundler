mod helpers;
pub mod platforms;
mod tests;

use self::platforms::{Arch, Os};
use crate::js_config::{PackageConfig, ProjectType, SEAConfig};
use anyhow::{Context, Result};
use log::{debug, warn};
use rand::distributions::{Alphanumeric, DistString};
use semver::Version;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use tempdir::TempDir;

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

    /// A possible path to the Node.js binary.
    custom_node: Option<PathBuf>,
}

impl Builder {
    pub fn new(
        project_dir: PathBuf,
        node_version: Version,
        node_os: Os,
        node_arch: Arch,
        bundle: bool,
        custom_node: Option<PathBuf>,
    ) -> Result<Self> {
        // Get the configuration
        let (sea_config, package_config) = get_configs(&project_dir)?;

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
            custom_node,
        })
    }

    /// Builds the Node.js binary with the SEA blob, outputting it in the current directory.
    pub fn build(&mut self) -> Result<()> {
        debug!("Build in directory: {}", self.build_dir.display());

        // Copy the project to the build directory
        self.copy_project()?;

        // Bundle the project if the user wants to, or if the project is a module or TypeScript project
        if self.bundle
            || self.package_config.project_type == ProjectType::Module
            || self
                .package_config
                .main
                .as_ref()
                .is_some_and(|m| m.ends_with(".mjs"))
            || self
                .package_config
                .main
                .as_ref()
                .is_some_and(|m| m.ends_with(".ts"))
        {
            debug!("Bundling project with esbuild...");

            self.bundle_project()?;

            debug!("Bundled!");
        }

        let node_bin = if let Some(custom_node) = &self.custom_node {
            debug!("Using custom Node.js binary: {}", custom_node.display());

            self.use_custom_node(custom_node)?
        } else {
            debug!("Downloading Node.js binary...");

            // Download the archive
            let archive = self.download_node_archive()?;

            debug!("Downloaded!");
            debug!("Extracting Node.js binary...");

            // Extract the archive
            let node_bin = self.extract_node_archive(&archive)?;

            debug!("Extracted!");

            node_bin
        };

        debug!("Generating SEA blob...");

        // Generate the SEA blob
        let sea_blob = self.gen_sea_blob()?;

        debug!("SEA blob generated!");
        debug!("Injecting app into Node.js binary...");

        // Inject the app into the node binary
        self.inject_app(&node_bin, &sea_blob)?;

        debug!("Injected!");

        // Move the binary to the current directory
        let (bin_name, app_name) = if self.node_os == Os::Windows {
            ("node.exe", self.package_config.name.clone() + ".exe")
        } else {
            ("node", self.package_config.name.clone())
        };

        let app_path = self.original_project_dir.join(app_name);

        fs::copy(self.build_dir.join(bin_name), &app_path)
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

            _ => {
                // Don't codesign the binary
            }
        }

        Ok(())
    }
}

/// Gets the `sea-config.json` and `package.json` configurations from the project directory.
fn get_configs(project_dir: &Path) -> Result<(SEAConfig, PackageConfig)> {
    let sea_config = serde_json::from_reader(
        File::open(project_dir.join("sea-config.json"))
            .context("Could not find or open the `sea-config.json` file!")?,
    )
    .context("Could not parse the `sea-config.json` file!")?;

    let package_config = serde_json::from_reader(
        File::open(project_dir.join("package.json"))
            .context("Could not find or open the `sea-config.json` file!")?,
    )
    .context("Could not parse the `package.json` file!")?;

    Ok((sea_config, package_config))
}
