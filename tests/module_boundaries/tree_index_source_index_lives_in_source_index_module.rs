use super::common::*;

#[test]
fn tree_index_source_index_lives_in_source_index_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tree_index = index_source(root);
    let source_index = production_source(&root.join("src/index/source_index.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod source_index;",
        "pub use source_index::{",
        "CppGate",
        "CppGateIndex",
        "IncludeSite",
        "IncludeSiteIndex",
        "use source_index::{scan_c_family_file, unique_cpp_gate_count};",
        "scan_c_family_file(&relative, &path)",
        "fn add_source_file_facts(&mut self, facts: source_index::SourceFileFacts)",
        "unique_cpp_gate_count(&self.cpp_gates_by_symbol)",
    ] {
        assert!(
            tree_index.contains(required),
            "src/index/mod.rs should route C-family source indexing through source_index item {required}"
        );
    }

    for required in [
        "pub type IncludeSiteIndex = BTreeSet<IncludeSite>",
        "pub type CppGateIndex = BTreeMap<String, BTreeSet<CppGate>>",
        "pub struct IncludeSite",
        "pub struct CppGate",
        "pub(in crate::index) struct SourceFileFacts",
        "pub(in crate::index) fn scan_c_family_file(",
        "std::fs::read_to_string(path)",
        "parse_include_target(line)",
        "is_host_absolute_path_like(target)",
        "fn insert_cpp_gate(cpp_gates_by_symbol: &mut CppGateIndex, gate: CppGate)",
        "collect_symbol_tokens(&gate.expression)",
        "pub(in crate::index) fn unique_cpp_gate_count(",
        "pub(in crate::index) fn parse_include_target(line: &str) -> Option<&str>",
        "fn parse_cpp_gate(file: &Path, line: usize, input: &str) -> Option<CppGate>",
    ] {
        assert!(
            source_index.contains(required),
            "src/index/source_index.rs should own C-family source index item {required}"
        );
    }

    for moved in [
        "\npub type IncludeSiteIndex",
        "\npub type CppGateIndex",
        "\npub struct IncludeSite",
        "\npub struct CppGate",
        "\nfn scan_c_family_file",
        "\nfn insert_cpp_gate",
        "\nfn unique_cpp_gate_count",
        "\nfn parse_include_target",
        "\nfn parse_cpp_gate",
    ] {
        assert!(
            !tree_index.contains(moved),
            "src/index/mod.rs should not retain moved source index implementation {moved}"
        );
    }

    for required in [
        "`src/index/source_index.rs`",
        "Tree index source indexing",
    ] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document tree index source ownership through {required}"
        );
    }
}
