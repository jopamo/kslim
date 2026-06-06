//! Candidate-tree materialization and mutation.
//!
//! This module may create and mutate private candidate trees. It must not
//! resolve requested state, verify candidates, write authoritative metadata,
//! commit output, or publish.

mod errors;
mod metadata;
mod model;
mod write;

#[cfg(test)]
#[allow(unused_imports)]
use errors::CandidateBuildStageFailure;
#[cfg(test)]
#[allow(unused_imports)]
use metadata::{
    record_partial_candidate_reducer_reports_at_dir, write_candidate_failure_attempt_metadata,
    ATTEMPT_METADATA_DIR, CANDIDATE_FAILURE_ATTEMPT_FILE, CANDIDATE_METADATA_FILE,
    KSLIM_METADATA_DIR,
};
pub(super) use metadata::write_candidate_metadata_for_verified_generate;
#[cfg(test)]
#[allow(unused_imports)]
use model::{ensure_candidate_mutation_target, WorkspacePaths};
#[allow(unused_imports)]
pub(super) use model::{CandidateMutationTarget, MaterializedTree};
#[cfg(test)]
#[allow(unused_imports)]
use write::build_candidate_tree;
#[allow(unused_imports)]
pub(super) use write::{
    materialize_integrate_and_reduce_candidate_tree, CandidateMaterialization,
    CandidateMaterializationEvent,
};
#[cfg(test)]
pub(super) use write::ensure_patch_application_matches_plan;
#[cfg(test)]
#[allow(unused_imports)]
use write::{
    apply_integrations, apply_patch_sources, materialize_resolved_candidate_tree, reduce_tree,
};

#[cfg(test)]
mod tests {
    use super::super::plan::GeneratePlan;
    use super::super::state::{
        CandidateTreeState, CliOverrides, IntegrationEntryPlan, ProfileName,
        RequestedGenerateState, ResolvedCandidateState,
    };
    use super::super::GenerateStage;
    use super::*;
    use crate::config::ReducerConfig;
    use crate::lockfile::ResolvedBase;
    use crate::paths::{
        AttemptMetadataDir, CandidateMetadataDir, CandidateTreePath, RelativeKernelPath,
        RequestedConfigPath,
    };
    use crate::removal_manifest::RemovalManifest;
    use crate::{config, manifest, output_repo, patches, reducer, upstream};
    use anyhow::Result;
    use serde::Deserialize;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    #[derive(Debug, Deserialize)]
    struct CandidateFailureStageFixture {
        stage: GenerateStage,
    }

    fn git_in(dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap();
        if !output.status.success() {
            panic!(
                "git {:?} failed in {}: {}",
                args,
                dir.display(),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn create_minimal_tree(root: &Path) {
        for dir in &[
            "arch", "drivers", "fs", "include", "kernel", "mm", "net", "scripts",
        ] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
            std::fs::write(root.join(dir).join(".keep"), "").unwrap();
        }
        std::fs::write(root.join("Makefile"), "# test\n").unwrap();
        std::fs::write(root.join("Kconfig"), "# test\n").unwrap();
    }

    fn materialize_upstream_tree(upstream_path: &str, commit: &str) -> Result<MaterializedTree> {
        let temp_dir = tempfile::Builder::new().prefix("kslim-gen-").tempdir()?;
        let path = temp_dir.path().to_string_lossy().to_string();
        upstream::archive_tree(upstream_path, commit, &path)?;
        let mutation_target = CandidateMutationTarget {
            tree_path: CandidateTreePath::new(temp_dir.path()).unwrap(),
        };
        Ok(MaterializedTree {
            temp_dir,
            path,
            mutation_target,
        })
    }

    fn candidate_failure_stage<'a>(
        err: &'a anyhow::Error,
        stage: GenerateStage,
    ) -> &'a CandidateBuildStageFailure {
        let failure = err
            .downcast_ref::<CandidateBuildStageFailure>()
            .unwrap_or_else(|| panic!("candidate failure missing stage {stage}: {err:#}"));
        assert_eq!(failure.stage, stage);
        failure
    }

    fn assert_candidate_failure_stage(err: &anyhow::Error, stage: GenerateStage) {
        let _ = candidate_failure_stage(err, stage);
    }

