#![cfg(test)]

use super::*;
use assert_fs::{NamedTempFile, TempDir};
use hex::FromHex;
use lock::{NodeExecutable, NodeExecutableMeta};
use sumfile_parser::parse_checksum_file;

/// Test that we can create a new NodeManager
#[test]
fn create_node_manager() {
    let tmp_dir = TempDir::new().unwrap();
    let tmp_path = tmp_dir.path().to_path_buf();

    let mut node_manager = NodeManager::new(tmp_path.clone()).unwrap();

    assert_eq!(node_manager.node_cache_dir, tmp_path);

    let expected_lockfile = NodeManagerLock::new(Vec::new(), tmp_path.join("jundler.lockb"));

    assert_eq!(node_manager.lockfile, expected_lockfile);

    // Check the contents of the file is equal to `expected_lockfile` serialized
    node_manager.lockfile.save().unwrap();

    let lockfile_contents = std::fs::read(tmp_path.join("jundler.lockb")).unwrap();
    let expected_lockfile_contents = bincode::serialize(&expected_lockfile).unwrap();

    assert_eq!(lockfile_contents, expected_lockfile_contents);
}

/// Test we can download node and calculate checksums
#[test]
fn download_save_unpack_remove_node() {
    let tmp_dir = TempDir::new().unwrap();
    let tmp_path = tmp_dir.path().to_path_buf();

    let mut node_manager = NodeManager::new(tmp_path.clone()).unwrap();

    // Download from https://nodejs.org/dist/v22.3.0/node-v22.3.0-linux-x64.tar.gz
    let target_version = "22.3.0".parse().unwrap();

    let (executable_path, archive_path) = node_manager
        .download(&target_version, Os::Linux, Arch::X64)
        .unwrap();

    // Check that the exe and archive exists
    assert!(executable_path.exists());
    assert!(archive_path.exists());

    // Check that the archive is inside the NodeManager
    let locked_binary = node_manager
        .lockfile
        .find(&target_version, Os::Linux, Arch::X64)
        .unwrap();

    assert_eq!(locked_binary.path, archive_path);
    assert!(locked_binary.validate_checksum().unwrap());

    // Check that when we unpack the binary, it's equal to the downloaded binary
    let unpacked_path = node_manager.unpack_archive(locked_binary).unwrap();

    // Get file contents
    let unpacked_contents = std::fs::read(unpacked_path).unwrap();
    let executable_contents = std::fs::read(&executable_path).unwrap();

    assert_eq!(unpacked_contents, executable_contents);

    // Remove the node binary
    node_manager.remove(&locked_binary.clone()).unwrap();

    // Test the archive doesn't exist
    assert!(!archive_path.exists());
    assert!(node_manager
        .lockfile
        .find(&target_version, Os::Linux, Arch::X64)
        .is_none());
}

/// Test we can clean the cache
#[test]
fn clear_cache() {
    let tmp_dir = TempDir::new().unwrap();
    let tmp_path = tmp_dir.path().to_path_buf();

    let mut node_manager = NodeManager::new(tmp_path.clone()).unwrap();

    // Download from https://nodejs.org/dist/v22.3.0/node-v22.3.0-linux-x64.tar.gz
    let target_version = "22.3.0".parse().unwrap();

    let (executable_path, archive_path) = node_manager
        .download(&target_version, Os::Linux, Arch::X64)
        .unwrap();

    // Check that the exe and archive exists
    assert!(executable_path.exists());
    assert!(archive_path.exists());

    // Clear the cache
    node_manager.clean_cache().unwrap();

    // Check that the archive doesn't exist, but the lockfile does
    assert!(!archive_path.exists());
    assert!(node_manager.lockfile.lockfile_path.exists());
}

