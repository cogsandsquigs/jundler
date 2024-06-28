mod helpers;
pub mod node;
mod tests;

use crate::js_config::{PackageConfig, ProjectType, SEAConfig};
use anyhow::{Context, Result};
use log::{debug, warn};
use node::{get_host_arch, get_host_os, Arch, NodeManager, Os};
use rand::distributions::{Alphanumeric, DistString};
use semver::Version;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use tempdir::TempDir;

pub struct Builder {
    /// The directory to build the project in.
    working_dir: TempDir,

    /// The Node.js manager
    node_manager: NodeManager,
}

impl Builder {
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
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
            working_dir: temp_dir,
            node_manager: NodeManager::new(cache_dir)?,
        })
    }

    /// Cleans the cache directory of the Node.js manager.
    pub fn clean_cache(&mut self) -> Result<()> {
        Ok(self.node_manager.clean_cache()?)
    }

    /// Builds the Node.js binary with the SEA blob, outputting it in the current directory.
    pub fn build(
        &mut self,
        project_dir: &Path,
        node_version: Version,
        target_os: Os,
        target_arch: Arch,
        bundle: bool,
    ) -> Result<()> {
        // Get the configuration
        let (mut sea_config, package_config) = get_configs(project_dir)?;
        let (host_os, host_arch) = (get_host_os(), get_host_arch());

        debug!("Build in directory: {}", self.working_dir.path().display());

        // Copy the project to the build directory
        self.copy_and_prepare_project(project_dir, target_os, target_arch)?;

        // Bundle the project if the user wants to, or if the project is a module or TypeScript project
        if bundle
            || package_config.project_type == ProjectType::Module
            || package_config
                .main
                .as_ref()
                .is_some_and(|m| m.ends_with(".mjs"))
            || package_config
                .main
                .as_ref()
                .is_some_and(|m| m.ends_with(".ts"))
        {
            debug!("Bundling project with esbuild...");

            self.bundle_project(&package_config, &mut sea_config)?;

            debug!("Bundled!");
        }

        // debug!("Downloading Node.js binary...");

        // // Download the archive
        // let archive = self.download_node_archive()?;

        // debug!("Downloaded!");
        // debug!("Extracting Node.js binary...");

        // // Extract the archive
        // let node_bin = self.extract_node_archive(&archive)?;

        // debug!("Extracted!");

        // Get the node binary
        let target_node_bin =
            self.node_manager
                .get_binary(&node_version, target_os, target_arch)?;
        let host_node_bin = self
            .node_manager
            .get_binary(&node_version, host_os, host_arch)?;

        debug!("Generating SEA blob..."); // TODO: Better ui

        // Generate the SEA blob
        let sea_blob = self.gen_sea_blob(&host_node_bin, sea_config)?;

        debug!("SEA blob generated!");
        debug!("Injecting app into Node.js binary...");

        // Inject the app into the node binary
        self.inject_app(&target_node_bin, &sea_blob, target_os)?;

        debug!("Injected!");

        // Move the binary to the current directory
        let app_name = if target_os == Os::Windows {
            package_config.name.clone() + ".exe"
        } else {
            package_config.name.clone()
        };

        let app_path = project_dir.join(app_name);

        fs::copy(target_node_bin, &app_path)
            .context("Error moving built binary to current working directory")?;

        debug!("Binary moved to: {}", app_path.display());

        // Codesign the binary if we're on MacOS
        match (host_os, target_os) {
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

            (Os::Windows, Os::Windows) => {
                debug!("Signing binary for Windows...");
                self.windows_sign(&app_path)?;
                debug!("Signed!");
            }

            (_, Os::Windows) => {
                warn!("Warning: Not signing the binary because the host OS is not Windows.");
                warn!("The binary will still be runnable, but it will raise a warning message with the user.");
                warn!("Please sign the binary manually before distributing or running it.");
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
