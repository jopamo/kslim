use super::common::*;

#[test]
fn feature_roots_resolve_to_initcalls() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let initcall_resolution = production_source(&root.join("src/feature/initcall_resolution.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("mod initcall_resolution;")
            && feature.contains("FeatureInitcallResolution")
            && feature.contains("FeatureResolvedInitcall")
            && feature.contains("FeatureResolvedInitcallKind"),
        "feature module should expose the feature initcall resolution slice"
    );

    for required in [
        "pub(crate) enum FeatureResolvedInitcallKind",
        "RemoveInitcallRoot",
        "ExplicitRemoveInitcall",
        "PreserveInitcallRoot",
        "\"remove_initcall_root\"",
        "\"explicit_remove_initcall\"",
        "\"preserve_initcall_root\"",
        "FeatureOwnershipKind::OwnedSolelyByRemovedFeature",
        "FeatureOwnershipKind::SharedWithLiveFeature",
        "pub(crate) struct FeatureResolvedInitcall",
        "feature: FeatureId",
        "initcall: Initcall",
        "kind: FeatureResolvedInitcallKind",
        "pub(crate) fn stable_key(&self) -> String",
        "pub(crate) fn ownership(&self) -> Result<FeatureOwnership>",
        "FeatureOwnershipSubject::new(format!(\"initcall:{}\", self.initcall.as_str()))?",
        "pub(crate) struct FeatureInitcallResolution",
        "initcalls: Vec<FeatureResolvedInitcall>",
        "pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self>",
        "let graph = FeatureGraph::from_profile(profile)?",
        "pub(crate) fn from_graph(graph: &FeatureGraph) -> Self",
        "for node in graph.nodes()",
        "initcalls.extend(initcalls_from_intent(node.intent()))",
        "FeatureIntentAction::Remove",
        "FeatureIntentAction::Preserve",
        "intent.initcalls.iter().cloned()",
        "intent.remove_initcalls.iter().cloned()",
        "pub(crate) fn remove_initcalls(&self) -> Vec<Initcall>",
        "pub(crate) fn preserve_initcalls(&self) -> Vec<Initcall>",
        "pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>>",
    ] {
        assert!(
            initcall_resolution.contains(required),
            "feature initcall resolution should own root-to-initcall fact {required}"
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
        "RuntimeRegistrationSurface",
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
        "early_initcall",
        "module_init",
        "runtime_registrations",
    ] {
        assert!(
            !initcall_resolution.contains(forbidden),
            "FeatureInitcallResolution must only resolve typed initcall intent, not own initcall macro extraction, proof, scanning, or lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("`FeatureInitcallResolution` is the seventeenth feature-graph")
            && architecture.contains("resolves typed feature initcall roots")
            && architecture.contains("`Initcall` facts")
            && architecture.contains("initcall ownership assertions")
            && architecture.contains("without scanning initcall macro sites")
            && architecture.contains("initcall macro proof")
            && kernel_build_guide
                .contains("`FeatureInitcallResolution` resolves typed feature initcall roots")
            && kernel_build_guide.contains("sorted initcall facts")
            && kernel_build_guide.contains("before initcall macro proof"),
        "docs should describe feature root-to-initcall resolution"
    );
}
