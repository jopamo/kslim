//! Internal reducer truth derived from user-facing profile removal input.
//!
//! This module normalizes removal and preservation paths/symbols into a single
//! manifest structure used by indexing and rewrite passes. It is intentionally
//! separate from `src/manifest.rs`, which only writes emitted file-hash
//! manifests for generated output trees.

mod match_rules;
mod model;
mod parse;
mod validate;

#[allow(unused_imports)]
pub use crate::model::HeaderPath;
#[allow(unused_imports)]
pub use model::{RelativePathBuf, RemovalKey, RemovalManifest, RemovalReason};
#[allow(unused_imports)]
pub(crate) use validate::{is_public_header_path, is_uapi_header_path, is_uapi_path};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abi::AbiPolicyConfig;
    use crate::config::SlimConfig;
    use crate::model::{DeviceCompatible, HeaderPath, KbuildObject};
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};

    fn allow_public_and_uapi_header_removal() -> AbiPolicyConfig {
        AbiPolicyConfig {
            allow_public_header_removal: true,
            allow_uapi_header_removal: true,
        }
    }

    fn kbuild_object_strings(objects: Vec<KbuildObject>) -> Vec<String> {
        objects
            .into_iter()
            .map(|object| object.as_str().to_string())
            .collect()
    }

    fn header_path_strings(headers: impl IntoIterator<Item = HeaderPath>) -> Vec<String> {
        headers
            .into_iter()
            .map(|header| header.as_str().to_string())
            .collect()
    }

    #[test]
    fn test_from_slim_config_normalizes_and_deduplicates_paths() {
        let slim = SlimConfig {
            remove_paths: vec![
                "zeta/last".to_string(),
                "drivers/gpu/drm/amd/amdgpu/helper.c".to_string(),
                "./drivers//gpu/drm/amd/amdgpu/".to_string(),
                "drivers/gpu/drm/amd/amdgpu".to_string(),
                "alpha/first".to_string(),
            ],
            remove_configs: vec![
                "DRM_AMDGPU_SI".to_string(),
                "DRM_AMDGPU".to_string(),
                "DRM_AMDGPU".to_string(),
            ],
            set_defaults: BTreeMap::from([(String::from("DRM_AMDGPU_WERROR"), String::from("n"))]),
            unsafe_allow_root_path_removal: false,
        };

        let manifest = RemovalManifest::from_slim_config(&slim).unwrap();

        assert_eq!(
            manifest.removed_paths_vec(),
            vec![
                PathBuf::from("alpha/first"),
                PathBuf::from("drivers/gpu/drm/amd/amdgpu"),
                PathBuf::from("zeta/last"),
            ]
        );
        assert_eq!(
            manifest.removed_config_symbols_vec(),
            vec![String::from("DRM_AMDGPU"), String::from("DRM_AMDGPU_SI")]
        );
        assert_eq!(
            manifest.default_overrides().get("DRM_AMDGPU_WERROR"),
            Some(&String::from("n"))
        );
        assert_eq!(
            manifest.removed_dirs,
            BTreeSet::from([PathBuf::from("drivers/gpu/drm/amd/amdgpu")])
        );
        assert!(manifest.removed_files.is_empty());
        assert!(manifest.removed_headers.is_empty());
        assert!(manifest.removed_public_headers.is_empty());
        assert!(manifest.removed_kconfig_sources.is_empty());
        assert!(manifest.removed_device_bindings.is_empty());
        assert!(manifest.removed_exported_symbols.is_empty());
        assert!(manifest.removed_runtime_registrations.is_empty());
        assert_eq!(
            manifest.removed_kbuild_objects,
            BTreeSet::from([KbuildObject::new("drivers/gpu/drm/amd/amdgpu/").unwrap()])
        );
        assert_eq!(
            manifest.reasons.get(&RemovalKey::Path(PathBuf::from(
                "drivers/gpu/drm/amd/amdgpu"
            ))),
            Some(&RemovalReason::SlimRemovePath {
                path: PathBuf::from("drivers/gpu/drm/amd/amdgpu")
            })
        );
        assert_eq!(
            manifest.reasons.get(&RemovalKey::Dir(PathBuf::from(
                "drivers/gpu/drm/amd/amdgpu"
            ))),
            Some(&RemovalReason::SlimRemovePath {
                path: PathBuf::from("drivers/gpu/drm/amd/amdgpu")
            })
        );
        assert_eq!(
            manifest
                .reasons
                .get(&RemovalKey::ConfigSymbol(String::from("DRM_AMDGPU"))),
            Some(&RemovalReason::SlimRemoveConfig {
                symbol: String::from("DRM_AMDGPU")
            })
        );
        assert_eq!(
            manifest
                .reasons
                .get(&RemovalKey::DefaultOverride(String::from(
                    "DRM_AMDGPU_WERROR"
                ))),
            Some(&RemovalReason::SlimDefaultOverride {
                symbol: String::from("DRM_AMDGPU_WERROR"),
                value: String::from("n"),
            })
        );
    }

    #[test]
    fn test_from_slim_config_rejects_absolute_paths() {
        for path in ["/tmp/outside", "C:/tmp/outside", "file:///tmp/outside"] {
            let slim = SlimConfig {
                remove_paths: vec![path.to_string()],
                remove_configs: Vec::new(),
                set_defaults: BTreeMap::new(),
                unsafe_allow_root_path_removal: false,
            };

            let err = format!(
                "{:#}",
                RemovalManifest::from_slim_config(&slim).unwrap_err()
            );

            assert!(
                err.contains("must be relative"),
                "unexpected error for {path}: {err}"
            );
        }
    }

    #[test]
    fn test_from_slim_config_rejects_empty_and_whitespace_paths() {
        for value in ["", "   ", "\t"] {
            let slim = SlimConfig {
                remove_paths: vec![value.to_string()],
                remove_configs: Vec::new(),
                set_defaults: BTreeMap::new(),
                unsafe_allow_root_path_removal: false,
            };

            let err = format!(
                "{:#}",
                RemovalManifest::from_slim_config(&slim).unwrap_err()
            );

            assert!(
                err.contains("must not contain empty values"),
                "unexpected error for {value:?}: {err}"
            );
        }
    }

    #[test]
    fn test_from_slim_config_rejects_parent_dir_paths() {
        for path in ["drivers/../net", r"drivers\..\net"] {
            let slim = SlimConfig {
                remove_paths: vec![path.to_string()],
                remove_configs: Vec::new(),
                set_defaults: BTreeMap::new(),
                unsafe_allow_root_path_removal: false,
            };

            let err = format!(
                "{:#}",
                RemovalManifest::from_slim_config(&slim).unwrap_err()
            );

            assert!(
                err.contains("must not contain '..'"),
                "unexpected error for {path}: {err}"
            );
        }
    }

    #[test]
    fn test_from_slim_config_rejects_tree_root_path() {
        for path in [".", "./"] {
            let slim = SlimConfig {
                remove_paths: vec![path.to_string()],
                remove_configs: Vec::new(),
                set_defaults: BTreeMap::new(),
                unsafe_allow_root_path_removal: false,
            };

            let err = format!(
                "{:#}",
                RemovalManifest::from_slim_config(&slim).unwrap_err()
            );

            assert!(
                err.contains("must not resolve to the tree root"),
                "unexpected error for {path}: {err}"
            );
            assert!(
                err.contains("slim.unsafe_allow_root_path_removal = true"),
                "unexpected error for {path}: {err}"
            );
        }
    }

    #[test]
    fn test_from_slim_config_allows_tree_root_path_with_explicit_unsafe_mode() {
        let slim = SlimConfig {
            remove_paths: vec!["./".to_string(), "drivers/ignored".to_string()],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: true,
        };

        let manifest = RemovalManifest::from_slim_config(&slim).unwrap();

        assert!(manifest.unsafe_allow_root_path_removal);
        assert_eq!(manifest.removed_paths_vec(), vec![PathBuf::from(".")]);
        assert_eq!(manifest.removed_dirs, BTreeSet::from([PathBuf::from(".")]));
    }

    #[test]
    fn test_from_slim_config_for_tree_allows_tree_root_path_with_explicit_unsafe_mode() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/foo")).unwrap();
        std::fs::write(tmp.path().join("drivers/foo/private.c"), "int private;\n").unwrap();
        let slim = SlimConfig {
            remove_paths: vec![".".to_string()],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: true,
        };

        let manifest = RemovalManifest::from_slim_config_for_tree(tmp.path(), &slim).unwrap();

        assert!(manifest.unsafe_allow_root_path_removal);
        assert_eq!(manifest.removed_paths_vec(), vec![PathBuf::from(".")]);
        assert_eq!(manifest.removed_dirs, BTreeSet::from([PathBuf::from(".")]));
    }

    #[test]
    fn test_from_slim_config_normalizes_separators_and_curdir_prefix() {
        let slim = SlimConfig {
            remove_paths: vec![
                "./drivers//foo///bar.c".to_string(),
                "drivers/foo/bar.c".to_string(),
            ],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let manifest = RemovalManifest::from_slim_config(&slim).unwrap();

        assert_eq!(
            manifest.removed_paths_vec(),
            vec![PathBuf::from("drivers/foo/bar.c")]
        );
    }

    #[test]
    fn test_from_slim_config_preserves_stable_path_ordering() {
        let slim = SlimConfig {
            remove_paths: vec![
                "zeta".to_string(),
                "alpha".to_string(),
                "middle".to_string(),
                "./alpha".to_string(),
            ],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let manifest = RemovalManifest::from_slim_config(&slim).unwrap();

        assert_eq!(
            manifest.removed_paths_vec(),
            vec![
                PathBuf::from("alpha"),
                PathBuf::from("middle"),
                PathBuf::from("zeta"),
            ]
        );
    }

    #[test]
    fn test_from_slim_config_for_tree_preserves_stable_derived_ordering() {
        let tmp = tempfile::tempdir().unwrap();
        for dir in ["drivers/a", "drivers/z", "include/linux", "include/net"] {
            std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
        }
        for (path, content) in [
            ("drivers/a/Kconfig", "config A\n"),
            ("drivers/a/a.c", "int a;\n"),
            ("drivers/a/private.h", "#define A 1\n"),
            ("drivers/z/Kconfig.debug", "config Z_DEBUG\n"),
            ("drivers/z/private.h", "#define Z 1\n"),
            ("drivers/z/z.c", "int z;\n"),
            ("include/linux/a.h", "#define LINUX_A 1\n"),
            ("include/net/z.h", "#define NET_Z 1\n"),
        ] {
            std::fs::write(tmp.path().join(path), content).unwrap();
        }
        let slim = SlimConfig {
            remove_paths: vec![
                "include/net/z.h".to_string(),
                "drivers/z/z.c".to_string(),
                "drivers/a/private.h".to_string(),
                "drivers/z/Kconfig.debug".to_string(),
                "include/linux/a.h".to_string(),
                "drivers/a/a.c".to_string(),
                "drivers/z/private.h".to_string(),
                "drivers/a/Kconfig".to_string(),
            ],
            remove_configs: vec!["Z_SYMBOL".to_string(), "A_SYMBOL".to_string()],
            set_defaults: BTreeMap::from([
                (String::from("Z_DEFAULT"), String::from("n")),
                (String::from("A_DEFAULT"), String::from("y")),
            ]),
            unsafe_allow_root_path_removal: false,
        };

        let manifest = RemovalManifest::from_slim_config_for_tree_with_abi_policy(
            tmp.path(),
            &slim,
            &allow_public_and_uapi_header_removal(),
        )
        .unwrap();

        assert_eq!(
            manifest.removed_paths_vec(),
            vec![
                PathBuf::from("drivers/a/Kconfig"),
                PathBuf::from("drivers/a/a.c"),
                PathBuf::from("drivers/a/private.h"),
                PathBuf::from("drivers/z/Kconfig.debug"),
                PathBuf::from("drivers/z/private.h"),
                PathBuf::from("drivers/z/z.c"),
                PathBuf::from("include/linux/a.h"),
                PathBuf::from("include/net/z.h"),
            ]
        );
        assert_eq!(
            manifest.removed_files.iter().cloned().collect::<Vec<_>>(),
            manifest.removed_paths_vec()
        );
        assert_eq!(
            header_path_strings(manifest.removed_headers.iter().cloned()),
            vec![
                String::from("drivers/a/private.h"),
                String::from("drivers/z/private.h"),
                String::from("include/linux/a.h"),
                String::from("include/net/z.h"),
            ]
        );
        assert_eq!(
            header_path_strings(manifest.removed_public_headers.iter().cloned()),
            vec![
                String::from("include/linux/a.h"),
                String::from("include/net/z.h"),
            ]
        );
        assert_eq!(
            manifest.removed_kconfig_sources_vec(),
            vec![
                PathBuf::from("drivers/a/Kconfig"),
                PathBuf::from("drivers/z/Kconfig.debug"),
            ]
        );
        assert_eq!(
            kbuild_object_strings(manifest.removed_kbuild_objects_vec()),
            vec![String::from("drivers/a/a.o"), String::from("drivers/z/z.o"),]
        );
        assert_eq!(
            manifest.removed_config_symbols_vec(),
            vec![String::from("A_SYMBOL"), String::from("Z_SYMBOL")]
        );
        assert_eq!(
            manifest
                .default_overrides()
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            vec![String::from("A_DEFAULT"), String::from("Z_DEFAULT")]
        );
    }

    #[test]
    fn test_from_slim_config_for_tree_derives_existing_dirs_and_files() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/foo")).unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/bar")).unwrap();
        std::fs::write(tmp.path().join("drivers/foo/file.c"), "int x;\n").unwrap();
        let slim = SlimConfig {
            remove_paths: vec![
                "drivers/foo".to_string(),
                "drivers/foo/file.c".to_string(),
                "drivers/missing".to_string(),
            ],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let manifest = RemovalManifest::from_slim_config_for_tree_with_abi_policy(
            tmp.path(),
            &slim,
            &allow_public_and_uapi_header_removal(),
        )
        .unwrap();

        assert_eq!(
            manifest.removed_paths_vec(),
            vec![
                PathBuf::from("drivers/foo"),
                PathBuf::from("drivers/missing"),
            ]
        );
        assert_eq!(
            manifest.removed_dirs,
            BTreeSet::from([PathBuf::from("drivers/foo")])
        );
        assert!(manifest.removed_files.is_empty());
        assert!(manifest
            .removed_paths
            .contains(Path::new("drivers/missing")));
        assert!(!manifest.removed_dirs.contains(Path::new("drivers/missing")));
        assert!(!manifest
            .removed_files
            .contains(Path::new("drivers/missing")));
    }

    #[test]
    fn test_from_slim_config_for_tree_path_category_derivation_is_stable_and_fail_closed() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/existing_dir")).unwrap();
        std::fs::write(tmp.path().join("drivers/existing_dir/child.txt"), "child\n").unwrap();
        std::fs::write(tmp.path().join("drivers/existing_file.txt"), "file\n").unwrap();
        let slim = SlimConfig {
            remove_paths: vec![
                "drivers/missing_file.txt".to_string(),
                "drivers/existing_file.txt".to_string(),
                "drivers/missing_declared_dir/".to_string(),
                "drivers/existing_dir/".to_string(),
            ],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let manifest = RemovalManifest::from_slim_config_for_tree_with_abi_policy(
            tmp.path(),
            &slim,
            &allow_public_and_uapi_header_removal(),
        )
        .unwrap();

        assert_eq!(
            manifest.removed_paths_vec(),
            vec![
                PathBuf::from("drivers/existing_dir"),
                PathBuf::from("drivers/existing_file.txt"),
                PathBuf::from("drivers/missing_declared_dir"),
                PathBuf::from("drivers/missing_file.txt"),
            ]
        );
        assert_eq!(
            manifest.removed_dirs.iter().cloned().collect::<Vec<_>>(),
            vec![
                PathBuf::from("drivers/existing_dir"),
                PathBuf::from("drivers/missing_declared_dir"),
            ]
        );
        assert_eq!(
            manifest.removed_files.iter().cloned().collect::<Vec<_>>(),
            vec![PathBuf::from("drivers/existing_file.txt")]
        );
        assert!(!manifest
            .removed_dirs
            .contains(Path::new("drivers/missing_file.txt")));
        assert!(!manifest
            .removed_files
            .contains(Path::new("drivers/missing_file.txt")));
        assert_eq!(
            manifest
                .reasons
                .get(&RemovalKey::Dir(PathBuf::from("drivers/existing_dir"))),
            Some(&RemovalReason::SlimRemovePath {
                path: PathBuf::from("drivers/existing_dir")
            })
        );
        assert_eq!(
            manifest.reasons.get(&RemovalKey::File(PathBuf::from(
                "drivers/existing_file.txt"
            ))),
            Some(&RemovalReason::SlimRemovePath {
                path: PathBuf::from("drivers/existing_file.txt")
            })
        );
    }

    #[test]
    fn test_from_slim_config_for_tree_derives_existing_file_without_parent_dedupe() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/foo")).unwrap();
        std::fs::write(tmp.path().join("drivers/foo/file.c"), "int x;\n").unwrap();
        let slim = SlimConfig {
            remove_paths: vec!["drivers/foo/file.c".to_string()],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let manifest = RemovalManifest::from_slim_config_for_tree_with_abi_policy(
            tmp.path(),
            &slim,
            &allow_public_and_uapi_header_removal(),
        )
        .unwrap();

        assert_eq!(
            manifest.removed_files,
            BTreeSet::from([PathBuf::from("drivers/foo/file.c")])
        );
        assert!(manifest.removed_dirs.is_empty());
        assert_eq!(
            manifest
                .reasons
                .get(&RemovalKey::File(PathBuf::from("drivers/foo/file.c"))),
            Some(&RemovalReason::SlimRemovePath {
                path: PathBuf::from("drivers/foo/file.c")
            })
        );
    }

    #[test]
    fn test_from_slim_config_for_tree_retains_nonexistent_declared_path_only() {
        let tmp = tempfile::tempdir().unwrap();
        let slim = SlimConfig {
            remove_paths: vec!["drivers/missing/file.c".to_string()],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let manifest = RemovalManifest::from_slim_config_for_tree_with_abi_policy(
            tmp.path(),
            &slim,
            &allow_public_and_uapi_header_removal(),
        )
        .unwrap();

        assert_eq!(
            manifest.removed_paths_vec(),
            vec![PathBuf::from("drivers/missing/file.c")]
        );
        assert!(manifest.removed_dirs.is_empty());
        assert!(manifest.removed_files.is_empty());
    }

    #[test]
    fn test_from_slim_config_for_tree_treats_trailing_slash_as_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let slim = SlimConfig {
            remove_paths: vec!["drivers/missing/".to_string()],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let manifest = RemovalManifest::from_slim_config_for_tree_with_abi_policy(
            tmp.path(),
            &slim,
            &allow_public_and_uapi_header_removal(),
        )
        .unwrap();

        assert_eq!(
            manifest.removed_dirs,
            BTreeSet::from([PathBuf::from("drivers/missing")])
        );
        assert!(manifest.removed_files.is_empty());
    }

    #[test]
    fn test_from_slim_config_for_tree_rejects_trailing_slash_file_mismatch() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/foo")).unwrap();
        std::fs::write(tmp.path().join("drivers/foo/file.c"), "int x;\n").unwrap();
        let slim = SlimConfig {
            remove_paths: vec!["drivers/foo/file.c/".to_string()],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let err = format!(
            "{:#}",
            RemovalManifest::from_slim_config_for_tree(tmp.path(), &slim).unwrap_err()
        );

        assert!(err.contains("exists but is not a directory"));
    }

    #[test]
    fn test_from_slim_config_for_tree_derives_header_paths() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("include/linux")).unwrap();
        std::fs::create_dir_all(tmp.path().join("include/uapi/linux")).unwrap();
        std::fs::create_dir_all(tmp.path().join("include/net")).unwrap();
        std::fs::create_dir_all(tmp.path().join("arch/x86/include/asm")).unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/foo")).unwrap();
        std::fs::write(tmp.path().join("include/linux/foo.h"), "#define FOO 1\n").unwrap();
        std::fs::write(
            tmp.path().join("include/uapi/linux/abi.h"),
            "#define ABI 1\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("include/net/netfoo.h"),
            "#define NETFOO 1\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("arch/x86/include/asm/foo.h"),
            "#define ASMFOO 1\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("drivers/foo/private.h"),
            "#define PRIVATE 1\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("drivers/foo/not_header.c"),
            "int ignored;\n",
        )
        .unwrap();
        let slim = SlimConfig {
            remove_paths: vec![
                "include/linux/foo.h".to_string(),
                "include/uapi/linux/abi.h".to_string(),
                "include/net/netfoo.h".to_string(),
                "arch/x86/include".to_string(),
                "drivers/foo".to_string(),
            ],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let manifest = RemovalManifest::from_slim_config_for_tree_with_abi_policy(
            tmp.path(),
            &slim,
            &allow_public_and_uapi_header_removal(),
        )
        .unwrap();

        assert_eq!(
            header_path_strings(manifest.removed_headers.iter().cloned()),
            vec![
                String::from("arch/x86/include/asm/foo.h"),
                String::from("drivers/foo/private.h"),
                String::from("include/linux/foo.h"),
                String::from("include/net/netfoo.h"),
                String::from("include/uapi/linux/abi.h"),
            ]
        );
        assert_eq!(
            manifest.reasons.get(&RemovalKey::Header(
                HeaderPath::new("drivers/foo/private.h").unwrap()
            )),
            Some(&RemovalReason::SlimRemovePath {
                path: PathBuf::from("drivers/foo/private.h")
            })
        );
    }

    #[test]
    fn test_from_slim_config_for_tree_tags_removed_public_headers() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("include/linux")).unwrap();
        std::fs::create_dir_all(tmp.path().join("include/uapi/linux")).unwrap();
        std::fs::create_dir_all(tmp.path().join("include/net")).unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/foo")).unwrap();
        std::fs::write(tmp.path().join("include/linux/foo.h"), "#define FOO 1\n").unwrap();
        std::fs::write(
            tmp.path().join("include/uapi/linux/abi.h"),
            "#define ABI 1\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("include/net/netfoo.h"),
            "#define NETFOO 1\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("drivers/foo/private.h"),
            "#define PRIVATE 1\n",
        )
        .unwrap();
        let slim = SlimConfig {
            remove_paths: vec![
                "drivers/foo/private.h".to_string(),
                "include/linux/foo.h".to_string(),
                "include/net/netfoo.h".to_string(),
                "include/uapi/linux/abi.h".to_string(),
            ],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let manifest = RemovalManifest::from_slim_config_for_tree_with_abi_policy(
            tmp.path(),
            &slim,
            &allow_public_and_uapi_header_removal(),
        )
        .unwrap();

        assert_eq!(
            header_path_strings(manifest.removed_public_headers.iter().cloned()),
            vec![
                String::from("include/linux/foo.h"),
                String::from("include/net/netfoo.h"),
                String::from("include/uapi/linux/abi.h"),
            ]
        );
        assert!(!manifest
            .removed_public_headers
            .contains("drivers/foo/private.h"));
        assert_eq!(
            manifest.reasons.get(&RemovalKey::PublicHeader(
                HeaderPath::new("include/net/netfoo.h").unwrap()
            )),
            Some(&RemovalReason::SlimRemovePath {
                path: PathBuf::from("include/net/netfoo.h")
            })
        );
    }

    #[test]
    fn test_from_slim_config_for_tree_rejects_public_header_without_abi_policy() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("include/linux")).unwrap();
        std::fs::write(tmp.path().join("include/linux/foo.h"), "#define FOO 1\n").unwrap();
        let slim = SlimConfig {
            remove_paths: vec!["include/linux/foo.h".to_string()],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let err = format!(
            "{:#}",
            RemovalManifest::from_slim_config_for_tree(tmp.path(), &slim).unwrap_err()
        );

        assert!(err.contains("explicit ABI policy approval"));
        assert!(err.contains("abi.allow_public_header_removal"));
    }

    #[test]
    fn test_from_slim_config_for_tree_rejects_uapi_header_without_uapi_abi_policy() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("include/uapi/linux")).unwrap();
        std::fs::write(
            tmp.path().join("include/uapi/linux/abi.h"),
            "#define ABI 1\n",
        )
        .unwrap();
        let slim = SlimConfig {
            remove_paths: vec!["include/uapi/linux/abi.h".to_string()],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };
        let public_only = AbiPolicyConfig {
            allow_public_header_removal: true,
            allow_uapi_header_removal: false,
        };

        let err = format!(
            "{:#}",
            RemovalManifest::from_slim_config_for_tree_with_abi_policy(
                tmp.path(),
                &slim,
                &public_only,
            )
            .unwrap_err()
        );

        assert!(err.contains("explicit ABI policy approval"));
        assert!(err.contains("abi.allow_uapi_header_removal"));
    }

    #[test]
    fn test_from_slim_config_for_tree_rejects_exported_symbol_provider_with_live_consumer() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/foo")).unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/live")).unwrap();
        std::fs::write(
            tmp.path().join("drivers/foo/provider.c"),
            "void foo_api(void) {}\nEXPORT_SYMBOL_GPL(foo_api);\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("drivers/live/user.c"),
            "extern void foo_api(void);\nvoid user(void) { foo_api(); }\n",
        )
        .unwrap();
        let slim = SlimConfig {
            remove_paths: vec!["drivers/foo/provider.c".to_string()],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let err = format!(
            "{:#}",
            RemovalManifest::from_slim_config_for_tree(tmp.path(), &slim).unwrap_err()
        );

        assert!(err.contains("exported symbol provider removal requires proof"));
        assert!(err.contains("foo_api"));
        assert!(err.contains("drivers/live/user.c"));
    }

    #[test]
    fn test_from_slim_config_for_tree_records_exported_symbol_no_live_consumer_proof() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/foo")).unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/live")).unwrap();
        std::fs::write(
            tmp.path().join("drivers/foo/provider.c"),
            "void foo_api(void) {}\nEXPORT_SYMBOL_NS_GPL(foo_api, FOO_NS);\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("drivers/foo/user.c"),
            "extern void foo_api(void);\nvoid user(void) { foo_api(); }\n",
        )
        .unwrap();
        std::fs::write(tmp.path().join("drivers/live/other.c"), "int other;\n").unwrap();
        let slim = SlimConfig {
            remove_paths: vec!["drivers/foo".to_string()],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let manifest = RemovalManifest::from_slim_config_for_tree(tmp.path(), &slim).unwrap();

        let proofs = manifest.removed_exported_symbols_vec();
        assert_eq!(proofs.len(), 1);
        assert_eq!(proofs[0].symbol.as_str(), "foo_api");
        assert_eq!(proofs[0].provider, PathBuf::from("drivers/foo/provider.c"));
        assert_eq!(proofs[0].export_macro.as_str(), "EXPORT_SYMBOL_NS_GPL");
        assert_eq!(proofs[0].line, 2);
    }

    #[test]
    fn test_from_slim_config_for_tree_rejects_unparsable_removed_export_macro() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/foo")).unwrap();
        std::fs::write(
            tmp.path().join("drivers/foo/provider.c"),
            "EXPORT_SYMBOL_GPL();\n",
        )
        .unwrap();
        let slim = SlimConfig {
            remove_paths: vec!["drivers/foo/provider.c".to_string()],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let err = format!(
            "{:#}",
            RemovalManifest::from_slim_config_for_tree(tmp.path(), &slim).unwrap_err()
        );

        assert!(err.contains("parsable EXPORT_SYMBOL proof"));
        assert!(err.contains("drivers/foo/provider.c:1"));
    }

    #[test]
    fn test_from_slim_config_for_tree_rejects_device_binding_with_live_dts_reference() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("Documentation/devicetree/bindings/vendor"))
            .unwrap();
        std::fs::create_dir_all(tmp.path().join("arch/arm/boot/dts")).unwrap();
        std::fs::write(
            tmp.path()
                .join("Documentation/devicetree/bindings/vendor/foo.yaml"),
            "compatible:\n  const: vendor,foo\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("arch/arm/boot/dts/live.dts"),
            "/ { compatible = \"vendor,foo\"; };\n",
        )
        .unwrap();
        let slim = SlimConfig {
            remove_paths: vec!["Documentation/devicetree/bindings/vendor/foo.yaml".to_string()],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let err = format!(
            "{:#}",
            RemovalManifest::from_slim_config_for_tree(tmp.path(), &slim).unwrap_err()
        );

        assert!(err.contains("device binding removal requires proof"));
        assert!(err.contains("arch/arm/boot/dts/live.dts"));
        assert!(err.contains("vendor,foo"));
    }

    #[test]
    fn test_from_slim_config_for_tree_records_device_binding_no_live_reference_proof() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("Documentation/devicetree/bindings/vendor"))
            .unwrap();
        std::fs::create_dir_all(tmp.path().join("arch/arm/boot/dts")).unwrap();
        std::fs::write(
            tmp.path()
                .join("Documentation/devicetree/bindings/vendor/foo.yaml"),
            "compatible:\n  enum:\n    - vendor,foo\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("arch/arm/boot/dts/removed.dtsi"),
            "/ { compatible = \"vendor,foo\"; };\n",
        )
        .unwrap();
        std::fs::write(tmp.path().join("arch/arm/boot/dts/live.dts"), "/ { };\n").unwrap();
        let slim = SlimConfig {
            remove_paths: vec![
                "Documentation/devicetree/bindings/vendor/foo.yaml".to_string(),
                "arch/arm/boot/dts/removed.dtsi".to_string(),
            ],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let manifest = RemovalManifest::from_slim_config_for_tree(tmp.path(), &slim).unwrap();

        let proofs = manifest.removed_device_bindings_vec();
        assert_eq!(proofs.len(), 1);
        assert_eq!(
            proofs[0].binding,
            PathBuf::from("Documentation/devicetree/bindings/vendor/foo.yaml")
        );
        assert_eq!(
            proofs[0].compatible_strings,
            vec![DeviceCompatible::new("vendor,foo").unwrap()]
        );
    }

    #[test]
    fn test_from_slim_config_for_tree_rejects_runtime_registration_with_live_entry_point() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/foo")).unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/live")).unwrap();
        std::fs::write(
            tmp.path().join("drivers/foo/provider.c"),
            "static int foo_init(void) { return 0; }\nmodule_init(foo_init);\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("drivers/live/user.c"),
            "extern int foo_init(void);\nint call(void) { return foo_init(); }\n",
        )
        .unwrap();
        let slim = SlimConfig {
            remove_paths: vec!["drivers/foo/provider.c".to_string()],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let err = format!(
            "{:#}",
            RemovalManifest::from_slim_config_for_tree(tmp.path(), &slim).unwrap_err()
        );

        assert!(err.contains("runtime registration removal requires proof"));
        assert!(err.contains("drivers/live/user.c"));
        assert!(err.contains("foo_init"));
    }

    #[test]
    fn test_from_slim_config_for_tree_records_runtime_registration_no_live_entry_point_proof() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/foo")).unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/live")).unwrap();
        std::fs::write(
            tmp.path().join("drivers/foo/provider.c"),
            "static int foo_init(void) { return 0; }\nmodule_init(foo_init);\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("drivers/foo/internal.c"),
            "extern int foo_init(void);\nint call(void) { return foo_init(); }\n",
        )
        .unwrap();
        std::fs::write(tmp.path().join("drivers/live/user.c"), "int live;\n").unwrap();
        let slim = SlimConfig {
            remove_paths: vec!["drivers/foo".to_string()],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let manifest = RemovalManifest::from_slim_config_for_tree(tmp.path(), &slim).unwrap();

        let proofs = manifest.removed_runtime_registrations_vec();
        assert_eq!(proofs.len(), 1);
        assert_eq!(proofs[0].provider, PathBuf::from("drivers/foo/provider.c"));
        assert_eq!(proofs[0].registration_macro.as_str(), "module_init");
        assert_eq!(proofs[0].entry_points, vec![String::from("foo_init")]);
        assert_eq!(proofs[0].line, 2);
    }

    #[test]
    fn test_from_slim_config_rejects_uapi_directory_without_uapi_abi_policy() {
        let slim = SlimConfig {
            remove_paths: vec!["include/uapi/linux".to_string()],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let err = format!(
            "{:#}",
            RemovalManifest::from_slim_config(&slim).unwrap_err()
        );

        assert!(err.contains("UAPI removal requires explicit ABI policy approval"));
        assert!(err.contains("abi.allow_uapi_header_removal"));
    }

    #[test]
    fn test_from_slim_config_for_tree_requires_exact_uapi_manifest_truth() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("include/uapi/linux")).unwrap();
        std::fs::write(
            tmp.path().join("include/uapi/linux/abi.h"),
            "#define ABI 1\n",
        )
        .unwrap();
        let slim = SlimConfig {
            remove_paths: vec!["include/uapi".to_string()],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };
        let abi_policy = AbiPolicyConfig {
            allow_public_header_removal: false,
            allow_uapi_header_removal: true,
        };

        let manifest = RemovalManifest::from_slim_config_for_tree_with_abi_policy(
            tmp.path(),
            &slim,
            &abi_policy,
        )
        .unwrap();

        assert!(manifest.removed_headers.is_empty());
        assert!(manifest.removed_public_headers.is_empty());
        assert!(!manifest.reasons.contains_key(&RemovalKey::Header(
            HeaderPath::new("include/uapi/linux/abi.h").unwrap()
        )));
    }

    #[test]
    fn test_from_slim_config_for_tree_does_not_derive_missing_header_path() {
        let tmp = tempfile::tempdir().unwrap();
        let slim = SlimConfig {
            remove_paths: vec!["drivers/foo/missing.h".to_string()],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let manifest = RemovalManifest::from_slim_config_for_tree(tmp.path(), &slim).unwrap();

        assert_eq!(
            manifest.removed_paths_vec(),
            vec![PathBuf::from("drivers/foo/missing.h")]
        );
        assert!(manifest.removed_headers.is_empty());
        assert!(manifest.removed_public_headers.is_empty());
    }

    #[test]
    fn test_from_slim_config_for_tree_requires_exact_public_header_truth() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("include/linux")).unwrap();
        std::fs::create_dir_all(tmp.path().join("include/net")).unwrap();
        std::fs::create_dir_all(tmp.path().join("include/uapi/linux")).unwrap();
        std::fs::write(tmp.path().join("include/linux/foo.h"), "#define FOO 1\n").unwrap();
        std::fs::write(
            tmp.path().join("include/net/netfoo.h"),
            "#define NETFOO 1\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("include/uapi/linux/abi.h"),
            "#define ABI 1\n",
        )
        .unwrap();
        let slim = SlimConfig {
            remove_paths: vec!["include".to_string()],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let manifest = RemovalManifest::from_slim_config_for_tree(tmp.path(), &slim).unwrap();

        assert!(manifest.removed_headers.is_empty());
        assert!(manifest.removed_public_headers.is_empty());
    }

    #[test]
    fn test_from_slim_config_for_tree_derives_generated_headers_only_for_configured_roots() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("include/generated/linux")).unwrap();
        std::fs::create_dir_all(tmp.path().join("arch/x86/include/generated/asm")).unwrap();
        std::fs::write(
            tmp.path().join("include/generated/linux/version.h"),
            "#define VERSION 1\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("arch/x86/include/generated/asm/offsets.h"),
            "#define OFFSETS 1\n",
        )
        .unwrap();
        let slim = SlimConfig {
            remove_paths: vec![
                "include/generated".to_string(),
                "arch/x86/include/generated".to_string(),
            ],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let without_generated_roots =
            RemovalManifest::from_slim_config_for_tree(tmp.path(), &slim).unwrap();
        let with_generated_roots =
            RemovalManifest::from_slim_config_for_tree_with_generated_include_roots(
                tmp.path(),
                &slim,
                &[
                    PathBuf::from("include/generated"),
                    PathBuf::from("arch/x86/include/generated"),
                ],
            )
            .unwrap();

        assert!(without_generated_roots.removed_headers.is_empty());
        assert!(without_generated_roots.removed_public_headers.is_empty());
        assert_eq!(
            header_path_strings(with_generated_roots.removed_headers.iter().cloned()),
            vec![
                String::from("arch/x86/include/generated/asm/offsets.h"),
                String::from("include/generated/linux/version.h"),
            ]
        );
        assert!(with_generated_roots.removed_public_headers.is_empty());
    }

    #[test]
    fn test_generated_include_roots_reject_empty_absolute_or_parent_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let slim = SlimConfig {
            remove_paths: Vec::new(),
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        for root in [
            "",
            " ",
            "/tmp/include/generated",
            "C:/include/generated",
            r"include\..\generated",
        ] {
            let err = format!(
                "{:#}",
                RemovalManifest::from_slim_config_for_tree_with_generated_include_roots(
                    tmp.path(),
                    &slim,
                    &[PathBuf::from(root)],
                )
                .unwrap_err()
            );

            assert!(
                err.contains("generated include roots must not be empty")
                    || err.contains("generated include roots must be relative")
                    || err.contains("generated include roots must not contain '..'"),
                "unexpected error for {root}: {err}"
            );
        }
    }

    #[test]
    fn test_from_slim_config_for_tree_derives_kconfig_sources_from_removed_files() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/foo")).unwrap();
        std::fs::write(tmp.path().join("drivers/foo/Kconfig"), "config FOO\n").unwrap();
        std::fs::write(
            tmp.path().join("drivers/foo/Kconfig.debug"),
            "config FOO_DEBUG\n",
        )
        .unwrap();
        std::fs::write(tmp.path().join("drivers/foo/not_kconfig"), "ignored\n").unwrap();
        let slim = SlimConfig {
            remove_paths: vec![
                "drivers/foo/Kconfig".to_string(),
                "drivers/foo/Kconfig.debug".to_string(),
                "drivers/foo/not_kconfig".to_string(),
            ],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let manifest = RemovalManifest::from_slim_config_for_tree(tmp.path(), &slim).unwrap();

        assert_eq!(
            manifest.removed_kconfig_sources_vec(),
            vec![
                PathBuf::from("drivers/foo/Kconfig"),
                PathBuf::from("drivers/foo/Kconfig.debug"),
            ]
        );
        assert_eq!(
            manifest
                .reasons
                .get(&RemovalKey::KconfigSource(PathBuf::from(
                    "drivers/foo/Kconfig.debug"
                ))),
            Some(&RemovalReason::SlimRemovePath {
                path: PathBuf::from("drivers/foo/Kconfig.debug")
            })
        );
    }

    #[test]
    fn test_from_slim_config_for_tree_derives_kconfig_sources_from_removed_dir_and_source_refs() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/foo")).unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/keep")).unwrap();
        std::fs::write(
            tmp.path().join("Kconfig"),
            concat!(
                "source \"drivers/foo/CustomConfig\"\n",
                "source \"drivers/keep/Kconfig\"\n",
                "source \"$DYNAMIC/Kconfig\"\n",
            ),
        )
        .unwrap();
        std::fs::write(tmp.path().join("drivers/foo/Kconfig"), "config FOO\n").unwrap();
        std::fs::write(
            tmp.path().join("drivers/foo/Kconfig.extra"),
            "config FOO_EXTRA\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("drivers/foo/CustomConfig"),
            "config FOO_CUSTOM\n",
        )
        .unwrap();
        std::fs::write(tmp.path().join("drivers/keep/Kconfig"), "config KEEP\n").unwrap();
        let slim = SlimConfig {
            remove_paths: vec!["drivers/foo".to_string()],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let manifest = RemovalManifest::from_slim_config_for_tree(tmp.path(), &slim).unwrap();

        assert_eq!(
            manifest.removed_kconfig_sources_vec(),
            vec![
                PathBuf::from("drivers/foo/CustomConfig"),
                PathBuf::from("drivers/foo/Kconfig"),
                PathBuf::from("drivers/foo/Kconfig.extra"),
            ]
        );
    }

    #[test]
    fn test_from_slim_config_derives_declared_kconfig_sources_without_tree() {
        let slim = SlimConfig {
            remove_paths: vec![
                "drivers/foo/Kconfig".to_string(),
                "drivers/foo/Kconfig.debug".to_string(),
                "drivers/foo/not_kconfig".to_string(),
            ],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let manifest = RemovalManifest::from_slim_config_with_abi_policy(
            &slim,
            &allow_public_and_uapi_header_removal(),
        )
        .unwrap();

        assert_eq!(
            manifest.removed_kconfig_sources_vec(),
            vec![
                PathBuf::from("drivers/foo/Kconfig"),
                PathBuf::from("drivers/foo/Kconfig.debug"),
            ]
        );
    }

    #[test]
    fn test_from_slim_config_for_tree_derives_kbuild_objects_from_removed_sources() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/foo")).unwrap();
        std::fs::write(tmp.path().join("drivers/foo/remove.c"), "int remove;\n").unwrap();
        std::fs::write(tmp.path().join("drivers/foo/start.S"), "ENTRY(start)\n").unwrap();
        std::fs::write(
            tmp.path().join("drivers/foo/ignored.h"),
            "#define IGNORED 1\n",
        )
        .unwrap();
        let slim = SlimConfig {
            remove_paths: vec![
                "drivers/foo/remove.c".to_string(),
                "drivers/foo/start.S".to_string(),
                "drivers/foo/ignored.h".to_string(),
            ],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let manifest = RemovalManifest::from_slim_config_for_tree(tmp.path(), &slim).unwrap();

        assert_eq!(
            kbuild_object_strings(manifest.removed_kbuild_objects_vec()),
            vec![
                String::from("drivers/foo/remove.o"),
                String::from("drivers/foo/start.o"),
            ]
        );
        assert_eq!(
            manifest.reasons.get(&RemovalKey::KbuildObject(
                KbuildObject::new("drivers/foo/remove.o").unwrap()
            )),
            Some(&RemovalReason::SlimRemovePath {
                path: PathBuf::from("drivers/foo/remove.o")
            })
        );
    }

    #[test]
    fn test_from_slim_config_for_tree_derives_kbuild_directory_refs() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/foo/remove/subdir")).unwrap();
        std::fs::write(
            tmp.path().join("drivers/foo/Makefile"),
            "obj-y += remove/subdir/\n",
        )
        .unwrap();
        let slim = SlimConfig {
            remove_paths: vec!["drivers/foo/remove".to_string()],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let manifest = RemovalManifest::from_slim_config_for_tree(tmp.path(), &slim).unwrap();

        assert_eq!(
            kbuild_object_strings(manifest.removed_kbuild_objects_vec()),
            vec![
                String::from("drivers/foo/remove/"),
                String::from("drivers/foo/remove/subdir/"),
            ]
        );
    }

    #[test]
    fn test_from_slim_config_for_tree_skips_make_syntax_directory_refs_inside_removed_makefile() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/foo/bar/subdir")).unwrap();
        std::fs::write(
            tmp.path().join("drivers/foo/Makefile"),
            "obj-y += $(VAR)/subdir/\n",
        )
        .unwrap();
        let slim = SlimConfig {
            remove_paths: vec!["drivers/foo".to_string()],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let manifest = RemovalManifest::from_slim_config_for_tree(tmp.path(), &slim).unwrap();
        let removed = kbuild_object_strings(manifest.removed_kbuild_objects_vec());

        assert!(removed.contains(&String::from("drivers/foo/")));
        assert!(!removed.iter().any(|path| path.contains('$')));
    }

    #[test]
    fn test_from_slim_config_for_tree_derives_stale_composite_kbuild_objects_from_index() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/foo")).unwrap();
        std::fs::write(tmp.path().join("drivers/foo/remove.c"), "int remove;\n").unwrap();
        std::fs::write(
            tmp.path().join("drivers/foo/Makefile"),
            "foo-y += remove.o\nobj-y += foo.o\n",
        )
        .unwrap();
        let slim = SlimConfig {
            remove_paths: vec!["drivers/foo/remove.c".to_string()],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let manifest = RemovalManifest::from_slim_config_for_tree(tmp.path(), &slim).unwrap();

        assert_eq!(
            kbuild_object_strings(manifest.removed_kbuild_objects_vec()),
            vec![
                String::from("drivers/foo/foo.o"),
                String::from("drivers/foo/remove.o"),
            ]
        );
    }

    #[test]
    fn test_from_slim_config_for_tree_keeps_live_composite_kbuild_target() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/foo")).unwrap();
        std::fs::write(tmp.path().join("drivers/foo/live.c"), "int live;\n").unwrap();
        std::fs::write(tmp.path().join("drivers/foo/remove.c"), "int remove;\n").unwrap();
        std::fs::write(
            tmp.path().join("drivers/foo/Makefile"),
            "foo-y += live.o remove.o\nobj-y += foo.o\n",
        )
        .unwrap();
        let slim = SlimConfig {
            remove_paths: vec!["drivers/foo/remove.c".to_string()],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let manifest = RemovalManifest::from_slim_config_for_tree(tmp.path(), &slim).unwrap();

        assert_eq!(
            kbuild_object_strings(manifest.removed_kbuild_objects_vec()),
            vec![String::from("drivers/foo/remove.o")]
        );
    }

    #[test]
    fn test_from_slim_config_derives_declared_kbuild_objects_without_tree() {
        let slim = SlimConfig {
            remove_paths: vec![
                "drivers/foo/remove.c".to_string(),
                "drivers/foo/start.S".to_string(),
                "drivers/foo/remove/".to_string(),
            ],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let manifest = RemovalManifest::from_slim_config_with_abi_policy(
            &slim,
            &allow_public_and_uapi_header_removal(),
        )
        .unwrap();

        assert_eq!(
            kbuild_object_strings(manifest.removed_kbuild_objects_vec()),
            vec![
                String::from("drivers/foo/remove.o"),
                String::from("drivers/foo/remove/"),
                String::from("drivers/foo/start.o"),
            ]
        );
    }

    #[test]
    fn test_is_public_header_path_tags_public_roots() {
        assert!(is_public_header_path(Path::new("include/linux/foo.h")));
        assert!(is_public_header_path(Path::new("include/uapi/linux/foo.h")));
        assert!(is_public_header_path(Path::new("include/net/foo.h")));
        assert!(is_public_header_path(Path::new(
            "include/generated/uapi/linux/foo.h"
        )));
        assert!(is_public_header_path(Path::new(
            "arch/x86/include/uapi/asm/foo.h"
        )));
        assert!(!is_public_header_path(Path::new("drivers/foo/private.h")));
        assert!(!is_public_header_path(Path::new("include/drm/foo.h")));
    }

    #[test]
    fn test_is_uapi_header_path_tags_common_uapi_roots() {
        assert!(is_uapi_header_path(Path::new("include/uapi/linux/foo.h")));
        assert!(is_uapi_header_path(Path::new(
            "include/generated/uapi/linux/foo.h"
        )));
        assert!(is_uapi_header_path(Path::new(
            "arch/x86/include/uapi/asm/foo.h"
        )));
        assert!(is_uapi_header_path(Path::new(
            "arch/arm64/include/generated/uapi/asm/foo.h"
        )));
        assert!(!is_uapi_header_path(Path::new("include/linux/foo.h")));
        assert!(!is_uapi_header_path(Path::new("drivers/foo/uapi.h")));
    }

    #[test]
    fn test_is_uapi_path_tags_common_uapi_roots() {
        assert!(is_uapi_path(Path::new("include/uapi")));
        assert!(is_uapi_path(Path::new("include/uapi/linux/foo.h")));
        assert!(is_uapi_path(Path::new("include/generated/uapi")));
        assert!(is_uapi_path(Path::new("arch/x86/include/uapi/asm/foo.h")));
        assert!(is_uapi_path(Path::new("arch/arm64/include/generated/uapi")));
        assert!(!is_uapi_path(Path::new("include/linux/foo.h")));
        assert!(!is_uapi_path(Path::new("drivers/foo/uapi.h")));
    }

    #[test]
    fn test_from_slim_config_preserves_explicit_public_children_under_broad_parent() {
        let slim = SlimConfig {
            remove_paths: vec![
                "include".to_string(),
                "include/linux/foo.h".to_string(),
                "include/net/netfoo.h".to_string(),
                "include/uapi/linux/foo.h".to_string(),
            ],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let manifest = RemovalManifest::from_slim_config_with_abi_policy(
            &slim,
            &allow_public_and_uapi_header_removal(),
        )
        .unwrap();

        assert_eq!(
            manifest.removed_paths_vec(),
            vec![
                PathBuf::from("include"),
                PathBuf::from("include/linux/foo.h"),
                PathBuf::from("include/net/netfoo.h"),
                PathBuf::from("include/uapi/linux/foo.h"),
            ]
        );
    }

    #[test]
    fn test_from_slim_config_rejects_empty_removed_config_symbol() {
        let slim = SlimConfig {
            remove_paths: Vec::new(),
            remove_configs: vec![String::new()],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let err = format!(
            "{:#}",
            RemovalManifest::from_slim_config(&slim).unwrap_err()
        );

        assert!(err.contains("slim.remove_configs must not contain empty values"));
    }

    #[test]
    fn test_from_slim_config_rejects_invalid_removed_config_symbol() {
        let slim = SlimConfig {
            remove_paths: Vec::new(),
            remove_configs: vec![String::from("DRM_AMDGPU || DRM_RADEON")],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        };

        let err = format!(
            "{:#}",
            RemovalManifest::from_slim_config(&slim).unwrap_err()
        );

        assert!(err.contains("invalid Kconfig symbol"));
        assert!(err.contains("invalid characters"));
    }

    #[test]
    fn test_from_slim_config_rejects_conflicting_removed_symbol_and_default() {
        let slim = SlimConfig {
            remove_paths: Vec::new(),
            remove_configs: vec!["DRM_AMDGPU".to_string()],
            set_defaults: BTreeMap::from([(String::from("DRM_AMDGPU"), String::from("n"))]),
            unsafe_allow_root_path_removal: false,
        };

        let err = format!(
            "{:#}",
            RemovalManifest::from_slim_config(&slim).unwrap_err()
        );

        assert!(err.contains("both target 'DRM_AMDGPU'"));
    }
}
