use super::common::*;

#[test]
fn tree_index_kconfig_index_lives_in_kconfig_index_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tree_index = index_source(root);
    let kconfig_index = production_source(&root.join("src/index/kconfig_index.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod kconfig_index;",
        "pub use kconfig_index::{",
        "KconfigDefinition",
        "KconfigDefinitionIndex",
        "KconfigFileIndex",
        "KconfigReferenceIndex",
        "KconfigSourceIndex",
        "KconfigSourceReference",
        "KconfigSymbolReference",
        "kconfig_files_from_indexed_files(root, &index.files)",
        "scan_kconfig_file(&relative, &path)",
        "fn add_kconfig_file_facts(&mut self, facts: kconfig_index::KconfigFileFacts)",
    ] {
        assert!(
            tree_index.contains(required),
            "src/index/mod.rs should route Kconfig indexing through kconfig_index item {required}"
        );
    }

    for required in [
        "pub type KconfigFileIndex = BTreeSet<PathBuf>",
        "pub type KconfigDefinitionIndex = BTreeSet<KconfigDefinition>",
        "pub type KconfigReferenceIndex = BTreeSet<KconfigSymbolReference>",
        "pub type KconfigSourceIndex = BTreeSet<KconfigSourceReference>",
        "pub struct KconfigDefinition",
        "pub struct KconfigSymbolReference",
        "pub struct KconfigSourceReference",
        "pub(in crate::index) struct KconfigFileFacts",
        "pub(in crate::index) fn kconfig_files_from_indexed_files(",
        "pub(in crate::index) fn scan_kconfig_file(",
        "fn is_kconfig_path(path: &Path) -> bool",
        "fn parse_kconfig_definition(line: &str) -> Option<String>",
        "fn parse_kconfig_symbol_refs(line: &str) -> Option<(String, BTreeSet<String>)>",
        "crate::kconfig::parse_kconfig_source(line)",
        "is_host_absolute_path_like(&source.path)",
        "pub(in crate::index) fn collect_symbol_tokens(input: &str) -> BTreeSet<String>",
        "fn insert_symbol_token(",
        "fn normalize_symbol_token(",
        "fn is_symbol_token(",
        "fn is_non_symbol_keyword(",
    ] {
        assert!(
            kconfig_index.contains(required),
            "src/index/kconfig_index.rs should own Kconfig index item {required}"
        );
    }

    for moved in [
        "\npub type KconfigFileIndex",
        "\npub type KconfigDefinitionIndex",
        "\npub type KconfigReferenceIndex",
        "\npub type KconfigSourceIndex",
        "\npub struct KconfigDefinition",
        "\npub struct KconfigSymbolReference",
        "\npub struct KconfigSourceReference",
        "\nfn scan_kconfig_file",
        "\nfn kconfig_files_from_indexed_files",
        "\nfn is_kconfig_path",
        "\nfn parse_kconfig_definition",
        "\nfn parse_kconfig_symbol_refs",
        "\nfn collect_symbol_tokens",
        "\nfn insert_symbol_token",
        "\nfn normalize_symbol_token",
        "\nfn is_symbol_token",
        "\nfn is_non_symbol_keyword",
    ] {
        assert!(
            !tree_index.contains(moved),
            "src/index/mod.rs should not retain moved Kconfig index implementation {moved}"
        );
    }

    for required in [
        "`src/index/kconfig_index.rs`",
        "Tree index Kconfig indexing",
    ] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document tree index Kconfig ownership through {required}"
        );
    }
}
