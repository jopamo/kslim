use super::common::*;

#[test]
fn kconfig_expression_solver_detects_impossible_choices() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(
        &root,
        &["src/kconfig/mod.rs", "src/kconfig/expression.rs", "src/kconfig/solver.rs"],
    );
    let expression_tests = production_source(&root.join("src/kconfig/expression_tests.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    for required in [
        "struct KconfigImpossibleChoice",
        "fn detect_kconfig_impossible_choices(",
        "kconfig_choice_members_after_removed_symbols(",
        "evaluate_kconfig_visibility_after_removed_symbols(",
        "!kconfig_choice_is_optional(choice)",
        "!members.has_reachable_member",
        "fn kconfig_node_config_like_visibility(",
        "KconfigNode::Endchoice(_) if nested_choice_depth == 0",
        "removed_symbols.contains(symbol)",
        "directive.trim_start() == \"optional\"",
    ] {
        assert!(
            kconfig.contains(required),
            "Kconfig expression solver should detect impossible choices through {required}"
        );
    }

    for required in [
        "detect_kconfig_impossible_choices_reports_unreachable_mandatory_choices",
        "detect_kconfig_choice_invalidation_reports_removed_selected_choice_without_live_replacement",
        "BROKEN_CHOICE",
        "INVALIDATED_CHOICE",
        "REMOVED_SELECTED_MEMBER",
        "BLOCKED_REPLACEMENT",
        "STILL_VALID_CHOICE",
        "LIVE_REPLACEMENT",
        "UNREACHABLE_MEMBER",
        "HAS_LIVE_MEMBER",
        "HIDDEN_CHOICE",
        "OPTIONAL_CHOICE",
        "REMOVED_ANON_MEMBER",
        "depends on BLOCKED_DEP",
        "depends on MOD_GATE",
        "\\toptional",
        "detect_kconfig_impossible_choices",
        "choice.choice_symbol().map(str::to_string)",
        "choice.line()",
        "choice.visibility()",
        "choice.member_symbols().to_vec()",
        "TristateLiteral::M",
        "None",
    ] {
        assert!(
            expression_tests.contains(required),
            "Kconfig impossible-choice detection tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("detect impossible choices")
            && architecture.contains("non-optional reachable choice block")
            && architecture.contains("choice invalidation")
            && kernel_build_guide.contains("detect impossible choices")
            && kernel_build_guide.contains("non-optional reachable choice block")
            && kernel_build_guide.contains("choice invalidation"),
        "docs should describe Kconfig impossible-choice detection"
    );
}
