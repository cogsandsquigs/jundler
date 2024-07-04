use super::helpers::calculate_checksum;
use super::platforms::{Arch, Os};
use super::Error;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub type Checksum = [u8; 32];

/// The lock file for the node manager
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeManagerLock {
    /// A map of node executables by version, arch, and os
    pub node_executables: Vec<NodeExecutable>,

    /// A path to the lockfile. This is not (de)serialized
    #[serde(skip)]
    pub(super) lockfile_path: PathBuf,
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
    pub fn save(&mut self) -> Result<(), Error> {
        let lockfile_contents = bincode::serialize(&self.node_executables)?;

        fs::write(&self.lockfile_path, lockfile_contents).map_err(|err| Error::Io {
            err,
            path: self.lockfile_path.clone(),
            action: "writing to the node manager lockfile at".into(),
        })?;

        Ok(())
    }

    /// Get an executable with a specific version, arch, and os
    pub fn find(&self, version: &Version, os: Os, arch: Arch) -> Option<NodeExecutable> {
        self.node_executables
            .iter()
            .find(|exec| {
                exec.meta.version == *version && exec.meta.arch == arch && exec.meta.os == os
            })
            .cloned()
    }

    /// Given a node executable, insert it into the lockfile
    pub fn add(&mut self, node_executable: NodeExecutable) {
        self.node_executables.push(node_executable);
    }

    /// Remove a node executable from the lockfile
    pub fn remove(&mut self, node_executable: &NodeExecutable) {
        self.node_executables.retain(|exec| exec != node_executable);
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
    /// Validate that the checksum of the file matches it's stored checksum.
    pub fn validate_checksum(&self) -> Result<bool, Error> {
        Ok(self.checksum == calculate_checksum(&self.path)?)
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
