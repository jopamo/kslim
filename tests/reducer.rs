mod common;
use common::*;

#[test]
fn test_generate_slims_amdgpu_and_runs_selftests() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream_with_amdgpu(tmp.path(), "gpu", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Slim AMDGPU"

[base]
ref = "v1.0"

[slim]
remove_paths = ["drivers/gpu/drm/amd/amdgpu"]

[selftests]
commands = ["test ! -e drivers/gpu/drm/amd/amdgpu"]
"#,
    )
    .unwrap();

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["generate"]);
    assert!(
        ok,
        "generate failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(
        stdout.contains("selftests:"),
        "generate should report selftests: {}",
        stdout
    );
    assert!(!output_dir.join("drivers/gpu/drm/amd/amdgpu").exists());

    let kconfig = std::fs::read_to_string(output_dir.join("drivers/gpu/drm/Kconfig")).unwrap();
    assert!(kconfig.contains("# kslim: removed source \"drivers/gpu/drm/amd/amdgpu/Kconfig\""));

    let makefile = std::fs::read_to_string(output_dir.join("drivers/gpu/drm/Makefile")).unwrap();
    assert!(!makefile.contains("amd/amdgpu/"));

    let report = std::fs::read_to_string(output_meta_path(&output_dir, "report.txt")).unwrap();
    assert!(report.contains("Selftests:"));
    assert!(report.contains("Custom commands: 1"));

    let report_artifact_names = [
        "report.txt",
        "reducer-report.md",
        "reducer-report.json",
        "diagnostics.json",
        "edit-summary.json",
        "kconfig-solver-report.json",
        "kconfig-rewrite-report.json",
    ];
    let reports_before = report_artifact_names
        .iter()
        .map(|name| {
            (
                *name,
                std::fs::read_to_string(output_meta_path(&output_dir, name)).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    let output_head_before = git_in(output_dir.to_str().unwrap(), &["rev-parse", "HEAD"]);

    let (ok2, stdout2, stderr2) = kslim_in(&kslim_dir, &["generate"]);
    assert!(
        ok2,
        "second generate should succeed for deterministic report comparison: stdout={:?} stderr={:?}",
        stdout2, stderr2
    );
    assert_eq!(
        git_in(output_dir.to_str().unwrap(), &["rev-parse", "HEAD"]),
        output_head_before,
        "identical inputs must not create a new output commit"
    );
    for (name, before) in reports_before {
        let after = std::fs::read_to_string(output_meta_path(&output_dir, name)).unwrap();
        assert_eq!(
            after, before,
            "report artifact {} must be deterministic across identical inputs",
            name
        );
    }
}

#[test]
fn test_reduce_tree_rerun_on_own_output_converges_to_zero_edits() {
    let tmp = tempfile::tempdir().unwrap();
    let tree = tmp.path().join("kernel-tree");
    std::fs::create_dir_all(tree.join("drivers/foo")).unwrap();
    std::fs::create_dir_all(tree.join("drivers/live")).unwrap();
    std::fs::write(
        tree.join("Kconfig"),
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
        tree.join("Makefile"),
        concat!(
            "obj-$(CONFIG_REMOVE_ME) += drivers/foo/foo.o\n",
            "obj-y += drivers/live/helper.o\n",
        ),
    )
    .unwrap();
    std::fs::write(
        tree.join("drivers/foo/Kconfig"),
        "config FOO_DRIVER\n\tbool \"Foo\"\n",
    )
    .unwrap();
    std::fs::write(tree.join("drivers/foo/foo.c"), "int foo;\n").unwrap();
    std::fs::write(tree.join("drivers/foo/private.h"), "#define PRIVATE 1\n").unwrap();
    std::fs::write(
        tree.join("drivers/live/helper.c"),
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

    let output_dir = tmp.path().join("unused-output");
    let upstream = tmp.path().join("unused-upstream.git");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        upstream.to_str().unwrap(),
    );
    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Direct reducer rerun convergence"

[base]
ref = "v1.0"

[slim]
remove_paths = ["drivers/foo"]
remove_configs = ["REMOVE_ME"]
set_defaults = { KEEP_ME = "n" }
"#,
    )
    .unwrap();

    let (ok_first, stdout_first, stderr_first) =
        kslim_in(&kslim_dir, &["reduce-tree", "--tree", tree.to_str().unwrap()]);
    assert!(
        ok_first,
        "first reduce-tree failed: stdout={:?} stderr={:?}",
        stdout_first, stderr_first
    );
    assert!(stdout_first.contains("convergence: converged"));
    assert!(
        !stdout_first.contains("  total: 0"),
        "first reducer run should make concrete edits: {}",
        stdout_first
    );
    assert!(!tree.join("drivers/foo").exists());

    let kconfig_after_first = std::fs::read_to_string(tree.join("Kconfig")).unwrap();
    let makefile_after_first = std::fs::read_to_string(tree.join("Makefile")).unwrap();
    let helper_after_first = std::fs::read_to_string(tree.join("drivers/live/helper.c")).unwrap();

    let (ok_second, stdout_second, stderr_second) =
        kslim_in(&kslim_dir, &["reduce-tree", "--tree", tree.to_str().unwrap()]);
    assert!(
        ok_second,
        "second reduce-tree failed: stdout={:?} stderr={:?}",
        stdout_second, stderr_second
    );
    assert!(stdout_second.contains("convergence: converged"));
    assert!(stdout_second.contains("  total: 0"));
    assert!(stdout_second.contains("  files removed: 0"));
    assert!(stdout_second.contains("  dirs removed: 0"));
    assert!(stdout_second.contains("  configs disabled: 0"));
    assert!(stdout_second.contains("  defaults overridden: 0"));
    assert!(stdout_second.contains("  kconfig refs removed: 0"));
    assert!(stdout_second.contains("  makefile refs removed: 0"));
    assert!(stdout_second.contains("  cpp branches folded: 0"));
    assert!(stdout_second.contains("  include lines removed: 0"));
    assert_eq!(
        std::fs::read_to_string(tree.join("Kconfig")).unwrap(),
        kconfig_after_first
    );
    assert_eq!(
        std::fs::read_to_string(tree.join("Makefile")).unwrap(),
        makefile_after_first
    );
    assert_eq!(
        std::fs::read_to_string(tree.join("drivers/live/helper.c")).unwrap(),
        helper_after_first
    );
}

#[test]
fn test_reduce_tree_uapi_removal_requires_exact_manifest_truth_and_abi_policy() {
    let tmp = tempfile::tempdir().unwrap();
    let tree = tmp.path().join("kernel-tree");
    std::fs::create_dir_all(tree.join("include/uapi/linux")).unwrap();
    std::fs::write(tree.join("Makefile"), "# test\n").unwrap();
    std::fs::write(tree.join("Kconfig"), "# test\n").unwrap();
    std::fs::write(tree.join("include/uapi/linux/abi.h"), "#define ABI 1\n").unwrap();

    let output_dir = tmp.path().join("unused-output");
    let upstream = tmp.path().join("unused-upstream.git");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        upstream.to_str().unwrap(),
    );

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "UAPI without UAPI approval"

[base]
ref = "v1.0"

[slim]
remove_paths = ["include/uapi/linux/abi.h"]

[abi]
allow_public_header_removal = true
allow_uapi_header_removal = false
"#,
    )
    .unwrap();

    let (ok_without_uapi_policy, _, stderr_without_uapi_policy) =
        kslim_in(&kslim_dir, &["reduce-tree", "--tree", tree.to_str().unwrap()]);
    assert!(
        !ok_without_uapi_policy,
        "UAPI removal without UAPI ABI approval must fail"
    );
    assert!(
        stderr_without_uapi_policy.contains("UAPI removal requires explicit ABI policy approval")
    );
    assert!(stderr_without_uapi_policy.contains("abi.allow_uapi_header_removal"));
    assert!(tree.join("include/uapi/linux/abi.h").exists());

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Broad UAPI parent with approval"

