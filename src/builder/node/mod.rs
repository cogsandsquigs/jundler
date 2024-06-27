mod errors;
mod helpers;
mod lock;
mod platforms;
mod sumfile_parser;
mod tests;

// Re-export error types
pub use errors::Error;

use flate2::read::GzDecoder;
use helpers::calculate_checksum;
use lock::{Checksum, NodeExecutable, NodeExecutableMeta, NodeManagerLock};
use log::{debug, warn};
pub use platforms::{get_host_arch, get_host_os, Arch, Os};
use reqwest::blocking::get;
use semver::Version;
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};
use tar::Archive;
use tempdir::TempDir;

pub struct NodeManager {
    /// The host operating system
    host_os: Os,

    /// The host architecture
    host_arch: Arch,

    /// The target operating system
    target_arch: Arch,

    /// The target architecture
    target_os: Os,

    /// The directory where different node versions are stored.
    node_cache_dir: PathBuf,

    /// Loaded lockfile information
    lockfile: NodeManagerLock,
}

impl NodeManager {
    /// Creates a new NodeManager. We expect that `node_cache_dir` exists and is writable.
    pub fn new(target_os: Os, target_arch: Arch, node_cache_dir: PathBuf) -> Result<Self, Error> {
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

        Ok(Self {
            host_os: get_host_os(),
            host_arch: get_host_arch(),
            target_os,
            target_arch,
            node_cache_dir,
            lockfile,
        })
    }

    /// Download a new node binary, returning the path to the downloaded binary
    pub fn download(&mut self, version: Version, os: Os, arch: Arch) -> Result<PathBuf, Error> {
        // Path to where we're downloading the node binary into, for temporary storage. We do this here
        // because if we do it in a sub-function, once it goes out of socpe, the directory will be deleted.
        let tmp_dir = TempDir::new("jundler-node-download").map_err(|err| Error::Io {
            err,
            path: PathBuf::from("tempdir"),
            action: "creating temp dir for node download at".to_string(),
        })?;

        let download_dir = tmp_dir.path();

        // Download the checksum file
        let checksums = download_checksums(&version)?;

        // TODO: Check the signature of the checksum file (if available)

        // Find the correct checksum for the requested platform
        let (checksum, meta) = checksums
            .into_iter()
            .find(|(_, meta)| meta.version == version && meta.os == os && meta.arch == arch)
            .ok_or_else(|| Error::NodeBinaryDNE {
                version: version.clone(),
                os,
                arch,
            })?;

        // Download the node archive
        let archive_path = download_node_archive(download_dir, &version, &os, &arch)?;

        let actual_checksum = calculate_checksum(&archive_path)?;

        // Error out if the checksums don't match
        if actual_checksum != checksum {
            return Err(Error::ChecksumMismatch {
                path: archive_path,
                expected: checksum,
                actual: actual_checksum,
            });
        }

        // Unpack the archive. Needs version, os, and arch to determine the correct path to the binary (named folder).
        let node_executable_path =
            unpack_node_archive(download_dir, &archive_path, &version, &os, &arch)?;

        todo!()
    }
}

/// Extract the Node.js archive, and returns the path to the extracted binary. `extract_dir` is the directory where the archive will
/// be extracted to.
fn unpack_node_archive(
    extract_dir: &Path,
    archive_path: &Path,
    version: &Version,
    os: &Os,
    arch: &Arch,
) -> Result<PathBuf, Error> {
    // Extract the archive to `{build-dir}/node-v{version}-{os}-{arch}`
    let bin_path = match os {
        Os::MacOS | Os::Linux => {
            // Extract the tarball
            let tar_gz = File::open(archive_path).map_err(|err| Error::Io {
                err,
                path: archive_path.to_path_buf(),
                action: "opening node archive file at".to_string(),
            })?;

            let tar = GzDecoder::new(tar_gz);

            let mut archive = Archive::new(tar);

            archive
                .unpack(extract_dir)
                .map_err(|err: std::io::Error| Error::Io {
                    err,
                    path: archive_path.into(),
                    action: "extracting node archive file from".to_string(),
                })?;

            extract_dir
                .join(format!("node-v{}-{}-{}", version, os, arch))
                .join("bin")
                .join("node")
        }

        Os::Windows => {
            // Extract the zip file
            let file = File::open(archive_path).map_err(|err| Error::Io {
                err,
                path: archive_path.to_path_buf(),
                action: "opening node archive file at".to_string(),
            })?;

            let mut archive = zip::ZipArchive::new(file).map_err(|err| Error::Io {
                err: err.into(),
                path: archive_path.into(),
                action: "extracting node archive file from".to_string(),
            })?;

            archive.extract(extract_dir).map_err(|err| Error::Io {
                err: err.into(),
                path: archive_path.into(),
                action: "extracting node archive file from".to_string(),
            })?;

            extract_dir
                .join(format!("node-v{}-{}-{}", version, os, arch))
                .join("node.exe")
        }
    };

    Ok(bin_path)
}

/// Download the Node.js archive from the official website, and returns the path to the downloaded archive.
fn download_node_archive(
    download_dir: &Path,
    version: &Version,
    os: &Os,
    arch: &Arch,
) -> Result<PathBuf, Error> {
    let mut url = format!("https://nodejs.org/dist/v{version}/node-v{version}-{os}-{arch}",);

    if *os == Os::Windows {
        // Download a zip file
        url += ".zip";
    } else {
        // Download a tarball
        url += ".tar.gz";
    }

    debug!("Downloading Node.js from: {}", url); // TODO: Better UI

    // Download the file from the URL
    let content = get(&url)
        .map_err(|err| Error::Download {
            err,
            url: url.clone(),
        })?
        .bytes()
        .map_err(|err| Error::Download {
            err,
            url: url.clone(),
        })?;

    let file_name = download_dir
        .join("node")
        .with_extension(if *os == Os::Windows { "zip" } else { "tar.gz" });

    let mut file = File::create(&file_name).map_err(|err| Error::Io {
        err,
        path: file_name.clone(),
        action: "creating node archive file at".to_string(),
    })?;

    // Writing the content to the file
    let mut pos = 0;
    while pos < content.len() {
        let bytes_written = file.write(&content[pos..]).map_err(|err| Error::Io {
            err,
            path: file_name.clone(),
            action: "writing to node archive file at".to_string(),
        })?;
        pos += bytes_written;
    }

    Ok(file_name)
}

/// Download and parse the checksum file for a specific version of node
fn download_checksums(version: &Version) -> Result<Vec<(Checksum, NodeExecutableMeta)>, Error> {
    let checksum_file_url = format!(
        "https://nodejs.org/dist/v{}/SHASUMS256.txt",
        version.to_string()
    );

    let checksum_file = reqwest::blocking::get(&checksum_file_url)
        .map_err(|err| Error::Download {
            err,
            url: checksum_file_url.clone(),
        })?
        .text()
        .map_err(|err| Error::Download {
            err,
            url: checksum_file_url,
        })?;

    let checksums = sumfile_parser::parse_checksum_file(&checksum_file)?;

    Ok(checksums)
}
