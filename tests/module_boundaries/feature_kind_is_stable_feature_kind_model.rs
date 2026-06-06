use super::common::*;

#[test]
fn feature_kind_is_stable_feature_kind_model() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let generate_state = state_source(root);
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    for required in [
        "pub(crate) enum FeatureKind",
        "pub(crate) const ALL: [Self; 18]",
        "Subsystem",
        "Driver",
        "Bus",
        "Filesystem",
        "NetworkProtocol",
        "CryptoAlgorithm",
        "SchedulerFeature",
        "SecurityFeature",
        "TracingFeature",
        "BpfFeature",
        "ArchFeature",
        "SocPlatform",
        "BoardPlatformSupport",
        "FirmwareLoaderFeature",
        "ModuleOnlyFeature",
        "UserspaceAbiFeature",
        "GeneratedArtifactFamily",
        "DocsTestsToolsOnlyFeature",
        "\"network_protocol\"",
        "\"crypto_algorithm\"",
        "\"docs_tests_tools_only_feature\"",
        "unsupported feature kind",
    ] {
        assert!(
            feature.contains(required),
            "feature module should own stable feature kind token {required}"
        );
    }

    assert!(
        feature.contains("kind: Option<FeatureKind>,")
            && feature.contains(".map(FeatureKind::from_stable_name)")
            && !feature.contains("kind: Option<String>,"),
        "FeatureIntent should store typed FeatureKind, not raw kind strings"
    );
    assert!(
        generate_state.contains("let kind = intent.kind.map(|kind| kind.stable_name());")
            && generate_state.contains("(\"kind\", kind.unwrap_or(\"<none>\"))")
            && generate_state.contains("kind: kind.map(str::to_string)"),
        "resolved feature intent plan should serialize FeatureKind stable names"
    );
    assert!(
        architecture.contains("`FeatureKind` is the stable enum for supported feature")
            && kernel_build_guide.contains("Supported stable kind tokens")
            && kernel_build_guide.contains("docs_tests_tools_only_feature"),
        "docs should list supported stable feature kind tokens"
    );
}
