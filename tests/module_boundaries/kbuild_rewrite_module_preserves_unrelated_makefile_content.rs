use super::common::*;

#[test]
fn kbuild_rewrite_module_preserves_unrelated_makefile_content() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kbuild = production_sources(
        &root,
        &["src/kbuild/mod.rs", "src/kbuild/parser.rs", "src/kbuild/rewrite.rs"],
    );

    for required in [
        "protected_make_logical_line_starts",
        "is_make_recipe_line",
        "is_make_define_start",
        "render_kbuild_assignment_rewrite",
        "render_multiline_kbuild_assignment_rewrite",
        "split_make_trailing_comment",
        "strip_make_line_continuation",
        "KbuildSkippedLine",
    ] {
        assert!(
            kbuild.contains(required),
            "kbuild modules should preserve unrelated Makefile content through {required}"
        );
    }

    for forbidden in [
        "content.replace(",
        "line.replace(",
        "std::fs::write(&path, content",
    ] {
        assert!(
            !kbuild.contains(forbidden),
            "kbuild rewrites must not use broad whole-file replacement patterns; found {forbidden}"
        );
    }
}
