use assert_fs::{fixture::PathCopy, TempDir};
use std::env;
use std::path::PathBuf;
use std::process::Command;

// Function to generate lit runner for a fixture directory. Why do this instead of just
// searching for all shell files in the directory? Because we want to be able to only test some
// of the files in the directory, and we want to be able to pass in constants to the tests.
fn test_runner(
    test_name: &str,
    jundler_args: &[&str],
    expected_stdout: &str,
    expected_stderr: &str,
) {
    // Create tmp dir for test
    let tmp_dir = TempDir::new().unwrap();

    // Copy the fixture directory to the tmp dir
    let fixture_path = PathBuf::from("tests/fixtures").join(test_name);
    tmp_dir.copy_from(fixture_path, &["**/*"]).unwrap();

    // Set the RUST_LOG environment variable to debug so we can see the output of the build process.
    env::set_var("RUST_LOG", "debug");

    // Run the tests
    let result = Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg(tmp_dir.path())
        .args(jundler_args)
        .output()
        .unwrap();

    // Print outputs for debugging
    println!("JUNDLER OUTPUT");
    println!("----------------------------------------------------");
    println!("status: {}", result.status);
    println!("stdout:\n{}", String::from_utf8_lossy(&result.stdout));
    println!("stderr:\n{}", String::from_utf8_lossy(&result.stderr));

    assert!(result.status.success());

    // Run the generated file
    let result = Command::new(tmp_dir.path().join(test_name))
        .output()
        .unwrap();

    // Print outputs for debugging
    println!("GENERATED BINARY ({}) OUTPUT", test_name);
    println!("----------------------------------------------------");
    println!("status: {}", result.status);
    println!("stdout:\n{}", String::from_utf8_lossy(&result.stdout));
    println!("stderr:\n{}", String::from_utf8_lossy(&result.stderr));

    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains(expected_stdout));
    assert!(String::from_utf8_lossy(&result.stderr).contains(expected_stderr));
}

#[test]
fn test_simple() {
    test_runner("simple", &[], "Hello, world!", "");
}

#[test]
fn test_simple_bundle() {
    test_runner(
        "simple-bundle",
        &["--bundle"],
        "Hello, world!\n1 + 2 = 3",
        "",
    );
}
