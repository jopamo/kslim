use super::common::*;

const LEGACY_CATCH_ALL_PRODUCTION_BUDGETS: &[(&str, usize)] = &[
    ("src/generate.rs", 938),
    ("src/prune.rs", 37),
    ("src/fixups.rs", 236),
    ("src/diagnostics.rs", 13),
    ("src/edit_reason.rs", 204),
    ("src/output_repo.rs", 84),
];

#[test]
fn legacy_catch_all_files_do_not_accumulate_new_logic() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    for (relative, budget) in LEGACY_CATCH_ALL_PRODUCTION_BUDGETS {
        let source = production_source(&root.join(relative));
        let line_count = source.lines().count();
        assert!(
            line_count <= *budget,
            "{relative} is a legacy catch-all file with a no-growth production budget of {budget} lines; got {line_count}. Split new logic into an owned subsystem module instead."
        );
    }

    let doc = production_source(&root.join("docs/file-size-policy.md"));
    for required in [
        "## Legacy catch-all roots",
        "`src/generate.rs`",
        "`src/prune.rs`",
        "`src/diagnostics.rs`",
        "no-growth production budget",
        "owned subsystem directory",
    ] {
        assert!(
            doc.contains(required),
            "docs/file-size-policy.md should document catch-all growth guard {required}"
        );
    }
}
