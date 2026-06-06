use super::common::*;

#[test]
fn include_index_lives_in_index_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let includes = production_source(&root.join("src/source_scan/includes/mod.rs"));
    let index = production_source(&root.join("src/source_scan/includes/index.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod index;",
        "pub(crate) use index::{index_include_sites, IncludeKind, IncludeSite}",
        "pub(in crate::source_scan::includes) use index::{c_family_files, relative_to_root_path}",
    ] {
        assert!(
            includes.contains(required),
            "src/source_scan/includes/mod.rs should expose include indexing through {required}"
        );
    }

    for required in [
        "pub(crate) enum IncludeKind",
        "pub(crate) struct IncludeSite",
        "pub(crate) fn index_include_sites(",
        "pub(in crate::source_scan::includes) fn parse_include_site(",
        "pub(in crate::source_scan::includes) fn c_family_files(",
        "pub(in crate::source_scan::includes) fn relative_to_root_path(",
        "crate::source_scan::cpp::visible_cpp_directive_lines(&lines)",
        "walkdir::WalkDir::new(root)",
        "matches!(ext, \"c\" | \"h\" | \"S\" | \"s\" | \"cc\" | \"cpp\" | \"cxx\")",
        ".then(left.header.cmp(&right.header))",
    ] {
        assert!(
            index.contains(required),
            "src/source_scan/includes/index.rs should own include indexing item {required}"
        );
    }

    for forbidden in [
        "\npub(crate) enum IncludeKind",
        "\npub(crate) struct IncludeSite",
        "\npub(crate) fn index_include_sites(",
        "\nfn parse_include_site(",
        "crate::source_scan::cpp::visible_cpp_directive_lines(&lines)",
        "walkdir::WalkDir::new(root)",
    ] {
        assert!(
            !includes.contains(forbidden),
            "src/source_scan/includes/mod.rs should not retain extracted include index implementation {forbidden}"
        );
    }

    for required in ["`src/source_scan/includes/index.rs`", "Include-site indexing"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document include index module ownership through {required}"
        );
    }
}
