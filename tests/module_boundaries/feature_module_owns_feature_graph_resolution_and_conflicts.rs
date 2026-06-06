use super::common::*;

#[test]
fn feature_module_owns_feature_graph_resolution_and_conflicts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let main = production_source(&root.join("src/main.rs"));
    let feature_root = root.join("src/feature/mod.rs");
    let feature = production_source(&feature_root);
    let architecture = production_source(&root.join("docs/architecture.md"));

    assert!(
        main.contains("mod feature;"),
        "main.rs should register the feature ownership module"
    );
    assert!(
        !root.join("src/feature.rs").exists(),
        "feature ownership should live under src/feature/ instead of a top-level src/feature.rs root"
    );
    assert!(
        feature_root.exists(),
        "src/feature/mod.rs should be the feature module root"
    );

    for owned in [
        "path_resolution.rs",
        "kconfig_resolution.rs",
        "kbuild_resolution.rs",
        "source_resolution.rs",
        "public_header_resolution.rs",
        "private_header_resolution.rs",
        "uapi_header_resolution.rs",
        "conflict_detection.rs",
    ] {
        assert!(
            root.join("src/feature").join(owned).is_file(),
            "src/feature/{owned} should remain beside the feature module root"
        );
    }

    for required in [
        "//! Feature semantics, graph resolution, and conflict models.",
        "mod conflict_detection;",
        "mod path_resolution;",
        "mod kconfig_resolution;",
        "mod kbuild_resolution;",
        "pub(crate) use conflict_detection::FeatureKconfigSelection;",
        "pub(crate) use path_resolution::{",
        "pub(crate) struct FeatureGraph",
        "pub(crate) struct FeatureOwnership",
        "pub(crate) enum FeatureConflictKind",
        "pub(crate) struct FeatureConflictReport",
    ] {
        assert!(
            feature.contains(required),
            "src/feature/mod.rs should own feature graph/resolution/conflict item {required}"
        );
    }

    assert!(
        architecture.contains("`feature/*`")
            && architecture.contains("feature graph models")
            && architecture.contains("ownership resolution slices")
            && architecture.contains("semantic conflict reports"),
        "docs/architecture.md should document feature module ownership"
    );
}
