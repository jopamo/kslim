#[cfg(test)]
use anyhow::Result;
#[cfg(test)]
use std::collections::BTreeMap;
#[cfg(test)]
use std::io;
#[cfg(test)]
use crate::edit_reason::EditProofSource;
#[cfg(test)]
use crate::removal_manifest::RemovalManifest;

mod path;
mod orphan;
mod report;
mod semantic;
mod stale_reference;

pub(in crate::prune) use orphan::cleanup_empty_parent_chain;
#[cfg(test)]
pub(in crate::prune) use path::{failed_removal_kind_from_io_error, prune_declared_paths};
#[allow(unused_imports)]
pub use path::{
    DeclaredPathPruneResult, FailedRemoval, FailedRemovalKind, PruneResult, PrunedPath,
    RemovalAccounting, RemovalFailurePolicy,
};
pub(crate) use path::prune_declared_paths_from_manifest_with_policy;
pub(in crate::prune) use path::{
    normalize_and_sort_symbols, normalize_relative_path, RemovedArtifact,
};
pub(crate) use path::prune_declared_paths_from_manifest;
#[allow(unused_imports)]
pub use report::{prune_tree_from_manifest, PruneStats};
pub(crate) use report::continue_prune_after_kconfig;
pub(in crate::prune) use semantic::effective_removed_config_symbols_for_abi_policy;
pub(crate) use semantic::{rewrite_kconfig_stage, KconfigPruneStageResult};
pub(in crate::prune) use stale_reference::{rewrite_build_graph, rewrite_kconfig_sources};