[base]
ref = "v1.0"

[slim]
remove_paths = ["include/uapi"]

[abi]
allow_uapi_header_removal = true
"#,
    )
    .unwrap();

    let (ok_broad_parent, stdout_broad_parent, stderr_broad_parent) =
        kslim_in(&kslim_dir, &["reduce-tree", "--tree", tree.to_str().unwrap()]);
    assert!(
        ok_broad_parent,
        "broad UAPI parent reduce-tree should preserve exact UAPI header without failing: stdout={:?} stderr={:?}",
        stdout_broad_parent,
        stderr_broad_parent
    );
    assert!(stdout_broad_parent.contains("convergence: converged"));
    assert!(stdout_broad_parent.contains("  total: 0"));
    assert!(tree.join("include/uapi/linux/abi.h").exists());

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Exact UAPI with approval"

[base]
ref = "v1.0"

[slim]
remove_paths = ["include/uapi/linux/abi.h"]

[abi]
allow_uapi_header_removal = true
"#,
    )
    .unwrap();

    let (ok_exact, stdout_exact, stderr_exact) =
        kslim_in(&kslim_dir, &["reduce-tree", "--tree", tree.to_str().unwrap()]);
    assert!(
        ok_exact,
        "exact UAPI removal with explicit ABI approval should succeed: stdout={:?} stderr={:?}",
        stdout_exact,
        stderr_exact
    );
    assert!(stdout_exact.contains("convergence: converged"));
    assert!(stdout_exact.contains("  files removed: 1"));
    assert!(!tree.join("include/uapi/linux/abi.h").exists());
}

#[test]
fn test_reduce_tree_exported_symbol_provider_removal_requires_no_live_consumer_proof() {
    let tmp = tempfile::tempdir().unwrap();
    let tree = tmp.path().join("kernel-tree");
    std::fs::create_dir_all(tree.join("drivers/provider")).unwrap();
    std::fs::create_dir_all(tree.join("drivers/live")).unwrap();
    std::fs::write(tree.join("Makefile"), "# test\n").unwrap();
    std::fs::write(tree.join("Kconfig"), "# test\n").unwrap();
    std::fs::write(
        tree.join("drivers/provider/provider.c"),
        "void foo_api(void) {}\nEXPORT_SYMBOL(foo_api);\n",
    )
    .unwrap();
    std::fs::write(
        tree.join("drivers/live/user.c"),
        "extern void foo_api(void);\nvoid user(void) { foo_api(); }\n",
    )
    .unwrap();

    let output_dir = tmp.path().join("unused-output");
    let upstream = tmp.path().join("unused-upstream.git");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        upstream.to_str().unwrap(),
    );

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Removed provider with live exported-symbol consumer"

[base]
ref = "v1.0"

[slim]
remove_paths = ["drivers/provider/provider.c"]
"#,
    )
    .unwrap();

    let (ok_live_consumer, _, stderr_live_consumer) =
        kslim_in(&kslim_dir, &["reduce-tree", "--tree", tree.to_str().unwrap()]);
    assert!(
        !ok_live_consumer,
        "provider removal with a live exported-symbol consumer must fail"
    );
    assert!(stderr_live_consumer.contains("exported symbol provider removal requires proof"));
    assert!(stderr_live_consumer.contains("foo_api"));
    assert!(stderr_live_consumer.contains("drivers/live/user.c"));
    assert!(tree.join("drivers/provider/provider.c").exists());
    assert!(tree.join("drivers/live/user.c").exists());

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Removed provider with consumer removed too"

