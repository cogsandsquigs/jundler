use super::lock::Checksum;
use std::io;
use std::path::PathBuf;

/// Any errors that can occur when interacting with the NodeManager
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An IO error occurred
    #[error("An IO error occurred on while {action} {path}: {err}")]
    Io {
        /// The source of the error
        #[source]
        err: io::Error,

        /// The path that caused the error
        path: PathBuf,

        /// The action that caused the error. Should be insertable into a string of "...while {action} {path}:"
        action: String,
    },

    /// An error serializing/deserializing the lockfile occured
    #[error("An error occurred while serializing/deserializing the lockfile: {0}")]
    LockfileSerialization(#[from] bincode::Error),

    /// An error occured while trying to download a file
    #[error("An error occurred while trying to download a file from {url}: {err}")]
    Download {
        /// The source of the error
        #[source]
        err: reqwest::Error,

        /// The URL that caused the error
        url: String,
    },

    /// There was a mismatch between the expected checksum and the actual checksum
    #[error(
        "Checksum mismatch for file {path}! Expected: {}, Actual: {}",
        hex::encode(expected),
        hex::encode(actual)
    )]
    ChecksumMismatch {
        /// The path to the file
        path: PathBuf,

        /// The expected checksum
        expected: Checksum,

        /// The actual checksum
        actual: Checksum,
    },
}
