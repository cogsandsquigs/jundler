use std::path::PathBuf;

/// Any errors that can occur when interacting with the NodeManager
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error occurred while parsing the checksum file
    #[error("An error occurred while parsing the checksum file!")]
    UnparseableChecksumFile,

    /// An IO error occurred
    #[error("An IO error occurred on while {action} {path}: {err}")]
    Io {
        /// The source of the error
        #[source]
        err: std::io::Error,

        /// The path that caused the error
        path: PathBuf,

        /// The action that caused the error. Should be insertable into a string of "...while {action} {path}:"
        action: String,
    },

    /// An error serializing/deserializing the lockfile occured
    #[error("An error occurred while serializing/deserializing the lockfile: {0}")]
    LockfileSerialization(#[from] bincode::Error),
}
