#![cfg(test)]

use serde_json::Value;

use super::*;

/// Test that we were able to get a new a `Builder` instance from a project.
#[test]
fn test_new_builder() {
    let result = Builder::new(
        PathBuf::from("tests/fixtures/simple"),
        Version::parse("22.3.0").unwrap(),
        Os::Linux,
        Arch::X64,
        false,
        None,
    );

    // assert!(result.is_ok());

    let builder = result.unwrap();

    assert_eq!(builder.package_config.name, "simple");
    assert_eq!(builder.sea_config.main, "index.js");
    assert_eq!(
        builder.sea_config.output,
        "the-random-name-for-the-sea-prep-blob.blob"
    );

    assert_eq!(builder.sea_config.other.len(), 1);
    assert_eq!(
        builder.sea_config.other["disableExperimentalSEAWarning"],
        Value::Bool(true)
    );
}
