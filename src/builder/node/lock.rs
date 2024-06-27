use super::Error;
use crate::builder::platforms::{Arch, Os};
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::{fs, io};

pub type Checksum = [u8; 32];

/// The lock file for the node manager
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeManagerLock {
    /// A map of node executables by version, arch, and os
    pub node_executables: Vec<NodeExecutable>,

    /// A path to the lockfile. This is not (de)serialized
    #[serde(skip)]
    lockfile_path: PathBuf,
}

impl NodeManagerLock {
    /// Create a new node manager lockfile
    pub fn new(node_executables: Vec<NodeExecutable>, lockfile_path: PathBuf) -> Self {
        Self {
            node_executables,
            lockfile_path,
        }
    }

    /// Load from a lockfile
    pub fn load(lockfile_path: PathBuf) -> Result<Self, Error> {
        let lockfile_contents = fs::read(&lockfile_path).map_err(|err| Error::Io {
            err,
            path: lockfile_path.clone(),
            action: "reading the node manager lockfile at".into(),
        })?;

        let node_executables = bincode::deserialize(&lockfile_contents)?;

        Ok(Self::new(node_executables, lockfile_path))
    }

    /// Save the lockfile
    pub fn save(&self) -> Result<(), Error> {
        let lockfile_contents = bincode::serialize(&self.node_executables)?;

        fs::write(&self.lockfile_path, lockfile_contents).map_err(|err| Error::Io {
            err,
            path: self.lockfile_path.clone(),
            action: "writing to the node manager lockfile at".into(),
        })?;

        Ok(())
    }

    /// Get an executable with a specific version, arch, and os
    pub fn find(&self, version: Version, arch: Arch, os: Os) -> Option<&NodeExecutable> {
        self.node_executables.iter().find(|exec| {
            exec.meta.version == version && exec.meta.arch == arch && exec.meta.os == os
        })
    }

    /// Given a node executable, insert it into the lockfile
    pub fn insert(&mut self, node_executable: NodeExecutable) {
        self.node_executables.push(node_executable);
    }
}

/// A singular node executable with a specific arch, os, and version
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeExecutable {
    /// Metadata about the node executable
    pub meta: NodeExecutableMeta,

    /// The checksum of the node executable
    pub checksum: Checksum,

    /// The path to the node executable
    pub path: PathBuf,
}

/// A (compressed) node executable that can be uncompressed and used/ran
impl NodeExecutable {
    /// Create a new node executable
    pub fn new(version: Version, arch: Arch, os: Os, path: PathBuf) -> Self {
        let checksum = calculate_checksum(&path);

        Self {
            meta: NodeExecutableMeta { version, arch, os },
            checksum,
            path,
        }
    }

    /// Validate that the checksum of the file matches against any checksum
    pub fn validate_checksum_against(&self, checksum: &Checksum) -> bool {
        self.checksum == *checksum
    }
}

/// Information for a node executable
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeExecutableMeta {
    /// The version of the node executable
    pub version: Version,

    /// The architecture of the node executable
    pub arch: Arch,

    /// The operating system of the node executable
    pub os: Os,
}

/// Calculate the SHA256 checksum of a file
fn calculate_checksum(path: &PathBuf) -> Checksum {
    // Open the file
    let mut file = fs::File::open(path).unwrap();

    // Prepare the hasher
    let mut hasher = Sha256::new();

    // Copy the file into the hasher, and hash it
    io::copy(&mut file, &mut hasher).unwrap();

    // Output the hash and convert it into a 32-byte array
    hasher.finalize().into()
}
