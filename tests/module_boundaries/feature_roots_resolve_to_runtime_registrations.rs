use super::common::*;

#[test]
fn feature_roots_resolve_to_runtime_registrations() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let runtime_registration_resolution =
        production_source(&root.join("src/feature/runtime_registration_resolution.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("mod runtime_registration_resolution;")
            && feature.contains("FeatureRuntimeRegistrationResolution")
            && feature.contains("FeatureResolvedRuntimeRegistration")
            && feature.contains("FeatureResolvedRuntimeRegistrationKind"),
        "feature module should expose the feature runtime-registration resolution slice"
    );

    for required in [
        "pub(crate) enum FeatureResolvedRuntimeRegistrationKind",
        "RemoveRuntimeRegistrationRoot",
        "ExplicitRemoveRuntimeRegistration",
        "PreserveRuntimeRegistrationRoot",
        "\"remove_runtime_registration_root\"",
        "\"explicit_remove_runtime_registration\"",
        "\"preserve_runtime_registration_root\"",
        "FeatureOwnershipKind::OwnedSolelyByRemovedFeature",
        "FeatureOwnershipKind::SharedWithLiveFeature",
        "pub(crate) struct FeatureResolvedRuntimeRegistration",
        "feature: FeatureId",
        "runtime_registration: RuntimeRegistrationSurface",
        "kind: FeatureResolvedRuntimeRegistrationKind",
        "pub(crate) fn stable_key(&self) -> String",
        "pub(crate) fn ownership(&self) -> Result<FeatureOwnership>",
        "FeatureOwnershipSubject::new(format!(",
        "\"runtime_registration_surface:{}\"",
        "pub(crate) struct FeatureRuntimeRegistrationResolution",
        "runtime_registrations: Vec<FeatureResolvedRuntimeRegistration>",
        "pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self>",
        "let graph = FeatureGraph::from_profile(profile)?",
        "pub(crate) fn from_graph(graph: &FeatureGraph) -> Self",
        "for node in graph.nodes()",
        "runtime_registrations.extend(runtime_registrations_from_intent(node.intent()))",
        "FeatureIntentAction::Remove",
        "FeatureIntentAction::Preserve",
        "intent.runtime_registrations.iter().cloned()",
        "intent.remove_runtime_registrations.iter().cloned()",
        "pub(crate) fn remove_runtime_registrations(&self) -> Vec<RuntimeRegistrationSurface>",
        "pub(crate) fn preserve_runtime_registrations(&self) -> Vec<RuntimeRegistrationSurface>",
        "pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>>",
    ] {
        assert!(
            runtime_registration_resolution.contains(required),
            "feature runtime-registration resolution should own root-to-runtime-registration fact {required}"
        );
    }

    for forbidden in [
        "KconfigSymbol",
        "KbuildObject",
        "SourceFilePath",
        "HeaderPath",
        "UapiPath",
        "GeneratedArtifactPath",
        "ExportedSymbol",
        "ModuleName",
        "ModuleAlias",
        "DeviceCompatible",
        "AcpiId",
        "PciId",
        "UsbId",
        "FirmwarePath",
        "Initcall",
        "DocumentationPath",
        "ToolPath",
        "SamplePath",
        "KunitSuite",
        "KselftestTarget",
        "CandidateTreeState",
        "PublishedSnapshotState",
        "GeneratePlan",
        "RemovalManifest",
        "LockfilePath",
        "OutputRepoPath",
        "crate::hardware::",
        "crate::kbuild::",
        "crate::tree_index::",
        "std::fs::",
        "walkdir",
        "prove_removed_runtime_registrations_have_no_live_entry_points",
        "RuntimeRegistrationRemovalProof",
        "mask_c_comments_and_literals",
        "live entry point",
    ] {
        assert!(
            !runtime_registration_resolution.contains(forbidden),
            "FeatureRuntimeRegistrationResolution must only resolve typed runtime-registration intent, not own registration extraction, proof, scanning, or lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("`FeatureRuntimeRegistrationResolution` is the eighteenth feature-graph")
            && architecture.contains("resolves typed feature runtime registration surfaces")
            && architecture.contains("`RuntimeRegistrationSurface` facts")
            && architecture.contains("runtime-registration ownership assertions")
            && architecture.contains("without scanning runtime registration call sites")
            && architecture.contains("no-live-entry-point proof")
            && kernel_build_guide.contains(
                "`FeatureRuntimeRegistrationResolution` resolves typed feature runtime registration surfaces"
            )
            && kernel_build_guide.contains("sorted runtime-registration facts")
            && kernel_build_guide.contains("before no-live-entry-point proof"),
        "docs should describe feature root-to-runtime-registration resolution"
    );
}
