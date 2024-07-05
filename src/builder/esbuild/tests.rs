#![cfg(test)]

use super::*;
use assert_fs::{NamedTempFile, TempDir};
use lock::{ESBuildExecutable, ESBuildLock};

/// Test that we can create a new NodeManager
#[test]
fn create_esbuild() {
    let tmp_dir = TempDir::new().unwrap();
    let tmp_path = tmp_dir.path().to_path_buf();

    let mut esbuild_lock = ESBuild::new(tmp_path.clone()).unwrap();

    assert_eq!(esbuild_lock.cache_dir, tmp_path);

    let expected_lockfile = ESBuildLock::new(tmp_path.join("jundler.lockb"));

    assert_eq!(esbuild_lock.lockfile, expected_lockfile);

    // Check the contents of the file is equal to `expected_lockfile` serialized
    esbuild_lock.lockfile.save().unwrap();

    let lockfile_contents = std::fs::read(tmp_path.join("jundler.lockb")).unwrap();
    let expected_lockfile_contents = bincode::serialize(&expected_lockfile).unwrap();

    assert_eq!(lockfile_contents, expected_lockfile_contents);
}

/// Test we can download node and calculate checksums
#[test]
fn download_save_unpack_esbuild() {
    let tmp_dir = TempDir::new().unwrap();
    let tmp_path = tmp_dir.path().to_path_buf();

    let mut esbuild = ESBuild::new(tmp_path.clone()).unwrap();

    let executable_path = esbuild.download(&ESBUILD_VERSION).unwrap();

    // Check that the exe and archive exists
    assert!(executable_path.exists());

    let archive_path = tmp_path.join(format!("esbuild-v{}.zst", ESBUILD_VERSION));
    assert!(archive_path.exists());

    // Check that the archive is inside the NodeManager
    let locked_binary = esbuild.lockfile.get().unwrap();

    assert_eq!(locked_binary.path, archive_path);
    assert!(locked_binary.validate_checksum().unwrap());

    // Check that when we unpack the binary, it's equal to the downloaded binary
    let unpacked_path = esbuild.unpack_archive(&locked_binary).unwrap();

    // Get file contents
    let unpacked_contents = std::fs::read(unpacked_path).unwrap();
    let executable_contents = std::fs::read(&executable_path).unwrap();

    assert_eq!(unpacked_contents, executable_contents);

    // Remove the node binary
    esbuild.remove(&locked_binary).unwrap();

    // Test the archive doesn't exist
    assert!(!archive_path.exists());
    assert!(esbuild.lockfile.get().is_none());
}

/// Test we can clean the cache
#[test]
fn clear_cache() {
    let tmp_dir = TempDir::new().unwrap();
    let tmp_path = tmp_dir.path().to_path_buf();

    let mut esbuild = ESBuild::new(tmp_path.clone()).unwrap();

    let executable_path = esbuild.download(&ESBUILD_VERSION).unwrap();

    // Check that the exe and archive exists
    assert!(executable_path.exists());

    let archive_path = tmp_path.join(format!("esbuild-v{}.zst", ESBUILD_VERSION));
    assert!(archive_path.exists());

    // Clear the cache
    esbuild.clean_cache().unwrap();

    // Check that the archive doesn't exist, but the lockfile does
    assert!(!archive_path.exists());
    assert!(esbuild.lockfile.lockfile_path.exists());
}

/// Test that we can create, save and load a lockfile
#[test]
fn create_save_load_lockfile() {
    // Get random tempdir for lockfile
    let lockfile_path = NamedTempFile::new("jundler.lockb").unwrap();

    let mut lockfile = ESBuildLock {
        lockfile_path: lockfile_path.path().to_path_buf(),
        executable: Some(ESBuildExecutable {
            checksum: [0; 32],
            path: PathBuf::from("/path/to/esbuild"),
            version: "22.3.0".parse().unwrap(),
        }),
    };

    // Save the lockfile
    lockfile.save().unwrap();

    // Load the lockfile
    let loaded_lockfile = ESBuildLock::load(lockfile_path.path().to_path_buf()).unwrap();

    assert_eq!(lockfile, loaded_lockfile);
}
