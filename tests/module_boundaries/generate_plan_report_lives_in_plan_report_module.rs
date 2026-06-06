use super::common::*;

#[test]
fn generate_plan_report_lives_in_plan_report_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let generate = production_source(&root.join("src/generate.rs"));
    let plan_report = production_source(&root.join("src/generate/plan_report.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod plan_report;",
        "dry_run_result_from_plan",
        "deep_dry_run_result_from_candidate",
        "report_only_result_from_plan",
    ] {
        assert!(
            generate.contains(required),
            "src/generate.rs should delegate generate plan report helpers through {required}"
        );
    }

    for required in [
        "pub(super) fn dry_run_result_from_plan(",
        "pub(super) fn deep_dry_run_result_from_candidate(",
        "pub(super) fn report_only_result_from_plan(",
        "fn write_report_only_plan_report(",
        "fn render_report_only_plan_report(",
        "fn render_report_only_source_map_section(",
        "fn render_source_map_group(",
        "fn render_source_map_report_value(",
        "fn render_generate_stage_for_report(",
    ] {
        assert!(
            plan_report.contains(required),
            "src/generate/plan_report.rs should own plan report/dry-run helper {required}"
        );
    }

    for forbidden in [
        "\nfn dry_run_result_from_plan(",
        "\nfn deep_dry_run_result_from_candidate(",
        "\nfn report_only_result_from_plan(",
        "\nfn render_report_only_plan_report(",
        "\nfn render_report_only_source_map_section(",
    ] {
        assert!(
            !generate.contains(forbidden),
            "src/generate.rs should not retain extracted plan report implementation {forbidden}"
        );
    }

    for required in [
        "`src/generate/plan_report.rs`",
        "Generate plan report and dry-run rendering",
    ] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document extracted generate plan report ownership through {required}"
        );
    }
}
