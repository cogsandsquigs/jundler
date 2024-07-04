mod errors;
mod helpers;
mod lock;
mod platforms;
mod sumfile_parser;
mod tests;

// Re-export error types
pub use errors::Error;

use helpers::calculate_checksum;
use helpers::*;
use lock::{NodeExecutable, NodeManagerLock};
use log::warn;
pub use platforms::{get_host_arch, get_host_os, Arch, Os};
use semver::Version;
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
};
use tempdir::TempDir;

pub struct NodeManager {
    /// The directory where different node versions are stored.
    node_cache_dir: PathBuf,

    /// Loaded lockfile information
    lockfile: NodeManagerLock,

    /// A temporary directory for downloading and extracting node binaries. Need this b/c for as long as
    /// `NodeManager` is held, we may need to download and extract node binaries at arbitrary times during
    /// it's lifetime.
    tmp_dir: TempDir,
}

impl NodeManager {
    /// Creates a new NodeManager. We expect that `node_cache_dir` exists and is writable.
    pub fn new(node_cache_dir: PathBuf) -> Result<Self, Error> {
        let lockfile_path = node_cache_dir.join("jundler.lockb");

        let lockfile = if lockfile_path.exists() {
            match NodeManagerLock::load(lockfile_path.clone()) {
                Ok(lockfile) => lockfile,

                // If we can't load the lockfile, we'll just create a new one
                Err(Error::LockfileSerialization { .. }) => {
                    warn!("Failed to load lockfile, creating a new one"); // TODO: Better UI
                    NodeManagerLock::new(Vec::new(), lockfile_path.clone())
                }

                Err(e) => return Err(e),
            }
        } else {
            NodeManagerLock::new(Vec::new(), lockfile_path.clone())
        };

        let tmp_dir = TempDir::new("jundler-node-scratch").map_err(|err| Error::Io {
            err,
            path: PathBuf::from("tempdir"),
            action: "creating temp dir for node scratch at".to_string(),
        })?;

        Ok(Self {
            // host_os: get_host_os(),
            // host_arch: get_host_arch(),
            // target_os,
            // target_arch,
            node_cache_dir,
            lockfile,
            tmp_dir,
        })
    }

    /// Downloads a target binary if it doesn't exist, and returns the path to the binary.
    pub fn get_binary(&mut self, version: &Version, os: Os, arch: Arch) -> Result<PathBuf, Error> {
        let binary = self.lockfile.find(version, os, arch);

        // Return it if it exists
        let binary_path = if let Some(archive) = binary {
            // Check the checksum of the binary. If it's invalid, re-download it.
            if !archive.validate_checksum()? {
                warn!("Checksum mismatch for node binary, re-downloading"); // TODO: Better UI

                // Remove the binary from the cache
                self.remove(archive)?;

                // Download the binary again
                self.download(version, os, arch)?.0
            }
            // If the binary exists, and the checksum is valid, return the path to the binary
            else {
                self.unpack_archive(&archive)?
            }
        }
        // If it doesn't exist, download it
        else {
            self.download(version, os, arch)?.0
        };

        // Make the binary executable on Unix-based systems
        #[cfg(unix)]
        make_executable(&binary_path)?;

        Ok(binary_path)
    }

    /// Removes a node binary from the cache.
    pub fn remove(&mut self, node_executable: NodeExecutable) -> Result<(), Error> {
        let path = &node_executable.path;

        // Remove the binary from the lockfile
        self.lockfile.remove(&node_executable);

        // Save the lockfile
        self.lockfile.save()?;

        // Delete the file from the cache
        fs::remove_file(path).map_err(|err| Error::Io {
            err,
            path: path.clone(),
            action: "deleting node binary archive at".to_string(),
        })?;

        Ok(())
    }

