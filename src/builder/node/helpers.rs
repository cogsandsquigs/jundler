use super::{lock::Checksum, Error};
use sha2::{Digest, Sha256};
use std::{fs::File, io, path::Path};

/// Calculate the SHA256 checksum of a file. Expects that the file is readable.
pub fn calculate_checksum(path: &Path) -> Result<Checksum, Error> {
    // Prepare the hasher
    let mut hasher = Sha256::new();

    let mut file = File::open(path).map_err(|err| Error::Io {
        err,
        path: path.into(),
        action: "opening a file at".into(),
    })?;

    io::copy(&mut file, &mut hasher).map_err(|err| Error::Io {
        err,
        path: path.into(), // TODO: Get the path from the file
        action: "calculating the SHA256 checksum of a file at".into(),
    })?;

    // Output the hash and convert it into a 32-byte array
    Ok(hasher.finalize().into())
}
