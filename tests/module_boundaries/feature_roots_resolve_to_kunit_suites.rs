use super::common::*;

#[test]
fn feature_roots_resolve_to_kunit_suites() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let kunit_resolution = production_source(&root.join("src/feature/kunit_suite_resolution.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("mod kunit_suite_resolution;")
            && feature.contains("FeatureKunitSuiteResolution")
            && feature.contains("FeatureResolvedKunitSuite")
            && feature.contains("FeatureResolvedKunitSuiteKind"),
        "feature module should expose the feature KUnit-suite resolution slice"
    );

    for required in [
        "pub(crate) enum FeatureResolvedKunitSuiteKind",
        "RemoveKunitSuiteRoot",
        "ExplicitRemoveKunitSuite",
        "PreserveKunitSuiteRoot",
        "\"remove_kunit_suite_root\"",
        "\"explicit_remove_kunit_suite\"",
        "\"preserve_kunit_suite_root\"",
        "FeatureOwnershipKind::OwnedSolelyByRemovedFeature",
        "FeatureOwnershipKind::SharedWithLiveFeature",
        "pub(crate) struct FeatureResolvedKunitSuite",
        "feature: FeatureId",
        "suite: KunitSuite",
        "kind: FeatureResolvedKunitSuiteKind",
        "pub(crate) fn stable_key(&self) -> String",
        "pub(crate) fn ownership(&self) -> Result<FeatureOwnership>",
        "FeatureOwnershipSubject::new(format!(\"kunit_suite:{}\", self.suite.as_str()))?",
        "pub(crate) struct FeatureKunitSuiteResolution",
        "suites: Vec<FeatureResolvedKunitSuite>",
        "pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self>",
        "let graph = FeatureGraph::from_profile(profile)?",
        "pub(crate) fn from_graph(graph: &FeatureGraph) -> Self",
        "for node in graph.nodes()",
        "suites.extend(kunit_suites_from_intent(node.intent()))",
        "FeatureIntentAction::Remove",
        "FeatureIntentAction::Preserve",
        "intent.kunit_suites.iter().cloned()",
        "intent.remove_kunit_suites.iter().cloned()",
        "pub(crate) fn remove_kunit_suites(&self) -> Vec<KunitSuite>",
        "pub(crate) fn preserve_kunit_suites(&self) -> Vec<KunitSuite>",
        "pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>>",
    ] {
        assert!(
            kunit_resolution.contains(required),
            "feature KUnit-suite resolution should own root-to-KUnit-suite fact {required}"
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
        "KselftestTarget",
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
        "selftest",
        "kselftest",
    ] {
        assert!(
            !kunit_resolution.contains(forbidden),
            "FeatureKunitSuiteResolution must only resolve typed KUnit suite intent, not own test execution, build proof, scanning, or lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("`FeatureKunitSuiteResolution` is the twenty-second feature-graph")
            && architecture.contains("resolves typed feature KUnit suite roots")
            && architecture.contains("`KunitSuite` facts")
            && architecture.contains("KUnit suite ownership assertions")
            && architecture.contains("without scanning KUnit test sources")
            && architecture.contains("or executing KUnit")
            && kernel_build_guide
                .contains("`FeatureKunitSuiteResolution` resolves typed feature KUnit suite roots")
            && kernel_build_guide.contains("sorted KUnit suite facts")
            && kernel_build_guide.contains("before KUnit execution proof"),
        "docs should describe feature root-to-KUnit-suite resolution"
    );
}
