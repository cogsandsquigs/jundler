use super::Error;
use crate::builder::helpers::calculate_checksum;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub type Checksum = [u8; 32];

/// The lock file for the node manager
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ESBuildLock {
    /// A path to the lockfile. This is not (de)serialized
    #[serde(skip)]
    pub(super) lockfile_path: PathBuf,

    /// The executable
    pub(super) executable: Option<ESBuildExecutable>,
}

impl ESBuildLock {
    /// Create a new node manager lockfile
    pub fn new(lockfile_path: PathBuf) -> Self {
        Self {
            lockfile_path,
            executable: None,
        }
    }

    /// Load from a lockfile
    pub fn load(lockfile_path: PathBuf) -> Result<Self, Error> {
        let lockfile_contents = fs::read(&lockfile_path).map_err(|err| Error::Io {
            err,
            path: lockfile_path.clone(),
            action: "reading the esbuild lockfile at".into(),
        })?;

        let mut lock: ESBuildLock = bincode::deserialize(&lockfile_contents)?;

        lock.lockfile_path = lockfile_path;

        Ok(lock)
    }

    /// Save the lockfile
    pub fn save(&mut self) -> Result<(), Error> {
        let lockfile_contents = bincode::serialize(self)?;

        fs::write(&self.lockfile_path, lockfile_contents).map_err(|err| Error::Io {
            err,
            path: self.lockfile_path.clone(),
            action: "writing to the esbuild lockfile at".into(),
        })?;

        Ok(())
    }

    /// Get the executable
    pub fn get(&self) -> Option<ESBuildExecutable> {
        self.executable.clone()
    }

    /// Given a node executable, insert it into the lockfile
    pub fn add(&mut self, esbuild_executable: ESBuildExecutable) {
        self.executable = Some(esbuild_executable);
    }

    /// Remove a node executable from the lockfile
    pub fn remove(&mut self, node_executable: &ESBuildExecutable) {
        if self.executable.as_ref() == Some(node_executable) {
            self.executable = None;
        }
    }
}

/// A singular esbuild executable with a specific version
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ESBuildExecutable {
    /// The checksum of the executable
    pub checksum: Checksum,

    /// The version of the executable
    pub version: Version,

    /// The path to the node executable
    pub path: PathBuf,
}

impl ESBuildExecutable {
    /// Validate the checksum of the executable
    pub fn validate_checksum(&self) -> Result<bool, Error> {
        let actual = calculate_checksum(&self.path).map_err(|err| Error::Io {
            err,
            path: self.path.clone(),
            action: "calculating checksum of node executable at".into(),
        })?;

        Ok(actual == self.checksum)
    }
}