#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SlimConfig;
    use crate::edit_reason::{EditReason, LineRange};
    use std::path::{Path, PathBuf};

    fn prune_tree(root: &str, slim: &SlimConfig) -> Result<PruneStats> {
        let root_path = Path::new(root);
        let manifest = RemovalManifest::from_slim_config_for_tree(root_path, slim)?;
        prune_tree_from_manifest(root, &manifest)
    }

    #[test]
    fn test_prune_single_file_rewrites_makefile_object_ref() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("drivers/gpu/drm/amd")).unwrap();
        std::fs::write(root.join("drivers/gpu/drm/amd/amdgpu_drv.c"), "code").unwrap();
        std::fs::write(root.join("drivers/gpu/drm/amd/helper.c"), "helper").unwrap();
        std::fs::write(
            root.join("drivers/gpu/drm/amd/Makefile"),
            "amdgpu-y := amdgpu_drv.o \\\n helper.o\n",
        )
        .unwrap();

        let slim = SlimConfig {
            remove_paths: vec!["drivers/gpu/drm/amd/amdgpu_drv.c".to_string()],
            remove_configs: vec![],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let stats = prune_tree(root.to_str().unwrap(), &slim).unwrap();

        assert_eq!(stats.files_removed, 1);
        assert_eq!(stats.makefile_refs_removed, 1);
        assert_eq!(
            stats.removal.removed_files,
            vec![PathBuf::from("drivers/gpu/drm/amd/amdgpu_drv.c")]
        );
        assert!(!root.join("drivers/gpu/drm/amd/amdgpu_drv.c").exists());
        let makefile = std::fs::read_to_string(root.join("drivers/gpu/drm/amd/Makefile")).unwrap();
        assert!(!makefile.contains("amdgpu_drv.o"));
        assert!(makefile.contains("helper.o"));
    }

    #[test]
    fn test_prune_declared_paths_removes_declared_tree_and_records_manifest_edits() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("drivers/gpu/drm/amd/amdgpu")).unwrap();
        std::fs::write(
            root.join("drivers/gpu/drm/amd/amdgpu/amdgpu_drv.c"),
            "code\n",
        )
        .unwrap();
        std::fs::write(root.join("drivers/gpu/drm/amd/amdgpu/helper.c"), "helper\n").unwrap();

        let declared = prune_declared_paths(
            root,
            &[PathBuf::from("drivers/gpu/drm/amd/amdgpu")],
            RemovalFailurePolicy::default(),
        )
        .unwrap();

        assert_eq!(declared.files_removed, 2);
        assert!(declared.dirs_removed >= 1);
        assert!(!root.join("drivers/gpu/drm/amd/amdgpu").exists());
        assert_eq!(
            declared.removal.removed_files,
            vec![
                PathBuf::from("drivers/gpu/drm/amd/amdgpu/amdgpu_drv.c"),
                PathBuf::from("drivers/gpu/drm/amd/amdgpu/helper.c"),
            ]
        );
        assert!(declared.removed_artifacts.iter().any(|artifact| {
            artifact.relative.as_path() == Path::new("drivers/gpu/drm/amd/amdgpu")
                && artifact.is_dir
        }));
        assert_eq!(declared.result.failed, Vec::<FailedRemoval>::new());
        assert_eq!(declared.result.edits, declared.edits);
        assert!(declared.result.removed.iter().any(|removed| {
            removed.path.as_path() == Path::new("drivers/gpu/drm/amd/amdgpu") && removed.is_dir
        }));
        assert!(declared.result.removed.iter().any(|removed| {
            removed.path.as_path() == Path::new("drivers/gpu/drm/amd/amdgpu/amdgpu_drv.c")
                && !removed.is_dir
        }));
        assert!(declared.edits.iter().any(|edit| matches!(
            edit.reason,
            EditReason::ManifestPath { ref path }
                if path == Path::new("drivers/gpu/drm/amd/amdgpu")
        )));
    }

    #[test]
    fn test_prune_removed_path_lists_are_sorted() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("remove/zdir")).unwrap();
        std::fs::create_dir_all(root.join("remove/adir")).unwrap();
        std::fs::write(root.join("remove/zdir/z.c"), "z\n").unwrap();
        std::fs::write(root.join("remove/adir/a.c"), "a\n").unwrap();

        let declared = prune_declared_paths(
            root,
            &[PathBuf::from("remove")],
            RemovalFailurePolicy::default(),
        )
        .unwrap();

        assert_eq!(
            declared.removal.removed_files,
            vec![
                PathBuf::from("remove/adir/a.c"),
                PathBuf::from("remove/zdir/z.c"),
            ]
        );
        assert_eq!(
            declared.removal.removed_dirs,
            vec![
                PathBuf::from("remove"),
                PathBuf::from("remove/adir"),
                PathBuf::from("remove/zdir"),
            ]
        );
        assert_eq!(
            declared
                .removed_artifacts
                .iter()
                .map(|artifact| artifact.relative.clone())
                .collect::<Vec<_>>(),
            vec![
                PathBuf::from("remove"),
                PathBuf::from("remove/adir"),
                PathBuf::from("remove/adir/a.c"),
                PathBuf::from("remove/zdir"),
                PathBuf::from("remove/zdir/z.c"),
            ]
        );
    }

    #[test]
    fn test_prune_from_manifest_preserves_named_feature_root_under_broad_removal() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("drivers/remove")).unwrap();
        std::fs::create_dir_all(root.join("drivers/keep")).unwrap();
        std::fs::write(root.join("drivers/remove/old.c"), "old\n").unwrap();
        std::fs::write(root.join("drivers/keep/live.c"), "live\n").unwrap();
        std::fs::write(root.join("drivers/top.c"), "top\n").unwrap();

        let slim = SlimConfig {
            remove_paths: vec![String::from("drivers")],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };
        let preservation = crate::config::FeaturePreservationInput {
            preserve_paths: vec![String::from("drivers/keep")],
            preserve_configs: Vec::new(),
        };
        let manifest = RemovalManifest::from_slim_config_for_tree_with_abi_policy_and_preservation(
            root,
            &slim,
            Some(&preservation),
            &crate::config::AbiPolicyConfig::default(),
        )
        .unwrap();

        let stats = prune_tree_from_manifest(root.to_str().unwrap(), &manifest).unwrap();

        assert!(root.join("drivers/keep/live.c").exists());
        assert!(!root.join("drivers/remove").exists());
        assert!(!root.join("drivers/top.c").exists());
        assert_eq!(
            manifest.preserved_paths_vec(),
            vec![PathBuf::from("drivers/keep")]
        );
        assert_eq!(stats.files_removed, 2);
    }

    #[test]
    fn test_prune_preserves_uapi_headers_under_broad_manifest_directory_removal() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("include/uapi/linux")).unwrap();
        std::fs::write(root.join("include/uapi/linux/abi.h"), "uapi\n").unwrap();

        let declared = prune_declared_paths(
            root,
            &[PathBuf::from("include/uapi")],
            RemovalFailurePolicy::default(),
        )
        .unwrap();

        assert_eq!(declared.files_removed, 0);
        assert_eq!(declared.dirs_removed, 0);
        assert!(declared.edits.is_empty());
        assert!(root.join("include/uapi/linux/abi.h").exists());
        assert!(declared.removal.removed_files.is_empty());
        assert!(declared.removal.removed_dirs.is_empty());
    }

    #[test]
    fn test_prune_from_manifest_rejects_uapi_path_without_abi_policy() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("include/uapi/linux")).unwrap();
        std::fs::write(root.join("include/uapi/linux/abi.h"), "uapi\n").unwrap();
        let mut manifest = RemovalManifest::default();
        manifest
            .removed_paths
            .insert(PathBuf::from("include/uapi/linux/abi.h"));

        let err = format!(
            "{:#}",
            prune_declared_paths_from_manifest(root, &manifest).unwrap_err()
        );

        assert!(err.contains("UAPI removal requires explicit ABI policy approval"));
        assert!(err.contains("abi.allow_uapi_header_removal"));
        assert!(root.join("include/uapi/linux/abi.h").exists());
    }

    #[test]
    fn test_prune_from_manifest_requires_exact_uapi_manifest_truth() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("include/uapi/linux")).unwrap();
        std::fs::write(root.join("include/uapi/linux/abi.h"), "uapi\n").unwrap();
        let slim = SlimConfig {
            remove_paths: vec!["include/uapi".to_string()],
            remove_configs: vec![],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };
        let abi_policy = crate::config::AbiPolicyConfig {
            allow_public_header_removal: false,
            allow_uapi_header_removal: true,
        };

        let manifest =
            RemovalManifest::from_slim_config_for_tree_with_abi_policy(root, &slim, &abi_policy)
                .unwrap();
        let stats = prune_tree_from_manifest(root.to_str().unwrap(), &manifest).unwrap();

        assert_eq!(stats.files_removed, 0);
        assert_eq!(stats.dirs_removed, 0);
        assert!(root.join("include/uapi/linux/abi.h").exists());
        assert!(stats.removal.removed_files.is_empty());
        assert!(stats.removal.removed_dirs.is_empty());
    }

    #[test]
    fn test_prune_tree_rejects_exported_symbol_provider_with_live_consumer_before_removal() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
        std::fs::create_dir_all(root.join("drivers/live")).unwrap();
        std::fs::write(
            root.join("drivers/foo/provider.c"),
            "void foo_api(void) {}\nEXPORT_SYMBOL(foo_api);\n",
        )
        .unwrap();
        std::fs::write(
            root.join("drivers/live/user.c"),
            "extern void foo_api(void);\nvoid user(void) { foo_api(); }\n",
        )
        .unwrap();
        let slim = SlimConfig {
            remove_paths: vec!["drivers/foo/provider.c".to_string()],
            remove_configs: vec![],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let err = format!(
            "{:#}",
            prune_tree(root.to_str().unwrap(), &slim).unwrap_err()
        );

        assert!(err.contains("exported symbol provider removal requires proof"));
        assert!(root.join("drivers/foo/provider.c").exists());
        assert!(root.join("drivers/live/user.c").exists());
    }

    #[test]
    fn test_prune_preserves_public_headers_under_broad_manifest_directory_removal() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("include/linux")).unwrap();
        std::fs::create_dir_all(root.join("include/net")).unwrap();
        std::fs::create_dir_all(root.join("include/uapi/linux")).unwrap();
        std::fs::create_dir_all(root.join("include/drm")).unwrap();
        std::fs::write(root.join("include/linux/public.h"), "linux\n").unwrap();
        std::fs::write(root.join("include/net/public.h"), "net\n").unwrap();
        std::fs::write(root.join("include/uapi/linux/abi.h"), "uapi\n").unwrap();
        std::fs::write(root.join("include/drm/private.h"), "private\n").unwrap();

        let declared = prune_declared_paths(
            root,
            &[PathBuf::from("include")],
            RemovalFailurePolicy::default(),
        )
        .unwrap();

        assert_eq!(declared.files_removed, 1);
        assert!(root.join("include/linux/public.h").exists());
        assert!(root.join("include/net/public.h").exists());
        assert!(root.join("include/uapi/linux/abi.h").exists());
        assert!(!root.join("include/drm/private.h").exists());
        assert_eq!(
            declared.removal.removed_files,
            vec![PathBuf::from("include/drm/private.h")]
        );
    }

    #[test]
    fn test_prune_removes_uapi_header_only_when_exactly_declared() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("include/uapi/linux")).unwrap();
        std::fs::write(root.join("include/uapi/linux/abi.h"), "uapi\n").unwrap();

        let declared = prune_declared_paths(
            root,
            &[PathBuf::from("include/uapi/linux/abi.h")],
            RemovalFailurePolicy::default(),
        )
        .unwrap();

        assert_eq!(declared.files_removed, 1);
        assert_eq!(declared.dirs_removed, 0);
        assert_eq!(
            declared.removal.removed_files,
            vec![PathBuf::from("include/uapi/linux/abi.h")]
        );
        assert!(!root.join("include/uapi/linux/abi.h").exists());
        assert!(root.join("include/uapi/linux").exists());
        assert!(declared.edits.iter().any(|edit| matches!(
            edit.reason,
            EditReason::ManifestPath { ref path }
                if path == Path::new("include/uapi/linux/abi.h")
        )));
    }

    #[test]
    fn test_prune_removes_public_header_only_when_exactly_declared() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("include/linux")).unwrap();
        std::fs::create_dir_all(root.join("include/net")).unwrap();
        std::fs::write(root.join("include/linux/remove.h"), "linux\n").unwrap();
        std::fs::write(root.join("include/linux/keep.h"), "keep\n").unwrap();
        std::fs::write(root.join("include/net/remove.h"), "net\n").unwrap();

        let declared = prune_declared_paths(
            root,
            &[
                PathBuf::from("include/linux/remove.h"),
                PathBuf::from("include/net/remove.h"),
            ],
            RemovalFailurePolicy::default(),
        )
        .unwrap();

        assert_eq!(declared.files_removed, 2);
        assert!(!root.join("include/linux/remove.h").exists());
        assert!(!root.join("include/net/remove.h").exists());
        assert!(root.join("include/linux/keep.h").exists());
        assert_eq!(
            declared.removal.removed_files,
            vec![
                PathBuf::from("include/linux/remove.h"),
                PathBuf::from("include/net/remove.h"),
            ]
        );
        assert!(root.join("include/linux").exists());
        assert!(root.join("include/net").exists());
    }

    #[test]
    fn test_prune_broad_parent_removal_does_not_swallow_explicit_uapi_header_truth() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("include/drm")).unwrap();
        std::fs::create_dir_all(root.join("include/uapi/linux")).unwrap();
        std::fs::write(root.join("include/drm/private.h"), "private\n").unwrap();
        std::fs::write(root.join("include/uapi/linux/abi.h"), "uapi\n").unwrap();
        std::fs::write(root.join("include/uapi/linux/keep.h"), "keep\n").unwrap();

        let slim = SlimConfig {
            remove_paths: vec![
                "include".to_string(),
                "include/uapi/linux/abi.h".to_string(),
            ],
            remove_configs: vec![],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };
        let abi_policy = crate::config::AbiPolicyConfig {
            allow_public_header_removal: false,
            allow_uapi_header_removal: true,
        };

        let manifest =
            RemovalManifest::from_slim_config_for_tree_with_abi_policy(root, &slim, &abi_policy)
                .unwrap();
        let stats = prune_tree_from_manifest(root.to_str().unwrap(), &manifest).unwrap();

        assert_eq!(stats.files_removed, 2);
        assert_eq!(
            stats
                .result
                .removed
                .iter()
                .filter(|removed| !removed.is_dir)
                .count(),
            stats.files_removed
        );
        assert!(stats.result.failed.is_empty());
        assert_eq!(stats.result.edits, stats.edits);
        assert!(!root.join("include/drm/private.h").exists());
        assert!(!root.join("include/uapi/linux/abi.h").exists());
        assert!(root.join("include/uapi/linux/keep.h").exists());
        assert_eq!(
            stats.removal.removed_files,
            vec![
                PathBuf::from("include/drm/private.h"),
                PathBuf::from("include/uapi/linux/abi.h"),
            ]
        );
        assert!(stats.removal.missing_paths.is_empty());
    }

    #[test]
    fn test_prune_declared_paths_from_manifest_seeds_removed_config_symbols() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
        std::fs::write(
            root.join("drivers/foo/Kconfig"),
            "config Z_REMOVED_FROM_FILE\n\tbool \"Z\"\nconfig A_REMOVED_FROM_FILE\n\tbool \"A\"\n",
        )
        .unwrap();

        let manifest = RemovalManifest::from_slim_config(&SlimConfig {
            remove_paths: vec!["drivers/foo/Kconfig".to_string()],
            remove_configs: vec!["EXPLICIT_REMOVED".to_string(), "B_REMOVED".to_string()],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        })
        .unwrap();

        let declared = prune_declared_paths_from_manifest(root, &manifest).unwrap();

        assert_eq!(
            declared.removal.removed_config_symbols,
            vec![
                String::from("A_REMOVED_FROM_FILE"),
                String::from("B_REMOVED"),
                String::from("EXPLICIT_REMOVED"),
                String::from("Z_REMOVED_FROM_FILE"),
            ]
        );
    }

    #[test]
    fn test_prune_declared_paths_from_manifest_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
        std::fs::write(root.join("drivers/foo/remove.c"), "int remove;\n").unwrap();

        let manifest = RemovalManifest::from_slim_config(&SlimConfig {
            remove_paths: vec!["drivers/foo/remove.c".to_string()],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        })
        .unwrap();

        let first = prune_declared_paths_from_manifest(root, &manifest).unwrap();
        let after_first_drivers_exists = root.join("drivers").exists();
        let second = prune_declared_paths_from_manifest(root, &manifest).unwrap();

        assert_eq!(first.files_removed, 1);
        assert!(first.dirs_removed >= 1);
        assert!(first
            .edits
            .iter()
            .any(|edit| edit.pass_name == "prune.remove_path"));
        assert!(first
            .edits
            .iter()
            .any(|edit| edit.pass_name == "prune.cleanup_empty_parents"));
        assert!(!root.join("drivers/foo/remove.c").exists());
        assert_eq!(root.join("drivers").exists(), after_first_drivers_exists);
        assert_eq!(second.files_removed, 0);
        assert_eq!(second.dirs_removed, 0);
        assert!(second.edits.is_empty());
    }

    #[test]
    fn test_rewrite_kconfig_stage_rewrites_kconfig_without_touching_makefiles() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
        std::fs::write(
            root.join("Kconfig"),
            concat!(
                "config REMOVED\n",
                "\tbool \"Removed\"\n",
                "config LIVE\n",
                "\tbool \"Live\"\n",
                "\tdepends on REMOVED || OTHER\n",
            ),
        )
        .unwrap();
        std::fs::write(
            root.join("drivers/foo/Makefile"),
            "obj-$(CONFIG_REMOVED) += helper.o\n",
        )
        .unwrap();

        let manifest = RemovalManifest::from_slim_config(&SlimConfig {
            remove_paths: vec![],
            remove_configs: vec!["REMOVED".to_string()],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        })
        .unwrap();

        let stage = rewrite_kconfig_stage(root, &manifest).unwrap();

        assert_eq!(stage.configs_disabled, 1);
        assert_eq!(stage.kconfig_report.simplified_depends, 1);
        assert_eq!(stage.kconfig_report.removed_sources, 0);
        assert!(stage
            .edits
            .iter()
            .any(|edit| edit.pass_name == "prune.prune_configs"));
        assert!(stage
            .edits
            .iter()
            .any(|edit| edit.pass_name == "kconfig.rewrite_relations"));
        assert_eq!(
            std::fs::read_to_string(root.join("Kconfig")).unwrap(),
            concat!(
                "# kslim: removed config REMOVED\n",
                "config LIVE\n",
                "\tbool \"Live\"\n",
                "\tdepends on OTHER\n",
            )
        );
        assert_eq!(
            std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
            "obj-$(CONFIG_REMOVED) += helper.o\n"
        );
    }

    #[test]
    fn test_rewrite_kconfig_stage_is_idempotent_for_each_mutating_subpass() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("drivers/remove")).unwrap();
        std::fs::write(
            root.join("Kconfig"),
            concat!(
                "source \"drivers/remove/Kconfig\"\n",
                "menu \"Dead menu\"\n",
                "config REMOVE_ME\n",
                "\tbool \"Remove me\"\n",
                "endmenu\n",
                "config KEEP_ME\n",
                "\tbool \"Keep me\"\n",
                "\tdefault y\n",
                "\tdepends on REMOVE_ME || LIVE\n",
            ),
        )
        .unwrap();
        std::fs::write(
            root.join("drivers/remove/Kconfig"),
            "config REMOVE_CHILD\n\tbool \"Remove child\"\n",
        )
        .unwrap();

        let slim = SlimConfig {
            remove_paths: vec!["drivers/remove/Kconfig".to_string()],
            remove_configs: vec!["REMOVE_ME".to_string()],
            set_defaults: BTreeMap::from([(String::from("KEEP_ME"), String::from("n"))]),
            unsafe_allow_root_path_removal: false,
        };
        let manifest = RemovalManifest::from_slim_config_for_tree(root, &slim).unwrap();

        let first = rewrite_kconfig_stage(root, &manifest).unwrap();
        let after_first = std::fs::read_to_string(root.join("Kconfig")).unwrap();
        let second = rewrite_kconfig_stage(root, &manifest).unwrap();

        assert_eq!(first.configs_disabled, 1);
        assert_eq!(first.defaults_overridden, 1);
        assert_eq!(first.kconfig_report.simplified_depends, 1);
        assert_eq!(first.kconfig_report.removed_empty_menus, 1);
        for pass in [
            "prune.prune_configs",
            "prune.rewrite_kconfig_defaults",
            "kconfig.rewrite_relations",
            "kconfig.rewrite_empty_menus",
        ] {
            assert!(
                first.edits.iter().any(|edit| edit.pass_name == pass),
                "first kconfig stage should exercise {pass}"
            );
        }
        assert_eq!(second.configs_disabled, 0);
        assert_eq!(second.defaults_overridden, 0);
        assert_eq!(second.kconfig_report.simplified_depends, 0);
        assert_eq!(second.kconfig_report.removed_empty_menus, 0);
        assert!(second.edits.is_empty());
        assert_eq!(
            std::fs::read_to_string(root.join("Kconfig")).unwrap(),
            after_first
        );
    }

    #[test]
    fn test_rewrite_kconfig_stage_cleans_empty_menus_after_config_removal() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::write(
            root.join("Kconfig"),
            concat!(
                "menu \"Dead menu\"\n",
                "config REMOVED\n",
                "\tbool \"Removed\"\n",
                "endmenu\n",
                "menu \"Live menu\"\n",
                "config LIVE\n",
                "\tbool \"Live\"\n",
                "endmenu\n",
            ),
        )
        .unwrap();

        let manifest = RemovalManifest::from_slim_config(&SlimConfig {
            remove_paths: vec![],
            remove_configs: vec!["REMOVED".to_string()],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        })
        .unwrap();

        let stage = rewrite_kconfig_stage(root, &manifest).unwrap();

        assert_eq!(stage.configs_disabled, 1);
        assert_eq!(stage.kconfig_report.removed_empty_menus, 1);
        assert!(stage
            .edits
            .iter()
            .any(|edit| edit.pass_name == "kconfig.rewrite_empty_menus"));
        assert_eq!(
            std::fs::read_to_string(root.join("Kconfig")).unwrap(),
            concat!(
                "# kslim: removed empty menu \"Dead menu\"\n",
                "menu \"Live menu\"\n",
                "config LIVE\n",
                "\tbool \"Live\"\n",
                "endmenu\n",
            )
        );
    }

    #[test]
    fn test_rewrite_kconfig_stage_reports_removed_symbol_reenabled_by_default() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::write(
            root.join("Kconfig"),
            concat!(
                "config REMOVED_DEFAULT\n",
                "\ttristate \"Removed default\"\n",
                "\tdefault y\n",
                "config REMOVED_CONDITION_OFF\n",
                "\ttristate \"Removed condition off\"\n",
                "\tdefault y if REMOVED_GATE\n",
                "\tdefault n\n",
                "config LIVE_DEFAULT\n",
                "\ttristate \"Live default\"\n",
                "\tdefault y\n",
            ),
        )
        .unwrap();

        let manifest = RemovalManifest::from_slim_config(&SlimConfig {
            remove_paths: vec![],
            remove_configs: vec![
                "REMOVED_CONDITION_OFF".to_string(),
                "REMOVED_DEFAULT".to_string(),
                "REMOVED_GATE".to_string(),
            ],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        })
        .unwrap();

        let stage = rewrite_kconfig_stage(root, &manifest).unwrap();

        assert_eq!(stage.configs_disabled, 2);
        assert_eq!(stage.kconfig_solver_report.default_reenabled_symbols.len(), 1);
        assert_eq!(
            stage.kconfig_solver_report.default_reenabled_symbols[0].symbol,
            "REMOVED_DEFAULT"
        );
        assert_eq!(
            stage.kconfig_solver_report.default_reenabled_symbols[0].value,
            "y"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("Kconfig")).unwrap(),
            concat!(
                "# kslim: removed config REMOVED_DEFAULT\n",
                "# kslim: removed config REMOVED_CONDITION_OFF\n",
                "config LIVE_DEFAULT\n",
                "\ttristate \"Live default\"\n",
                "\tdefault y\n",
            )
        );
    }

    #[test]
    fn test_rewrite_kconfig_stage_reports_removed_symbol_selected_by_live_feature() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::write(
            root.join(".config"),
            concat!(
                "CONFIG_LIVE_FEATURE=y\n",
                "CONFIG_LIVE_GATE=y\n",
                "# CONFIG_BLOCKED_DEP is not set\n",
            ),
        )
        .unwrap();
        std::fs::write(
            root.join("Kconfig"),
            concat!(
                "config REMOVED_SELECTED\n",
                "\ttristate \"Removed selected\"\n",
                "\tdepends on BLOCKED_DEP\n",
                "config LIVE_FEATURE\n",
                "\ttristate \"Live feature\"\n",
                "\tselect REMOVED_SELECTED if LIVE_GATE\n",
                "config LIVE_GATE\n",
                "\tbool \"Live gate\"\n",
            ),
        )
        .unwrap();

        let manifest = RemovalManifest::from_slim_config(&SlimConfig {
            remove_paths: vec![],
            remove_configs: vec!["REMOVED_SELECTED".to_string()],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        })
        .unwrap();

        let stage = rewrite_kconfig_stage(root, &manifest).unwrap();

        assert_eq!(stage.configs_disabled, 1);
        assert_eq!(stage.kconfig_report.dropped_selects, 1);
        assert_eq!(stage.kconfig_solver_report.forced_selects.len(), 1);
        assert_eq!(
            stage.kconfig_solver_report.forced_selects[0].source_symbol,
            "LIVE_FEATURE"
        );
        assert_eq!(
            stage.kconfig_solver_report.forced_selects[0].target_symbol,
            "REMOVED_SELECTED"
        );
        assert_eq!(stage.kconfig_solver_report.forced_selects[0].value, "y");
        assert_eq!(
            std::fs::read_to_string(root.join("Kconfig")).unwrap(),
            concat!(
                "# kslim: removed config REMOVED_SELECTED\n",
                "config LIVE_FEATURE\n",
                "\ttristate \"Live feature\"\n",
                "config LIVE_GATE\n",
                "\tbool \"Live gate\"\n",
            )
        );
    }

    #[test]
    fn test_prune_preserves_abi_guard_config_symbols_without_abi_policy() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("include/uapi/linux")).unwrap();
        std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
        std::fs::write(
            root.join("include/uapi/linux/abi.h"),
            "#if IS_ENABLED(CONFIG_ABI_GUARD)\nint abi_guarded(void);\n#endif\n",
        )
        .unwrap();
        std::fs::write(
            root.join("Kconfig"),
            concat!(
                "config ABI_GUARD\n",
                "\tbool \"ABI guard\"\n",
                "config REMOVE_ME\n",
                "\tbool \"Remove me\"\n",
            ),
        )
        .unwrap();
        std::fs::write(
            root.join("drivers/foo/Makefile"),
            concat!(
                "obj-$(CONFIG_ABI_GUARD) += abi.o\n",
                "obj-$(CONFIG_REMOVE_ME) += gone.o\n",
            ),
        )
        .unwrap();

        let slim = SlimConfig {
            remove_paths: vec![],
            remove_configs: vec!["ABI_GUARD".to_string(), "REMOVE_ME".to_string()],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let stats = prune_tree(root.to_str().unwrap(), &slim).unwrap();

        assert_eq!(stats.configs_disabled, 1);
        assert_eq!(stats.makefile_refs_removed, 1);
        assert_eq!(
            stats.removal.removed_config_symbols,
            vec![String::from("REMOVE_ME")]
        );
        assert_eq!(
            std::fs::read_to_string(root.join("Kconfig")).unwrap(),
            concat!(
                "config ABI_GUARD\n",
                "\tbool \"ABI guard\"\n",
                "# kslim: removed config REMOVE_ME\n",
            )
        );
        assert_eq!(
            std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
            concat!(
                "obj-$(CONFIG_ABI_GUARD) += abi.o\n",
                "# kslim: removed stale make refs from obj-$(CONFIG_REMOVE_ME)\n",
            )
        );
    }

    #[test]
    fn test_prune_removes_abi_guard_config_symbols_with_matching_abi_policy() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("include/uapi/linux")).unwrap();
        std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
        std::fs::write(
            root.join("include/uapi/linux/abi.h"),
            "#ifdef CONFIG_ABI_GUARD\nint abi_guarded(void);\n#endif\n",
        )
        .unwrap();
        std::fs::write(
            root.join("Kconfig"),
            "config ABI_GUARD\n\tbool \"ABI guard\"\n",
        )
        .unwrap();
        std::fs::write(
            root.join("drivers/foo/Makefile"),
            "obj-$(CONFIG_ABI_GUARD) += abi.o\n",
        )
        .unwrap();

        let slim = SlimConfig {
            remove_paths: vec![],
            remove_configs: vec!["ABI_GUARD".to_string()],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };
        let abi_policy = crate::config::AbiPolicyConfig {
            allow_public_header_removal: false,
            allow_uapi_header_removal: true,
        };
        let manifest =
            RemovalManifest::from_slim_config_for_tree_with_abi_policy(root, &slim, &abi_policy)
                .unwrap();

        let stats = prune_tree_from_manifest(root.to_str().unwrap(), &manifest).unwrap();

        assert_eq!(stats.configs_disabled, 1);
        assert_eq!(stats.makefile_refs_removed, 1);
        assert_eq!(
            stats.removal.removed_config_symbols,
            vec![String::from("ABI_GUARD")]
        );
        assert_eq!(
            std::fs::read_to_string(root.join("Kconfig")).unwrap(),
            "# kslim: removed config ABI_GUARD\n"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
            "# kslim: removed stale make refs from obj-$(CONFIG_ABI_GUARD)\n"
        );
    }

    #[test]
    fn test_cleanup_empty_parent_chain_removes_only_empty_ancestors() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("a/b/c")).unwrap();
        std::fs::create_dir_all(root.join("a/keep")).unwrap();

        let cleanup = cleanup_empty_parent_chain(
            &root.join("a/b/c"),
            root,
            Path::new("a/b/c"),
            &[PathBuf::from("a/b/c")],
            &[],
        )
        .unwrap();

        assert_eq!(cleanup.dirs_removed, 2);
        assert_eq!(
            cleanup.empty_parents_cleaned,
            vec![PathBuf::from("a/b/c"), PathBuf::from("a/b")]
        );
        assert!(!root.join("a/b").exists());
        assert!(root.join("a").exists());
        assert!(root.join("a/keep").exists());
        assert!(cleanup
            .edits
            .iter()
            .any(|edit| edit.file.as_path() == Path::new("a/b/c")));
        assert!(cleanup
            .edits
            .iter()
            .any(|edit| edit.file.as_path() == Path::new("a/b")));
    }

    #[test]
    fn test_prune_directory_rewrites_parent_kconfig_and_makefile() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("drivers/gpu/drm/amd/amdgpu")).unwrap();
        std::fs::write(
            root.join("drivers/gpu/drm/amd/amdgpu/Kconfig"),
            "config DRM_AMDGPU\n\tbool \"AMD GPU\"\n",
        )
        .unwrap();
        std::fs::write(root.join("drivers/gpu/drm/amd/amdgpu/amdgpu_drv.c"), "code").unwrap();
        std::fs::write(
            root.join("drivers/gpu/drm/Kconfig"),
            "source \"drivers/gpu/drm/amd/amdgpu/Kconfig\"\n",
        )
        .unwrap();
        std::fs::write(
            root.join("drivers/gpu/drm/Makefile"),
            "obj-$(CONFIG_DRM_AMDGPU) += amd/amdgpu/\n",
        )
        .unwrap();

        let slim = SlimConfig {
            remove_paths: vec!["drivers/gpu/drm/amd/amdgpu".to_string()],
            remove_configs: vec![],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let stats = prune_tree(root.to_str().unwrap(), &slim).unwrap();

        assert_eq!(stats.makefile_refs_removed, 1);
        assert_eq!(stats.kconfig_refs_removed, 1);
        assert_eq!(stats.kconfig_report.removed_sources, 1);
        assert!(!root.join("drivers/gpu/drm/amd/amdgpu").exists());

        let kconfig = std::fs::read_to_string(root.join("drivers/gpu/drm/Kconfig")).unwrap();
        assert!(kconfig.contains("# kslim: removed source \"drivers/gpu/drm/amd/amdgpu/Kconfig\""));

        let makefile = std::fs::read_to_string(root.join("drivers/gpu/drm/Makefile")).unwrap();
        assert!(makefile.contains("removed stale make refs"));
        assert!(!makefile.contains("amd/amdgpu/"));
    }

    #[test]
    fn test_prune_cleans_empty_parents() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("a/b/c")).unwrap();
        std::fs::write(root.join("a/b/c/d.txt"), "d").unwrap();

        let slim = SlimConfig {
            remove_paths: vec!["a/b/c".to_string()],
            remove_configs: vec![],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let stats = prune_tree(root.to_str().unwrap(), &slim).unwrap();

        assert_eq!(stats.files_removed, 1);
        assert_eq!(stats.configs_disabled, 0);
        assert_eq!(
            stats.removal.empty_parents_cleaned,
            vec![PathBuf::from("a"), PathBuf::from("a/b")]
        );
        assert!(!root.join("a").exists());
    }

    #[test]
    fn test_prune_missing_path_warns() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        let slim = SlimConfig {
            remove_paths: vec!["z/missing".to_string(), "./nonexistent/path".to_string()],
            remove_configs: vec![],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let stats = prune_tree(root.to_str().unwrap(), &slim).unwrap();
        assert_eq!(stats.files_removed, 0);
        assert_eq!(stats.dirs_removed, 0);
        assert_eq!(stats.configs_disabled, 0);
        assert_eq!(
            stats.removal.missing_paths,
            vec![
                PathBuf::from("nonexistent/path"),
                PathBuf::from("z/missing")
            ]
        );
        assert_eq!(
            stats.result.failed,
            vec![
                FailedRemoval::new(
                    PathBuf::from("nonexistent/path"),
                    FailedRemovalKind::MissingPath,
                    "path not found in tree",
                ),
                FailedRemoval::new(
                    PathBuf::from("z/missing"),
                    FailedRemovalKind::MissingPath,
                    "path not found in tree",
                ),
            ]
        );
        assert!(stats.result.edits.is_empty());
    }

    #[test]
    fn test_prune_missing_path_can_fail_by_config_policy() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let slim = SlimConfig {
            remove_paths: vec!["z/missing".to_string()],
            remove_configs: vec![],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };
        let manifest = RemovalManifest::from_slim_config(&slim).unwrap();

        let mut config = crate::config::ReducerConfig::default();
        config.fail_on_missing_prune_paths = true;
        let policy = RemovalFailurePolicy::from_reducer_config(&config);
        let err = prune_declared_paths_from_manifest_with_policy(root, &manifest, policy)
            .unwrap_err()
            .to_string();

        assert!(err.contains("missing_path"), "unexpected error: {err}");
        assert!(err.contains("z/missing"), "unexpected error: {err}");
    }

    #[test]
    fn test_prune_escaped_root_is_always_fatal() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let policy = RemovalFailurePolicy {
            fail_on_missing_path: false,
            strict_permission_failures: false,
            ignore_unsupported_special_files: true,
        };

        for path in ["../escape", r"..\escape"] {
            let err = prune_declared_paths(root, &[PathBuf::from(path)], policy)
                .unwrap_err()
                .to_string();

            assert!(err.contains("escaped_root"), "unexpected error: {err}");
            assert!(err.contains(path), "unexpected error: {err}");
        }
    }

    #[test]
    fn test_prune_permission_failure_policy_is_strict_by_default() {
        let strict =
            RemovalFailurePolicy::from_reducer_config(&crate::config::ReducerConfig::default());
        assert!(strict.is_fatal(FailedRemovalKind::PermissionDenied));

        let mut relaxed = crate::config::ReducerConfig::default();
        relaxed.reject_unproven_fixups = false;
        let relaxed = RemovalFailurePolicy::from_reducer_config(&relaxed);
        assert!(!relaxed.is_fatal(FailedRemovalKind::PermissionDenied));

        let permission_error = io::Error::from(io::ErrorKind::PermissionDenied);
        assert_eq!(
            failed_removal_kind_from_io_error(&permission_error),
            FailedRemovalKind::PermissionDenied
        );
    }

    #[test]
    fn test_failed_removal_kind_stable_names_cover_all_variants() {
        let cases = [
            (FailedRemovalKind::MissingPath, "missing_path"),
            (FailedRemovalKind::PermissionDenied, "permission_denied"),
            (FailedRemovalKind::EscapedRoot, "escaped_root"),
            (
                FailedRemovalKind::UnsupportedSpecialFile,
                "unsupported_special_file",
            ),
            (FailedRemovalKind::IoError, "io_error"),
        ];

        for (kind, stable_name) in cases {
            assert_eq!(kind.stable_name(), stable_name);
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_prune_symlink_is_removed_like_file() {
        use std::os::unix::fs::symlink;

        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("target"), "target\n").unwrap();
        symlink(root.join("target"), root.join("link")).unwrap();

        let result = prune_declared_paths(
            root,
            &[PathBuf::from("link")],
            RemovalFailurePolicy::default(),
        )
        .unwrap();
        assert_eq!(result.files_removed, 1);
        assert!(!root.join("link").exists());
        assert!(root.join("target").exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_prune_unsupported_special_file_policy() {
        use std::os::unix::net::UnixListener;

        let tmp = tempfile::Builder::new()
            .prefix("ks")
            .tempdir_in("/tmp")
            .unwrap();
        let root = tmp.path();
        let socket_path = root.join("socket");
        let _listener = UnixListener::bind(&socket_path).unwrap();

        let err = prune_declared_paths(
            root,
            &[PathBuf::from("socket")],
            RemovalFailurePolicy::default(),
        )
        .unwrap_err()
        .to_string();
        assert!(
            err.contains("unsupported_special_file"),
            "unexpected error: {err}"
        );
        assert!(socket_path.exists());

        let ignored = prune_declared_paths(
            root,
            &[PathBuf::from("socket")],
            RemovalFailurePolicy {
                ignore_unsupported_special_files: true,
                ..RemovalFailurePolicy::default()
            },
        )
        .unwrap();
        assert_eq!(ignored.files_removed, 0);
        assert!(ignored.edits.is_empty());
        assert_eq!(
            ignored.result.failed,
            vec![FailedRemoval::new(
                PathBuf::from("socket"),
                FailedRemovalKind::UnsupportedSpecialFile,
                "path is not a regular file or directory",
            )]
        );
        assert!(socket_path.exists());
    }

    #[test]
    fn test_prune_configs_removes_full_blocks() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("drivers/gpu/drm/amd")).unwrap();
        let kconfig = r#"# AMD GPU driver
config DRM_AMDGPU
	tristate "AMD GPU"
	depends on DRM
	help
	  Choose this option if you have an AMD GPU.

config DRM_AMDGPU_SI
	bool "Southern Islands"
	depends on DRM_AMDGPU

source "drivers/gpu/drm/amd/display/Kconfig"
"#;
        std::fs::write(root.join("drivers/gpu/drm/amd/Kconfig"), kconfig).unwrap();
        std::fs::create_dir_all(root.join("drivers/gpu/drm/amd/display")).unwrap();
        std::fs::write(
            root.join("drivers/gpu/drm/amd/display/Kconfig"),
            "config DRM_AMD_DC\n\tbool \"DC\"\n",
        )
        .unwrap();

        std::fs::create_dir_all(root.join("drivers/gpu/drm/i915")).unwrap();
        std::fs::write(
            root.join("drivers/gpu/drm/i915/Kconfig"),
            "config DRM_I915\n\ttristate \"Intel\"\n",
        )
        .unwrap();

        let slim = SlimConfig {
            remove_paths: vec![],
            remove_configs: vec!["DRM_AMDGPU".to_string(), "DRM_AMDGPU_SI".to_string()],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let stats = prune_tree(root.to_str().unwrap(), &slim).unwrap();
        assert_eq!(stats.files_removed, 0);
        assert_eq!(stats.configs_disabled, 2);
        assert_eq!(stats.defaults_overridden, 0);

        let modified = std::fs::read_to_string(root.join("drivers/gpu/drm/amd/Kconfig")).unwrap();
        assert!(modified.contains("# kslim: removed config DRM_AMDGPU"));
        assert!(modified.contains("# kslim: removed config DRM_AMDGPU_SI"));
        assert!(!modified.contains("tristate \"AMD GPU\""));
        assert!(!modified.contains("depends on DRM_AMDGPU"));
        assert!(modified.contains("source \"drivers/gpu/drm/amd/display/Kconfig\""));

        let i915 = std::fs::read_to_string(root.join("drivers/gpu/drm/i915/Kconfig")).unwrap();
        assert!(i915.contains("config DRM_I915"));
    }

    #[test]
    fn test_prune_simplifies_kconfig_relations_for_removed_symbols() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::write(
            root.join("Kconfig"),
            concat!(
                "config REMOVED\n",
                "\tbool \"Removed\"\n",
                "\n",
                "config LIVE\n",
                "\tbool \"Live\"\n",
                "\tdepends on REMOVED || OTHER\n",
                "\tselect REMOVED\n",
                "\tvisible if !REMOVED\n",
                "\tdefault y if REMOVED\n",
            ),
        )
        .unwrap();

        let slim = SlimConfig {
            remove_paths: vec![],
            remove_configs: vec!["REMOVED".to_string()],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let stats = prune_tree(root.to_str().unwrap(), &slim).unwrap();
        assert_eq!(stats.configs_disabled, 1);
        assert_eq!(stats.kconfig_report.dropped_selects, 1);
        assert_eq!(stats.kconfig_report.simplified_depends, 1);
        assert_eq!(stats.kconfig_report.simplified_visible_if, 1);
        assert_eq!(stats.kconfig_report.simplified_defaults, 1);

        let kconfig = std::fs::read_to_string(root.join("Kconfig")).unwrap();
        assert_eq!(
            kconfig,
            concat!(
                "# kslim: removed config REMOVED\n",
                "config LIVE\n",
                "\tbool \"Live\"\n",
                "\tdepends on OTHER\n",
                "\tvisible if y\n",
            )
        );
        assert!(stats
            .edits
            .iter()
            .any(|edit| edit.pass_name == "kconfig.rewrite_relations"));
        assert!(stats.edits.iter().any(|edit| matches!(
            edit.reason,
            EditReason::SimplifiedTristateExpr { ref symbol } if symbol == "REMOVED"
        )));
    }

    #[test]
    fn test_prune_rewrites_kbuild_refs_gated_by_removed_config_symbols() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
        std::fs::write(root.join("Kconfig"), "config REMOVED\n\tbool \"Removed\"\n").unwrap();
        std::fs::write(root.join("drivers/foo/helper.c"), "int helper;\n").unwrap();
        std::fs::write(
            root.join("drivers/foo/Makefile"),
            "foo-$(CONFIG_REMOVED) += helper.o\nobj-y += foo.o\n",
        )
        .unwrap();

        let slim = SlimConfig {
            remove_paths: vec![],
            remove_configs: vec!["REMOVED".to_string()],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let stats = prune_tree(root.to_str().unwrap(), &slim).unwrap();

        assert_eq!(stats.configs_disabled, 1);
        assert_eq!(stats.makefile_refs_removed, 2);
        assert_eq!(
            std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
            "# kslim: removed stale make refs from foo-$(CONFIG_REMOVED)\n# kslim: removed stale make refs from obj-y\n"
        );
        assert!(stats.edits.iter().any(|edit| matches!(
            edit.reason,
            EditReason::RemovedKbuildRef { ref reference } if reference == "helper.o"
        )));
        assert!(stats.edits.iter().any(|edit| matches!(
            edit.reason,
            EditReason::RemovedKbuildRef { ref reference } if reference == "foo.o"
        )));
    }

    #[test]
    fn test_prune_unrelated_file_does_not_rewrite_makefiles() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("Documentation")).unwrap();
        std::fs::write(root.join("Documentation/.gitignore"), "tmp\n").unwrap();
        std::fs::create_dir_all(root.join("drivers/gpu/drm/nouveau/dispnv04/i2c")).unwrap();
        std::fs::write(
            root.join("drivers/gpu/drm/nouveau/dispnv04/i2c/Kbuild"),
            "ch7006-y := dispnv04/i2c/ch7006_drv.o dispnv04/i2c/ch7006_mode.o\nobj-y += ch7006.o\n",
        )
        .unwrap();

        let original =
            std::fs::read_to_string(root.join("drivers/gpu/drm/nouveau/dispnv04/i2c/Kbuild"))
                .unwrap();

        let slim = SlimConfig {
            remove_paths: vec!["Documentation/.gitignore".to_string()],
            remove_configs: vec![],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let stats = prune_tree(root.to_str().unwrap(), &slim).unwrap();
        assert_eq!(stats.files_removed, 1);
        assert_eq!(stats.makefile_refs_removed, 0);

        let after =
            std::fs::read_to_string(root.join("drivers/gpu/drm/nouveau/dispnv04/i2c/Kbuild"))
                .unwrap();
        assert_eq!(original, after);
    }

    #[test]
    fn test_prune_does_not_rewrite_root_relative_arch_dir_refs() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("arch/x86/lib")).unwrap();
        std::fs::write(root.join("arch/x86/lib/retpoline.S"), "code").unwrap();
        std::fs::write(root.join("arch/x86/Makefile"), "libs-y += arch/x86/lib/\n").unwrap();
        std::fs::create_dir_all(root.join("Documentation")).unwrap();
        std::fs::write(root.join("Documentation/.gitignore"), "tmp\n").unwrap();

        let original = std::fs::read_to_string(root.join("arch/x86/Makefile")).unwrap();
        let slim = SlimConfig {
            remove_paths: vec!["Documentation/.gitignore".to_string()],
            remove_configs: vec![],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let stats = prune_tree(root.to_str().unwrap(), &slim).unwrap();

        assert_eq!(stats.files_removed, 1);
        assert_eq!(stats.makefile_refs_removed, 0);

        let after = std::fs::read_to_string(root.join("arch/x86/Makefile")).unwrap();
        assert_eq!(original, after);
    }

    #[test]
    fn test_prune_rewrites_removed_root_relative_arch_dir_refs() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("arch/x86/lib")).unwrap();
        std::fs::write(root.join("arch/x86/lib/retpoline.S"), "code").unwrap();
        std::fs::write(root.join("arch/x86/Makefile"), "libs-y += arch/x86/lib/\n").unwrap();

        let slim = SlimConfig {
            remove_paths: vec!["arch/x86/lib".to_string()],
            remove_configs: vec![],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let stats = prune_tree(root.to_str().unwrap(), &slim).unwrap();

        assert_eq!(stats.makefile_refs_removed, 1);

        let after = std::fs::read_to_string(root.join("arch/x86/Makefile")).unwrap();
        assert!(after.contains("removed stale make refs"));
        assert!(!after.contains("arch/x86/lib/"));
    }

    #[test]
    fn test_prune_rewrites_parent_dir_ref_after_empty_dir_cleanup() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("drivers/foo/bar")).unwrap();
        std::fs::write(root.join("drivers/foo/bar/only.c"), "int only;\n").unwrap();
        std::fs::write(root.join("drivers/foo/Makefile"), "obj-y += bar/\n").unwrap();

        let slim = SlimConfig {
            remove_paths: vec!["drivers/foo/bar/only.c".to_string()],
            remove_configs: vec![],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let stats = prune_tree(root.to_str().unwrap(), &slim).unwrap();

        assert_eq!(stats.files_removed, 1);
        assert_eq!(stats.makefile_refs_removed, 1);
        assert!(!root.join("drivers/foo/bar").exists());
        assert_eq!(
            std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
            "# kslim: removed stale make refs from obj-y\n"
        );
    }

    #[test]
    fn test_prune_collects_removed_config_symbols_from_removed_kconfig_files() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
        std::fs::write(
            root.join("drivers/foo/Kconfig"),
            "config REMOVED_Z\n\tbool \"Z\"\nmenuconfig REMOVED_A\n\tbool \"A\"\n",
        )
        .unwrap();

        let slim = SlimConfig {
            remove_paths: vec!["drivers/foo/Kconfig".to_string()],
            remove_configs: vec!["EXPLICIT".to_string()],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let stats = prune_tree(root.to_str().unwrap(), &slim).unwrap();

        assert_eq!(
            stats.removal.removed_config_symbols,
            vec![
                String::from("EXPLICIT"),
                String::from("REMOVED_A"),
                String::from("REMOVED_Z"),
            ]
        );
    }

    #[test]
    fn test_prune_rewrites_stale_ccflags_include_paths_after_dir_removal() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("drivers/foo/include")).unwrap();
        std::fs::create_dir_all(root.join("drivers/foo/headers")).unwrap();
        std::fs::write(
            root.join("drivers/foo/include/local.h"),
            "#define LOCAL 1\n",
        )
        .unwrap();
        std::fs::write(
            root.join("drivers/foo/Makefile"),
            "ccflags-y += -Iinclude -Iheaders -Werror\n",
        )
        .unwrap();

        let slim = SlimConfig {
            remove_paths: vec!["drivers/foo/include".to_string()],
            remove_configs: vec![],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let stats = prune_tree(root.to_str().unwrap(), &slim).unwrap();

        assert_eq!(stats.files_removed, 1);
        assert_eq!(stats.makefile_refs_removed, 1);
        assert!(stats.skipped_makefile_lines.is_empty());
        assert!(!root.join("drivers/foo/include").exists());
        assert_eq!(
            std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
            "ccflags-y += -Iheaders -Werror\n"
        );
        assert!(stats.edits.iter().any(|edit| matches!(
            edit.reason,
            EditReason::RemovedKbuildRef { ref reference }
                if reference == "-Iinclude"
        )));
    }

    #[test]
    fn test_prune_reports_ambiguous_ccflags_include_paths_after_dir_removal() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("drivers/foo/include")).unwrap();
        std::fs::create_dir_all(root.join("include")).unwrap();
        std::fs::write(
            root.join("drivers/foo/include/local.h"),
            "#define LOCAL 1\n",
        )
        .unwrap();
        std::fs::write(
            root.join("drivers/foo/Makefile"),
            "ccflags-y += -Iinclude -Werror\n",
        )
        .unwrap();

        let original = std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap();
        let slim = SlimConfig {
            remove_paths: vec!["drivers/foo/include".to_string()],
            remove_configs: vec![],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let stats = prune_tree(root.to_str().unwrap(), &slim).unwrap();

        assert_eq!(stats.files_removed, 1);
        assert_eq!(stats.makefile_refs_removed, 0);
        assert_eq!(stats.skipped_makefile_lines.len(), 1);
        assert_eq!(
            stats.skipped_makefile_lines[0].file,
            PathBuf::from("drivers/foo/Makefile")
        );
        assert_eq!(stats.skipped_makefile_lines[0].line, 1);
        assert_eq!(stats.skipped_makefile_lines[0].assignment_lhs, "ccflags-y");
        assert!(stats.skipped_makefile_lines[0]
            .reason
            .contains("ambiguous include path flag '-Iinclude'"));
        assert_eq!(
            std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
            original
        );
    }

    #[test]
    fn test_prune_does_not_rewrite_shell_command_assignments() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("Documentation")).unwrap();
        std::fs::write(root.join("Documentation/.gitignore"), "tmp\n").unwrap();
        std::fs::write(
            root.join("Kbuild"),
            concat!(
                "quiet_cmd_check_sha1 = CHKSHA1 $<\n",
                "cmd_check_sha1 = if ! command -v sha1sum >/dev/null; then echo \"warning: cannot check the header due to sha1sum missing\"; exit 0; fi; if [ \"$$(sed -n '$$s:// ::p' $<)\" != \"$$(sed '$$d' $< | sha1sum | sed 's/ .*//')\" ]; then echo \"error: $< has been modified.\" >&2; exit 1; fi; touch $@\n",
                "obj-y += keep/\n",
            ),
        )
        .unwrap();
        std::fs::create_dir_all(root.join("keep")).unwrap();

        let original = std::fs::read_to_string(root.join("Kbuild")).unwrap();

        let slim = SlimConfig {
            remove_paths: vec!["Documentation/.gitignore".to_string()],
            remove_configs: vec![],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let stats = prune_tree(root.to_str().unwrap(), &slim).unwrap();
        assert_eq!(stats.files_removed, 1);
        assert_eq!(stats.makefile_refs_removed, 0);

        let after = std::fs::read_to_string(root.join("Kbuild")).unwrap();
        assert_eq!(original, after);
        assert!(after.contains("sed 's/ .*//'"));
    }

    #[test]
    fn test_prune_preserves_shell_fragments_while_rewriting_same_makefile() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("drivers/foo/keep")).unwrap();
        std::fs::create_dir_all(root.join("drivers/foo/remove")).unwrap();
        std::fs::write(
            root.join("drivers/foo/Makefile"),
            concat!(
                "quiet_cmd_check_sha1 = CHKSHA1 $<\n",
                "cmd_check_sha1 = if ! command -v sha1sum >/dev/null; then echo \"warning: cannot check the header due to sha1sum missing\"; exit 0; fi; if [ \"$$(sed -n '$$s:// ::p' $<)\" != \"$$(sed '$$d' $< | sha1sum | sed 's/ .*//')\" ]; then echo \"error: $< has been modified.\" >&2; exit 1; fi; touch $@\n",
                "obj-y += keep/ remove/\n",
            ),
        )
        .unwrap();

        let slim = SlimConfig {
            remove_paths: vec!["drivers/foo/remove".to_string()],
            remove_configs: vec![],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let stats = prune_tree(root.to_str().unwrap(), &slim).unwrap();

        assert_eq!(stats.makefile_refs_removed, 1);
        assert_eq!(
            std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
            concat!(
                "quiet_cmd_check_sha1 = CHKSHA1 $<\n",
                "cmd_check_sha1 = if ! command -v sha1sum >/dev/null; then echo \"warning: cannot check the header due to sha1sum missing\"; exit 0; fi; if [ \"$$(sed -n '$$s:// ::p' $<)\" != \"$$(sed '$$d' $< | sha1sum | sed 's/ .*//')\" ]; then echo \"error: $< has been modified.\" >&2; exit 1; fi; touch $@\n",
                "obj-y += keep/\n",
            )
        );
    }

    #[test]
    fn test_prune_preserves_non_build_assignments_while_rewriting_same_makefile() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("drivers/foo/keep")).unwrap();
        std::fs::create_dir_all(root.join("drivers/foo/remove")).unwrap();
        std::fs::write(
            root.join("drivers/foo/Makefile"),
            concat!(
                "ARCH_PROCESSED := x86\n",
                "SOME_OTHER_VAR = keep-me\n",
                "obj-y += keep/ remove/\n",
            ),
        )
        .unwrap();

        let slim = SlimConfig {
            remove_paths: vec!["drivers/foo/remove".to_string()],
            remove_configs: vec![],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let stats = prune_tree(root.to_str().unwrap(), &slim).unwrap();

        assert_eq!(stats.makefile_refs_removed, 1);
        assert_eq!(
            std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
            concat!(
                "ARCH_PROCESSED := x86\n",
                "SOME_OTHER_VAR = keep-me\n",
                "obj-y += keep/\n",
            )
        );
    }

    #[test]
    fn test_prune_preserves_comments_while_rewriting_same_makefile() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("drivers/foo/keep")).unwrap();
        std::fs::create_dir_all(root.join("drivers/foo/remove")).unwrap();
        std::fs::write(
            root.join("drivers/foo/Makefile"),
            concat!(
                "# keep intro comment\n",
                "obj-y += keep/ remove/  # keep trailing comment\n",
                "# keep outro comment\n",
            ),
        )
        .unwrap();

        let slim = SlimConfig {
            remove_paths: vec!["drivers/foo/remove".to_string()],
            remove_configs: vec![],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let stats = prune_tree(root.to_str().unwrap(), &slim).unwrap();

        assert_eq!(stats.makefile_refs_removed, 1);
        assert_eq!(
            std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
            concat!(
                "# keep intro comment\n",
                "obj-y += keep/  # keep trailing comment\n",
                "# keep outro comment\n",
            )
        );
    }

    #[test]
    fn test_prune_preserves_surviving_token_and_line_order() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("drivers/foo/keep")).unwrap();
        std::fs::create_dir_all(root.join("drivers/foo/remove")).unwrap();
        std::fs::create_dir_all(root.join("drivers/foo/child")).unwrap();
        std::fs::write(root.join("drivers/foo/before.c"), "int before;\n").unwrap();
        std::fs::write(root.join("drivers/foo/first.c"), "int first;\n").unwrap();
        std::fs::write(root.join("drivers/foo/remove.c"), "int remove;\n").unwrap();
        std::fs::write(root.join("drivers/foo/second.c"), "int second;\n").unwrap();
        std::fs::write(
            root.join("drivers/foo/Makefile"),
            concat!(
                "lib-y += before.o\n",
                "obj-y += first.o remove.o keep/ remove/ second.o\n",
                "subdir-y += child/\n",
            ),
        )
        .unwrap();

        let slim = SlimConfig {
            remove_paths: vec![
                "drivers/foo/remove".to_string(),
                "drivers/foo/remove.c".to_string(),
            ],
            remove_configs: vec![],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let stats = prune_tree(root.to_str().unwrap(), &slim).unwrap();

        assert_eq!(stats.makefile_refs_removed, 2);
        assert_eq!(
            std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
            concat!(
                "lib-y += before.o\n",
                "obj-y += first.o keep/ second.o\n",
                "subdir-y += child/\n",
            )
        );
    }

    #[test]
    fn test_prune_set_defaults_rewrites_existing_default_lines() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::write(
            root.join("Kconfig"),
            concat!(
                "config FOO\n",
                "\tbool \"Foo\"\n",
                "\tdefault y\n",
                "\tdefault BAR if BAZ\n",
                "\thelp\n",
                "\t  hi\n",
            ),
        )
        .unwrap();

        let slim = SlimConfig {
            remove_paths: vec![],
            remove_configs: vec![],
            set_defaults: BTreeMap::from([(String::from("FOO"), String::from("n"))]),
            unsafe_allow_root_path_removal: false,
        };

        let stats = prune_tree(root.to_str().unwrap(), &slim).unwrap();
        assert_eq!(stats.defaults_overridden, 1);

        let kconfig = std::fs::read_to_string(root.join("Kconfig")).unwrap();
        assert!(kconfig.contains("\tbool \"Foo\"\n\tdefault n\n"));
        assert!(!kconfig.contains("default y"));
        assert!(!kconfig.contains("default BAR if BAZ"));
    }

    #[test]
    fn test_prune_set_defaults_rewrites_def_bool_blocks() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::write(root.join("Kconfig"), "config FOO\n\tdef_bool y\n").unwrap();

        let slim = SlimConfig {
            remove_paths: vec![],
            remove_configs: vec![],
            set_defaults: BTreeMap::from([(String::from("FOO"), String::from("n"))]),
            unsafe_allow_root_path_removal: false,
        };

        let stats = prune_tree(root.to_str().unwrap(), &slim).unwrap();
        assert_eq!(stats.defaults_overridden, 1);

        let kconfig = std::fs::read_to_string(root.join("Kconfig")).unwrap();
        assert_eq!(kconfig, "config FOO\n\tbool\n\tdefault n\n");
    }

    #[test]
    fn test_prune_set_defaults_preserves_help_text() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::write(
            root.join("Kconfig"),
            concat!(
                "config FOO\n",
                "\tbool \"Foo\"\n",
                "\thelp\n",
                "\t  default should remain plain text here\n",
            ),
        )
        .unwrap();

        let slim = SlimConfig {
            remove_paths: vec![],
            remove_configs: vec![],
            set_defaults: BTreeMap::from([(String::from("FOO"), String::from("n"))]),
            unsafe_allow_root_path_removal: false,
        };

        let stats = prune_tree(root.to_str().unwrap(), &slim).unwrap();
        assert_eq!(stats.defaults_overridden, 1);

        let kconfig = std::fs::read_to_string(root.join("Kconfig")).unwrap();
        assert!(kconfig.contains("\tdefault n\n\thelp\n"));
        assert!(kconfig.contains("default should remain plain text here"));
    }

    #[test]
    fn test_prune_set_defaults_requires_existing_symbol() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::write(root.join("Kconfig"), "config BAR\n\tbool \"Bar\"\n").unwrap();

        let slim = SlimConfig {
            remove_paths: vec![],
            remove_configs: vec![],
            set_defaults: BTreeMap::from([(String::from("FOO"), String::from("n"))]),
            unsafe_allow_root_path_removal: false,
        };

        let err = prune_tree(root.to_str().unwrap(), &slim)
            .unwrap_err()
            .to_string();
        assert!(err.contains("slim.set_defaults symbol 'FOO' was not found"));
    }

    #[test]
    fn test_prune_records_manifest_and_graph_edit_reasons() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join("drivers/gpu/drm/amd/amdgpu")).unwrap();
        std::fs::write(
            root.join("drivers/gpu/drm/amd/amdgpu/Kconfig"),
            "config DRM_AMDGPU\n\tbool \"AMD GPU\"\n",
        )
        .unwrap();
        std::fs::write(
            root.join("drivers/gpu/drm/amd/amdgpu/amdgpu_drv.c"),
            "code\n",
        )
        .unwrap();
        std::fs::write(
            root.join("drivers/gpu/drm/Kconfig"),
            "source \"drivers/gpu/drm/amd/amdgpu/Kconfig\"\n",
        )
        .unwrap();
        std::fs::write(
            root.join("drivers/gpu/drm/Makefile"),
            "obj-$(CONFIG_DRM_AMDGPU) += amd/amdgpu/\n",
        )
        .unwrap();

        let slim = SlimConfig {
            remove_paths: vec!["drivers/gpu/drm/amd/amdgpu".to_string()],
            remove_configs: vec![],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let stats = prune_tree(root.to_str().unwrap(), &slim).unwrap();

        assert!(stats.edits.iter().any(|edit| matches!(
            edit.reason,
            EditReason::ManifestPath { ref path }
                if path == Path::new("drivers/gpu/drm/amd/amdgpu")
        )));
        assert!(stats.edits.iter().any(|edit| matches!(
            (&edit.reason, &edit.proof_source),
            (
                EditReason::RemovedKconfigSource,
                EditProofSource::RemovalManifest {
                    key: crate::edit_reason::RemovalKey::KconfigSource(path),
                }
            )
                if path == Path::new("drivers/gpu/drm/amd/amdgpu/Kconfig")
        )));
        assert!(stats.edits.iter().any(|edit| matches!(
            edit.reason,
            EditReason::RemovedKbuildRef { ref reference }
                if reference == "amd/amdgpu/"
        )));
    }

    #[test]
    fn test_prune_records_manifest_config_edit_reason() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::write(
            root.join("Kconfig"),
            concat!(
                "config FOO\n",
                "\tbool \"Foo\"\n",
                "\tdefault y\n",
                "\thelp\n",
                "\t  hi\n",
            ),
        )
        .unwrap();

        let slim = SlimConfig {
            remove_paths: vec![],
            remove_configs: vec![],
            set_defaults: BTreeMap::from([(String::from("FOO"), String::from("n"))]),
            unsafe_allow_root_path_removal: false,
        };

        let stats = prune_tree(root.to_str().unwrap(), &slim).unwrap();
        let edit = stats
            .edits
            .iter()
            .find(|edit| {
                matches!(
                    edit.reason,
                    EditReason::ManifestConfig { ref symbol } if symbol == "FOO"
                )
            })
            .unwrap();

        assert_eq!(edit.pass_name, "prune.rewrite_kconfig_defaults");
        assert_eq!(edit.file, PathBuf::from("Kconfig"));
        assert_eq!(edit.line_range, Some(LineRange { start: 1, end: 5 }));
        assert!(edit.before.contains("default y"));
        assert!(edit.after.contains("default n"));
    }
}
