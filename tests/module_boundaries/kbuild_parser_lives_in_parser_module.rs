use super::common::*;

#[test]
fn kbuild_parser_lives_in_parser_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kbuild = production_source(&root.join("src/kbuild/mod.rs"));
    let parser = production_source(&root.join("src/kbuild/parser.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod parser;",
        "pub(crate) use parser::{",
        "logical_lines",
        "parse_kbuild_assignment",
        "parse_make_assignment",
        "protected_make_logical_line_starts",
        "pub(in crate::kbuild) use parser::parse_kbuild_assignment_kind",
    ] {
        assert!(
            kbuild.contains(required),
            "src/kbuild/mod.rs should expose Kbuild parser functions through {required}"
        );
    }

    for required in [
        "pub(crate) fn logical_lines(",
        "pub(crate) fn protected_make_logical_line_starts(",
        "fn is_make_recipe_line(",
        "fn is_make_define_start(",
        "fn is_make_define_end(",
        "fn starts_with_make_directive(",
        "pub(crate) fn parse_make_assignment(",
        "pub(crate) fn parse_kbuild_assignment(",
        "pub(in crate::kbuild) fn parse_kbuild_assignment_kind(",
        "fn is_object_list_family(",
        "fn config_gated_assignment(",
        "KbuildAssignment { lhs, op, rhs, kind }",
    ] {
        assert!(
            parser.contains(required),
            "src/kbuild/parser.rs should own Kbuild parser item {required}"
        );
    }

    for forbidden in [
        "\npub(crate) fn logical_lines(",
        "\npub(crate) fn protected_make_logical_line_starts(",
        "\nfn is_make_recipe_line(",
        "\nfn is_make_define_start(",
        "\nfn starts_with_make_directive(",
        "\npub(crate) fn parse_make_assignment(",
        "\npub(crate) fn parse_kbuild_assignment(",
        "\nfn config_gated_assignment(",
        "\nfn is_object_list_family(",
    ] {
        assert!(
            !kbuild.contains(forbidden),
            "src/kbuild/mod.rs should not retain extracted Kbuild parser implementation {forbidden}"
        );
    }

    for required in ["`src/kbuild/parser.rs`", "Kbuild parser"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document Kbuild parser ownership through {required}"
        );
    }
}
