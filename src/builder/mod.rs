mod errors;
mod esbuild;
mod helpers;
pub mod node_manager;
mod tests;

pub use errors::Error;

use crate::js_config::{PackageConfig, ProjectType, SEAConfig};
use crate::ui::messages::{
    BUNDLE_PROJ_MSG, CLEAN_CACHE_MSG, COPY_PROJ_MSG, GEN_SEA_BLOB_MSG, HOST_NODE_MSG,
    INJECT_APP_MSG, MACOS_CODESIGN_MSG, MAX_MSG_LEN, TARGET_NODE_MSG, WINDOWS_CODESIGN_MSG,
};
use crate::ui::Interface;
use anyhow::{Context, Ok, Result};
use log::{debug, warn};
use node_manager::{get_host_arch, get_host_os, Arch, NodeManager, Os};
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

    /// The interface to UI
    interface: Interface,
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
            interface: Interface::new(MAX_MSG_LEN),
        })
    }

    /// Cleans the cache directory of the Node.js manager.
    pub fn clean_cache(&mut self) -> Result<()> {
        let spinner = self.interface.spawn_spinner(CLEAN_CACHE_MSG);

        self.node_manager.clean_cache()?;

        spinner.close();

        Ok(())
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

        let spinner = self.interface.spawn_spinner(COPY_PROJ_MSG);

        // Copy the project to the build directory
        self.copy_and_prepare_project(project_dir, target_os, target_arch)?;

        spinner.close();

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
            let spinner = self.interface.spawn_spinner(BUNDLE_PROJ_MSG);

            self.bundle_project(&package_config, &mut sea_config)?;

            spinner.close();
        }

        let spinner = self.interface.spawn_spinner(TARGET_NODE_MSG);

        let target_node_bin =
            self.node_manager
                .get_binary(&node_version, target_os, target_arch)?;

        spinner.close();

        let spinner = self.interface.spawn_spinner(HOST_NODE_MSG);

        let host_node_bin = self
            .node_manager
            .get_binary(&node_version, host_os, host_arch)?;

        spinner.close();

        let spinner = self.interface.spawn_spinner(GEN_SEA_BLOB_MSG);

        // Generate the SEA blob
        let sea_blob = self.gen_sea_blob(&host_node_bin, sea_config)?;

        spinner.close();

        let spinner = self.interface.spawn_spinner(INJECT_APP_MSG);

        // Inject the app into the node binary
        self.inject_app(&target_node_bin, &sea_blob, target_os)?;

        spinner.close();

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
                let spinner = self.interface.spawn_spinner(MACOS_CODESIGN_MSG);
                self.macos_codesign(&app_path)?;
                spinner.close();
            }

            (_, Os::MacOS) => {
                // TODO: Better UI for warnings
                warn!("Warning: Not codesigning the binary because the host OS is not MacOS.");
                warn!("This will cause an error when running the binary on MacOS.");
                warn!("Please codesign the binary manually before distributing or running it.");
            }

            (Os::Windows, Os::Windows) => {
                let spinner = self.interface.spawn_spinner(WINDOWS_CODESIGN_MSG);
                self.windows_sign(&app_path)?;
                spinner.close();
            }

            (_, Os::Windows) => {
                // TODO: Better UI for warnings
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
