use super::common::*;

#[test]
fn kbuild_rewrite_lives_in_rewrite_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kbuild = production_source(&root.join("src/kbuild/mod.rs"));
    let rewrite = production_source(&root.join("src/kbuild/rewrite.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod rewrite;",
        "pub(crate) use rewrite::{composite_objects, rewrite_makefiles_report}",
        "pub(crate) use rewrite::{rewrite_makefiles, rewrite_makefiles_with_removed_configs}",
    ] {
        assert!(
            kbuild.contains(required),
            "src/kbuild/mod.rs should expose Kbuild rewrite logic through {required}"
        );
    }

    for required in [
        "pub(crate) fn rewrite_makefiles_report(",
        "pub(crate) fn rewrite_makefiles(",
        "pub(crate) fn rewrite_makefiles_with_removed_configs(",
        "struct RemovedIndex",
        "enum RewriteTokenDecision",
        "fn assignment_token_decision(",
        "fn should_drop_make_token(",
        "fn include_path_flag_decision(",
        "fn stale_composite_objects(",
        "pub(crate) fn composite_objects(",
        "fn composite_objects_with_protected(",
        "fn render_kbuild_assignment_rewrite(",
        "fn render_multiline_kbuild_assignment_rewrite(",
        "fn split_make_trailing_comment(",
        "fn strip_make_line_continuation(",
        "write_verified_rewrite(",
        "ensure_edit_records_for_mutation(\"prune.rewrite_makefiles\"",
        "sort_edit_records(&mut edits)",
        "KbuildRewriteReport {",
    ] {
        assert!(
            rewrite.contains(required),
            "src/kbuild/rewrite.rs should own Kbuild rewrite item {required}"
        );
    }

    for forbidden in [
        "\npub(crate) fn rewrite_makefiles_report(",
        "\npub(crate) fn rewrite_makefiles(",
        "\nstruct RemovedIndex",
        "\nenum RewriteTokenDecision",
        "\nfn assignment_token_decision(",
        "\nfn should_drop_make_token(",
        "\nfn stale_composite_objects(",
        "\nfn render_kbuild_assignment_rewrite(",
        "\nfn render_multiline_kbuild_assignment_rewrite(",
        "\nfn split_make_trailing_comment(",
        "\nfn strip_make_line_continuation(",
        "write_verified_rewrite(",
    ] {
        assert!(
            !kbuild.contains(forbidden),
            "src/kbuild/mod.rs should not retain extracted Kbuild rewrite implementation {forbidden}"
        );
    }

    for required in ["`src/kbuild/rewrite.rs`", "Kbuild rewrite logic"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document Kbuild rewrite ownership through {required}"
        );
    }
}
