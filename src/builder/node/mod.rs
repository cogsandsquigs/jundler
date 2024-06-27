mod errors;
mod lock;
mod platforms;
mod sumfile_parser;
mod tests;

// Re-export error types
pub use errors::Error;
use log::warn;
pub use platforms::{get_host_arch, get_host_os, Arch, Os};

use lock::NodeManagerLock;
use std::path::PathBuf;

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
}
