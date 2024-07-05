use super::platforms::{Arch, Os};
use super::Builder;
use crate::js_config::{PackageConfig, SEAConfig};
use crate::ui::messages::{BUNDLING_MSG, ESBUILD_BINARY_MSG};
use anyhow::{anyhow, Context, Result};
use log::warn;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{fs, io};

/// On Unix-based systems, make the binary executable.
pub fn make_executable(binary_path: &Path) -> Result<(), io::Error> {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = binary_path.metadata()?.permissions();

    perms.set_mode(0o755);

    fs::set_permissions(binary_path, perms)?;

    Ok(())
}

/// Calculate the SHA256 checksum of a file. Expects that the file is readable.
pub fn calculate_checksum(path: &Path) -> Result<[u8; 32], io::Error> {
    // Prepare the hasher
    let mut hasher = Sha256::new();

    let mut file = File::open(path)?;

    io::copy(&mut file, &mut hasher)?;

    // Output the hash and convert it into a 32-byte array
    Ok(hasher.finalize().into())
}

// Private helper functions to do steps of the build process
impl Builder {
    /// Copy the project to the build directory, into a project folder.
    pub(super) fn copy_and_prepare_project(
        &self,
        original_project_dir: &Path,
        target_os: Os,
        target_arch: Arch,
    ) -> Result<()> {
        let project_dir = self.working_dir.path().join("project");

        // Create the project directory in the build directory
        fs::create_dir(self.working_dir.path().join("project")).context(format!(
            "Error creating temporary project directory at {}",
            project_dir.display()
        ))?;

        // Copy the project to the build directory
        fs_extra::dir::copy(
            original_project_dir,
            &project_dir,
            &fs_extra::dir::CopyOptions::new()
                .content_only(true)
                .overwrite(true),
        )
        .context(format!(
            "Error copying project from {} to {}",
            original_project_dir.display(),
            project_dir.display()
        ))?;

        // Install any and all packages required for the project
        let npm_install_cmd_output = Command::new("npm")
            .current_dir(&self.working_dir.path().join("project")) // Run the command in the project directory
            .arg("install")
            .arg(format!("--target_platform={}", target_os))
            .arg(format!("--target_arch={}", target_arch))
            .output()
            .context("Error running npm install")?;

        if !npm_install_cmd_output.status.success() {
            return Err(anyhow!(
                "Error running npm install:\n{}\n{}",
                String::from_utf8_lossy(&npm_install_cmd_output.stdout),
                String::from_utf8_lossy(&npm_install_cmd_output.stderr)
            ));
        }

        Ok(())
    }

    /// Bundle the project using `esbuild` if desired by the user.
    pub(super) fn bundle_project(
        &mut self,
        package_config: &PackageConfig,
        sea_config: &mut SEAConfig,
    ) -> Result<()> {
        // Get the ESBuild binary
        // TODO: UI display this.

        let spinner = self.interface.spawn_spinner(ESBUILD_BINARY_MSG, 2);

        let esbuild_bin = self.esbuild.get_binary()?;

        spinner.close();

        let spinner = self.interface.spawn_spinner(BUNDLING_MSG, 2);

        // Run the esbuild command
        let esbuild_cmd_output = Command::new(esbuild_bin)
            .current_dir(&self.working_dir.path().join("project")) // Run the command in the project directory
            .arg(package_config.main.as_ref().unwrap_or(&sea_config.main)) // Use the main entrypoint from the package.json file, or the default from the sea-config.json file
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

        spinner.close();

        // Rewrite `sea-config.json` to point to the bundled file
        let new_sea_config = SEAConfig {
            main: "bundled.js".to_string(),
            ..sea_config.clone()
        };

        *sea_config = new_sea_config;

        // Write the new `sea-config.json` to the project directory
        let sea_config_path = self
            .working_dir
            .path()
            .join("project")
            .join("sea-config.json");

        let sea_config_file =
            File::create(&sea_config_path).context("Error creating new `sea-config.json` file")?;

        serde_json::to_writer_pretty(sea_config_file, sea_config).context(format!(
            "Error writing new `sea-config.json` file to {}",
            sea_config_path.display()
        ))?;

        Ok(())
    }

    /// Generate the SEA blob for the Node.js binary.
    pub(super) fn gen_sea_blob(
        &self,
        host_node_bin: &Path,
        sea_config: SEAConfig,
    ) -> Result<PathBuf> {
        // Get the path to `sea-config.json` from `{build-dir}/project/sea-config.json` because we want to use the
        // configuration that points to the bundled file IF the project was bundled (which is modified in the project
        // directory). Otherwise, this is the same as the original `sea-config.json` file.
        let sea_conf_path = self
            .working_dir
            .path()
            .join("project")
            .join("sea-config.json");
        // Generate the SEA blob
        let sea_blob_cmd_output = Command::new(host_node_bin)
            .current_dir(&self.working_dir.path().join("project")) // Run the command in the project directory
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
            .working_dir
            .path()
            .join("project") // Expect the sea blob to be in the project directory
            .join(&sea_config.output);

        let new_sea_blob_path = self.working_dir.path().join(&sea_config.output);

        // Move the sea blob to the build directory
        fs::rename(sea_blob, &new_sea_blob_path)
            .context("Error moving SEA blob file to build directory")?;

        Ok(new_sea_blob_path)
    }

    /// Injects the app into the node binary.
    pub(super) fn inject_app(&self, node_bin: &Path, sea_blob: &Path, target_os: Os) -> Result<()> {
        // Run the postject command
        let postject_cmd_output = Command::new("npm")
            .current_dir(&self.working_dir)
            .arg("exec")
            .arg("--yes")
            .arg("--")
            .arg("postject")
            .arg(node_bin)
            .arg("NODE_SEA_BLOB")
            .arg(sea_blob)
            .arg("--sentinel-fuse")
            .arg("NODE_SEA_FUSE_fce680ab2cc467b6e072b8b5df1996b2")
            .args(if target_os == Os::MacOS {
                &["--macho-segment-name", "NODE_SEA"]
            } else {
                &["", ""]
            })
            .output()
            .context("Error injecting app into node binary")?;

        if !postject_cmd_output.status.success() {
            return Err(anyhow!(
                "Error injecting app into node binary:\n{}\n{}\n",
                String::from_utf8_lossy(&postject_cmd_output.stdout).trim(),
                String::from_utf8_lossy(&postject_cmd_output.stderr).trim()
            ));
        }

        Ok(())
    }

    /// Codesign the binary for MacOS
    pub(super) fn macos_codesign(&self, binary: &Path) -> Result<()> {
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

    /// Codesign the binary for Windows
    pub(super) fn windows_sign(&self, binary: &Path) -> Result<()> {
        self.interface.warn("Windows signing is in beta and may not work as expected. Please report any issues here: https://github.com/cogsandsquigs/jundler/issues/new");
        let sign_cmd_output = Command::new("signtool")
            .arg("sign")
            .arg("/fd")
            .arg("SHA256")
            .arg(binary)
            .output()
            .context("Error signing the binary")?;

        if !sign_cmd_output.status.success() {
            return Err(anyhow!(
                "Error signing the binary:\n{}\n{}",
                String::from_utf8_lossy(&sign_cmd_output.stdout),
                String::from_utf8_lossy(&sign_cmd_output.stderr)
            ));
        }

        Ok(())
    }
}
