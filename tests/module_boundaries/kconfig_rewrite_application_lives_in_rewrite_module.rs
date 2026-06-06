use super::common::*;

#[test]
fn kconfig_rewrite_application_lives_in_rewrite_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_source(&root.join("src/kconfig/mod.rs"));
    let rewrite = production_source(&root.join("src/kconfig/rewrite.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod rewrite;",
        "pub(crate) use rewrite::{",
        "rewrite_kconfig_defaults",
        "rewrite_kconfig_relations",
        "rewrite_kconfig_sources",
        "rewrite_empty_kconfig_menus",
    ] {
        assert!(
            kconfig.contains(required),
            "src/kconfig/mod.rs should delegate rewrite application through {required}"
        );
    }

    for required in [
        "pub(crate) fn prune_configs(",
        "pub(crate) fn rewrite_kconfig_defaults(",
        "pub(crate) fn rewrite_kconfig_relations(",
        "pub(crate) fn rewrite_kconfig_sources(",
        "pub(crate) fn rewrite_dead_kconfig_symbol_definitions(",
        "pub(crate) fn rewrite_empty_kconfig_menus(",
        "fn analyze_kconfig_relation_line(",
        "fn rewrite_config_default_block(",
    ] {
        assert!(
            rewrite.contains(required),
            "src/kconfig/rewrite.rs should own rewrite application through {required}"
        );
    }

    for forbidden in [
        "\npub(crate) fn rewrite_kconfig_defaults(",
        "\npub(crate) fn rewrite_kconfig_relations(",
        "\npub(crate) fn rewrite_kconfig_sources(",
        "\nfn analyze_kconfig_relation_line(",
        "\nfn rewrite_config_default_block(",
    ] {
        assert!(
            !kconfig.contains(forbidden),
            "src/kconfig/mod.rs should not retain extracted rewrite implementation {forbidden}"
        );
    }

    for required in [
        "`src/kconfig/rewrite.rs`",
        "Kconfig rewrite application",
    ] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document extracted Kconfig rewrite ownership through {required}"
        );
    }
}