/// Test that we can create, save and load a lockfile
#[test]
fn create_save_load_lockfile() {
    // Get random tempdir for lockfile
    let lockfile_path = NamedTempFile::new("jundler.lockb").unwrap();

    let mut lockfile = NodeManagerLock::new(
        vec![
            NodeExecutable {
                meta: NodeExecutableMeta {
                    version: "22.3.0".parse().unwrap(),
                    arch: Arch::Arm64,
                    os: Os::MacOS,
                },
                checksum: <[u8; 32]>::from_hex(
                    "b6723f1e4972af1ca8a7ef9ec63305ee8cd4380fce3071e0e1630dfe055d77e3",
                )
                .unwrap(),
                path: PathBuf::from("test"),
            },
            NodeExecutable {
                meta: NodeExecutableMeta {
                    version: "22.3.0".parse().unwrap(),
                    arch: Arch::X86,
                    os: Os::Windows,
                },
                checksum: <[u8; 32]>::from_hex(
                    "a56e1446e45adbfc716023c8e903eef829e84e5ac8aae3a65b455213bef9cdb1",
                )
                .unwrap(),
                path: PathBuf::from("test"),
            },
        ],
        lockfile_path.path().to_path_buf(),
    );

    // Save the lockfile
    lockfile.save().unwrap();

    // Load the lockfile
    let loaded_lockfile = NodeManagerLock::load(lockfile_path.path().to_path_buf()).unwrap();

    assert_eq!(lockfile, loaded_lockfile);
}

/// Test that we can parse a sample sumfile
#[test]
fn parse_sumfile() {
    let parsed = parse_checksum_file(TEST_SUMFILE_V22).unwrap();

    assert_eq!(parsed.len(), 7);

    // b6723f1e4972af1ca8a7ef9ec63305ee8cd4380fce3071e0e1630dfe055d77e3  node-v22.3.0-darwin-arm64.tar.gz
    assert_eq!(
        parsed[0],
        (
            <[u8; 32]>::from_hex(
                "b6723f1e4972af1ca8a7ef9ec63305ee8cd4380fce3071e0e1630dfe055d77e3"
            )
            .unwrap(),
            NodeExecutableMeta {
                version: "22.3.0".parse().unwrap(),
                arch: Arch::Arm64,
                os: Os::MacOS,
            }
        )
    );

    // 7fe139f9d769d65c27212f8be8f858e1ee522edf3a66eed1d08d42ba102995f8  node-v22.3.0-darwin-x64.tar.gz
    assert_eq!(
        parsed[1],
        (
            <[u8; 32]>::from_hex(
                "7fe139f9d769d65c27212f8be8f858e1ee522edf3a66eed1d08d42ba102995f8"
            )
            .unwrap(),
            NodeExecutableMeta {
                version: "22.3.0".parse().unwrap(),
                arch: Arch::X64,
                os: Os::MacOS,
            }
        )
    );

    // 0e25b9a4bc78080de826a90dff82743bec6d9c5085186e75521dc195c8be9ce3  node-v22.3.0-linux-arm64.tar.gz
    assert_eq!(
        parsed[2],
        (
            <[u8; 32]>::from_hex(
                "0e25b9a4bc78080de826a90dff82743bec6d9c5085186e75521dc195c8be9ce3"
            )
            .unwrap(),
            NodeExecutableMeta {
                version: "22.3.0".parse().unwrap(),
                arch: Arch::Arm64,
                os: Os::Linux,
            }
        )
    );

    // a6d4fbf4306a883b8e1d235a8a890be84b9d95d2d39b929520bed64da41ce540  node-v22.3.0-linux-x64.tar.gz
    assert_eq!(
        parsed[3],
        (
            <[u8; 32]>::from_hex(
                "a6d4fbf4306a883b8e1d235a8a890be84b9d95d2d39b929520bed64da41ce540"
            )
            .unwrap(),
            NodeExecutableMeta {
                version: "22.3.0".parse().unwrap(),
                arch: Arch::X64,
                os: Os::Linux,
            }
        )
    );

    // 727426f9a97238d2dc269fb00bbe50c77629f76adb99a19d68abc41e8cdb4bc5  node-v22.3.0-win-arm64.zip
    assert_eq!(
        parsed[4],
        (
            <[u8; 32]>::from_hex(
                "727426f9a97238d2dc269fb00bbe50c77629f76adb99a19d68abc41e8cdb4bc5"
            )
            .unwrap(),
            NodeExecutableMeta {
                version: "22.3.0".parse().unwrap(),
                arch: Arch::Arm64,
                os: Os::Windows,
            }
        )
    );

    // 3dadc19ba6b36c6fb93aeda08247107fdb2ed55c24831304566d32de6b6080d7  node-v22.3.0-win-x64.zip
    assert_eq!(
        parsed[5],
        (
            <[u8; 32]>::from_hex(
                "3dadc19ba6b36c6fb93aeda08247107fdb2ed55c24831304566d32de6b6080d7"
            )
            .unwrap(),
            NodeExecutableMeta {
                version: "22.3.0".parse().unwrap(),
                arch: Arch::X64,
                os: Os::Windows,
            }
        )
    );

    // a56e1446e45adbfc716023c8e903eef829e84e5ac8aae3a65b455213bef9cdb1  node-v22.3.0-win-x86.zip
    assert_eq!(
        parsed[6],
        (
            <[u8; 32]>::from_hex(
                "a56e1446e45adbfc716023c8e903eef829e84e5ac8aae3a65b455213bef9cdb1"
            )
            .unwrap(),
            NodeExecutableMeta {
                version: "22.3.0".parse().unwrap(),
                arch: Arch::X86,
                os: Os::Windows,
            }
        )
    );
}

