#![cfg(test)]

use super::*;

/// Test that we were able to get a new a `Builder` instance from a project.
#[test]
fn new_builder() {
    let result = Builder::new(TempDir::new("test").unwrap().into_path());

    assert!(result.is_ok());

    // let builder = result.unwrap();

    // assert_eq!(builder.package_config.name, "simple");
    // assert_eq!(builder.sea_config.main, "index.js");
    // assert_eq!(
    //     builder.sea_config.output,
    //     "the-random-name-for-the-sea-prep-blob.blob"
    // );
    // // assert!(builder.builder_dir.exists());

    // assert_eq!(builder.sea_config.other.len(), 1);
    // assert_eq!(
    //     builder.sea_config.other["disableExperimentalSEAWarning"],
    //     Value::Bool(true)
    // );
}
