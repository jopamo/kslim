use super::common::*;

#[test]
fn integration_tests_are_split_by_feature_area() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert!(
        !root.join("tests/integration.rs").exists(),
        "tests/integration.rs should be split into focused integration suites"
    );

    for suite in [
        "tests/generate.rs",
        "tests/features.rs",
        "tests/publish.rs",
        "tests/reducer.rs",
        "tests/manifest.rs",
        "tests/failure.rs",
        "tests/transactions.rs",
    ] {
        let path = root.join(suite);
        assert!(
            path.is_file(),
            "{suite} should exist after integration split"
        );
        let source = production_source(&path);
        assert!(
            source.contains("mod common;") && source.contains("use common::*;"),
            "{suite} should use shared integration helpers from tests/common"
        );
        assert!(
            source.contains("#[test]"),
            "{suite} should contain focused integration tests"
        );
    }

    let common = production_source(&root.join("tests/common/mod.rs"));
    for helper in [
        "pub fn create_fake_upstream(",
        "pub fn create_kslim_project(",
        "pub fn kslim_in(",
    ] {
        assert!(
            common.contains(helper),
            "tests/common/mod.rs should own shared integration helper {helper}"
        );
    }
}
