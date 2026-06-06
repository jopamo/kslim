use super::common::*;

#[test]
fn kconfig_expression_solver_evaluates_profile_reachability() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(
        &root,
        &["src/kconfig/mod.rs", "src/kconfig/expression.rs", "src/kconfig/solver.rs"],
    );
    let expression_tests = production_source(&root.join("src/kconfig/expression_tests.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    for required in [
        "fn evaluate_kconfig_reachability_under_selected_profile(",
        "selected_profile_values: &BTreeMap<String, TristateLiteral>",
        "evaluate_kconfig_visibility(",
        "selected_profile_values",
        ".map(|visibility| visibility != TristateLiteral::N)",
    ] {
        assert!(
            kconfig.contains(required),
            "Kconfig expression solver should evaluate selected-profile reachability through {required}"
        );
    }

    for required in [
        "evaluate_kconfig_reachability_under_selected_profile_uses_visibility",
        "selected_profile_values",
        "\\\"Reachable\\\" if FEATURE",
        "depends on MODULES",
        "\\\"Blocked\\\" if FEATURE",
        "depends on BLOCKER",
        "\\\"Unknown\\\" if MISSING",
        "Some(true)",
        "Some(false)",
        "None",
    ] {
        assert!(
            expression_tests.contains(required),
            "Kconfig selected-profile reachability tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("evaluate reachability under selected profiles")
            && architecture.contains("selected-profile symbol values")
            && architecture.contains("visibility is not `n`")
            && kernel_build_guide.contains("evaluate reachability under selected profiles")
            && kernel_build_guide.contains("selected-profile symbol values")
            && kernel_build_guide.contains("visibility is not `n`"),
        "docs should describe Kconfig selected-profile reachability evaluation"
    );
}
