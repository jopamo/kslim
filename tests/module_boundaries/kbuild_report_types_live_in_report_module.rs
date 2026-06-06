use super::common::*;

#[test]
fn kbuild_report_types_live_in_report_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kbuild = production_source(&root.join("src/kbuild/mod.rs"));
    let report = production_source(&root.join("src/kbuild/report.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod report;",
        "pub(crate) use report::{KbuildRewriteReport, KbuildSkippedLine}",
    ] {
        assert!(
            kbuild.contains(required),
            "src/kbuild/mod.rs should expose Kbuild report types through {required}"
        );
    }

    for required in [
        "pub(crate) struct KbuildSkippedLine",
        "pub(crate) struct KbuildRewriteReport",
        "pub removed_refs: usize",
        "pub edits: Vec<EditRecord>",
        "pub skipped_ambiguous_lines: Vec<KbuildSkippedLine>",
    ] {
        assert!(
            report.contains(required),
            "src/kbuild/report.rs should own Kbuild report item {required}"
        );
    }

    for forbidden in [
        "\npub(crate) struct KbuildSkippedLine",
        "\npub(crate) struct KbuildRewriteReport",
        "pub skipped_ambiguous_lines: Vec<KbuildSkippedLine>",
    ] {
        assert!(
            !kbuild.contains(forbidden),
            "src/kbuild/mod.rs should not retain extracted Kbuild report implementation {forbidden}"
        );
    }

    for required in ["`src/kbuild/report.rs`", "Kbuild report types"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document Kbuild report ownership through {required}"
        );
    }
}