[base]
ref = "v1.0"

[slim]
remove_paths = ["drivers/provider/provider.c", "drivers/live/user.c"]
"#,
    )
    .unwrap();

    let (ok_removed_consumer, stdout_removed_consumer, stderr_removed_consumer) =
        kslim_in(&kslim_dir, &["reduce-tree", "--tree", tree.to_str().unwrap()]);
    assert!(
        ok_removed_consumer,
        "provider removal should succeed once no live consumer remains: stdout={:?} stderr={:?}",
        stdout_removed_consumer,
        stderr_removed_consumer
    );
    assert!(stdout_removed_consumer.contains("convergence: converged"));
    assert!(stdout_removed_consumer.contains("  files removed: 2"));
    assert!(!tree.join("drivers/provider/provider.c").exists());
    assert!(!tree.join("drivers/live/user.c").exists());
}

#[test]
fn test_reduce_tree_device_binding_removal_requires_no_live_reference_proof() {
    let tmp = tempfile::tempdir().unwrap();
    let tree = tmp.path().join("kernel-tree");
    std::fs::create_dir_all(tree.join("Documentation/devicetree/bindings/vendor")).unwrap();
    std::fs::create_dir_all(tree.join("arch/arm/boot/dts")).unwrap();
    std::fs::write(tree.join("Makefile"), "# test\n").unwrap();
    std::fs::write(tree.join("Kconfig"), "# test\n").unwrap();
    std::fs::write(
        tree.join("Documentation/devicetree/bindings/vendor/foo.yaml"),
        "compatible:\n  const: vendor,foo\n",
    )
    .unwrap();
    std::fs::write(
        tree.join("Documentation/devicetree/bindings/vendor/live-schema.yaml"),
        "compatible:\n  const: vendor,live-schema\nallOf:\n  - $ref: /schemas/vendor/foo.yaml#\n",
    )
    .unwrap();
    std::fs::write(
        tree.join("arch/arm/boot/dts/live.dts"),
        "/ { compatible = \"vendor,foo\"; };\n",
    )
    .unwrap();
    std::fs::write(
        tree.join("arch/arm/boot/dts/live.dtsi"),
        "/ { compatible = \"vendor,foo\"; };\n",
    )
    .unwrap();

    let output_dir = tmp.path().join("unused-output");
    let upstream = tmp.path().join("unused-upstream.git");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        upstream.to_str().unwrap(),
    );

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Removed device binding with live references"

[base]
ref = "v1.0"

[slim]
remove_paths = ["Documentation/devicetree/bindings/vendor/foo.yaml"]
"#,
    )
    .unwrap();

    let (ok_live_reference, _, stderr_live_reference) =
        kslim_in(&kslim_dir, &["reduce-tree", "--tree", tree.to_str().unwrap()]);
    assert!(
        !ok_live_reference,
        "device binding removal with live DTS/DTSI/schema references must fail"
    );
    assert!(stderr_live_reference.contains("device binding removal requires proof"));
    assert!(stderr_live_reference.contains("arch/arm/boot/dts/live.dts"));
    assert!(stderr_live_reference.contains("arch/arm/boot/dts/live.dtsi"));
    assert!(stderr_live_reference
        .contains("Documentation/devicetree/bindings/vendor/live-schema.yaml"));
    assert!(stderr_live_reference.contains("schema_ref:/schemas/vendor/foo.yaml"));
    assert!(tree
        .join("Documentation/devicetree/bindings/vendor/foo.yaml")
        .exists());
    assert!(tree.join("arch/arm/boot/dts/live.dts").exists());
    assert!(tree.join("arch/arm/boot/dts/live.dtsi").exists());

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Removed device binding with references removed too"

[base]
ref = "v1.0"

[slim]
remove_paths = [
  "Documentation/devicetree/bindings/vendor/foo.yaml",
  "Documentation/devicetree/bindings/vendor/live-schema.yaml",
  "arch/arm/boot/dts/live.dts",
  "arch/arm/boot/dts/live.dtsi",
]
"#,
    )
    .unwrap();

    let (ok_removed_references, stdout_removed_references, stderr_removed_references) =
        kslim_in(&kslim_dir, &["reduce-tree", "--tree", tree.to_str().unwrap()]);
    assert!(
        ok_removed_references,
        "device binding removal should succeed once no live reference remains: stdout={:?} stderr={:?}",
        stdout_removed_references,
        stderr_removed_references
    );
    assert!(stdout_removed_references.contains("convergence: converged"));
    assert!(stdout_removed_references.contains("  files removed: 4"));
    assert!(!tree
        .join("Documentation/devicetree/bindings/vendor/foo.yaml")
        .exists());
    assert!(!tree
        .join("Documentation/devicetree/bindings/vendor/live-schema.yaml")
        .exists());
    assert!(!tree.join("arch/arm/boot/dts/live.dts").exists());
    assert!(!tree.join("arch/arm/boot/dts/live.dtsi").exists());
}

#[test]
fn test_reduce_tree_runtime_registration_removal_requires_no_live_entry_point_proof() {
    let tmp = tempfile::tempdir().unwrap();
    let tree = tmp.path().join("kernel-tree");
    std::fs::create_dir_all(tree.join("drivers/provider")).unwrap();
    std::fs::create_dir_all(tree.join("drivers/live")).unwrap();
    std::fs::write(tree.join("Makefile"), "# test\n").unwrap();
    std::fs::write(tree.join("Kconfig"), "# test\n").unwrap();
    std::fs::write(
        tree.join("drivers/provider/provider.c"),
        "static int foo_init(void) { return 0; }\nmodule_init(foo_init);\n",
    )
    .unwrap();
    std::fs::write(
        tree.join("drivers/live/user.c"),
        "extern int foo_init(void);\nint call(void) { return foo_init(); }\n",
    )
    .unwrap();

    let output_dir = tmp.path().join("unused-output");
    let upstream = tmp.path().join("unused-upstream.git");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        upstream.to_str().unwrap(),
    );

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Removed runtime registration with live entry point"

