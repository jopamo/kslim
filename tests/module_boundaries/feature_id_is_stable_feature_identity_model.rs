use super::common::*;

#[test]
fn feature_id_is_stable_feature_identity_model() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let generate_state = state_source(root);
    let architecture = production_source(&root.join("docs/architecture.md"));

    assert!(
        feature.contains("pub(crate) struct FeatureId(String);")
            && feature.contains("impl FeatureId")
            && feature.contains("normalize_feature_name(&id.into())?")
            && feature.contains("pub(crate) fn as_str(&self) -> &str"),
        "FeatureId should be the normalized named-feature identity wrapper"
    );
    assert!(
        feature.contains("id: FeatureId,") && !feature.contains("name: String,"),
        "FeatureIntent should carry FeatureId instead of a raw string name"
    );
    assert!(
        generate_state.contains("(\"name\", intent.id.as_str())")
            && generate_state.contains("name: intent.id.as_str().to_string()"),
        "resolved feature intent plan should derive legacy name fields from FeatureId"
    );
    assert!(
        architecture.contains("`FeatureId` is the typed stable identity for a named feature"),
        "architecture docs should describe FeatureId ownership"
    );
}
