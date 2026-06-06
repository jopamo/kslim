use super::common::*;

#[test]
fn source_scan_module_owns_c_family_scanning() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let main = production_source(&root.join("src/main.rs"));
    let source_scan = production_source(&root.join("src/source_scan/mod.rs"));
    let cpp = production_source(&root.join("src/source_scan/cpp.rs"));
    let includes = production_source(&root.join("src/source_scan/includes/mod.rs"));
    let cpp_facade = production_source(&root.join("src/cpp.rs"));
    let includes_facade = production_source(&root.join("src/includes.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    assert!(
        main.contains("mod source_scan;"),
        "main.rs should register the C-family source_scan ownership module"
    );
    for required in [
        "pub(crate) mod cpp;",
        "pub(crate) mod includes;",
        "C-family source scanning, CPP folding, and include cleanup",
    ] {
        assert!(
            source_scan.contains(required),
            "src/source_scan/mod.rs should own source scan module declaration {required}"
        );
    }

    for required in [
        "pub(crate) struct CppFoldReport",
        "pub(crate) fn fold_removed_config_branches_report(",
        "pub(crate) fn apply_fold_report(",
        "pub(crate) fn visible_cpp_directive_lines(",
        "pub(crate) fn proven_dead_cpp_branch_lines(",
    ] {
        assert!(
            cpp.contains(required),
            "src/source_scan/cpp.rs should own CPP folding item {required}"
        );
    }

    for required in [
        "mod cleanup;",
        "mod index;",
        "mod private_header;",
        "mod policy;",
        "pub(crate) use cleanup::{",
        "pub(crate) use index::{index_include_sites, IncludeKind, IncludeSite}",
        "pub(crate) struct HeaderRemovalProofs",
        "pub(crate) fn resolve_include_targets(",
    ] {
        assert!(
            includes.contains(required),
            "src/source_scan/includes/mod.rs should own include scan/cleanup item {required}"
        );
    }

    assert!(
        cpp_facade.contains("pub(crate) use crate::source_scan::cpp::*;")
            && includes_facade.contains("pub(crate) use crate::source_scan::includes::*;"),
        "legacy cpp.rs and includes.rs should be compatibility facades over source_scan"
    );

    assert!(
        architecture.contains("`source_scan/*`")
            && architecture.contains("C-family source scanning")
            && architecture.contains("`cpp.rs` and `includes.rs` are compatibility facades"),
        "docs/architecture.md should document source_scan ownership and facades"
    );
}