const TEST_SUMFILE_V22: &str = r#"8c349a9164f25d8a1de886a47db045b50ae11aba4c4c1e1a4d1ac34a1e5d20e3  node-v22.3.0-aix-ppc64.tar.gz
69ee53b3262ae727453d97f8e0fb3ba51363065351fcf2a389d0bdab688c021c  node-v22.3.0-arm64.msi
b6723f1e4972af1ca8a7ef9ec63305ee8cd4380fce3071e0e1630dfe055d77e3  node-v22.3.0-darwin-arm64.tar.gz
b63eac38d610ffcd9ae35340f3a28d16f566d44441845d1f73dd3e5294d0dcae  node-v22.3.0-darwin-arm64.tar.xz
7fe139f9d769d65c27212f8be8f858e1ee522edf3a66eed1d08d42ba102995f8  node-v22.3.0-darwin-x64.tar.gz
a633700fae61e3f078be40561df241ead763d30cfdc463b623e8b895c36bb481  node-v22.3.0-darwin-x64.tar.xz
d2460c13bb1b723d0773b3c18162ec8d3bc15c18c25643520c1f03d80e014999  node-v22.3.0-headers.tar.gz
6f62ffb3f189a4797471f0334888e2471ee7352e1c5d3bbfc6feaf2175a990fc  node-v22.3.0-headers.tar.xz
0e25b9a4bc78080de826a90dff82743bec6d9c5085186e75521dc195c8be9ce3  node-v22.3.0-linux-arm64.tar.gz
c0324bbcfd5627bdcdc18830e563af1742c2173e86297a502a86db54c15bba70  node-v22.3.0-linux-arm64.tar.xz
46b640d23708f899689059cc2a8431842c2e3ad50a9144828ddabea5e1a7c3ae  node-v22.3.0-linux-armv7l.tar.gz
973731137ea1ab9415115b9ec447d34628c5aa45c33115df1a2dfb20e7f79b5f  node-v22.3.0-linux-armv7l.tar.xz
a01c2263a01efa7c6efa3607d202487127e268d73b68b6cce9c44a481412ece0  node-v22.3.0-linux-ppc64le.tar.gz
50c91e0b1ba7472e3ff609ecd503810308c990a1fd1ea1a721f9029c01c9d2a7  node-v22.3.0-linux-ppc64le.tar.xz
3aa6a22f525a6f8ddb0fd2ce3646414c316a41cab6bdaac812276196607bc187  node-v22.3.0-linux-s390x.tar.gz
decbeb778aa4e490ba4b60a7d13ef92f6db4647ccd2d452d7e52067b5503d4a9  node-v22.3.0-linux-s390x.tar.xz
a6d4fbf4306a883b8e1d235a8a890be84b9d95d2d39b929520bed64da41ce540  node-v22.3.0-linux-x64.tar.gz
33429139d4c4416439bf023b2eb2dc257da188fd793b64f21c8c03a0f04a5840  node-v22.3.0-linux-x64.tar.xz
a76b8e529e5dc162f9739aa25d380b416e1bacc29cf36f2b178db24764ba359d  node-v22.3.0.pkg
6326484853093ab6b8f361a267445f4a5bff469042cda11a3585497b13136b55  node-v22.3.0.tar.gz
bfb85bd1dca517761f9046d61600f830d19935d6d6c36eded01578a19326104c  node-v22.3.0.tar.xz
57a44a7c956581e2939c8c040cb49f72dfa148c4e97178e54be67e78cc45ca69  node-v22.3.0-win-arm64.7z
727426f9a97238d2dc269fb00bbe50c77629f76adb99a19d68abc41e8cdb4bc5  node-v22.3.0-win-arm64.zip
5eead5f9946b5381ffb36430970a2e3d0bcf90383a9432ea76e93d0efdc70691  node-v22.3.0-win-x64.7z
3dadc19ba6b36c6fb93aeda08247107fdb2ed55c24831304566d32de6b6080d7  node-v22.3.0-win-x64.zip
e8e34fbef56216f8d58499215d3c5220ce429c455ee2bfa97b29bb0e9ba57e1b  node-v22.3.0-win-x86.7z
a56e1446e45adbfc716023c8e903eef829e84e5ac8aae3a65b455213bef9cdb1  node-v22.3.0-win-x86.zip
da5b1cbc773371fd11415a893ce229f51052e9aa9b656ddcbd79730ce4b93a7b  node-v22.3.0-x64.msi
ae86fec0828744ac9c9a9b0186cd984e64d45602b267deac6fc140eb1c13262f  node-v22.3.0-x86.msi
17608e0e2c587fca141bfc43ce9299db192b8506def389b8e30a9935e6fc6f83  win-arm64/node.exe
30e63a6726cda6539eeb37c311adf915bccd5c1462723b97a6c07ac91e8ae728  win-arm64/node.lib
8e71a3f8a27a14f0c0f5198aa0e34d9c58d0bf39cd3b0e5e89c3079884c427b3  win-arm64/node_pdb.7z
483e6e8e418fac0c311b2ca6ca5414dbbf61c8da1c1ced7a7736fc9c8a44ca94  win-arm64/node_pdb.zip
b3e0d6bf8224d43d5c6e756c8ebaffe1daef0d5ed0eeba40eef0ca62f1c4232a  win-x64/node.exe
c4d08d45267da3625a30730bf5c8e41518f25d9809179feb267f1b393f5c5f05  win-x64/node.lib
fdc88d7ef4ee2bee3bb94947786ea425a30c2d5fb26b0ad25cb33cad165c8a5c  win-x64/node_pdb.7z
689c6831018340256aa33e2cd0a5da8168c835e5d3070dd0688803c0cd1157cd  win-x64/node_pdb.zip
195a4cc5eb1d9235043a34f423a732d54f73d9b2b7404c86ef10ff1c17dff6d6  win-x86/node.exe
fc3bf3c1e561da1e1c152be9aa5ed1bce8d263a5124841a4ba41ebc37c727f3e  win-x86/node.lib
b471579503255732d862c8eaa9a3dff77cf2ef8e7c80ccb484b5e46f83cd6438  win-x86/node_pdb.7z
fadd1b6e3071a8d095913aa959be1f1a701621cc9cc7f6a685bcf3c74b884c84  win-x86/node_pdb.zip"#;
