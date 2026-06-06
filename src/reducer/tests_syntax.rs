use super::*;

#[test]
fn test_reducer_run_fails_closed_on_unsupported_kconfig_expression_by_default() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("Kconfig"),
        "config FOO\n\tbool \"Foo\"\n\tdepends on REMOVED + LIVE\n",
    )
    .unwrap();

    let mut profile = default_profile_config("v1.0");
    profile.slim = Some(SlimConfig {
        remove_paths: vec![],
        remove_configs: vec!["REMOVED".to_string()],
        set_defaults: BTreeMap::new(),
        unsafe_allow_root_path_removal: false,
    });

    let err = run(root.to_str().unwrap(), &profile)
        .unwrap_err()
        .to_string();
    assert!(err.contains("unsupported Kconfig expressions"));
    assert!(err.contains("Kconfig:3"));
    assert!(err.contains("depends on"));
    assert!(err.contains("REMOVED + LIVE"));
}
#[test]
fn test_run_reducer_result_reports_unsupported_syntax_status() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("Kconfig"),
        "config FOO\n\tbool \"Foo\"\n\tdepends on REMOVED + LIVE\n",
    )
    .unwrap();

    let result = run_reducer(
        &kernel_root(root),
        &SlimConfig {
            remove_paths: vec![],
            remove_configs: vec!["REMOVED".to_string()],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        },
        &ReducerConfig::default(),
    )
    .unwrap();

    assert_eq!(result.status, ReducerStatus::FailedUnsupportedSyntax);
    assert_eq!(result.convergence, ConvergenceStatus::NotEvaluated);
    assert_eq!(result.diagnostic_summary.unsupported_kconfig_expressions, 1);
    assert_eq!(result.skipped_sites.len(), 1);
    assert_eq!(
        result.skipped_sites[0].kind,
        "unsupported_kconfig_expression"
    );
    assert_eq!(result.skipped_sites[0].file, Some(PathBuf::from("Kconfig")));
}
#[test]
fn test_run_reducer_result_allows_unsupported_syntax_when_policy_disabled() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("Kconfig"),
        "config FOO\n\tbool \"Foo\"\n\tdepends on REMOVED + LIVE\n",
    )
    .unwrap();
    let mut config = ReducerConfig::default();
    config.report_unsupported_expressions = false;

    let result = run_reducer(
        &kernel_root(root),
        &SlimConfig {
            remove_paths: vec![],
            remove_configs: vec!["REMOVED".to_string()],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        },
        &config,
    )
    .unwrap();

    assert_eq!(result.status, ReducerStatus::Success);
    assert!(result.publishable);
    assert_eq!(result.convergence, ConvergenceStatus::Converged);
    assert_eq!(result.diagnostic_summary.unsupported_kconfig_expressions, 1);
    assert_eq!(
        result.skipped_sites[0].kind,
        "unsupported_kconfig_expression"
    );
}
#[test]
fn test_render_unsupported_expression_report_sorts_sites_by_stable_keys() {
    let unsupported_kconfig = vec![
        crate::kconfig::UnsupportedKconfigExpression {
            file: PathBuf::from("z/Kconfig"),
            line: 9,
            directive: String::from("if"),
            expression: String::from("Z"),
            reason: String::from("z"),
        },
        crate::kconfig::UnsupportedKconfigExpression {
            file: PathBuf::from("a/Kconfig"),
            line: 1,
            directive: String::from("if"),
            expression: String::from("A"),
            reason: String::from("a"),
        },
    ];
    let unsupported_cpp = vec![
        crate::cpp::UnsupportedCppExpression {
            file: PathBuf::from("z.c"),
            line: 5,
            directive: String::from("if"),
            expression: String::from("Z"),
            reason: String::from("z"),
        },
        crate::cpp::UnsupportedCppExpression {
            file: PathBuf::from("a.c"),
            line: 1,
            directive: String::from("if"),
            expression: String::from("A"),
            reason: String::from("a"),
        },
    ];

    let report = render_unsupported_expression_report(&unsupported_kconfig, &unsupported_cpp);

    fn index_of(haystack: &str, needle: &str) -> usize {
        haystack
            .find(needle)
            .unwrap_or_else(|| panic!("missing {needle:?} in rendered unsupported report"))
    }

    assert!(index_of(&report, "a/Kconfig:1") < index_of(&report, "z/Kconfig:9"));
    assert!(index_of(&report, "a.c:1") < index_of(&report, "z.c:5"));
}
#[test]
fn test_reducer_run_simplifies_supported_nested_if_expression() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(root.join("Kconfig"), "if REMOVED || LIVE\nendif\n").unwrap();

    let mut profile = default_profile_config("v1.0");
    profile.slim = Some(SlimConfig {
        remove_paths: vec![],
        remove_configs: vec!["REMOVED".to_string()],
        set_defaults: BTreeMap::new(),
        unsafe_allow_root_path_removal: false,
    });

    let stats = run(root.to_str().unwrap(), &profile).unwrap();

    assert!(stats.ran);
    assert!(stats.unsupported_kconfig_expressions.is_empty());
    assert_eq!(
        std::fs::read_to_string(root.join("Kconfig")).unwrap(),
        "if LIVE\nendif\n"
    );
}
#[test]
fn test_reducer_run_can_report_unsupported_kconfig_expression_without_failing() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(root.join("Kconfig"), "if REMOVED + LIVE\nendif\n").unwrap();

    let mut profile = default_profile_config("v1.0");
    profile.slim = Some(SlimConfig {
        remove_paths: vec![],
        remove_configs: vec!["REMOVED".to_string()],
        set_defaults: BTreeMap::new(),
        unsafe_allow_root_path_removal: false,
    });
    profile.reducer.report_unsupported_expressions = false;

    let stats = run(root.to_str().unwrap(), &profile).unwrap();

    assert!(stats.ran);
    assert_eq!(stats.unsupported_kconfig_expressions.len(), 1);
    let site = &stats.unsupported_kconfig_expressions[0];
    assert_eq!(site.file, PathBuf::from("Kconfig"));
    assert_eq!(site.line, 1);
    assert_eq!(site.directive, "if");
    assert_eq!(site.expression, "REMOVED + LIVE");
}
