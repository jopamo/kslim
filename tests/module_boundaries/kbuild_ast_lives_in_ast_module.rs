use super::common::*;

#[test]
fn kbuild_ast_lives_in_ast_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kbuild = production_source(&root.join("src/kbuild/mod.rs"));
    let ast = production_source(&root.join("src/kbuild/ast.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod ast;",
        "pub(crate) use ast::{",
        "CompositeKind",
        "KbuildAssignment",
        "KbuildAssignmentKind",
        "LogicalLine",
        "ObjListKind",
    ] {
        assert!(
            kbuild.contains(required),
            "src/kbuild/mod.rs should expose Kbuild AST types through {required}"
        );
    }

    for required in [
        "pub(crate) struct LogicalLine",
        "pub(crate) struct KbuildAssignment",
        "pub(crate) enum KbuildAssignmentKind",
        "pub(crate) enum ObjListKind",
        "pub(crate) enum CompositeKind",
        "CompositeMembers(CompositeKind",
        "Config(&'a str)",
        "Config { target: &'a str, symbol: &'a str }",
    ] {
        assert!(
            ast.contains(required),
            "src/kbuild/ast.rs should own Kbuild AST item {required}"
        );
    }

    for forbidden in [
        "\npub(crate) struct LogicalLine",
        "\npub(crate) struct KbuildAssignment",
        "\npub(crate) enum KbuildAssignmentKind",
        "\npub(crate) enum ObjListKind",
        "\npub(crate) enum CompositeKind",
    ] {
        assert!(
            !kbuild.contains(forbidden),
            "src/kbuild/mod.rs should not retain extracted Kbuild AST implementation {forbidden}"
        );
    }

    for required in ["`src/kbuild/ast.rs`", "Kbuild AST"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document Kbuild AST ownership through {required}"
        );
    }
}
