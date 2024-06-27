mod errors;
mod lock;
mod sumfile_parser;
mod tests;

// Re-export error types
pub use errors::Error;

use super::platforms::{Arch, Os};
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
