use assert_fs::TempDir;

// Function to generate lit runner for a fixture directory. Why do this instead of just
// searching for all shell files in the directory? Because we want to be able to only test some
// of the files in the directory, and we want to be able to pass in constants to the tests.
fn lit_runner(fixture_dir: &str) {
    lit::run::tests(lit::event_handler::Default::default(), |config| {
        config.add_search_path("tests/fixtures/".to_owned() + fixture_dir);
        config.add_extension("sh");

        config.constants.insert(
            "tmpdir".to_owned(),
            TempDir::new()
                .unwrap()
                .path()
                .to_owned()
                .to_str()
                .unwrap()
                .to_string(),
        );
    })
    .expect("Lit tests failed");
}

#[test]
fn test_simple() {
    lit_runner("simple");
}

#[test]
fn test_simple_bundle() {
    lit_runner("simple");
}