[base]
ref = "v1.0"

[slim]
remove_paths = ["drivers/provider/provider.c"]
"#,
    )
    .unwrap();

    let (ok_live_entry_point, _, stderr_live_entry_point) =
        kslim_in(&kslim_dir, &["reduce-tree", "--tree", tree.to_str().unwrap()]);
    assert!(
        !ok_live_entry_point,
        "runtime registration removal with a live entry point must fail"
    );
    assert!(stderr_live_entry_point.contains("runtime registration removal requires proof"));
    assert!(stderr_live_entry_point.contains("drivers/live/user.c"));
    assert!(stderr_live_entry_point.contains("foo_init"));
    assert!(tree.join("drivers/provider/provider.c").exists());
    assert!(tree.join("drivers/live/user.c").exists());

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Removed runtime registration with entry point removed too"

[base]
ref = "v1.0"

[slim]
remove_paths = ["drivers/provider/provider.c", "drivers/live/user.c"]
"#,
    )
    .unwrap();

    let (ok_removed_entry_point, stdout_removed_entry_point, stderr_removed_entry_point) =
        kslim_in(&kslim_dir, &["reduce-tree", "--tree", tree.to_str().unwrap()]);
    assert!(
        ok_removed_entry_point,
        "runtime registration removal should succeed once no live entry point remains: stdout={:?} stderr={:?}",
        stdout_removed_entry_point,
        stderr_removed_entry_point
    );
    assert!(stdout_removed_entry_point.contains("convergence: converged"));
    assert!(stdout_removed_entry_point.contains("  files removed: 2"));
    assert!(!tree.join("drivers/provider/provider.c").exists());
    assert!(!tree.join("drivers/live/user.c").exists());
}

#[test]
fn test_explain_edit_reports_published_edit_record_for_path_line() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream_with_amdgpu(tmp.path(), "explain-edit", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Explain edit"

[base]
ref = "v1.0"

[slim]
remove_paths = ["drivers/gpu/drm/amd/amdgpu"]
"#,
    )
    .unwrap();

    let (ok_generate, stdout_generate, stderr_generate) = kslim_in(&kslim_dir, &["generate"]);
    assert!(
        ok_generate,
        "generate failed: stdout={:?} stderr={:?}",
        stdout_generate, stderr_generate
    );

    let (ok_explain, stdout_explain, stderr_explain) = kslim_in(
        &kslim_dir,
        &["explain-edit", "drivers/gpu/drm/amd/amdgpu/amdgpu_drv.c:1"],
    );
    assert!(
        ok_explain,
        "explain-edit failed: stdout={:?} stderr={:?}",
        stdout_explain, stderr_explain
    );
    assert!(stdout_explain.contains("explain-edit: drivers/gpu/drm/amd/amdgpu/amdgpu_drv.c:1"));
    assert!(stdout_explain.contains("report scope: published"));
    assert!(stdout_explain.contains("authoritative: true"));
    assert!(stdout_explain.contains("decision: removed"));
    assert!(stdout_explain.contains("owner: manifest_path path=drivers/gpu/drm/amd/amdgpu"));
    assert!(stdout_explain.contains("pass: prune.remove_path"));
    assert!(stdout_explain.contains("edit kind: remove_path"));
    assert!(stdout_explain.contains("line range: structural"));
    assert!(stdout_explain
        .contains("proof source: removal_manifest_entry path=drivers/gpu/drm/amd/amdgpu"));
    assert!(stdout_explain.contains("old: int amdgpu_drv;\\n"));
    assert!(stdout_explain.contains("new: <empty>"));
    assert!(stdout_explain.contains("related reports:"));
    assert!(stdout_explain.contains("edit-summary.json"));
    assert!(stdout_explain.contains("reducer-report.json"));
    assert!(stdout_explain.contains("diagnostics.json"));
    assert!(stdout_explain.contains("kconfig-solver-report.json"));
    assert!(stdout_explain.contains("kconfig-rewrite-report.json"));

    let (ok_explain_flag, stdout_explain_flag, stderr_explain_flag) = kslim_in(
        &kslim_dir,
        &["--explain", "drivers/gpu/drm/amd/amdgpu/amdgpu_drv.c"],
    );
    assert!(
        ok_explain_flag,
        "--explain path failed: stdout={:?} stderr={:?}",
        stdout_explain_flag, stderr_explain_flag
    );
    assert!(stdout_explain_flag.contains("explain-path: drivers/gpu/drm/amd/amdgpu/amdgpu_drv.c"));
    assert!(stdout_explain_flag.contains("matches: 1"));

    let (ok_explain_line, stdout_explain_line, stderr_explain_line) = kslim_in(
        &kslim_dir,
        &["--explain", "drivers/gpu/drm/amd/amdgpu/amdgpu_drv.c:1"],
    );
    assert!(
        ok_explain_line,
        "--explain PATH:LINE failed: stdout={:?} stderr={:?}",
        stdout_explain_line, stderr_explain_line
    );
    assert!(stdout_explain_line.contains("explain-edit: drivers/gpu/drm/amd/amdgpu/amdgpu_drv.c:1"));
}

