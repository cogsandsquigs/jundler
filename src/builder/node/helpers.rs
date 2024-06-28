use super::lock::{Checksum, NodeExecutableMeta};
pub use super::platforms::{Arch, Os};
use super::{sumfile_parser, Error};
use flate2::read::GzDecoder;
use log::debug;
use reqwest::blocking::get;
use semver::Version;
use sha2::{Digest, Sha256};
use std::{fs::File, io, path::Path};
use std::{
    fs::{self},
    io::{Read, Write},
    path::PathBuf,
};
use tar::Archive;
use zstd::Encoder;

/// On Unix-based systems, make the binary executable.
pub fn make_executable(binary_path: &Path) -> Result<(), Error> {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = binary_path
        .metadata()
        .map_err(|err| Error::Io {
            err,
            path: binary_path.to_path_buf(),
            action: "getting metadata for node binary at".to_string(),
        })?
        .permissions();

    perms.set_mode(0o755);

    fs::set_permissions(binary_path, perms).map_err(|err| Error::Io {
        err,
        path: binary_path.to_path_buf(),
        action: "setting permissions for node binary at".to_string(),
    })?;

    Ok(())
}

/// Rearchive *just* the binary and copy the node binary into the cache directory. Returns the path to the copied binary.
pub fn repack_node_binary(
    node_executable_path: &Path,
    version: &Version,
    os: Os,
    arch: Arch,
    cache_dir: &Path,
) -> Result<PathBuf, Error> {
    let archive_path = cache_dir.join(format!("node-v{}-{}-{}.zst", version, os, arch));

    let archive = File::create(&archive_path).map_err(|err| Error::Io {
        err,
        path: archive_path.clone(),
        action: "creating archive file at".to_string(),
    })?;

    let mut node_executable = File::open(node_executable_path).map_err(|err| Error::Io {
        err,
        path: node_executable_path.to_path_buf(),
        action: "opening node executable file at".to_string(),
    })?;

    let mut zstd_encoder = Encoder::new(archive, 0).map_err(|err| Error::Io {
        err,
        path: archive_path.clone(),
        action: "creating zstd encoder for archive file at".to_string(),
    })?;

    // Encode!
    let mut buf: Vec<u8> = vec![];

    node_executable
        .read_to_end(&mut buf)
        .map_err(|err| Error::Io {
            err,
            path: node_executable_path.to_path_buf(),
            action: "reading from node executable file at".to_string(),
        })?;

    zstd_encoder.write_all(&buf).map_err(|err| Error::Io {
        err,
        path: archive_path.clone(),
        action: "writing to archive file at".to_string(),
    })?;

    zstd_encoder.finish().map_err(|err| Error::Io {
        err,
        path: archive_path.clone(),
        action: "finishing zstd encoder for archive file at".to_string(),
    })?;

    Ok(archive_path)
}

/// Extract the Node.js archive, and returns the path to the extracted binary. `extract_dir` is the directory where the archive will
/// be extracted to.
pub fn unpack_downloaded_node_archive(
    extract_dir: &Path,
    archive_path: &Path,
    version: &Version,
    os: Os,
    arch: Arch,
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
pub fn download_node_archive(
    download_dir: &Path,
    version: &Version,
    os: Os,
    arch: Arch,
) -> Result<PathBuf, Error> {
    let mut url = format!("https://nodejs.org/dist/v{version}/node-v{version}-{os}-{arch}",);

    if os == Os::Windows {
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
        .with_extension(if os == Os::Windows { "zip" } else { "tar.gz" });

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
pub fn download_checksums(version: &Version) -> Result<Vec<(Checksum, NodeExecutableMeta)>, Error> {
    let checksum_file_url = format!("https://nodejs.org/dist/v{}/SHASUMS256.txt", version);

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
