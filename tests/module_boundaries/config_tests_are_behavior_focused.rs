use super::common::*;

#[test]
fn config_tests_are_behavior_focused() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config_tests = production_source(&root.join("src/config/tests.rs"));
    let defaults = production_source(&root.join("src/config/tests_defaults.rs"));
    let load = production_source(&root.join("src/config/tests_load.rs"));
    let validation = production_source(&root.join("src/config/tests_validation.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "#[path = \"tests_defaults.rs\"]\nmod defaults;",
        "#[path = \"tests_load.rs\"]\nmod load;",
        "#[path = \"tests_validation.rs\"]\nmod validation;",
    ] {
        assert!(
            config_tests.contains(required),
            "src/config/tests.rs should register behavior-focused config test module {required}"
        );
    }
    assert!(
        !config_tests.contains("#[test]"),
        "src/config/tests.rs should keep shared imports and test module declarations only"
    );

    assert!(
        defaults.contains("test_kslim_config_parse_defaults_project_root_fields")
            && defaults.contains("test_profile_config_parse_optional_user_intent_sections")
            && defaults.contains("test_performance_config_parse_hot_path_policy_fields"),
        "src/config/tests_defaults.rs should own config default and parse behavior tests"
    );
    assert!(
        load.contains("test_load_kslim_config_resolves_relative_upstream_url_from_project_root")
            && load.contains("test_load_profile_reads_named_profile_from_profiles_dir")
            && load.contains("test_load_profile_rejects_file_name_mismatch"),
        "src/config/tests_load.rs should own config/profile load behavior tests"
    );
    assert!(
        validation.contains("test_validate_config_rejects_invalid_output_config")
            && validation.contains("test_validate_profile_rejects_nondefault_security_config_until_supported")
            && validation.contains("test_validate_profile_rejects_unsupported_performance_config"),
        "src/config/tests_validation.rs should own config/profile validation behavior tests"
    );
    assert!(
        architecture.contains("Config unit tests are split by behavior"),
        "docs/architecture.md should document config test ownership"
    );
}