#[test]
fn test_generate_slims_named_feature_remove_intent() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream_with_amdgpu(tmp.path(), "gpu-feature", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Slim AMDGPU by named feature"

[base]
ref = "v1.0"

[features.remove.amdgpu]
kind = "subsystem"
remove_paths = ["drivers/gpu/drm/amd/amdgpu"]
remove_configs = ["DRM_AMDGPU", "DRM_AMDGPU_SI"]

[selftests]
commands = ["test ! -e drivers/gpu/drm/amd/amdgpu"]
"#,
    )
    .unwrap();

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["generate"]);
    assert!(
        ok,
        "generate failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(!output_dir.join("drivers/gpu/drm/amd/amdgpu").exists());

    let makefile = std::fs::read_to_string(output_dir.join("drivers/gpu/drm/Makefile")).unwrap();
    assert!(!makefile.contains("amd/amdgpu/"));
    let report = std::fs::read_to_string(output_meta_path(&output_dir, "report.txt")).unwrap();
    assert!(report.contains("Mode: slimmed"));

    let (ok_explain, stdout_explain, stderr_explain) =
        kslim_in(&kslim_dir, &["explain-symbol", "CONFIG_DRM_AMDGPU"]);
    assert!(
        ok_explain,
        "explain-symbol failed: stdout={:?} stderr={:?}",
        stdout_explain, stderr_explain
    );
    assert!(stdout_explain.contains("explain-symbol: CONFIG_DRM_AMDGPU"));
    assert!(stdout_explain.contains("normalized symbol: DRM_AMDGPU"));
    assert!(stdout_explain.contains("report scope: published"));
    assert!(stdout_explain.contains("authoritative: true"));
    assert!(stdout_explain.contains("decision: removed"));
    assert!(stdout_explain.contains("owner: removal manifest symbol=DRM_AMDGPU"));
    assert!(stdout_explain.contains("proof source: removal_manifest_entry symbol=DRM_AMDGPU"));
    assert!(stdout_explain.contains("matching edits:"));
    assert!(stdout_explain.contains("related reports:"));

    let (ok_explain_flag, stdout_explain_flag, stderr_explain_flag) =
        kslim_in(&kslim_dir, &["--explain", "CONFIG_DRM_AMDGPU"]);
    assert!(
        ok_explain_flag,
        "--explain symbol failed: stdout={:?} stderr={:?}",
        stdout_explain_flag, stderr_explain_flag
    );
    assert!(stdout_explain_flag.contains("explain-symbol: CONFIG_DRM_AMDGPU"));
    assert!(stdout_explain_flag.contains("decision: removed"));
}

#[test]
fn test_generate_preserves_named_feature_under_broad_remove_intent() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream_with_gpu_siblings(tmp.path(), "gpu-preserve", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Remove GPU stack while preserving AMDGPU"

[base]
ref = "v1.0"

[features.remove.gpu_stack]
kind = "subsystem"
roots = ["drivers/gpu/drm"]
configs = ["DRM_NOUVEAU"]

[features.preserve.amdgpu]
kind = "driver"
roots = ["drivers/gpu/drm/amd/amdgpu"]
configs = ["DRM_AMDGPU"]

[selftests]
check_kconfig_sources = false
check_makefiles = false
commands = [
  "test -e drivers/gpu/drm/amd/amdgpu/amdgpu_drv.c",
  "test ! -e drivers/gpu/drm/nouveau",
]
"#,
    )
    .unwrap();

    // Strict mode now blocks broad removed/preserved path overlap before mutation.
    // This legacy fixture keeps exercising the non-strict preservation rewrite.
    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["generate", "--no-strict"]);
    assert!(
        ok,
        "generate failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(output_dir
        .join("drivers/gpu/drm/amd/amdgpu/amdgpu_drv.c")
        .exists());
    assert!(!output_dir.join("drivers/gpu/drm/nouveau").exists());

    let reducer_report =
        std::fs::read_to_string(output_meta_path(&output_dir, "reducer-report.json")).unwrap();
    assert!(reducer_report.contains("\"preserved_paths\":[\"drivers/gpu/drm/amd/amdgpu\"]"));
    assert!(reducer_report.contains("\"preserved_config_symbols\":[\"DRM_AMDGPU\"]"));
}

#[test]
fn test_generate_rewrites_kconfig_defaults_from_profile() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream_with_amdgpu(tmp.path(), "defaults", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Rewrite Kconfig defaults"

[base]
ref = "v1.0"

[slim]
set_defaults = { DRM_AMDGPU = "n", DRM_AMDGPU_SI = "n" }
"#,
    )
    .unwrap();

    let (ok, _, stderr) = kslim_in(&kslim_dir, &["generate"]);
    assert!(ok, "generate should succeed: {}", stderr);

    let kconfig =
        std::fs::read_to_string(output_dir.join("drivers/gpu/drm/amd/amdgpu/Kconfig")).unwrap();
    assert!(kconfig.contains("config DRM_AMDGPU"));
    assert!(kconfig.contains("config DRM_AMDGPU_SI"));
    assert_eq!(kconfig.matches("\tdefault n\n").count(), 2);
}

