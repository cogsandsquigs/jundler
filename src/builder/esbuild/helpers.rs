use crate::builder::platforms::{get_host_arch, get_host_os, Os};

use super::Error;
use flate2::read::GzDecoder;
use log::debug;
use reqwest::blocking::get;
use semver::Version;
use std::{fs::File, path::Path};
use std::{
    io::{Read, Write},
    path::PathBuf,
};
use tar::Archive;
use zstd::Encoder;

/// Rearchive *just* the binary and copy the esbuild binary into the cache directory. Returns the path to the copied binary.
pub fn repack_esbuild_binary(
    esbuild_executable_path: &Path,
    version: &Version,
    cache_dir: &Path,
) -> Result<PathBuf, Error> {
    let archive_path = cache_dir.join(format!("esbuild-v{}.zst", version));

    let archive = File::create(&archive_path).map_err(|err| Error::Io {
        err,
        path: archive_path.clone(),
        action: "creating archive file at".to_string(),
    })?;

    let mut esbuild_executable = File::open(esbuild_executable_path).map_err(|err| Error::Io {
        err,
        path: esbuild_executable_path.to_path_buf(),
        action: "opening esbuild executable file at".to_string(),
    })?;

    let mut zstd_encoder = Encoder::new(archive, 0).map_err(|err| Error::Io {
        err,
        path: archive_path.clone(),
        action: "creating zstd encoder for archive file at".to_string(),
    })?;

    // Encode!
    let mut buf: Vec<u8> = vec![];

    esbuild_executable
        .read_to_end(&mut buf)
        .map_err(|err| Error::Io {
            err,
            path: esbuild_executable_path.to_path_buf(),
            action: "reading from esbuild executable file at".to_string(),
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

/// Extract the esbuild.js archive, and returns the path to the extracted binary. `extract_dir` is the directory where the archive will
/// be extracted to.
pub fn unpack_downloaded_esbuild_archive(
    extract_dir: &Path,
    archive_path: &Path,
) -> Result<PathBuf, Error> {
    // Extract the archive to `{build-dir}/esbuild-v{version}-{os}-{arch}`

    // Extract the tarball
    let tar_gz = File::open(archive_path).map_err(|err| Error::Io {
        err,
        path: archive_path.to_path_buf(),
        action: "opening esbuild archive file at".to_string(),
    })?;

    let tar = GzDecoder::new(tar_gz);

    let mut archive = Archive::new(tar);

    archive
        .unpack(extract_dir)
        .map_err(|err: std::io::Error| Error::Io {
            err,
            path: archive_path.into(),
            action: "extracting esbuild archive file from".to_string(),
        })?;

    let mut bin_path = extract_dir.join("package/bin/esbuild");

    if get_host_os() == Os::Windows {
        bin_path.set_extension("exe");
    }

    Ok(bin_path)
}

/// Download the esbuild.js archive from the official website, and returns the path to the downloaded archive.
pub fn download_esbuild_archive(download_dir: &Path, version: &Version) -> Result<PathBuf, Error> {
    let url = format!(
        "https://registry.npmjs.org/@esbuild/{os}-{arch}/-/{os}-{arch}-{version}.tgz",
        os = get_host_os(),     // TODO: Change
        arch = get_host_arch(), // TODO: Change
        version = version
    );

    debug!("Downloading esbuild.js from: {}", url); // TODO: Better UI

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

    let file_name = download_dir.join("esbuild.tar.gz");

    let mut file = File::create(&file_name).map_err(|err| Error::Io {
        err,
        path: file_name.clone(),
        action: "creating esbuild archive file at".to_string(),
    })?;

    // Writing the content to the file
    let mut pos = 0;
    while pos < content.len() {
        let bytes_written = file.write(&content[pos..]).map_err(|err| Error::Io {
            err,
            path: file_name.clone(),
            action: "writing to esbuild archive file at".to_string(),
        })?;
        pos += bytes_written;
    }

    Ok(file_name)
}
