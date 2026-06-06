use crate::state::BuildMatrixPlan;
use super::{append_fingerprint_line, bool_string};

pub(super) fn append_build_matrix_fingerprint_lines(out: &mut String, plan: &BuildMatrixPlan) {
    append_fingerprint_line(
        out,
        "resolved.build_matrix_plan.enabled",
        bool_string(plan.enabled),
    );
    append_fingerprint_line(
        out,
        "resolved.build_matrix_plan.preset_count",
        &plan.presets.len().to_string(),
    );
    for preset in &plan.presets {
        append_fingerprint_line(out, "resolved.build_matrix_plan.presets", preset);
    }
    append_fingerprint_line(
        out,
        "resolved.build_matrix_plan.arch_count",
        &plan.arches.len().to_string(),
    );
    for arch in &plan.arches {
        append_fingerprint_line(out, "resolved.build_matrix_plan.arches", arch.as_str());
    }
    append_fingerprint_line(
        out,
        "resolved.build_matrix_plan.config_target_count",
        &plan.config_targets.len().to_string(),
    );
    for target in &plan.config_targets {
        append_fingerprint_line(out, "resolved.build_matrix_plan.config_targets", target);
    }
    append_fingerprint_line(
        out,
        "resolved.build_matrix_plan.target_count",
        &plan.targets.len().to_string(),
    );
    for target in &plan.targets {
        append_fingerprint_line(out, "resolved.build_matrix_plan.targets", target);
    }
    append_fingerprint_line(
        out,
        "resolved.build_matrix_plan.randconfig_seed",
        plan.randconfig_seed.as_deref().unwrap_or("<none>"),
    );
    append_fingerprint_line(
        out,
        "resolved.build_matrix_plan.jobs",
        &plan
            .jobs
            .map(|jobs| jobs.to_string())
            .unwrap_or_else(|| String::from("<none>")),
    );
    append_fingerprint_line(
        out,
        "resolved.build_matrix_plan.fail_on_error",
        bool_string(plan.fail_on_error),
    );
}
