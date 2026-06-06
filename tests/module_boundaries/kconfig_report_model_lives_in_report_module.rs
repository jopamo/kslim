use super::common::*;

#[test]
fn kconfig_report_model_lives_in_report_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_source(&root.join("src/kconfig/mod.rs"));
    let report = production_source(&root.join("src/kconfig/report.rs"));
    let rewrite = production_source(&root.join("src/kconfig/rewrite.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod report;",
        "pub(crate) use report::{",
        "KconfigRelationRewriteStats",
        "KconfigReportCounts",
        "UnsupportedKconfigExpression",
        "KconfigSolverReport",
    ] {
        assert!(
            kconfig.contains(required),
            "src/kconfig/mod.rs should expose Kconfig report module item {required}"
        );
    }

    for required in [
        "pub(crate) struct KconfigSolverReport",
        "pub(crate) struct KconfigReportCounts",
        "pub(crate) struct KconfigRelationRewriteStats",
        "pub(crate) struct UnsupportedKconfigExpression",
        "pub(crate) fn kconfig_solver_report(",
        "fn collect_document_solver_report(",
        "fn tristate_report_value(",
    ] {
        assert!(
            report.contains(required),
            "src/kconfig/report.rs should own report model/rendering item {required}"
        );
    }

    for forbidden in [
        "\npub(crate) struct KconfigReportCounts",
        "\npub(crate) struct KconfigRelationRewriteStats",
        "\npub(crate) struct UnsupportedKconfigExpression",
    ] {
        assert!(
            !rewrite.contains(forbidden),
            "src/kconfig/rewrite.rs should not retain extracted report model {forbidden}"
        );
    }

    for required in [
        "`src/kconfig/report.rs`",
        "Kconfig report models and solver-report rendering",
    ] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document extracted Kconfig report ownership through {required}"
        );
    }
}
