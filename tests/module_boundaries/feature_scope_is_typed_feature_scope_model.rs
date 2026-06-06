use super::common::*;

#[test]
fn feature_scope_is_typed_feature_scope_model() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let generate_state = state_source(root);
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("pub(crate) struct FeatureScope")
            && feature.contains("arch_scope: Vec<ArchName>")
            && feature.contains("pub(crate) fn from_arch_scope(arches: &[String])")
            && feature.contains("pub(crate) fn unscoped() -> Self")
            && feature.contains("pub(crate) fn is_unscoped(&self) -> bool")
            && feature.contains("pub(crate) fn arch_scope(&self) -> &[ArchName]")
            && feature.contains("pub(crate) fn stable_key(&self) -> String"),
        "FeatureScope should own normalized feature applicability scope"
    );
    assert!(
        feature.contains("scope: FeatureScope,")
            && feature.contains("let scope = FeatureScope::from_arch_scope(&config.arch_scope)?")
            && !feature.contains("pub(crate) arch_scope: Vec<ArchName>,"),
        "FeatureIntent should store typed FeatureScope instead of raw arch scope"
    );
    assert!(
        generate_state.contains("arch_scope: intent.scope.arch_scope().to_vec()"),
        "resolved plan entries should derive legacy arch_scope from FeatureScope"
    );
    assert!(
        architecture.contains("`FeatureScope` is the typed applicability scope")
            && kernel_build_guide.contains("Feature arch scopes are typed")
            && kernel_build_guide.contains("empty scope means unscoped/all architectures"),
        "docs should describe FeatureScope semantics"
    );
}
