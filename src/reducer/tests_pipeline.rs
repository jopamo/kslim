use super::*;

#[test]
fn test_reducer_run_is_noop_without_slim_input() {
    let tmp = tempfile::tempdir().unwrap();
    let profile = default_profile_config("v1.0");

    let stats = run(tmp.path().to_str().unwrap(), &profile).unwrap();

    assert!(!stats.ran);
    assert_eq!(stats.files_removed, 0);
    assert!(stats.removal.removed_files.is_empty());
    assert!(stats.edits.is_empty());
}
#[test]
fn test_run_reducer_is_noop_for_noop_slim_config() {
    let tmp = tempfile::tempdir().unwrap();

    let result = run_reducer(
        &kernel_root(tmp.path()),
        &SlimConfig::default(),
        &crate::config::ReducerConfig::default(),
    )
    .unwrap();

    assert!(result.manifest.is_none());
    assert!(result.initial_index.is_none());
    assert!(result.declared_prune.is_none());
    assert!(result.post_prune_index.is_none());
    assert_eq!(result.status, ReducerStatus::Success);
    assert!(result.publishable);
    assert_eq!(result.convergence, ConvergenceStatus::Converged);
    assert_eq!(result.final_build_status, BuildMatrixStatus::NotRun);
    assert!(result.passes.is_empty());
    assert_eq!(result.edit_summary.total_edits, 0);
    assert_eq!(result.diagnostic_summary.unsupported_kconfig_expressions, 0);
    assert!(result.touched_files.is_empty());
    assert!(result.skipped_sites.is_empty());
    assert!(result.fixups_applied.is_empty());
    assert!(!result.stats.ran);
    assert!(result.stats.edits.is_empty());
}
#[test]
fn test_reducer_run_executes_prune_for_effective_slim_input() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(root.join("drivers/foo/bar.c"), "int bar;\n").unwrap();

    let mut profile = default_profile_config("v1.0");
    profile.slim = Some(SlimConfig {
        remove_paths: vec!["drivers/foo".to_string()],
        ..SlimConfig::default()
    });

    let stats = run(root.to_str().unwrap(), &profile).unwrap();

    assert!(stats.ran);
    assert_eq!(stats.files_removed, 1);
    assert_eq!(stats.removal.removed_files.len(), 1);
    assert!(!root.join("drivers/foo").exists());
    assert!(!stats.edits.is_empty());
}
#[test]
fn test_run_reducer_entrypoint_executes_manifest_driven_pipeline() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(root.join("drivers/foo/bar.c"), "int bar;\n").unwrap();
    std::fs::write(
        root.join("drivers/foo/helper.c"),
        "#include \"local.h\"\nint helper;\n",
    )
    .unwrap();

    let result = run_reducer(
        &kernel_root(root),
        &SlimConfig {
            remove_paths: vec!["drivers/foo".to_string()],
            ..SlimConfig::default()
        },
        &crate::config::ReducerConfig::default(),
    )
    .unwrap();

    assert_eq!(
        result
            .manifest
            .as_ref()
            .map(RemovalManifest::removed_paths)
            .map(|paths| paths.iter().cloned().collect::<Vec<_>>())
            .unwrap(),
        vec![PathBuf::from("drivers/foo")]
    );
    let initial_index = result.initial_index.as_ref().unwrap();
    assert!(initial_index.contains_file(Path::new("drivers/foo/bar.c")));
    assert!(initial_index
        .find_include_site(Path::new("drivers/foo/helper.c"), "local.h")
        .is_some());
    let declared_prune = result.declared_prune.as_ref().unwrap();
    assert_eq!(declared_prune.files_removed, 2);
    assert_eq!(declared_prune.removal.removed_files.len(), 2);
    let post_prune_index = result.post_prune_index.as_ref().unwrap();
    assert!(!post_prune_index.contains_file(Path::new("drivers/foo/bar.c")));
    assert!(post_prune_index
        .find_include_site(Path::new("drivers/foo/helper.c"), "local.h")
        .is_none());
    assert!(result.stats.ran);
    assert_eq!(result.status, ReducerStatus::Success);
    assert!(result.publishable);
    assert_eq!(result.convergence, ConvergenceStatus::Converged);
    assert_eq!(result.final_build_status, BuildMatrixStatus::NotRun);
    assert_eq!(result.passes.len(), 1);
    assert_eq!(result.passes[0].name, "reducer.pipeline");
    assert!(result.passes[0].changed);
    assert_eq!(result.edit_summary.total_edits, result.stats.edits.len());
    assert_eq!(
        result.edit_summary.files_removed,
        result.stats.files_removed
    );
    assert_eq!(result.diagnostic_summary.unsupported_kconfig_expressions, 0);
    assert!(result
        .touched_files
        .contains(&PathBuf::from("drivers/foo/bar.c")));
    assert_eq!(result.stats.files_removed, 2);
    assert_eq!(result.stats.removal.removed_files.len(), 2);
    assert!(!root.join("drivers/foo").exists());
    assert!(!result.stats.edits.is_empty());
}
#[test]
fn test_fixed_reducer_pipeline_order_is_explicit() {
    assert_eq!(
        crate::reducer::pipeline::FIXED_REDUCER_PIPELINE,
        &[
            "build RemovalManifest",
            "build initial TreeIndex",
            "prune declared paths",
            "rebuild full index",
            "rewrite Kconfig",
            "rebuild Kconfig index",
            "rewrite kbuild",
            "rebuild kbuild index",
            "fold preprocessor branches",
            "rebuild C/header index",
            "rewrite/report include sites",
            "run selected builds/tests",
            "classify diagnostics",
            "apply deterministic fixers",
            "reindex and repeat until stable or pass limit reached",
        ]
    );
}
#[test]
fn test_run_reducer_rebuilds_indexes_after_each_mutating_stage() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::create_dir_all(root.join("drivers/live")).unwrap();
    std::fs::write(
        root.join("Kconfig"),
        concat!(
            "source \"drivers/foo/Kconfig\"\n",
            "config REMOVE_ME\n",
            "\tbool \"Remove me\"\n",
            "\tdefault y\n",
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("drivers/foo/Kconfig"),
        "config FOO\n\tbool \"Foo\"\n",
    )
    .unwrap();
    std::fs::write(root.join("Makefile"), "obj-y += drivers/foo/foo.o\n").unwrap();
    std::fs::write(root.join("drivers/foo/foo.c"), "int foo;\n").unwrap();
    std::fs::write(root.join("drivers/foo/private.h"), "#define PRIVATE 1\n").unwrap();
    std::fs::write(
        root.join("drivers/foo/live_removed.h"),
        "#define LIVE_REMOVED 1\n",
    )
    .unwrap();
    std::fs::write(
        root.join("drivers/live/helper.c"),
        concat!(
            "#ifdef CONFIG_REMOVE_ME\n",
            "#include \"../foo/private.h\"\n",
            "#endif\n",
            "#include \"../foo/live_removed.h\"\n",
            "int live;\n",
        ),
    )
    .unwrap();

    let result = run_reducer(
        &kernel_root(root),
        &SlimConfig {
            remove_paths: vec!["drivers/foo".to_string()],
            remove_configs: vec!["REMOVE_ME".to_string()],
            set_defaults: BTreeMap::new(),
            unsafe_allow_root_path_removal: false,
        },
        &ReducerConfig::default(),
    )
    .unwrap();

    assert!(result
        .post_kconfig_index
        .as_ref()
        .unwrap()
        .find_kconfig_source_ref(Path::new("Kconfig"), 1, "drivers/foo/Kconfig")
        .is_none());
    assert!(result
        .post_kbuild_index
        .as_ref()
        .unwrap()
        .find_kbuild_object_refs("drivers/foo/foo.o")
        .is_empty());
    assert!(result
        .post_cpp_index
        .as_ref()
        .unwrap()
        .find_include_site(Path::new("drivers/live/helper.c"), "../foo/private.h")
        .is_none());
    assert!(result
        .post_cpp_index
        .as_ref()
        .unwrap()
        .find_include_site(Path::new("drivers/live/helper.c"), "../foo/live_removed.h")
        .is_some());
    assert!(result
        .post_include_index
        .as_ref()
        .unwrap()
        .find_include_site(Path::new("drivers/live/helper.c"), "../foo/live_removed.h")
        .is_none());
}
#[test]
fn test_reducer_rerun_on_already_reduced_tree_converges_to_zero_edits() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::create_dir_all(root.join("drivers/live")).unwrap();
    std::fs::write(
        root.join("Kconfig"),
        concat!(
            "source \"drivers/foo/Kconfig\"\n",
            "\n",
            "config REMOVE_ME\n",
            "\tbool \"Remove me\"\n",
            "\tdefault y\n",
            "\n",
            "config KEEP_ME\n",
            "\tbool \"Keep me\"\n",
            "\tdefault y\n",
            "\n",
            "config LIVE_DRIVER\n",
            "\tbool \"Live driver\"\n",
            "\tdepends on REMOVE_ME || KEEP_ME\n",
            "\tdefault y if REMOVE_ME\n",
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("Makefile"),
        concat!(
            "obj-$(CONFIG_REMOVE_ME) += drivers/foo/foo.o\n",
            "obj-y += drivers/live/helper.o\n",
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("drivers/foo/Kconfig"),
        "config FOO_DRIVER\n\tbool \"Foo\"\n",
    )
    .unwrap();
    std::fs::write(root.join("drivers/foo/foo.c"), "int foo;\n").unwrap();
    std::fs::write(root.join("drivers/foo/private.h"), "#define PRIVATE 1\n").unwrap();
    std::fs::write(
        root.join("drivers/live/helper.c"),
        concat!(
            "#include \"../foo/private.h\"\n",
            "#ifdef CONFIG_REMOVE_ME\n",
            "int dead;\n",
            "#else\n",
            "int live;\n",
            "#endif\n",
        ),
    )
    .unwrap();

    let slim = SlimConfig {
        remove_paths: vec!["drivers/foo".to_string()],
        remove_configs: vec!["REMOVE_ME".to_string()],
        set_defaults: BTreeMap::from([(String::from("KEEP_ME"), String::from("n"))]),
        unsafe_allow_root_path_removal: false,
    };

    let first = run_reducer(&kernel_root(root), &slim, &ReducerConfig::default()).unwrap();
    assert!(first.stats.ran);
    assert!(
        !first.stats.edits.is_empty(),
        "first reducer pass should record concrete edits"
    );
    assert!(!root.join("drivers/foo").exists());
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/live/helper.c")).unwrap(),
        "int live;\n"
    );

    let kconfig_after_first = std::fs::read_to_string(root.join("Kconfig")).unwrap();
    let makefile_after_first = std::fs::read_to_string(root.join("Makefile")).unwrap();
    let helper_after_first =
        std::fs::read_to_string(root.join("drivers/live/helper.c")).unwrap();
    let second = run_reducer(&kernel_root(root), &slim, &ReducerConfig::default()).unwrap();

    assert!(second.stats.ran);
    assert_zero_edit_reducer_rerun(&second);
    assert_eq!(
        std::fs::read_to_string(root.join("Kconfig")).unwrap(),
        kconfig_after_first
    );
    assert_eq!(
        std::fs::read_to_string(root.join("Makefile")).unwrap(),
        makefile_after_first
    );
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/live/helper.c")).unwrap(),
        helper_after_first
    );
}
#[test]
fn test_declared_prune_reducer_entrypoint_rerun_converges_to_zero_edits() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::create_dir_all(root.join("drivers/live")).unwrap();
    std::fs::write(
        root.join("Kconfig"),
        concat!(
            "source \"drivers/foo/Kconfig\"\n",
            "\n",
            "config REMOVE_ME\n",
            "\tbool \"Remove me\"\n",
            "\tdefault y\n",
            "\n",
            "config LIVE_DRIVER\n",
            "\tbool \"Live driver\"\n",
            "\tdepends on REMOVE_ME || KEEP_ME\n",
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("Makefile"),
        "obj-$(CONFIG_REMOVE_ME) += drivers/foo/foo.o\n",
    )
    .unwrap();
    std::fs::write(
        root.join("drivers/foo/Kconfig"),
        "config FOO_DRIVER\n\tbool \"Foo\"\n",
    )
    .unwrap();
    std::fs::write(root.join("drivers/foo/foo.c"), "int foo;\n").unwrap();
    std::fs::write(
        root.join("drivers/live/helper.c"),
        "#ifdef CONFIG_REMOVE_ME\nint dead;\n#else\nint live;\n#endif\n",
    )
    .unwrap();

    let slim = SlimConfig {
        remove_paths: vec!["drivers/foo".to_string()],
        remove_configs: vec!["REMOVE_ME".to_string()],
        set_defaults: BTreeMap::new(),
        unsafe_allow_root_path_removal: false,
    };
    let config = ReducerConfig::default();

    let manifest = RemovalManifest::from_slim_config_for_tree(root, &slim).unwrap();
    let declared = crate::prune::prune_declared_paths_from_manifest_with_policy(
        root,
        &manifest,
        crate::prune::RemovalFailurePolicy::from_reducer_config(&config),
    )
    .unwrap();
    let first =
        run_reducer_after_declared_prune(&kernel_root(root), manifest, declared, &config)
            .unwrap();
    assert!(first.stats.ran);
    assert!(!first.stats.edits.is_empty());

    let manifest = RemovalManifest::from_slim_config_for_tree(root, &slim).unwrap();
    let declared = crate::prune::prune_declared_paths_from_manifest_with_policy(
        root,
        &manifest,
        crate::prune::RemovalFailurePolicy::from_reducer_config(&config),
    )
    .unwrap();
    let second =
        run_reducer_after_declared_prune(&kernel_root(root), manifest, declared, &config)
            .unwrap();

    assert!(second.stats.ran);
    assert_zero_edit_reducer_rerun(&second);
}
