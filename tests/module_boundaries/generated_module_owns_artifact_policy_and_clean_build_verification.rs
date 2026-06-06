use super::common::*;

#[test]
fn generated_module_owns_artifact_policy_and_clean_build_verification() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let main = production_source(&root.join("src/main.rs"));
    let generated_mod = production_source(&root.join("src/generated/mod.rs"));
    let artifact = production_source(&root.join("src/generated/artifact.rs"));
    let policy = production_source(&root.join("src/generated/policy.rs"));
    let clean_build = production_source(&root.join("src/generated/clean_build.rs"));
    let feature_generated = production_source(&root.join("src/feature/generated_artifact_resolution.rs"));
    let manifest_parse = production_source(&root.join("src/removal_manifest/parse.rs"));
    let manifest_match_rules = production_source(&root.join("src/removal_manifest/match_rules.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    assert!(
        main.contains("mod generated;"),
        "main.rs should register the generated-artifact ownership module"
    );

    for required in [
        "//! Generated artifact discovery, policy, and clean-build verification models.",
        "mod artifact;",
        "mod clean_build;",
        "mod policy;",
        "pub(crate) use artifact::{",
        "pub(crate) use clean_build::{CleanBuildVerification, CleanBuildVerificationStatus}",
        "pub(crate) use policy::{is_generated_include_header_path, normalize_generated_include_roots}",
    ] {
        assert!(
            generated_mod.contains(required),
            "src/generated/mod.rs should declare/export generated module item {required}"
        );
    }

    for required in [
        "pub(crate) struct GeneratedArtifactDiscovery",
        "pub(crate) fn discover_generated_artifacts",
        "pub(crate) fn is_generated_artifact_path(path: &Path) -> bool",
        "GeneratedArtifactPath::matches_path(path)",
        "pub(crate) fn is_generated_artifact_like_path(path: &Path) -> bool",
        "pub(crate) fn raw_generated_artifact_path_parts_match(path: &Path) -> bool",
        "Module.symvers",
        "modules.order",
        "vmlinux.o",
    ] {
        assert!(
            artifact.contains(required),
            "src/generated/artifact.rs should own generated artifact discovery/classification item {required}"
        );
    }

    for required in [
        "pub(crate) fn normalize_generated_include_roots",
        "generated include roots must not be empty",
        "generated include roots must be relative to the tree",
        "generated include roots must not contain '..'",
        "generated include roots must not resolve to the tree root",
        "pub(crate) fn is_generated_include_header_path(path: &Path) -> bool",
        "path.starts_with(\"include/generated\")",
    ] {
        assert!(
            policy.contains(required),
            "src/generated/policy.rs should own generated include policy item {required}"
        );
    }

    for required in [
        "pub(crate) enum CleanBuildVerificationStatus",
        "NotRequested",
        "Required",
        "Verified",
        "pub(crate) struct CleanBuildVerification",
        "verified_artifacts: Vec<GeneratedArtifactPath>",
        "pub(crate) fn verified(",
    ] {
        assert!(
            clean_build.contains(required),
            "src/generated/clean_build.rs should own clean-build verification item {required}"
        );
    }

    assert!(
        feature_generated.contains("crate::generated::is_generated_artifact_like_path(path.as_path())"),
        "feature generated-artifact resolution should delegate generated path classification to src/generated"
    );
    assert!(
        manifest_parse.contains("use crate::generated::normalize_generated_include_roots;")
            && manifest_match_rules.contains("use crate::generated::is_generated_include_header_path;"),
        "removal manifest should consume generated include policy from src/generated"
    );

    assert!(
        architecture.contains("`generated/*`")
            && architecture.contains("Generated artifact discovery")
            && architecture.contains("generated include-root policy")
            && architecture.contains("clean-build verification state"),
        "docs/architecture.md should document generated module ownership"
    );
}