    fn report_path_names(reports: &[crate::model::ReportPath], attempt_dir: &Path) -> Vec<String> {
        reports
            .iter()
            .map(|report| {
                report
                    .as_path()
                    .strip_prefix(attempt_dir)
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect()
    }

    fn assert_report_paths_sorted_unique(reports: &[crate::model::ReportPath]) {
        let mut sorted = reports.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(reports, sorted.as_slice());
    }

    fn create_minimal_tree_with_realtek(root: &Path) {
        create_minimal_tree(root);
        let realtek = root.join("drivers/net/ethernet/realtek");
        std::fs::create_dir_all(&realtek).unwrap();
        std::fs::write(realtek.join("Kconfig"), "config RTASE\n").unwrap();
        std::fs::write(realtek.join("Makefile"), "obj-$(CONFIG_RTASE) += rtase/\n").unwrap();
    }

    fn create_rtlmq_source(root: &Path) {
        std::fs::create_dir_all(root).unwrap();
        std::fs::write(root.join("Makefile"), "obj-$(CONFIG_RTLMQ) += rtlmq.o\n").unwrap();
        std::fs::write(root.join("Kconfig"), "config RTLMQ\n\ttristate \"RTLMQ\"\n").unwrap();
        std::fs::write(root.join("rtlmq.c"), "int rtlmq;\n").unwrap();
        std::fs::write(root.join("rtlmq.h"), "#pragma once\n").unwrap();
    }

    fn requested_state() -> RequestedGenerateState {
        requested_state_for_config("/tmp/project/kslim.toml")
    }

    fn requested_state_for_config(config_path: impl Into<PathBuf>) -> RequestedGenerateState {
        RequestedGenerateState::new(
            RequestedConfigPath::new(config_path).unwrap(),
            ProfileName::new("default").unwrap(),
            CliOverrides {
                dry_run: false,
                deep_dry_run: false,
                report_only: false,
                force: false,
                offline: false,
                base_ref: None,
                feature: None,
                remove_feature: None,
                preserve_feature: None,
                arch: None,
                primary_arch: None,
                secondary_arch: None,
                safety: None,
                max_fixup_passes: None,
                matrix: None,
                strict: false,
                no_strict: false,
                run_selftests: false,
            },
        )
    }

    fn with_requested_config(plan: GeneratePlan, config_path: &Path) -> GeneratePlan {
        GeneratePlan::new(requested_state_for_config(config_path), plan.resolved).unwrap()
    }

    fn plan_for_upstream(git_dir: &Path, commit: &str, output: &Path) -> GeneratePlan {
        plan_for_upstream_with_patches(git_dir, commit, output, None)
    }

    fn plan_for_upstream_with_patches(
        git_dir: &Path,
        commit: &str,
        output: &Path,
        patch_infos: Option<&[patches::PatchInfo]>,
    ) -> GeneratePlan {
        plan_for_upstream_with_profile(
            git_dir,
            commit,
            output,
            config::default_profile_config("v1.0"),
            patch_infos,
        )
    }

    fn plan_for_upstream_with_profile(
        git_dir: &Path,
        commit: &str,
        output: &Path,
        profile: config::ProfileConfig,
        patch_infos: Option<&[patches::PatchInfo]>,
    ) -> GeneratePlan {
        let mut config = config::default_kslim_config("demo", output.to_str().unwrap());
        config.upstream.url = git_dir.to_string_lossy().to_string();
        let resolved = ResolvedCandidateState::from_resolved_inputs(
            &config,
            &profile,
            ResolvedBase {
                upstream: config.upstream.name.clone(),
                url: config.upstream.url.clone(),
                r#ref: String::from("v1.0"),
                commit: commit.to_string(),
                resolved_at: String::from("2026-01-01T00:00:00Z"),
            },
            patch_infos,
            "unmodified-upstream",
            "kslim/v1.0/default",
        )
        .unwrap();

        GeneratePlan::new(requested_state(), resolved).unwrap()
    }

    fn mutation_target_for(plan: &GeneratePlan, candidate: &Path) -> CandidateMutationTarget {
        ensure_candidate_mutation_target(plan, candidate).unwrap()
    }

    fn create_patch_repo(
        root: &Path,
        base_files: &[(&str, &str)],
        head_files: &[(&str, &str)],
    ) -> patches::PatchInfo {
        std::fs::create_dir_all(root).unwrap();
        git_in(root, &["init"]);
        git_in(root, &["config", "user.email", "test@kslim.local"]);
        git_in(root, &["config", "user.name", "kslim test"]);
        for (path, contents) in base_files {
            let path = root.join(path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(path, contents).unwrap();
        }
        git_in(root, &["add", "-A"]);
        git_in(root, &["commit", "--allow-empty", "-m", "base"]);
        let merge_base = git_in(root, &["rev-parse", "HEAD"]);

        for (path, contents) in head_files {
            let path = root.join(path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(path, contents).unwrap();
        }
        git_in(root, &["add", "-A"]);
        git_in(root, &["commit", "-m", "patch"]);
        let head_commit = git_in(root, &["rev-parse", "HEAD"]);

        patches::PatchInfo {
            source: String::from("worktree"),
            worktree_path: root.to_string_lossy().to_string(),
            branch: git_in(root, &["branch", "--show-current"]),
            head_commit,
            merge_base,
            base_remote: String::from("origin"),
            base_ref: String::from("main"),
            patch_count: 1,
        }
    }

    #[test]
    fn test_workspace_paths_create_isolated_temp_workspace() {
        let workspace = WorkspacePaths::new_isolated_temp().unwrap();
        let temp_root = workspace.temp_dir.as_ref().unwrap().path().to_path_buf();

        assert_eq!(workspace.workspace_root(), temp_root.as_path());
        assert!(temp_root.is_dir());
        assert!(workspace.candidate_tree().starts_with(&temp_root));
        assert_eq!(
            workspace
                .candidate_tree()
                .file_name()
                .and_then(|name| name.to_str()),
            Some("tree")
        );
        assert!(!workspace.candidate_tree().exists());

        drop(workspace);
        assert!(!temp_root.exists());
    }

    #[test]
    fn test_failed_candidate_temp_workspace_is_removed_by_default() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("upstream");
        let output = tmp.path().join("output");
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(&repo).unwrap();
        create_minimal_tree(&repo);
        git_in(&repo, &["init"]);
        git_in(&repo, &["config", "user.email", "test@kslim.local"]);
        git_in(&repo, &["config", "user.name", "kslim test"]);
        git_in(&repo, &["add", "-A"]);
        git_in(&repo, &["commit", "-m", "initial"]);
        let plan = with_requested_config(
            plan_for_upstream(&repo.join(".git"), "missing-commit", &output),
            &project.join("kslim.toml"),
        );
        let workspace = WorkspacePaths::new_isolated_temp().unwrap();
        let temp_root = workspace.temp_dir.as_ref().unwrap().path().to_path_buf();

        let err = build_candidate_tree(&plan, &workspace).unwrap_err();
        let failure = candidate_failure_stage(&err, GenerateStage::Materialize);
        assert!(temp_root.exists());
        assert!(failure
            .partial_reports
            .iter()
            .any(|report| report.as_path().ends_with(CANDIDATE_FAILURE_ATTEMPT_FILE)));
        let attempt = project
            .join(KSLIM_METADATA_DIR)
            .join(ATTEMPT_METADATA_DIR)
            .join(CANDIDATE_FAILURE_ATTEMPT_FILE);
        assert!(attempt.exists());
        let attempt_metadata = std::fs::read_to_string(attempt).unwrap();
        assert!(attempt_metadata.contains("metadata_scope = \"non-authoritative-attempt\""));
        assert!(attempt_metadata.contains("authoritative = false"));
        assert!(attempt_metadata.contains("stage = \"materialize\""));

        drop(workspace);
        assert!(!temp_root.exists());
        assert!(!output.exists());
    }

    #[test]
    fn test_failed_candidate_temp_workspace_can_remain_only_with_keep_temp() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("upstream");
        let output = tmp.path().join("output");
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(&repo).unwrap();
        create_minimal_tree(&repo);
        git_in(&repo, &["init"]);
        git_in(&repo, &["config", "user.email", "test@kslim.local"]);
        git_in(&repo, &["config", "user.name", "kslim test"]);
        git_in(&repo, &["add", "-A"]);
        git_in(&repo, &["commit", "-m", "initial"]);
        let plan = with_requested_config(
            plan_for_upstream(&repo.join(".git"), "missing-commit", &output),
            &project.join("kslim.toml"),
        );
        let workspace = WorkspacePaths::new_isolated_temp_with_keep(true).unwrap();
        let temp_root = workspace.temp_dir.as_ref().unwrap().path().to_path_buf();

        let err = build_candidate_tree(&plan, &workspace).unwrap_err();
        assert_candidate_failure_stage(&err, GenerateStage::Materialize);

        drop(workspace);
        assert!(temp_root.exists());
        std::fs::remove_dir_all(temp_root).unwrap();
        assert!(!output.exists());
    }

    #[test]
    fn test_materialize_upstream_tree_archives_commit_into_temp_tree() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("upstream");
        std::fs::create_dir_all(&repo).unwrap();
        create_minimal_tree(&repo);

        git_in(&repo, &["init"]);
        git_in(&repo, &["config", "user.email", "test@kslim.local"]);
        git_in(&repo, &["config", "user.name", "kslim test"]);
        git_in(&repo, &["add", "-A"]);
        git_in(&repo, &["commit", "-m", "initial"]);
        let commit = git_in(&repo, &["rev-parse", "HEAD"]);

        let git_dir = repo.join(".git");
        let materialized =
            materialize_upstream_tree(git_dir.to_str().unwrap(), commit.trim()).unwrap();

        upstream::validate_tree(&materialized.path).unwrap();
        assert!(Path::new(&materialized.path).join("Makefile").exists());
        assert_eq!(
            std::fs::read_to_string(Path::new(&materialized.path).join("Makefile")).unwrap(),
            "# test\n"
        );
    }

    #[test]
    fn test_build_candidate_tree_materializes_resolved_commit_only() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("upstream");
        let output = tmp.path().join("output");
        std::fs::create_dir_all(&repo).unwrap();
        create_minimal_tree(&repo);
        std::fs::write(repo.join("Makefile"), "# v1\n").unwrap();

        git_in(&repo, &["init"]);
        git_in(&repo, &["config", "user.email", "test@kslim.local"]);
        git_in(&repo, &["config", "user.name", "kslim test"]);
        git_in(&repo, &["add", "-A"]);
        git_in(&repo, &["commit", "-m", "initial"]);
        let resolved_commit = git_in(&repo, &["rev-parse", "HEAD"]);

        std::fs::write(repo.join("Makefile"), "# head\n").unwrap();
        git_in(&repo, &["add", "-A"]);
        git_in(&repo, &["commit", "-m", "head"]);

        let plan = plan_for_upstream(&repo.join(".git"), &resolved_commit, &output);
        let workspace = WorkspacePaths::new_isolated_temp().unwrap();

        let state = build_candidate_tree(&plan, &workspace).unwrap();

        assert_eq!(state.tree.as_path(), workspace.candidate_tree());
        assert_eq!(
            state.metadata_dir.as_path(),
            workspace.candidate_tree().join(".kslim").as_path()
        );
        assert!(state.materialized);
        assert!(!state.integrated);
        assert!(!state.pruned);
        assert!(!state.reduced);
        assert!(!state.selftested);
        assert_eq!(
            std::fs::read_to_string(workspace.candidate_tree().join("Makefile")).unwrap(),
            "# v1\n"
        );
        assert!(!output.exists());
    }

