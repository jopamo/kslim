use super::common::*;

#[test]
fn documentation_appendices_live_in_reference_docs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kernel_build = production_source(&root.join("docs/kernel-build-iteration.md"));
    let reference_index = production_source(&root.join("docs/reference/README.md"));
    let profile_reference =
        production_source(&root.join("docs/reference/profile-field-reference.md"));

    assert!(
        kernel_build.contains("[Profile field reference](reference/profile-field-reference.md)"),
        "docs/kernel-build-iteration.md should link to the extracted profile reference appendix"
    );
    assert!(
        reference_index.contains("profile-field-reference.md"),
        "docs/reference/README.md should index the extracted profile reference appendix"
    );
    for required in [
        "## Kernel build selftest fields",
        "## Removal and feature intent fields",
        "### Reducer policy knobs",
        "FeatureKconfigResolution",
        "Kconfig solver reports are emitted as `kconfig-solver-report.json`",
        "Supported stable kind tokens",
    ] {
        assert!(
            profile_reference.contains(required),
            "docs/reference/profile-field-reference.md should retain reference appendix content through {required}"
        );
    }
    for moved_reference_detail in [
        "Each `[[selftests.kernel_builds]]` entry supports:",
        "FeatureKconfigResolution",
        "Kconfig solver reports are emitted as `kconfig-solver-report.json`",
    ] {
        assert!(
            !kernel_build.contains(moved_reference_detail),
            "docs/kernel-build-iteration.md should point at reference appendices instead of retaining long detail {moved_reference_detail}"
        );
    }
}
