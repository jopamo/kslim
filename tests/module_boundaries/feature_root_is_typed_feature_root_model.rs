use super::common::*;

#[test]
fn feature_root_is_typed_feature_root_model() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let generate_state = state_source(root);
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("pub(crate) struct FeatureRoot(RelativeKernelPath);")
            && feature.contains("impl FeatureRoot")
            && feature.contains("pub(crate) fn new(root: impl Into<PathBuf>)")
            && feature.contains("RelativeKernelPath::new(root)?")
            && feature.contains("pub(crate) fn as_path(&self) -> &Path")
            && feature
                .contains("pub(crate) fn as_relative_kernel_path(&self) -> &RelativeKernelPath"),
        "FeatureRoot should wrap validated relative kernel-tree roots"
    );
    assert!(
        feature.contains("roots: Vec<FeatureRoot>,")
            && feature.contains("fn sorted_feature_roots(values: &[String])")
            && feature.contains("FeatureRoot::new(value.as_str())")
            && feature.contains("fn join_feature_roots(values: &[FeatureRoot])"),
        "FeatureIntent should store typed FeatureRoot values"
    );
    assert!(
        generate_state.contains(".map(|root| root.as_relative_kernel_path().clone())"),
        "resolved plan entries should derive relative paths from FeatureRoot"
    );
    assert!(
        architecture.contains("`FeatureRoot`")
            && architecture.contains("typed relative kernel-tree root")
            && kernel_build_guide.contains("Feature roots are relative kernel-tree roots")
            && kernel_build_guide.contains("absolute paths, parent")
            && kernel_build_guide.contains("tree root itself are rejected"),
        "docs should describe FeatureRoot path boundaries"
    );
}
