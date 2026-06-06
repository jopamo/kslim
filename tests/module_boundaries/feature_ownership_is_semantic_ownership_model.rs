use super::common::*;

#[test]
fn feature_ownership_is_semantic_ownership_model() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    let ownership_section = feature
        .split("pub(crate) enum FeatureOwnershipKind")
        .nth(1)
        .and_then(|rest| rest.split("fn normalize_feature_kind_token").next())
        .expect("feature module should define FeatureOwnership before feature helpers");

    for required in [
        "pub(crate) const ALL: [Self; 15]",
        "ExplicitlyRemoved",
        "ExplicitlyPreserved",
        "OwnedSolelyByRemovedFeature",
        "SharedWithLiveFeature",
        "GeneratedByLiveBuild",
        "PublicAbiSurface",
        "PublicUapiSurface",
        "ArchLocal",
        "ArchShared",
        "RuntimeOnlySurface",
        "TestOnlySurface",
        "DocumentationOnlySurface",
        "UnknownOwnership",
        "AmbiguousOwnership",
        "UnsupportedOwnership",
        "pub(crate) fn from_stable_name(value: &str) -> Result<Self>",
        "pub(crate) const fn stable_name(self) -> &'static str",
        "\"explicitly_removed\"",
        "\"explicitly_preserved\"",
        "\"owned_solely_by_removed_feature\"",
        "\"shared_with_live_feature\"",
        "\"generated_by_live_build\"",
        "\"public_abi_surface\"",
        "\"public_uapi_surface\"",
        "\"arch_local\"",
        "\"arch_shared\"",
        "\"runtime_only_surface\"",
        "\"test_only_surface\"",
        "\"documentation_only_surface\"",
        "\"unknown_ownership\"",
        "\"ambiguous_ownership\"",
        "\"unsupported_ownership\"",
        "pub(crate) struct FeatureOwnershipSubject(String);",
        "pub(crate) fn new(subject: impl Into<String>) -> Result<Self>",
        "pub(crate) fn as_str(&self) -> &str",
        "pub(crate) struct FeatureOwnership",
        "feature: FeatureId",
        "subject: FeatureOwnershipSubject",
        "kind: FeatureOwnershipKind",
        "pub(crate) fn from_name(",
        "FeatureId::new(feature)?",
        "FeatureOwnershipSubject::new(subject)?",
        "pub(crate) fn stable_key(&self) -> String",
    ] {
        assert!(
            ownership_section.contains(required),
            "FeatureOwnership should own semantic ownership fact {required}"
        );
    }

    for forbidden in [
        "CandidateTreeState",
        "PublishedSnapshotState",
        "GeneratePlan",
        "RemovalManifest",
        "LockfilePath",
        "OutputRepoPath",
        "std::fs::",
    ] {
        assert!(
            !ownership_section.contains(forbidden),
            "FeatureOwnership must not own reducer, candidate, published, lockfile, or mutation state {forbidden}"
        );
    }

    let graph_section = feature
        .split("pub(crate) struct FeatureGraph")
        .nth(1)
        .and_then(|rest| rest.split("pub(crate) enum FeatureIntentAction").next())
        .expect("feature module should define FeatureGraph before FeatureIntentAction");
    assert!(
        !graph_section.contains("FeatureOwnership"),
        "pre-resolution FeatureGraph should not own resolved ownership assertions"
    );

    assert!(
        architecture.contains("`FeatureOwnership` is the resolved")
            && architecture.contains("semantic ownership assertion")
            && architecture.contains("stable ownership classification")
            && architecture.contains("explicitly removed")
            && architecture.contains("ambiguous")
            && architecture.contains("unsupported")
            && kernel_build_guide.contains("`FeatureOwnership` records resolved semantic")
            && kernel_build_guide.contains("stable classifications"),
        "docs should describe FeatureOwnership ownership"
    );
}
