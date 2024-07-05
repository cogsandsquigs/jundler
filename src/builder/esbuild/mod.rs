pub mod errors;
mod helpers;
mod lock;
mod tests;

pub use errors::Error;

use super::helpers::make_executable;
use crate::builder::helpers::calculate_checksum;
use helpers::{download_esbuild_archive, repack_esbuild_binary, unpack_downloaded_esbuild_archive};
use lock::{ESBuildExecutable, ESBuildLock};
use log::warn;
use semver::Version;
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
};
use tempdir::TempDir;

/// The version of ESBuild to use. This should be updated whenever the version of ESBuild is updated. The version is specified as
/// `Version::new(<major>, <minor>, <patch>)`.
const ESBUILD_VERSION: Version = Version::new(0, 23, 0);

/// An esbuild instance
pub struct ESBuild {
    /// The temporary directory for the esbuild instance
    tmp_dir: TempDir,

    /// The lockfile for the esbuild instance
    lockfile: ESBuildLock,

    /// The directory where the esbuild instance is located
    cache_dir: PathBuf,
}

impl ESBuild {
    /// Creates a new esbuild instance. Expects that `esbuild_cache_dir` is a valid directory.
    pub fn new(esbuild_cache_dir: PathBuf) -> Result<Self, Error> {
        let lockfile_path = esbuild_cache_dir.join("jundler.lockb");

        let lockfile = if lockfile_path.exists() {
            match ESBuildLock::load(lockfile_path.clone()) {
                Ok(lockfile) => lockfile,

                // If we can't load the lockfile, we'll just create a new one
                Err(Error::LockfileSerialization { .. }) => {
                    warn!("Failed to load lockfile, creating a new one"); // TODO: Better UI
                    ESBuildLock::new(lockfile_path.clone())
                }

                Err(e) => return Err(e),
            }
        } else {
            ESBuildLock::new(lockfile_path.clone())
        };

        let tmp_dir = TempDir::new("jundler-node-scratch").map_err(|err| Error::Io {
            err,
            path: PathBuf::from("tempdir"),
            action: "creating temp dir for node scratch at".to_string(),
        })?;

        Ok(Self {
            cache_dir: esbuild_cache_dir,
            lockfile,
            tmp_dir,
        })
    }

    /// Downloads a target binary if it doesn't exist, and returns the path to the binary.
    pub fn get_binary(&mut self) -> Result<PathBuf, Error> {
        let binary = self.lockfile.get();

        // Return it if it exists
        let binary_path = if let Some(archive) = binary {
            // Check the checksum of the binary. If it's invalid, re-download it.
            if !archive.validate_checksum()? {
                warn!("Checksum mismatch for node binary, re-downloading"); // TODO: Better UI

                // Remove the binary from the cache
                self.remove(&archive)?;

                // Download the binary again
                self.download(&ESBUILD_VERSION)?
            }
            // If the binary exists, and the checksum is valid, return the path to the binary
            else {
                self.unpack_archive(&archive)?
            }
        }
        // If it doesn't exist, download it
        else {
            self.download(&ESBUILD_VERSION)?
        };

        // Make the binary executable on Unix-based systems
        #[cfg(unix)]
        make_executable(&binary_path).map_err(|err| Error::Io {
            err,
            path: binary_path.to_path_buf(),
            action: "making binary executable at".to_string(),
        })?;

        Ok(binary_path)
    }

    /// Cleans the cache directory by removing all node binaries and clearing the lockfile.
    pub fn clean_cache(&mut self) -> Result<(), Error> {
        // First, clean the lockfile by removing all entries.
        self.lockfile.executable = None;

        // Delete the entire cache directory
        fs::remove_dir_all(&self.cache_dir).map_err(|err| Error::Io {
            err,
            path: self.cache_dir.clone(),
            action: "deleting node cache directory at".to_string(),
        })?;

        // Recreate the cache directory
        fs::create_dir_all(&self.cache_dir).map_err(|err| Error::Io {
            err,
            path: self.cache_dir.clone(),
            action: "recreating node cache directory at".to_string(),
        })?;

        // Save the lockfile
        self.lockfile.save()?;

        Ok(())
    }
}

impl ESBuild {
    /// Download a new node binary, and store it in the cache. Returns a tuple of the form `(path to the binary, path to the archive)`.
    fn download(&mut self, version: &Version) -> Result<PathBuf, Error> {
        // Download the node archive
        let downloaded_archive_path = download_esbuild_archive(self.tmp_dir.path(), version)?;

        // Unpack the archive. Needs version, os, and arch to determine the correct path to the binary (named folder).
        let node_executable_path =
            unpack_downloaded_esbuild_archive(self.tmp_dir.path(), &downloaded_archive_path)?;

        let node_archive_path =
            repack_esbuild_binary(&node_executable_path, version, &self.cache_dir)?;

        let archive_checksum = calculate_checksum(&node_archive_path).map_err(|err| Error::Io {
            err,
            path: node_archive_path.clone(),
            action: "calculating checksum of node executable at".into(),
        })?;

        // Add the node binary to the lockfile
        self.lockfile.add(ESBuildExecutable {
            version: version.clone(),
            path: node_archive_path.clone(),
            checksum: archive_checksum,
        })?;

        Ok(node_executable_path)
    }

    /// Unpack a node binary from the cache. Returns the path to the binary.
    pub fn unpack_archive(&self, esbuild_archive: &ESBuildExecutable) -> Result<PathBuf, Error> {
        // Undo the process in `repack_node_binary`
        let archived_binary = File::open(&esbuild_archive.path).map_err(|err| Error::Io {
            err,
            path: esbuild_archive.path.clone(),
            action: "opening esbuild archive file at".to_string(),
        })?;

        let mut zstd_decoder = zstd::Decoder::new(archived_binary).map_err(|err| Error::Io {
            err,
            path: esbuild_archive.path.clone(),
            action: "creating zstd decoder for archive file at".to_string(),
        })?;

        let extracted_binary_path = self.tmp_dir.path().join(format!(
            // .exe for windows, doesn't matter for other platforms. Also, avoids collision with folders of the same name.
            "esbuild-v{}.exe",
            esbuild_archive.version
        ));

        let mut extracted_binary =
            File::create(&extracted_binary_path).map_err(|err| Error::Io {
                err,
                path: extracted_binary_path.clone(),
                action: "creating extracted esbuild binary file at".to_string(),
            })?;

        let mut buf: Vec<u8> = vec![];

        zstd_decoder
            .read_to_end(&mut buf)
            .map_err(|err| Error::Io {
                err,
                path: esbuild_archive.path.clone(),
                action: "reading from archive file at".to_string(),
            })?;

        extracted_binary.write_all(&buf).map_err(|err| Error::Io {
            err,
            path: extracted_binary_path.clone(),
            action: "writing to extracted esbuild binary file at".to_string(),
        })?;

        Ok(extracted_binary_path)
    }

    /// Remove the binary from the cache. Returns the path to the binary.
    pub fn remove(&mut self, esbuild_archive: &ESBuildExecutable) -> Result<PathBuf, Error> {
        // Remove the binary from the cache
        fs::remove_file(&esbuild_archive.path).map_err(|err| Error::Io {
            err,
            path: esbuild_archive.path.clone(),
            action: "removing esbuild binary at".to_string(),
        })?;

        // Remove the binary from the lockfile
        self.lockfile.remove(esbuild_archive)?;

        Ok(esbuild_archive.path.clone())
    }
}
