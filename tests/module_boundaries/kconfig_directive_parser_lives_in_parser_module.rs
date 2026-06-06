use super::common::*;

#[test]
fn kconfig_directive_parser_lives_in_parser_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(&root, &["src/kconfig/mod.rs", "src/kconfig/rewrite.rs"]);
    let parser = production_source(&root.join("src/kconfig/parser.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod parser;",
        "parse_kconfig_directive",
        "split_kconfig_trailing_comment",
        "strip_kconfig_keyword",
        "KconfigDirective",
        "pub(crate) use parser::parse_kconfig_source",
    ] {
        assert!(
            kconfig.contains(required),
            "src/kconfig/mod.rs should delegate directive/source parsing through {required}"
        );
    }

    for required in [
        "pub(super) enum KconfigEntryKind",
        "pub(super) enum KconfigDirective",
        "pub(crate) fn parse_kconfig_source(",
        "pub(super) fn parse_kconfig_directive(",
        "fn parse_kconfig_source_directive(",
        "pub(super) fn split_kconfig_if_clause(",
        "pub(super) fn split_kconfig_trailing_comment(",
    ] {
        assert!(
            parser.contains(required),
            "src/kconfig/parser.rs should own directive/source parsing through {required}"
        );
    }

    for forbidden in [
        "\nenum KconfigDirective",
        "\nfn parse_kconfig_directive(",
        "\nfn parse_kconfig_source_directive(",
        "\nfn split_kconfig_if_clause(",
    ] {
        assert!(
            !kconfig.contains(forbidden),
            "src/kconfig/mod.rs should not retain extracted parser implementation {forbidden}"
        );
    }

    for required in [
        "`src/kconfig/parser.rs`",
        "directive and source-line parsing",
    ] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document extracted Kconfig parser ownership through {required}"
        );
    }
}
