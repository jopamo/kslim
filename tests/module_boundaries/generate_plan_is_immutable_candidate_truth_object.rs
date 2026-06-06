use super::common::*;

#[test]
fn generate_plan_is_immutable_candidate_truth_object() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let plan = plan_source(root);
    let feature_intent_fingerprint =
        production_source(&root.join("src/plan/feature_intent_fingerprint.rs"));
    let build_matrix_fingerprint =
        production_source(&root.join("src/plan/build_matrix_fingerprint.rs"));
    let source_map_sanitization =
        production_source(&root.join("src/plan/source_map_sanitization.rs"));
    let frozen_plan = production_source(&root.join("src/plan/frozen_plan.rs"));
    let plan_object = plan
        .split("/// Immutable generate plan tying requested state to resolved candidate state.")
        .nth(1)
        .and_then(|rest| {
            rest.split("#[derive(Debug, Clone, Default)]\n#[allow(dead_code)]\npub(crate) struct ConfigLoader")
                .next()
        })
        .expect("src/plan/mod.rs should define GeneratePlan before ConfigLoader");

    for required in [
        "A `GeneratePlan` is still candidate truth only",
        "pub(crate) struct GeneratePlan",
        "pub(crate) requested: RequestedGenerateState",
        "pub(crate) resolved: ResolvedCandidateState",
        "pub(crate) plan_id: PlanId",
        "pub(crate) created_with: ToolVersion",
        "pub(crate) config_content_hash: ConfigContentHash",
        "pub(crate) fingerprint: PlanFingerprint",
        "pub(crate) source_maps: Option<GeneratePlanSourceMaps>",
        "pub(crate) fn new(",
        "ConfigContentHash::from_resolved_state(&resolved)?",
        "ToolVersion::current()?",
        "pub(crate) fn from_parts(",
        "created_with: ToolVersion",
        "PlanFingerprint::from_parts(",
        "&created_with",
        "PlanId::from_fingerprint(&fingerprint)?",
        "created_with,",
        "source_maps: None",
        "pub(crate) fn with_source_maps(",
        "source_map_sanitization::without_temporary_workspace_or_host_paths(",
        "&self.requested",
        "source_maps,",
        "self.fingerprint = PlanFingerprint::from_parts(",
        "self.plan_id = PlanId::from_fingerprint(&self.fingerprint)?",
    ] {
        assert!(
            plan_object.contains(required),
            "GeneratePlan should be the immutable resolved request object; missing {required}"
        );
    }

    for required in [
        "pub(crate) struct ConfigContentHash",
        "fn from_resolved_state(resolved: &ResolvedCandidateState) -> Result<Self>",
        "append_resolved_candidate_fingerprint_lines(&mut source, resolved, false)",
    ] {
        assert!(
            plan.contains(required),
            "GeneratePlan should include resolved config content hash truth; missing {required}"
        );
    }

    for required in [
        "pub(super) fn without_temporary_workspace_or_host_paths(",
        "path_policy::is_absolute_path_like(source)",
        "RequestedGenerateState",
        "GeneratePlanSourceMaps::new(",
        "\"<requested-config>\"",
        "\"<selected-profile>\"",
        "\"<temporary-path>\"",
        "\"<workspace-path>\"",
        "\"<host-absolute-path>\"",
        "std::env::temp_dir()",
        "std::env::current_dir()",
    ] {
        assert!(
            source_map_sanitization.contains(required),
            "GeneratePlan source-map provenance must be sanitized before plan publication; missing {required}"
        );
    }

    for required in [
        "PLAN_FINGERPRINT_SERIALIZATION_FORMAT",
        "PLAN_FINGERPRINT_SCHEMA_VERSION",
        "let stable_serialization = stable_plan_fingerprint_serialization(",
        "format!(\"fingerprint-{}\", sha256_hex(&stable_serialization))",
        "stable_serialization,",
        "fn append_fingerprint_line(out: &mut String, key: &str, value: &str)",
        "fn escape_fingerprint_value(value: &str) -> String",
        "append_fingerprint_line(&mut out, \"version\", PLAN_FINGERPRINT_SCHEMA_VERSION)",
        "tool_version: &ToolVersion",
        "append_fingerprint_line(&mut out, \"tool_version\", tool_version.as_str())",
        "\"requested.selected_profile\"",
        "requested.selected_profile.as_str()",
        "let cli = &requested.cli_overrides",
        "\"requested.cli_overrides.{field}\"",
        "\"requested.cli_overrides.max_fixup_passes\"",
        "\"requested.cli_overrides.run_selftests\"",
        "\"resolved.base.commit\"",
        "config_content_hash: &ConfigContentHash",
        "append_fingerprint_line(\n        &mut out,\n        \"config_content_hash\",\n        config_content_hash.as_str(),\n    )",
    ] {
        assert!(
            plan.contains(required),
            "GeneratePlan fingerprint should use stable serialization and include plan truth; missing {required}"
        );
    }

    for required in [
        "feature_intent_fingerprint::append_feature_intent_fingerprint_lines(out, resolved)",
        "\"resolved.feature_graph_fingerprint\"",
        "resolved.feature_graph_fingerprint.as_str()",
        "\"resolved.abi_policy_fingerprint\"",
        "resolved.abi_policy_fingerprint.as_str()",
    ] {
        assert!(
            plan.contains(required) || feature_intent_fingerprint.contains(required),
            "GeneratePlan fingerprint should include resolved feature/ABI fingerprint truth; missing {required}"
        );
    }

    for required in [
        "\"resolved.feature_resolution.abi_policy.allow_public_header_removal\"",
        "\"resolved.feature_resolution.abi_policy.allow_uapi_header_removal\"",
        "\"resolved.abi_decision.allow_public_header_removal\"",
        "\"resolved.abi_decision.allow_uapi_header_removal\"",
        "\"resolved.prune_plan.abi_policy.allow_public_header_removal\"",
        "\"resolved.prune_plan.abi_policy.allow_uapi_header_removal\"",
    ] {
        assert!(
            plan.contains(required),
            "GeneratePlan fingerprint should include resolved ABI policy truth; missing {required}"
        );
    }

    for required in [
        "build_matrix_fingerprint::append_build_matrix_fingerprint_lines(",
        "\"resolved.build_matrix_plan.enabled\"",
        "\"resolved.build_matrix_plan.fail_on_error\"",
    ] {
        assert!(
            plan.contains(required) || build_matrix_fingerprint.contains(required),
            "GeneratePlan fingerprint should include resolved build matrix truth; missing {required}"
        );
    }

    for required in [
        "tool_version: String",
        "config_content_hash: String",
        "tool_version: ToolVersion::current()?.as_str().to_string()",
        "config_content_hash: plan.config_content_hash.as_str().to_string()",
        "let current = ToolVersion::current()?",
        "frozen plan tool_version",
        "ensure_equal(\n            \"config_content_hash\",\n            plan.config_content_hash.as_str(),\n            &self.config_content_hash,\n        )",
    ] {
        assert!(
            frozen_plan.contains(required),
            "frozen plan should publish and verify tool version and config content hash truth; missing {required}"
        );
    }

    for forbidden in [
        "CandidateTreeState",
        "CandidateTreePath",
        "CandidateMetadataDir",
        "PublishedSnapshotState",
        "PublishedMetadataDir",
        "GenerateAttemptFailure",
        "AttemptMetadataDir",
        "SuccessfulCommitResult",
        "TempDir",
        "WorkspaceRoot",
        "WorkspacePaths",
        "tempfile::",
        "temp_dir",
        "tempdir",
        "keep_temp",
        "write_authoritative_lockfile",
        "commit_output_repo_state",
        "materialize_resolved_candidate_tree",
        "std::fs::write",
    ] {
        assert!(
            !plan_object.contains(forbidden),
            "GeneratePlan must not alias candidate tree, published, failure, commit, or filesystem mutation state; found {forbidden}"
        );
    }
}