#[test]
fn test_generate_slims_rxrpc_and_afs_fixture() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream_with_rxrpc_afs(tmp.path(), "rxrpc-afs", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Slim RXRPC and AFS"

[base]
ref = "v1.0"

[slim]
remove_paths = ["net/rxrpc", "fs/afs"]
remove_configs = ["AF_RXRPC", "AFS_FS"]

[selftests]
commands = ["test ! -e net/rxrpc", "test ! -e fs/afs"]
"#,
    )
    .unwrap();

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["generate"]);
    assert!(
        ok,
        "generate failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(
        stdout.contains("selftests: 2 built-in, 2 custom"),
        "expected selftest summary in stdout: {}",
        stdout
    );
    assert!(!output_dir.join("net/rxrpc").exists());
    assert!(!output_dir.join("fs/afs").exists());

    let net_kconfig = std::fs::read_to_string(output_dir.join("net/Kconfig")).unwrap();
    assert!(net_kconfig.contains("# kslim: removed source \"net/rxrpc/Kconfig\""));

    let net_makefile = std::fs::read_to_string(output_dir.join("net/Makefile")).unwrap();
    assert!(!net_makefile.contains("rxrpc/"));

    let fs_kconfig = std::fs::read_to_string(output_dir.join("fs/Kconfig")).unwrap();
    assert!(fs_kconfig.contains("# kslim: removed source \"fs/afs/Kconfig\""));

    let fs_makefile = std::fs::read_to_string(output_dir.join("fs/Makefile")).unwrap();
    assert!(!fs_makefile.contains("afs/"));

    let report = std::fs::read_to_string(output_meta_path(&output_dir, "report.txt")).unwrap();
    assert!(report.contains("Mode: slimmed"));
    assert!(report.contains("Selftests:"));
    assert!(output_meta_path(&output_dir, "reducer-report.md").exists());
    assert!(output_meta_path(&output_dir, "edit-summary.json").exists());
    assert!(output_meta_path(&output_dir, "kconfig-solver-report.json").exists());
    assert!(output_meta_path(&output_dir, "kconfig-rewrite-report.json").exists());
}

#[test]
fn test_generate_slims_nfs_stack_fixture() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream_with_nfs_stack(tmp.path(), "nfs-stack", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Slim NFS stack"

[base]
ref = "v1.0"

[slim]
remove_paths = ["fs/nfs", "fs/nfsd", "fs/lockd", "fs/nfs_common", "net/sunrpc"]
remove_configs = ["NFS_FS", "NFSD", "LOCKD", "NFS_COMMON", "SUNRPC"]

[selftests]
commands = [
  "test ! -e fs/nfs",
  "test ! -e fs/nfsd",
  "test ! -e fs/lockd",
  "test ! -e fs/nfs_common",
  "test ! -e net/sunrpc",
  "test -e include/linux/nfs_fs.h",
  "test -e include/uapi/linux/nfs_mount.h"
]
"#,
    )
    .unwrap();

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["generate"]);
    assert!(
        ok,
        "generate failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(!output_dir.join("fs/nfs").exists());
    assert!(!output_dir.join("fs/nfsd").exists());
    assert!(!output_dir.join("fs/lockd").exists());
    assert!(!output_dir.join("fs/nfs_common").exists());
    assert!(!output_dir.join("net/sunrpc").exists());
    assert!(output_dir.join("include/linux/nfs_fs.h").exists());
    assert!(output_dir.join("include/uapi/linux/nfs_mount.h").exists());

    let fs_kconfig = std::fs::read_to_string(output_dir.join("fs/Kconfig")).unwrap();
    assert!(fs_kconfig.contains("# kslim: removed source \"fs/nfs/Kconfig\""));
    assert!(fs_kconfig.contains("# kslim: removed source \"fs/nfsd/Kconfig\""));
    assert!(fs_kconfig.contains("# kslim: removed source \"fs/lockd/Kconfig\""));
    assert!(fs_kconfig.contains("# kslim: removed source \"fs/nfs_common/Kconfig\""));

    let net_kconfig = std::fs::read_to_string(output_dir.join("net/Kconfig")).unwrap();
    assert!(net_kconfig.contains("# kslim: removed source \"net/sunrpc/Kconfig\""));

    let fs_makefile = std::fs::read_to_string(output_dir.join("fs/Makefile")).unwrap();
    assert!(!fs_makefile.contains("nfs/"));
    assert!(!fs_makefile.contains("nfsd/"));
    assert!(!fs_makefile.contains("lockd/"));
    assert!(!fs_makefile.contains("nfs_common/"));

    let net_makefile = std::fs::read_to_string(output_dir.join("net/Makefile")).unwrap();
    assert!(!net_makefile.contains("sunrpc/"));
}

#[test]
fn test_generate_preprocessor_fixture_folds_removed_branch_and_keeps_unsupported_expression() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream_with_preprocessor_fixture(tmp.path(), "cpp", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Preprocessor folding fixture"

[base]
ref = "v1.0"

[reducer]
report_unsupported_expressions = false

[slim]
remove_configs = ["TEST_REMOVED"]
"#,
    )
    .unwrap();

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["generate"]);
    assert!(
        ok,
        "generate failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );

    let feature = std::fs::read_to_string(output_dir.join("drivers/test/feature.c")).unwrap();
    assert!(!feature.contains("removed_branch"));
    assert!(feature.contains("int live_branch;"));
    assert!(feature.contains("#if defined(CONFIG_TEST_REMOVED) || defined(CONFIG_OTHER)"));
    assert!(feature.contains("int unsupported_expression;"));
}

