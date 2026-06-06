use super::common::*;

#[test]
fn primitive_rewrite_modules_do_not_depend_on_orchestration_policy_modules() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let primitive_modules = [
        "src/kconfig/mod.rs",
        "src/kbuild/mod.rs",
        "src/kbuild/ast.rs",
        "src/kbuild/object_graph.rs",
        "src/kbuild/parser.rs",
        "src/kbuild/report.rs",
        "src/kbuild/rewrite.rs",
        "src/source_scan/mod.rs",
        "src/source_scan/cpp.rs",
        "src/source_scan/includes/mod.rs",
        "src/source_scan/includes/cleanup.rs",
        "src/source_scan/includes/index.rs",
        "src/source_scan/includes/policy.rs",
        "src/source_scan/includes/private_header.rs",
    ];
    let forbidden_dependencies = [
        "crate::config",
        "crate::generate",
        "crate::output_repo",
        "crate::patches",
        "crate::prune",
        "crate::publish",
        "crate::reducer",
        "crate::removal_manifest",
        "crate::selftest",
        "crate::upstream",
    ];

    for module in primitive_modules {
        let path = root.join(module);
        let production = production_source(&path);
        for dependency in forbidden_dependencies {
            assert!(
                !production.contains(dependency),
                "{module} must remain a local syntax-aware rewrite primitive; found forbidden dependency {dependency}"
            );
        }
    }
}
