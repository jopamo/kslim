use super::common::*;

#[test]
fn feature_roots_resolve_to_kselftest_targets() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let kselftest_resolution =
        production_source(&root.join("src/feature/kselftest_target_resolution.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("mod kselftest_target_resolution;")
            && feature.contains("FeatureKselftestTargetResolution")
            && feature.contains("FeatureResolvedKselftestTarget")
            && feature.contains("FeatureResolvedKselftestTargetKind"),
        "feature module should expose the feature kselftest-target resolution slice"
    );

    for required in [
        "pub(crate) enum FeatureResolvedKselftestTargetKind",
        "RemoveKselftestTargetRoot",
        "ExplicitRemoveKselftestTarget",
        "PreserveKselftestTargetRoot",
        "\"remove_kselftest_target_root\"",
        "\"explicit_remove_kselftest_target\"",
        "\"preserve_kselftest_target_root\"",
        "FeatureOwnershipKind::OwnedSolelyByRemovedFeature",
        "FeatureOwnershipKind::SharedWithLiveFeature",
        "pub(crate) struct FeatureResolvedKselftestTarget",
        "feature: FeatureId",
        "target: KselftestTarget",
        "kind: FeatureResolvedKselftestTargetKind",
        "pub(crate) fn stable_key(&self) -> String",
        "pub(crate) fn ownership(&self) -> Result<FeatureOwnership>",
        "FeatureOwnershipSubject::new(format!(\"kselftest_target:{}\", self.target.as_str()))?",
        "pub(crate) struct FeatureKselftestTargetResolution",
        "targets: Vec<FeatureResolvedKselftestTarget>",
        "pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self>",
        "let graph = FeatureGraph::from_profile(profile)?",
        "pub(crate) fn from_graph(graph: &FeatureGraph) -> Self",
        "for node in graph.nodes()",
        "targets.extend(kselftest_targets_from_intent(node.intent()))",
        "FeatureIntentAction::Remove",
        "FeatureIntentAction::Preserve",
        "intent.kselftest_targets.iter().cloned()",
        "intent.remove_kselftest_targets.iter().cloned()",
        "pub(crate) fn remove_kselftest_targets(&self) -> Vec<KselftestTarget>",
        "pub(crate) fn preserve_kselftest_targets(&self) -> Vec<KselftestTarget>",
        "pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>>",
    ] {
        assert!(
            kselftest_resolution.contains(required),
            "feature kselftest-target resolution should own root-to-kselftest-target fact {required}"
        );
    }

    for forbidden in [
        "KconfigSymbol",
        "KbuildObject",
        "SourceFilePath",
        "HeaderPath",
        "UapiPath",
        "GeneratedArtifactPath",
        "DocumentationPath",
        "ToolPath",
        "SamplePath",
        "KunitSuite",
        "ExportedSymbol",
        "ModuleName",
        "ModuleAlias",
        "DeviceCompatible",
        "AcpiId",
        "PciId",
        "UsbId",
        "FirmwarePath",
        "Initcall",
        "RuntimeRegistrationSurface",
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
        "kunit.py",
        "make",
        "selftest::",
    ] {
        assert!(
            !kselftest_resolution.contains(forbidden),
            "FeatureKselftestTargetResolution must only resolve typed kselftest target intent, not own test execution, build proof, scanning, or lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture
            .contains("`FeatureKselftestTargetResolution` is the twenty-third feature-graph")
            && architecture.contains("resolves typed feature kselftest target roots")
            && architecture.contains("`KselftestTarget` facts")
            && architecture.contains("kselftest target ownership assertions")
            && architecture.contains("without scanning kselftest sources")
            && architecture.contains("or executing kselftest")
            && kernel_build_guide.contains(
                "`FeatureKselftestTargetResolution` resolves typed feature kselftest target roots"
            )
            && kernel_build_guide.contains("sorted kselftest target facts")
            && kernel_build_guide.contains("before kselftest execution proof"),
        "docs should describe feature root-to-kselftest-target resolution"
    );
}