#[test]
fn test_generate_preprocessor_fixture_fails_closed_on_unsupported_expression_by_default() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream_with_preprocessor_fixture(tmp.path(), "cpp-fail", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Preprocessor unsupported expression fixture"

[base]
ref = "v1.0"

[slim]
remove_configs = ["TEST_REMOVED"]
"#,
    )
    .unwrap();

    let (ok, _stdout, stderr) = kslim_in(&kslim_dir, &["generate"]);
    assert!(!ok, "generate unexpectedly succeeded");
    assert!(stderr.contains("unsupported preprocessor expressions"));
    assert!(stderr.contains("drivers/test/feature.c:"));
    assert!(stderr.contains("defined(CONFIG_TEST_REMOVED) || defined(CONFIG_OTHER)"));
    assert!(
        !output_dir.exists(),
        "unsupported reducer syntax must not publish a fresh output repository"
    );
    assert!(
        !kslim_dir.join("kslim.lock").exists(),
        "unsupported reducer syntax must not write an authoritative lockfile"
    );

    let report = std::fs::read_to_string(project_failure_report_path(&kslim_dir)).unwrap();
    assert!(report.contains("Status: failure"));
    assert!(report.contains("Stage: reduce"));
    assert!(report.contains("Reducer artifacts:"));
    assert!(report.contains("Diagnostics JSON: diagnostics.json"));

    let reducer_report =
        std::fs::read_to_string(project_failure_meta_path(&kslim_dir, "reducer-report.json"))
            .unwrap();
    assert!(reducer_report.contains("\"present\": true"));
    assert!(reducer_report.contains("\"unsupported_cpp_expressions\": 1"));

    let diagnostics =
        std::fs::read_to_string(project_failure_meta_path(&kslim_dir, "diagnostics.json")).unwrap();
    assert!(diagnostics.contains("\"unsupported_cpp_expression\""));
    assert!(diagnostics.contains("\"drivers/test/feature.c\""));

    let reducer_failure = std::fs::read_to_string(project_failure_meta_path(
        &kslim_dir,
        "reducer-failure.json",
    ))
    .unwrap();
    assert!(reducer_failure.contains("\"kind\": \"reducer_non_convergence\""));
    assert!(reducer_failure.contains("\"stage\": \"reduce\""));
    assert!(reducer_failure.contains("\"termination\": \"unsupported_syntax_in_strict_mode\""));
    assert!(reducer_failure.contains("\"fixup_passes\": null"));
    assert!(reducer_failure.contains("defined(CONFIG_TEST_REMOVED) || defined(CONFIG_OTHER)"));
}

#[test]
fn test_generate_include_fixture_removes_only_private_removed_header_include() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream_with_include_fixture(tmp.path(), "includes", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Include cleanup fixture"

[base]
ref = "v1.0"

[slim]
remove_paths = ["drivers/gpu/drm/amd/amdgpu"]

[selftests]
check_kconfig_sources = false
check_makefiles = false

[[selftests.kernel_builds]]
name = "include-fixup"
config_target = "defconfig"
make_program = "./fake-make.sh"
"#,
    )
    .unwrap();

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["generate"]);
    assert!(
        ok,
        "generate should succeed after include cleanup: stdout={:?} stderr={:?}",
        stdout, stderr
    );

    let helper = std::fs::read_to_string(output_dir.join("drivers/gpu/drm/helper.c")).unwrap();
    assert!(!helper.contains("#include <amd/amdgpu/internal.h>"));
    assert!(helper.contains("#include <linux/drm_public.h>"));
    assert!(output_dir.join("include/linux/drm_public.h").exists());

    let summary =
        std::fs::read_to_string(output_meta_path(&output_dir, "edit-summary.json")).unwrap();
    assert!(summary.contains("\"includes.rewrite_removed_headers\": 1"));
    assert!(summary.contains("\"removed_include_lines\": 1"));
    assert!(summary.contains("\"public_headers_preserved\": 1"));
}

#[test]
fn test_generate_hard_stops_on_kernel_build_failure_and_reports_diagnostic_context() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream_with_amdgpu_hard_stop(tmp.path(), "hard-stop", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Hard-stop on unsupported compiler failure"

[base]
ref = "v1.0"

[slim]
remove_paths = ["drivers/gpu/drm/amd/amdgpu"]

[selftests]
check_kconfig_sources = false
check_makefiles = false

[[selftests.kernel_builds]]
name = "hard-stop"
config_target = "defconfig"
make_program = "./fake-make.sh"
"#,
    )
    .unwrap();

    let (ok, _, stderr) = kslim_in(&kslim_dir, &["generate"]);
    assert!(!ok, "generate should fail on hard-stop fixture");
    assert!(
        stderr.contains("kernel build selftest 'hard-stop' failed"),
        "expected kernel build failure, got: {}",
        stderr
    );
    assert!(
        stderr.contains("fatal error: generated/missing.h"),
        "expected compiler diagnostic context, got: {}",
        stderr
    );

    let report_path = project_failure_report_path(&kslim_dir);
    assert!(
        report_path.exists(),
        "failure report should be written at {}",
        report_path.display()
    );
    let report = std::fs::read_to_string(report_path).unwrap();
    assert!(report.contains("Status: failure"));
    assert!(report.contains("Stage: selftest"));
    assert!(report.contains("kernel build selftest 'hard-stop' failed"));
    assert!(report.contains("fatal error: generated/missing.h"));
    assert!(report.contains("Reducer artifacts:"));

    let diagnostics =
        std::fs::read_to_string(project_failure_meta_path(&kslim_dir, "diagnostics.json")).unwrap();
    assert!(diagnostics.contains("\"skipped_fixup_diagnostic\""));
    assert!(diagnostics.contains("generated/missing.h"));
    assert!(diagnostics.contains("applicable fixup found no remaining proven site"));
}