    /// Cleans the cache directory by removing all node binaries and clearing the lockfile.
    pub fn clean_cache(&mut self) -> Result<(), Error> {
        // First, clean the lockfile by removing all entries.
        self.lockfile.node_executables.clear();

        // Delete the entire cache directory
        fs::remove_dir_all(&self.node_cache_dir).map_err(|err| Error::Io {
            err,
            path: self.node_cache_dir.clone(),
            action: "deleting node cache directory at".to_string(),
        })?;

        // Recreate the cache directory
        fs::create_dir_all(&self.node_cache_dir).map_err(|err| Error::Io {
            err,
            path: self.node_cache_dir.clone(),
            action: "recreating node cache directory at".to_string(),
        })?;

        // Save the lockfile
        self.lockfile.save()?;

        Ok(())
    }

    /// Download a new node binary, and store it in the cache. Returns a tuple of the form `(path to the binary, path to the archive)`.
    fn download(
        &mut self,
        version: &Version,
        os: Os,
        arch: Arch,
    ) -> Result<(PathBuf, PathBuf), Error> {
        // Download the checksum file
        let checksums = download_checksums(version)?;

        // TODO: Check the signature of the checksum file (if available)

        // Find the correct checksum for the requested platform
        let (checksum, meta) = checksums
            .into_iter()
            .find(|(_, meta)| meta.version == *version && meta.os == os && meta.arch == arch)
            .ok_or_else(|| Error::NodeBinaryDNE {
                version: version.clone(),
                os,
                arch,
            })?;

        // Download the node archive
        let downloaded_archive_path =
            download_node_archive(self.tmp_dir.path(), version, os, arch)?;

        let actual_checksum = calculate_checksum(&downloaded_archive_path)?;

        // Error out if the checksums don't match
        if actual_checksum != checksum {
            return Err(Error::ChecksumMismatch {
                path: downloaded_archive_path,
                expected: checksum,
                actual: actual_checksum,
            });
        }

        // Unpack the archive. Needs version, os, and arch to determine the correct path to the binary (named folder).
        let node_executable_path = unpack_downloaded_node_archive(
            self.tmp_dir.path(),
            &downloaded_archive_path,
            version,
            os,
            arch,
        )?;

        let node_archive_path = repack_node_binary(
            &node_executable_path,
            version,
            os,
            arch,
            &self.node_cache_dir,
        )?;

        let archive_checksum = calculate_checksum(&node_archive_path)?;

        // Add the node binary to the lockfile
        self.lockfile.add(NodeExecutable {
            meta,
            path: node_archive_path.clone(),
            checksum: archive_checksum,
        });

        // Save the lockfile
        self.lockfile.save()?;

        Ok((node_executable_path, node_archive_path))
    }

    /// Unpack a node binary from the cache. Returns the path to the binary.
    pub fn unpack_archive(&self, node_archive: &NodeExecutable) -> Result<PathBuf, Error> {
        // Undo the process in `repack_node_binary`
        let archived_binary = File::open(&node_archive.path).map_err(|err| Error::Io {
            err,
            path: node_archive.path.clone(),
            action: "opening node archive file at".to_string(),
        })?;

        let mut zstd_decoder = zstd::Decoder::new(archived_binary).map_err(|err| Error::Io {
            err,
            path: node_archive.path.clone(),
            action: "creating zstd decoder for archive file at".to_string(),
        })?;

        let extracted_binary_path = self.tmp_dir.path().join(format!(
            // .exe for windows, doesn't matter for other platforms. Also, avoids collision with folders of the same name.
            "node-v{}-{}-{}.exe",
            node_archive.meta.version, node_archive.meta.os, node_archive.meta.arch
        ));

        let mut extracted_binary =
            File::create(&extracted_binary_path).map_err(|err| Error::Io {
                err,
                path: extracted_binary_path.clone(),
                action: "creating extracted node binary file at".to_string(),
            })?;

        let mut buf: Vec<u8> = vec![];

        zstd_decoder
            .read_to_end(&mut buf)
            .map_err(|err| Error::Io {
                err,
                path: node_archive.path.clone(),
                action: "reading from archive file at".to_string(),
            })?;

        extracted_binary.write_all(&buf).map_err(|err| Error::Io {
            err,
            path: extracted_binary_path.clone(),
            action: "writing to extracted node binary file at".to_string(),
        })?;

        Ok(extracted_binary_path)
    }
}