    #[test]
    fn test_build_candidate_tree_writes_candidate_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("upstream");
        let output = tmp.path().join("output");
        std::fs::create_dir_all(&repo).unwrap();
        create_minimal_tree(&repo);
        git_in(&repo, &["init"]);
        git_in(&repo, &["config", "user.email", "test@kslim.local"]);
        git_in(&repo, &["config", "user.name", "kslim test"]);
        git_in(&repo, &["add", "-A"]);
        git_in(&repo, &["commit", "-m", "initial"]);
        let resolved_commit = git_in(&repo, &["rev-parse", "HEAD"]);
        let plan = plan_for_upstream(&repo.join(".git"), &resolved_commit, &output);
        let workspace = WorkspacePaths::new_isolated_temp().unwrap();

        let state = build_candidate_tree(&plan, &workspace).unwrap();

        let metadata_dir = state.metadata_dir.as_path();
        let candidate_metadata =
            std::fs::read_to_string(metadata_dir.join(CANDIDATE_METADATA_FILE)).unwrap();
        assert!(candidate_metadata.contains("metadata_scope = \"candidate\""));
        assert!(candidate_metadata.contains("authoritative = false"));
        assert!(candidate_metadata.contains(plan.plan_id.as_str()));
        assert!(candidate_metadata.contains(plan.fingerprint.as_str()));
        let entries = manifest::generate_manifest(state.tree.as_path().to_str().unwrap()).unwrap();
        let tree_fingerprint = manifest::tree_fingerprint(&entries);
        assert!(candidate_metadata.contains(&format!("tree_fingerprint = \"{tree_fingerprint}\"")));
        assert!(!candidate_metadata.contains(output.to_string_lossy().as_ref()));
        let manifest = std::fs::read_to_string(metadata_dir.join("manifest.txt")).unwrap();
        assert!(manifest.contains("Makefile"));
        assert!(!metadata_dir.join("published.toml").exists());
        assert!(!output.exists());
    }

    #[test]
    fn test_build_candidate_tree_rejects_output_path_as_mutation_target() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("upstream");
        let output = tmp.path().join("output");
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(&repo).unwrap();
        create_minimal_tree(&repo);
        git_in(&repo, &["init"]);
        git_in(&repo, &["config", "user.email", "test@kslim.local"]);
        git_in(&repo, &["config", "user.name", "kslim test"]);
        git_in(&repo, &["add", "-A"]);
        git_in(&repo, &["commit", "-m", "initial"]);
        let resolved_commit = git_in(&repo, &["rev-parse", "HEAD"]);
        let plan = with_requested_config(
            plan_for_upstream(&repo.join(".git"), &resolved_commit, &output),
            &project.join("kslim.toml"),
        );
        let workspace = WorkspacePaths::new(&output).unwrap();

        let err = build_candidate_tree(&plan, &workspace).unwrap_err();

        assert_candidate_failure_stage(&err, GenerateStage::Materialize);
        assert!(
            format!("{err:#}").contains("candidate mutation target aliases resolved output path")
        );
        assert!(!output.exists());
    }

    #[test]
    fn test_candidate_mutation_target_rejects_output_path_parent_or_child_alias() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        let plan = plan_for_upstream(&tmp.path().join("upstream/.git"), "deadbeef", &output);
        let sibling = tmp.path().join("candidate");

        let target = ensure_candidate_mutation_target(&plan, &sibling).unwrap();
        assert_eq!(target.as_path(), sibling.as_path());

        let output_child = output.join("child");
        for candidate in [&output, &output_child, tmp.path()] {
            let err = ensure_candidate_mutation_target(&plan, candidate)
                .unwrap_err()
                .to_string();
            assert!(err.contains("candidate mutation target aliases resolved output path"));
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_candidate_mutation_target_rejects_symlink_alias_to_output_path() {
        use std::os::unix::fs::symlink;

        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        std::fs::create_dir_all(&output).unwrap();

        let candidate_link = tmp.path().join("candidate-link");
        symlink(&output, &candidate_link).unwrap();
        let plan = plan_for_upstream(&tmp.path().join("upstream/.git"), "deadbeef", &output);
        let err = ensure_candidate_mutation_target(&plan, &candidate_link)
            .unwrap_err()
            .to_string();
        assert!(err.contains("candidate mutation target aliases resolved output path"));

        let candidate = tmp.path().join("candidate");
        std::fs::create_dir_all(&candidate).unwrap();
        let output_link = tmp.path().join("output-link");
        symlink(&candidate, &output_link).unwrap();
        let plan =
            plan_for_upstream(&tmp.path().join("upstream/.git"), "deadbeef", &output_link);
        let err = ensure_candidate_mutation_target(&plan, &candidate)
            .unwrap_err()
            .to_string();
        assert!(err.contains("candidate mutation target aliases resolved output path"));
    }

    #[test]
    fn test_build_candidate_tree_does_not_open_existing_output_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("upstream");
        let output = tmp.path().join("output");
        std::fs::create_dir_all(&repo).unwrap();
        create_minimal_tree(&repo);
        git_in(&repo, &["init"]);
        git_in(&repo, &["config", "user.email", "test@kslim.local"]);
        git_in(&repo, &["config", "user.name", "kslim test"]);
        git_in(&repo, &["add", "-A"]);
        git_in(&repo, &["commit", "-m", "initial"]);
        let resolved_commit = git_in(&repo, &["rev-parse", "HEAD"]);
        std::fs::create_dir_all(output.join(".git")).unwrap();
        std::fs::write(output.join(".git/config"), "not a real git config\n").unwrap();
        std::fs::write(output.join("sentinel.txt"), "unchanged\n").unwrap();
        let plan = plan_for_upstream(&repo.join(".git"), &resolved_commit, &output);
        let workspace = WorkspacePaths::new_isolated_temp().unwrap();

        build_candidate_tree(&plan, &workspace).unwrap();

        assert_eq!(
            std::fs::read_to_string(output.join(".git/config")).unwrap(),
            "not a real git config\n"
        );
        assert_eq!(
            std::fs::read_to_string(output.join("sentinel.txt")).unwrap(),
            "unchanged\n"
        );
        assert!(!output.join(".git/kslim").exists());
        assert!(!output.join(".kslim").exists());
    }

    #[test]
    fn test_build_candidate_tree_does_not_update_project_lockfile() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("upstream");
        let output = tmp.path().join("output");
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::create_dir_all(&project).unwrap();
        create_minimal_tree(&repo);
        git_in(&repo, &["init"]);
        git_in(&repo, &["config", "user.email", "test@kslim.local"]);
        git_in(&repo, &["config", "user.name", "kslim test"]);
        git_in(&repo, &["add", "-A"]);
        git_in(&repo, &["commit", "-m", "initial"]);
        let resolved_commit = git_in(&repo, &["rev-parse", "HEAD"]);
        let lockfile = project.join("kslim.lock");
        std::fs::write(&lockfile, "authoritative-lockfile-sentinel\n").unwrap();
        let plan = with_requested_config(
            plan_for_upstream(&repo.join(".git"), &resolved_commit, &output),
            &project.join("kslim.toml"),
        );
        let workspace = WorkspacePaths::new_isolated_temp().unwrap();

        build_candidate_tree(&plan, &workspace).unwrap();

        assert_eq!(
            std::fs::read_to_string(&lockfile).unwrap(),
            "authoritative-lockfile-sentinel\n"
        );
        assert!(!output.exists());
    }

    #[test]
    fn test_candidate_tree_state_rejects_metadata_outside_candidate_tree_before_write() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let outside_metadata = tmp.path().join("outside/.kslim");
        let err = CandidateTreeState::new(
            CandidateTreePath::new(&tree).unwrap(),
            CandidateMetadataDir::new(&outside_metadata).unwrap(),
            true,
            false,
            false,
            false,
            false,
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("candidate metadata dir is not the candidate tree metadata dir"));
        assert!(!outside_metadata.exists());
    }

    #[test]
    fn test_build_candidate_tree_applies_patch_stack_in_resolved_order() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("upstream");
        let output = tmp.path().join("output");
        std::fs::create_dir_all(&repo).unwrap();
        create_minimal_tree(&repo);
        git_in(&repo, &["init"]);
        git_in(&repo, &["config", "user.email", "test@kslim.local"]);
        git_in(&repo, &["config", "user.name", "kslim test"]);
        git_in(&repo, &["add", "-A"]);
        git_in(&repo, &["commit", "-m", "initial"]);
        let resolved_commit = git_in(&repo, &["rev-parse", "HEAD"]);

        let first = create_patch_repo(
            &tmp.path().join("patch-first"),
            &[],
            &[("ordered.txt", "first\n")],
        );
        let second = create_patch_repo(
            &tmp.path().join("patch-second"),
            &[("ordered.txt", "first\n")],
            &[("ordered.txt", "first\nsecond\n")],
        );
        let patch_infos = vec![first, second];
        let plan = plan_for_upstream_with_patches(
            &repo.join(".git"),
            &resolved_commit,
            &output,
            Some(&patch_infos),
        );
        let workspace = WorkspacePaths::new_isolated_temp().unwrap();

        let state = build_candidate_tree(&plan, &workspace).unwrap();

        assert_eq!(state.tree.as_path(), workspace.candidate_tree());
        assert_eq!(
            std::fs::read_to_string(workspace.candidate_tree().join("ordered.txt")).unwrap(),
            "first\nsecond\n"
        );
        assert!(!output.exists());
    }

    #[test]
    fn test_build_candidate_tree_applies_integrations_in_resolved_order() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("upstream");
        let output = tmp.path().join("output");
        std::fs::create_dir_all(&repo).unwrap();
        create_minimal_tree_with_realtek(&repo);
        git_in(&repo, &["init"]);
        git_in(&repo, &["config", "user.email", "test@kslim.local"]);
        git_in(&repo, &["config", "user.name", "kslim test"]);
        git_in(&repo, &["add", "-A"]);
        git_in(&repo, &["commit", "-m", "initial"]);
        let resolved_commit = git_in(&repo, &["rev-parse", "HEAD"]);

        let rtlmq_source = tmp.path().join("rtlmq-source");
        create_rtlmq_source(&rtlmq_source);
        let mut profile = config::default_profile_config("v1.0");
        profile.integrations.rtlmq = Some(config::RtlmqIntegrationConfig {
            source: rtlmq_source.to_string_lossy().to_string(),
            tests_source: None,
        });
        let plan = plan_for_upstream_with_profile(
            &repo.join(".git"),
            &resolved_commit,
            &output,
            profile,
            None,
        );
        let workspace = WorkspacePaths::new_isolated_temp().unwrap();

        let state = build_candidate_tree(&plan, &workspace).unwrap();

        let target = workspace
            .candidate_tree()
            .join("drivers/net/ethernet/realtek/rtlmq");
        assert!(state.integrated);
        assert!(target.join("Makefile").is_file());
        assert!(target.join("Kconfig").is_file());
        assert_eq!(
            std::fs::read_to_string(target.join("rtlmq.c")).unwrap(),
            "int rtlmq;\n"
        );
        assert!(std::fs::read_to_string(
            workspace
                .candidate_tree()
                .join("drivers/net/ethernet/realtek/Kconfig")
        )
        .unwrap()
        .contains(r#"source "drivers/net/ethernet/realtek/rtlmq/Kconfig""#));
        assert!(std::fs::read_to_string(
            workspace
                .candidate_tree()
                .join("drivers/net/ethernet/realtek/Makefile")
        )
        .unwrap()
        .contains("obj-$(CONFIG_RTLMQ) += rtlmq/"));
        assert!(!output.exists());
    }

    #[test]
    fn test_build_candidate_tree_runs_resolved_path_pruning() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("upstream");
        let output = tmp.path().join("output");
        std::fs::create_dir_all(&repo).unwrap();
        create_minimal_tree(&repo);
        std::fs::create_dir_all(repo.join("drivers/foo")).unwrap();
        std::fs::write(repo.join("drivers/foo/remove.c"), "int remove;\n").unwrap();
        std::fs::write(repo.join("drivers/foo/keep.c"), "int keep;\n").unwrap();
        git_in(&repo, &["init"]);
        git_in(&repo, &["config", "user.email", "test@kslim.local"]);
        git_in(&repo, &["config", "user.name", "kslim test"]);
        git_in(&repo, &["add", "-A"]);
        git_in(&repo, &["commit", "-m", "initial"]);
        let resolved_commit = git_in(&repo, &["rev-parse", "HEAD"]);

        let mut profile = config::default_profile_config("v1.0");
        profile.slim = Some(config::SlimConfig {
            remove_paths: vec![String::from("drivers/foo/remove.c")],
            remove_configs: Vec::new(),
            set_defaults: Default::default(),
            unsafe_allow_root_path_removal: false,
        });
        let plan = plan_for_upstream_with_profile(
            &repo.join(".git"),
            &resolved_commit,
            &output,
            profile,
            None,
        );
        let workspace = WorkspacePaths::new_isolated_temp().unwrap();

        let state = build_candidate_tree(&plan, &workspace).unwrap();

        assert!(state.pruned);
        assert!(!workspace
            .candidate_tree()
            .join("drivers/foo/remove.c")
            .exists());
        assert!(workspace
            .candidate_tree()
            .join("drivers/foo/keep.c")
            .exists());
        assert!(!output.exists());
    }

    #[test]
    fn test_build_candidate_tree_runs_resolved_reducer() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("upstream");
        let output = tmp.path().join("output");
        std::fs::create_dir_all(&repo).unwrap();
        create_minimal_tree(&repo);
        std::fs::write(
            repo.join("Kconfig"),
            concat!(
                "config REMOVED\n",
                "\tbool \"Removed\"\n",
                "\n",
                "config LIVE\n",
                "\tbool \"Live\"\n",
                "\tdepends on REMOVED || OTHER\n",
            ),
        )
        .unwrap();
        git_in(&repo, &["init"]);
        git_in(&repo, &["config", "user.email", "test@kslim.local"]);
        git_in(&repo, &["config", "user.name", "kslim test"]);
        git_in(&repo, &["add", "-A"]);
        git_in(&repo, &["commit", "-m", "initial"]);
        let resolved_commit = git_in(&repo, &["rev-parse", "HEAD"]);

        let mut profile = config::default_profile_config("v1.0");
        profile.slim = Some(config::SlimConfig {
            remove_paths: Vec::new(),
            remove_configs: vec![String::from("REMOVED")],
            set_defaults: Default::default(),
            unsafe_allow_root_path_removal: false,
        });
        let plan = plan_for_upstream_with_profile(
            &repo.join(".git"),
            &resolved_commit,
            &output,
            profile,
            None,
        );
        let workspace = WorkspacePaths::new_isolated_temp().unwrap();

        let state = build_candidate_tree(&plan, &workspace).unwrap();

        assert!(!state.pruned);
        assert!(state.reduced);
        assert!(state
            .metadata_dir
            .as_path()
            .join(crate::output_repo::REDUCER_REPORT_JSON)
            .exists());
        assert!(state
            .metadata_dir
            .as_path()
            .join(crate::output_repo::REDUCER_KCONFIG_SOLVER_REPORT_JSON)
            .exists());
        assert!(state
            .metadata_dir
            .as_path()
            .join(crate::output_repo::REDUCER_KCONFIG_REWRITE_REPORT_JSON)
            .exists());
        let kconfig = std::fs::read_to_string(workspace.candidate_tree().join("Kconfig")).unwrap();
        assert!(kconfig.contains("# kslim: removed config REMOVED"));
        assert!(kconfig.contains("\tdepends on OTHER"));
        let candidate_metadata =
            std::fs::read_to_string(state.metadata_dir.as_path().join(CANDIDATE_METADATA_FILE))
                .unwrap();
        assert!(candidate_metadata.contains("reducer_ran = true"));
        assert!(!output.exists());
    }

    #[test]
    fn test_apply_integration_plan_rejects_unknown_resolved_entry_before_later_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("upstream");
        let output = tmp.path().join("output");
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(&repo).unwrap();
        create_minimal_tree_with_realtek(&repo);
        git_in(&repo, &["init"]);
        git_in(&repo, &["config", "user.email", "test@kslim.local"]);
        git_in(&repo, &["config", "user.name", "kslim test"]);
        git_in(&repo, &["add", "-A"]);
        git_in(&repo, &["commit", "-m", "initial"]);
        let resolved_commit = git_in(&repo, &["rev-parse", "HEAD"]);

        let rtlmq_source = tmp.path().join("rtlmq-source");
        create_rtlmq_source(&rtlmq_source);
        let mut profile = config::default_profile_config("v1.0");
        profile.integrations.rtlmq = Some(config::RtlmqIntegrationConfig {
            source: rtlmq_source.to_string_lossy().to_string(),
            tests_source: None,
        });
        let mut plan = with_requested_config(
            plan_for_upstream_with_profile(
                &repo.join(".git"),
                &resolved_commit,
                &output,
                profile,
                None,
            ),
            &project.join("kslim.toml"),
        );
        plan.resolved.integration_plan.entries.insert(
            0,
            IntegrationEntryPlan {
                stable_id: String::from("integration-unknown"),
                kind: String::from("unknown"),
            },
        );
        let workspace = WorkspacePaths::new_isolated_temp().unwrap();

        let err = build_candidate_tree(&plan, &workspace).unwrap_err();

        assert_candidate_failure_stage(&err, GenerateStage::Integrate);
        assert!(format!("{err:#}").contains("unsupported resolved integration kind: unknown"));
        assert!(!workspace
            .candidate_tree()
            .join("drivers/net/ethernet/realtek/rtlmq")
            .exists());
        assert!(!output.exists());
    }

    #[test]
    fn test_build_candidate_tree_records_prune_stage_on_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("upstream");
        let output = tmp.path().join("output");
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(&repo).unwrap();
        create_minimal_tree(&repo);
        git_in(&repo, &["init"]);
        git_in(&repo, &["config", "user.email", "test@kslim.local"]);
        git_in(&repo, &["config", "user.name", "kslim test"]);
        git_in(&repo, &["add", "-A"]);
        git_in(&repo, &["commit", "-m", "initial"]);
        let resolved_commit = git_in(&repo, &["rev-parse", "HEAD"]);
        let mut plan = with_requested_config(
            plan_for_upstream(&repo.join(".git"), &resolved_commit, &output),
            &project.join("kslim.toml"),
        );
        plan.resolved.prune_plan.remove_paths =
            vec![RelativeKernelPath::new("missing/path.c").unwrap()];
        plan.resolved.reducer_plan.fail_on_missing_prune_paths = true;
        let workspace = WorkspacePaths::new_isolated_temp().unwrap();

        let err = build_candidate_tree(&plan, &workspace).unwrap_err();

        assert_candidate_failure_stage(&err, GenerateStage::Prune);
        assert!(format!("{err:#}").contains("missing_path"));
        assert!(format!("{err:#}").contains("missing/path.c"));
        assert!(!output.exists());
    }

    #[test]
    fn test_build_candidate_tree_records_reduce_stage_on_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("upstream");
        let output = tmp.path().join("output");
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(&repo).unwrap();
        create_minimal_tree(&repo);
        std::fs::write(
            repo.join("Kconfig"),
            "config FOO\n\tbool \"Foo\"\n\tdepends on REMOVED + LIVE\n",
        )
        .unwrap();
        git_in(&repo, &["init"]);
        git_in(&repo, &["config", "user.email", "test@kslim.local"]);
        git_in(&repo, &["config", "user.name", "kslim test"]);
        git_in(&repo, &["add", "-A"]);
        git_in(&repo, &["commit", "-m", "initial"]);
        let resolved_commit = git_in(&repo, &["rev-parse", "HEAD"]);
        let mut profile = config::default_profile_config("v1.0");
        profile.slim = Some(config::SlimConfig {
            remove_paths: Vec::new(),
            remove_configs: vec![String::from("REMOVED")],
            set_defaults: Default::default(),
            unsafe_allow_root_path_removal: false,
        });
        let plan = with_requested_config(
            plan_for_upstream_with_profile(
                &repo.join(".git"),
                &resolved_commit,
                &output,
                profile,
                None,
            ),
            &project.join("kslim.toml"),
        );
        let workspace = WorkspacePaths::new_isolated_temp().unwrap();
        let attempt_dir = project.join(KSLIM_METADATA_DIR).join(ATTEMPT_METADATA_DIR);

        let err = build_candidate_tree(&plan, &workspace).unwrap_err();
        let failure = candidate_failure_stage(&err, GenerateStage::Reduce);

        assert!(format!("{err:#}").contains("unsupported Kconfig expressions"));
        assert!(!failure.partial_reports.is_empty());
        assert!(failure
            .partial_reports
            .iter()
            .all(|report| report.as_path().starts_with(&attempt_dir)));
        assert!(attempt_dir.join(output_repo::REDUCER_REPORT_JSON).exists());
        assert!(attempt_dir
            .join(output_repo::REDUCER_KCONFIG_SOLVER_REPORT_JSON)
            .exists());
        assert!(attempt_dir
            .join(output_repo::REDUCER_KCONFIG_REWRITE_REPORT_JSON)
            .exists());
        assert!(attempt_dir
            .join(output_repo::REDUCER_DIAGNOSTICS_JSON)
            .exists());
        assert!(!workspace
            .candidate_tree()
            .join(KSLIM_METADATA_DIR)
            .join(output_repo::REDUCER_REPORT_JSON)
            .exists());
        assert!(!output.exists());
    }

    #[test]
    fn test_failed_candidate_build_leaves_existing_output_repo_untouched() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("upstream");
        let output = tmp.path().join("output");
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(&repo).unwrap();
        create_minimal_tree(&repo);
        std::fs::write(
            repo.join("Kconfig"),
            "config FOO\n\tbool \"Foo\"\n\tdepends on REMOVED + LIVE\n",
        )
        .unwrap();
        git_in(&repo, &["init"]);
        git_in(&repo, &["config", "user.email", "test@kslim.local"]);
        git_in(&repo, &["config", "user.name", "kslim test"]);
        git_in(&repo, &["add", "-A"]);
        git_in(&repo, &["commit", "-m", "initial"]);
        let resolved_commit = git_in(&repo, &["rev-parse", "HEAD"]);
        std::fs::create_dir_all(output.join(".git")).unwrap();
        std::fs::write(output.join(".git/config"), "output config\n").unwrap();
        std::fs::write(output.join("sentinel.txt"), "do not touch\n").unwrap();
        let mut profile = config::default_profile_config("v1.0");
        profile.slim = Some(config::SlimConfig {
            remove_paths: Vec::new(),
            remove_configs: vec![String::from("REMOVED")],
            set_defaults: Default::default(),
            unsafe_allow_root_path_removal: false,
        });
        let plan = with_requested_config(
            plan_for_upstream_with_profile(
                &repo.join(".git"),
                &resolved_commit,
                &output,
                profile,
                None,
            ),
            &project.join("kslim.toml"),
        );
        let workspace = WorkspacePaths::new_isolated_temp().unwrap();

        let err = build_candidate_tree(&plan, &workspace).unwrap_err();
        let failure = candidate_failure_stage(&err, GenerateStage::Reduce);

        assert!(format!("{err:#}").contains("unsupported Kconfig expressions"));
        assert!(!failure.partial_reports.is_empty());
        assert!(project
            .join(KSLIM_METADATA_DIR)
            .join(ATTEMPT_METADATA_DIR)
            .join(CANDIDATE_FAILURE_ATTEMPT_FILE)
            .exists());
        assert_eq!(
            std::fs::read_to_string(output.join(".git/config")).unwrap(),
            "output config\n"
        );
        assert_eq!(
            std::fs::read_to_string(output.join("sentinel.txt")).unwrap(),
            "do not touch\n"
        );
        assert!(!output.join(".git/kslim").exists());
        assert!(!output.join(KSLIM_METADATA_DIR).exists());
    }

    #[test]
    fn test_candidate_failure_attempt_metadata_rejects_output_alias() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("upstream");
        let output = tmp.path().join("output");
        std::fs::create_dir_all(&repo).unwrap();
        create_minimal_tree(&repo);
        git_in(&repo, &["init"]);
        git_in(&repo, &["config", "user.email", "test@kslim.local"]);
        git_in(&repo, &["config", "user.name", "kslim test"]);
        git_in(&repo, &["add", "-A"]);
        git_in(&repo, &["commit", "-m", "initial"]);
        std::fs::create_dir_all(output.join(".git")).unwrap();
        std::fs::write(output.join(".git/config"), "output config\n").unwrap();
        std::fs::write(output.join("sentinel.txt"), "do not touch\n").unwrap();
        let plan = with_requested_config(
            plan_for_upstream(&repo.join(".git"), "missing-commit", &output),
            &output.join("kslim.toml"),
        );
        let workspace = WorkspacePaths::new_isolated_temp().unwrap();

        let err = build_candidate_tree(&plan, &workspace).unwrap_err();

        assert!(
            format!("{err:#}").contains("candidate attempt metadata aliases resolved output path")
        );
        assert_eq!(
            std::fs::read_to_string(output.join(".git/config")).unwrap(),
            "output config\n"
        );
        assert_eq!(
            std::fs::read_to_string(output.join("sentinel.txt")).unwrap(),
            "do not touch\n"
        );
        assert!(!output.join(KSLIM_METADATA_DIR).exists());
        assert!(!output.join(".git/kslim").exists());
    }

    #[test]
    fn test_candidate_failure_attempt_metadata_sorts_partial_report_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let output = tmp.path().join("output");
        let plan = with_requested_config(
            plan_for_upstream(&tmp.path().join("upstream.git"), "deadbeef", &output),
            &project.join("kslim.toml"),
        );
        let attempt_dir = project.join(KSLIM_METADATA_DIR).join(ATTEMPT_METADATA_DIR);
        let partial_reports = vec![
            crate::model::ReportPath::new(attempt_dir.join(output_repo::REDUCER_REPORT_MD))
                .unwrap(),
            crate::model::ReportPath::new(attempt_dir.join(output_repo::REDUCER_DIAGNOSTICS_JSON))
                .unwrap(),
            crate::model::ReportPath::new(attempt_dir.join(output_repo::REDUCER_REPORT_MD))
                .unwrap(),
            crate::model::ReportPath::new(attempt_dir.join(output_repo::REDUCER_EDIT_SUMMARY_JSON))
                .unwrap(),
        ];

        let reports = write_candidate_failure_attempt_metadata(
            &plan,
            GenerateStage::Reduce,
            "reducer failed",
            &partial_reports,
        )
        .unwrap();

        assert_report_paths_sorted_unique(&reports);
        assert_eq!(
            report_path_names(&reports, &attempt_dir),
            [
                CANDIDATE_FAILURE_ATTEMPT_FILE,
                output_repo::REDUCER_DIAGNOSTICS_JSON,
                output_repo::REDUCER_EDIT_SUMMARY_JSON,
                output_repo::REDUCER_REPORT_MD
            ]
        );
        let metadata =
            std::fs::read_to_string(attempt_dir.join(CANDIDATE_FAILURE_ATTEMPT_FILE)).unwrap();
        let decoded: CandidateFailureStageFixture = toml::from_str(&metadata).unwrap();
        assert_eq!(decoded.stage, GenerateStage::Reduce);
        let metadata_value: toml::Value = toml::from_str(&metadata).unwrap();
        let partial_report_names = metadata_value["partial_reports"]
            .as_array()
            .unwrap()
            .iter()
            .map(|path| path.as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            partial_report_names,
            [
                output_repo::REDUCER_DIAGNOSTICS_JSON,
                output_repo::REDUCER_EDIT_SUMMARY_JSON,
                output_repo::REDUCER_REPORT_MD
            ]
        );
    }

    #[test]
    fn test_candidate_failure_attempt_metadata_rejects_legacy_stage_aliases() {
        for legacy_stage in [
            "prepare",
            "source",
            "lockfile",
            "reducer",
            "verify",
            "output-commit",
            "output-publish",
        ] {
            let decoded = toml::from_str::<CandidateFailureStageFixture>(&format!(
                "stage = \"{}\"\n",
                legacy_stage
            ));
            assert!(
                decoded.is_err(),
                "candidate failure attempt metadata must reject legacy stage alias: {}",
                legacy_stage
            );
        }
    }

    #[test]
    fn test_partial_candidate_reports_require_attempt_metadata_dir_type() {
        let tmp = tempfile::tempdir().unwrap();
        let candidate_metadata = tmp.path().join("candidate").join(KSLIM_METADATA_DIR);

        let err = AttemptMetadataDir::new(&candidate_metadata)
            .unwrap_err()
            .to_string();

        assert!(err.contains("non-authoritative attempt dir"));
        assert!(!candidate_metadata.exists());
    }

    #[test]
    fn test_partial_candidate_report_paths_are_sorted() {
        let tmp = tempfile::tempdir().unwrap();
        let attempt_dir = tmp.path().join("project/.kslim/attempt");
        let candidate_metadata = AttemptMetadataDir::new(attempt_dir.clone()).unwrap();
        let stats = reducer::ReducerStats {
            ran: true,
            ..Default::default()
        };

        let reports = record_partial_candidate_reducer_reports_at_dir(
            &candidate_metadata,
            &stats,
            &ReducerConfig::default(),
            &RemovalManifest::default(),
        )
        .unwrap();

        assert_report_paths_sorted_unique(&reports);
        assert_eq!(
            report_path_names(&reports, &attempt_dir),
            [
                output_repo::REDUCER_DIAGNOSTICS_JSON,
                output_repo::REDUCER_EDIT_SUMMARY_JSON,
                output_repo::REDUCER_KCONFIG_REWRITE_REPORT_JSON,
                output_repo::REDUCER_KCONFIG_SOLVER_REPORT_JSON,
                output_repo::REDUCER_REPORT_JSON,
                output_repo::REDUCER_REPORT_MD
            ]
        );
    }

    #[test]
    fn test_build_candidate_tree_records_metadata_stage_on_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("upstream");
        let output = tmp.path().join("output");
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(&repo).unwrap();
        create_minimal_tree(&repo);
        std::fs::write(repo.join(".kslim"), "not a metadata directory\n").unwrap();
        git_in(&repo, &["init"]);
        git_in(&repo, &["config", "user.email", "test@kslim.local"]);
        git_in(&repo, &["config", "user.name", "kslim test"]);
        git_in(&repo, &["add", "-A"]);
        git_in(&repo, &["commit", "-m", "initial"]);
        let resolved_commit = git_in(&repo, &["rev-parse", "HEAD"]);
        let plan = with_requested_config(
            plan_for_upstream(&repo.join(".git"), &resolved_commit, &output),
            &project.join("kslim.toml"),
        );
        let workspace = WorkspacePaths::new_isolated_temp().unwrap();

        let err = build_candidate_tree(&plan, &workspace).unwrap_err();

        assert_candidate_failure_stage(&err, GenerateStage::Metadata);
        assert!(format!("{err:#}").contains("candidate build failed during metadata stage"));
        assert!(!output.exists());
    }

    #[test]
    fn test_build_candidate_tree_rejects_non_empty_workspace() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("upstream");
        let output = tmp.path().join("output");
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(&repo).unwrap();
        create_minimal_tree(&repo);
        git_in(&repo, &["init"]);
        git_in(&repo, &["config", "user.email", "test@kslim.local"]);
        git_in(&repo, &["config", "user.name", "kslim test"]);
        git_in(&repo, &["add", "-A"]);
        git_in(&repo, &["commit", "-m", "initial"]);
        let commit = git_in(&repo, &["rev-parse", "HEAD"]);
        let plan = with_requested_config(
            plan_for_upstream(&repo.join(".git"), &commit, &output),
            &project.join("kslim.toml"),
        );
        let candidate = tmp.path().join("candidate");
        std::fs::create_dir_all(&candidate).unwrap();
        std::fs::write(candidate.join("stale"), "do not overwrite\n").unwrap();
        let workspace = WorkspacePaths::new(&candidate).unwrap();

        let err = build_candidate_tree(&plan, &workspace).unwrap_err();

        assert_candidate_failure_stage(&err, GenerateStage::Materialize);
        assert!(format!("{err:#}").contains("candidate workspace is not empty"));
        assert_eq!(
            std::fs::read_to_string(candidate.join("stale")).unwrap(),
            "do not overwrite\n"
        );
        assert!(!output.exists());
    }

    #[test]
    fn test_apply_patch_sources_is_noop_without_patch_config() {
        let profile = crate::config::default_profile_config("v1.0");
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        let candidate = tmp.path().join("candidate");
        let plan = plan_for_upstream(&tmp.path().join("upstream/.git"), "deadbeef", &output);
        let target = mutation_target_for(&plan, &candidate);

        let infos = apply_patch_sources(&profile, &target).unwrap();

        assert!(infos.is_none());
    }

    #[test]
    fn test_apply_integrations_is_noop_without_integration_config() {
        let profile = crate::config::default_profile_config("v1.0");
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        let candidate = tmp.path().join("candidate");
        let plan = plan_for_upstream(&tmp.path().join("upstream/.git"), "deadbeef", &output);
        let target = mutation_target_for(&plan, &candidate);

        apply_integrations(&profile, &target).unwrap();
    }

    #[test]
    fn test_reduce_tree_removes_slim_declared_candidate_path() {
        let tmp = tempfile::tempdir().unwrap();
        let candidate = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        create_minimal_tree(&candidate);
        std::fs::create_dir_all(candidate.join("drivers/foo")).unwrap();
        std::fs::write(candidate.join("drivers/foo/remove.c"), "int remove;\n").unwrap();

        let mut profile = config::default_profile_config("v1.0");
        profile.slim = Some(config::SlimConfig {
            remove_paths: vec!["drivers/foo/remove.c".to_string()],
            remove_configs: Vec::new(),
            set_defaults: Default::default(),
            unsafe_allow_root_path_removal: false,
        });
        let plan = plan_for_upstream(&tmp.path().join("upstream/.git"), "deadbeef", &output);
        let target = mutation_target_for(&plan, &candidate);

        let stats = reduce_tree(&target, &profile).unwrap();

        assert!(stats.ran);
        assert_eq!(stats.files_removed, 1);
        assert!(!candidate.join("drivers/foo/remove.c").exists());
        assert!(!output.exists());
    }
}
