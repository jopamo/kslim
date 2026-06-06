use super::common::*;

#[test]
fn kconfig_expression_solver_detects_empty_menus() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(
        &root,
        &["src/kconfig/mod.rs", "src/kconfig/expression.rs", "src/kconfig/solver.rs"],
    );
    let expression_tests = production_source(&root.join("src/kconfig/expression_tests.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    for required in [
        "struct KconfigEmptyMenu",
        "fn detect_kconfig_empty_menus(",
        "KconfigNode::Menu(menu)",
        "evaluate_kconfig_body_visibility_after_removed_symbols(",
        "kconfig_menu_has_reachable_content_after_removed_symbols(",
        "menu.prompt().to_string()",
        "KconfigNode::Endmenu(_) if nested_menu_depth == 0",
        "KconfigNode::Source(_)",
        "KconfigNode::Comment(comment)",
        "kconfig_node_config_like_visibility(node)",
        "removed_symbols.contains(symbol)",
        "fn evaluate_kconfig_body_visibility_after_removed_symbols(",
        "Some(KconfigDirective::DependsOn { expr })",
        "Some(KconfigDirective::VisibleIf { expr })",
    ] {
        assert!(
            kconfig.contains(required),
            "Kconfig expression solver should detect empty menus through {required}"
        );
    }

    for required in [
        "detect_kconfig_empty_menus_reports_reachable_menus_without_live_content",
        "REMOVED_MENU_MEMBER",
        "HIDDEN_MENU_MEMBER",
        "REMOVED_MOD_MEMBER",
        "REMOVED_WITH_LIVE_MEMBER",
        "REMOVED_COMMENT_MEMBER",
        "REMOVED_SOURCE_MEMBER",
        "menu \\\"Empty\\\"",
        "visible if MOD_GATE",
        "menu \\\"Has live\\\"",
        "menu \\\"Hidden menu\\\"",
        "comment \\\"Still visible\\\"",
        "source \\\"Kconfig.live\\\"",
        "detect_kconfig_empty_menus",
        "menu.prompt().to_string()",
        "menu.line()",
        "menu.visibility()",
        "TristateLiteral::M",
    ] {
        assert!(
            expression_tests.contains(required),
            "Kconfig empty-menu detection tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("detect empty menus")
            && architecture.contains("reachable menu block")
            && kernel_build_guide.contains("detect empty menus")
            && kernel_build_guide.contains("reachable menu block"),
        "docs should describe Kconfig empty-menu detection"
    );
}