#[test]
fn test_generate_build_loop_fixture_applies_missing_header_fixup_and_second_passes() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream =
        create_fake_upstream_with_known_missing_header_diag(tmp.path(), "build-loop", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Known missing-header diagnostic fixture"

[base]
ref = "v1.0"

[slim]
remove_paths = ["drivers/gpu/drm/amd/amdgpu"]

[selftests]
check_kconfig_sources = false
check_makefiles = false

[[selftests.kernel_builds]]
name = "known-missing-header"
config_target = "defconfig"
make_program = "./fake-make.sh"
"#,
    )
    .unwrap();

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["generate"]);
    assert!(
        ok,
        "generate should succeed after deterministic fixup: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(
        stdout.contains("selftests: 1 built-in, 0 custom"),
        "expected selftest success summary, got: {}",
        stdout
    );

    let helper = std::fs::read_to_string(output_dir.join("drivers/gpu/drm/helper.c")).unwrap();
    assert_eq!(helper, "int drm_helper;\n");

    let reducer_report =
        std::fs::read_to_string(output_meta_path(&output_dir, "reducer-report.md")).unwrap();
    assert!(reducer_report.contains("fixups.remove_missing_header_include: 1"));

    let edit_summary =
        std::fs::read_to_string(output_meta_path(&output_dir, "edit-summary.json")).unwrap();
    assert!(edit_summary.contains("\"fixups.remove_missing_header_include\": 1"));
}

#[test]
fn test_generate_build_loop_fixture_applies_multiple_missing_header_fixups_until_success() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream_with_missing_header_diag_sequence(
        tmp.path(),
        "build-loop-multi",
        "1.0",
        &[
            "amd/amdgpu/amdgpu_missing_1.h",
            "amd/amdgpu/amdgpu_missing_2.h",
        ],
    );
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Known multi-pass missing-header diagnostic fixture"

[base]
ref = "v1.0"

[slim]
remove_paths = ["drivers/gpu/drm/amd/amdgpu"]

[selftests]
check_kconfig_sources = false
check_makefiles = false

[[selftests.kernel_builds]]
name = "known-missing-header-multi"
config_target = "defconfig"
make_program = "./fake-make.sh"
"#,
    )
    .unwrap();

    let (ok, _, stderr) = kslim_in(&kslim_dir, &["generate"]);
    assert!(
        ok,
        "generate should succeed after two fixup passes: {stderr}"
    );

    let helper = std::fs::read_to_string(output_dir.join("drivers/gpu/drm/helper.c")).unwrap();
    assert_eq!(helper, "int drm_helper;\n");

    let reducer_report =
        std::fs::read_to_string(output_meta_path(&output_dir, "reducer-report.md")).unwrap();
    assert!(reducer_report.contains("fixups.remove_missing_header_include: 2"));
}

#[test]
fn test_generate_build_loop_fixture_respects_fixup_pass_limit() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream_with_missing_header_diag_sequence(
        tmp.path(),
        "build-loop-limit",
        "1.0",
        &[
            "amd/amdgpu/amdgpu_missing_1.h",
            "amd/amdgpu/amdgpu_missing_2.h",
        ],
    );
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let (ok_initial, _, stderr_initial) = kslim_in(&kslim_dir, &["generate"]);
    assert!(
        ok_initial,
        "initial generate should publish the last known good output: {}",
        stderr_initial
    );
    let output_head_before = git_in(output_dir.to_str().unwrap(), &["rev-parse", "HEAD"]);
    let output_status_before = git_in(output_dir.to_str().unwrap(), &["status", "--short"]);
    let lockfile_before = std::fs::read_to_string(kslim_dir.join("kslim.lock")).unwrap();
    assert!(
        output_status_before.trim().is_empty(),
        "initial output repo should be clean before non-converging generate"
    );

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Known missing-header diagnostic fixture with bounded retries"

[base]
ref = "v1.0"

[slim]
remove_paths = ["drivers/gpu/drm/amd/amdgpu"]

[selftests]
check_kconfig_sources = false
check_makefiles = false

[[selftests.kernel_builds]]
name = "known-missing-header-limit"
config_target = "defconfig"
make_program = "./fake-make.sh"
"#,
    )
    .unwrap();

    let (ok, _, stderr) = kslim_in(&kslim_dir, &["generate", "--max-fixup-passes", "1"]);
    assert!(
        !ok,
        "generate should fail after hitting the fixup pass limit"
    );
    assert!(
        stderr.contains("amdgpu_missing_2.h"),
        "expected final diagnostic from the second missing header, got: {}",
        stderr
    );

    let report_path = project_failure_report_path(&kslim_dir);
    assert!(
        report_path.exists(),
        "failure report should be written at {}",
        report_path.display()
    );
    assert_eq!(
        git_in(output_dir.to_str().unwrap(), &["rev-parse", "HEAD"]),
        output_head_before,
        "non-converging reducer must not create or move an output commit"
    );
    assert_eq!(
        git_in(output_dir.to_str().unwrap(), &["status", "--short"]),
        output_status_before,
        "non-converging reducer must not dirty the published output repo"
    );
    assert_eq!(
        std::fs::read_to_string(kslim_dir.join("kslim.lock")).unwrap(),
        lockfile_before,
        "non-converging reducer must not update the authoritative lockfile"
    );
    assert!(
        !committed_output_meta_path(&output_dir, "reducer-failure.json").exists(),
        "reducer failure report must stay in non-authoritative attempt metadata"
    );
    assert!(
        !output_meta_path(&output_dir, "reducer-failure.json").exists(),
        "reducer failure report must not be published into output repo metadata"
    );

    let reducer_failure = std::fs::read_to_string(project_failure_meta_path(
        &kslim_dir,
        "reducer-failure.json",
    ))
    .unwrap();
    assert!(reducer_failure.contains("\"kind\": \"reducer_non_convergence\""));
    assert!(reducer_failure.contains("\"stage\": \"selftest\""));
    assert!(reducer_failure.contains("\"termination\": \"max_pass_count_reached\""));
    assert!(reducer_failure.contains("\"fixup_passes\": 1"));
    assert!(reducer_failure.contains("amdgpu_missing_2.h"));
}
