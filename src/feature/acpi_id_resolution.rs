use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureResolvedAcpiIdKind {
    RemoveAcpiIdRoot,
    ExplicitRemoveAcpiId,
    PreserveAcpiIdRoot,
}

#[allow(dead_code)]
impl FeatureResolvedAcpiIdKind {
    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::RemoveAcpiIdRoot => "remove_acpi_id_root",
            Self::ExplicitRemoveAcpiId => "explicit_remove_acpi_id",
            Self::PreserveAcpiIdRoot => "preserve_acpi_id_root",
        }
    }

    pub(crate) const fn ownership_kind(self) -> FeatureOwnershipKind {
        match self {
            Self::RemoveAcpiIdRoot | Self::ExplicitRemoveAcpiId => {
                FeatureOwnershipKind::OwnedSolelyByRemovedFeature
            }
            Self::PreserveAcpiIdRoot => FeatureOwnershipKind::SharedWithLiveFeature,
        }
    }

    pub(crate) const fn is_removal(self) -> bool {
        matches!(self, Self::RemoveAcpiIdRoot | Self::ExplicitRemoveAcpiId)
    }

    pub(crate) const fn is_preservation(self) -> bool {
        matches!(self, Self::PreserveAcpiIdRoot)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureResolvedAcpiId {
    feature: FeatureId,
    id: AcpiId,
    kind: FeatureResolvedAcpiIdKind,
}

#[allow(dead_code)]
impl FeatureResolvedAcpiId {
    pub(crate) fn new(feature: FeatureId, id: AcpiId, kind: FeatureResolvedAcpiIdKind) -> Self {
        Self { feature, id, kind }
    }

    pub(crate) fn feature(&self) -> &FeatureId {
        &self.feature
    }

    pub(crate) fn id(&self) -> &AcpiId {
        &self.id
    }

    pub(crate) fn kind(&self) -> FeatureResolvedAcpiIdKind {
        self.kind
    }

    pub(crate) fn stable_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.kind.stable_name(),
            self.feature.as_str(),
            self.id.as_str()
        )
    }

    pub(crate) fn ownership(&self) -> Result<FeatureOwnership> {
        Ok(FeatureOwnership::new(
            self.kind.ownership_kind(),
            self.feature.clone(),
            FeatureOwnershipSubject::new(format!("acpi_id:{}", self.id.as_str()))?,
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FeatureAcpiIdResolution {
    ids: Vec<FeatureResolvedAcpiId>,
}

#[allow(dead_code)]
impl FeatureAcpiIdResolution {
    pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self> {
        let graph = FeatureGraph::from_profile(profile)?;
        Ok(Self::from_graph(&graph))
    }

    pub(crate) fn from_graph(graph: &FeatureGraph) -> Self {
        let mut ids = Vec::new();
        for node in graph.nodes() {
            ids.extend(acpi_ids_from_intent(node.intent()));
        }
        Self::new(ids)
    }

    pub(crate) fn new(ids: impl IntoIterator<Item = FeatureResolvedAcpiId>) -> Self {
        let mut ids = ids.into_iter().collect::<Vec<_>>();
        ids.sort_by_key(|id| id.stable_key());
        ids.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Self { ids }
    }

    pub(crate) fn ids(&self) -> &[FeatureResolvedAcpiId] {
        &self.ids
    }

    pub(crate) fn id_count(&self) -> usize {
        self.ids.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub(crate) fn remove_acpi_ids(&self) -> Vec<AcpiId> {
        sorted_ids_for_kind(&self.ids, FeatureResolvedAcpiIdKind::is_removal)
    }

    pub(crate) fn preserve_acpi_ids(&self) -> Vec<AcpiId> {
        sorted_ids_for_kind(&self.ids, FeatureResolvedAcpiIdKind::is_preservation)
    }

    pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>> {
        let mut ownerships = self
            .ids
            .iter()
            .map(FeatureResolvedAcpiId::ownership)
            .collect::<Result<Vec<_>>>()?;
        ownerships.sort_by_key(|ownership| ownership.stable_key());
        ownerships.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Ok(ownerships)
    }
}

fn acpi_ids_from_intent(intent: &FeatureIntent) -> Vec<FeatureResolvedAcpiId> {
    let mut ids = Vec::new();
    match intent.action {
        FeatureIntentAction::Remove => {
            ids.extend(intent.acpi_ids.iter().cloned().map(|id| {
                FeatureResolvedAcpiId::new(
                    intent.id.clone(),
                    id,
                    FeatureResolvedAcpiIdKind::RemoveAcpiIdRoot,
                )
            }));
            let explicit_ids = intent.remove_acpi_ids.iter().cloned();
            ids.extend(explicit_ids.map(|id| {
                FeatureResolvedAcpiId::new(
                    intent.id.clone(),
                    id,
                    FeatureResolvedAcpiIdKind::ExplicitRemoveAcpiId,
                )
            }));
        }
        FeatureIntentAction::Preserve => {
            ids.extend(intent.acpi_ids.iter().cloned().map(|id| {
                FeatureResolvedAcpiId::new(
                    intent.id.clone(),
                    id,
                    FeatureResolvedAcpiIdKind::PreserveAcpiIdRoot,
                )
            }));
        }
    }
    ids
}

fn sorted_ids_for_kind(
    ids: &[FeatureResolvedAcpiId],
    matches_kind: impl Fn(FeatureResolvedAcpiIdKind) -> bool,
) -> Vec<AcpiId> {
    let mut ids = ids
        .iter()
        .filter(|id| matches_kind(id.kind()))
        .map(|id| id.id().clone())
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_acpi_id_resolution_resolves_roots_to_acpi_ids() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                acpi_ids: vec![String::from("PNP0C09"), String::from("PNP0C09")],
                remove_acpi_ids: vec![String::from("ACPI0003")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                acpi_ids: vec![String::from("PRP0001")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let resolution = FeatureAcpiIdResolution::from_profile(&profile).unwrap();

        assert_eq!(resolution.id_count(), 3);
        assert!(!resolution.is_empty());
        assert_eq!(
            resolution
                .ids()
                .iter()
                .map(FeatureResolvedAcpiId::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicit_remove_acpi_id:bluetooth:ACPI0003",
                "preserve_acpi_id_root:netfilter:PRP0001",
                "remove_acpi_id_root:bluetooth:PNP0C09",
            ]
        );
        assert_eq!(
            resolution
                .remove_acpi_ids()
                .iter()
                .map(AcpiId::as_str)
                .collect::<Vec<_>>(),
            vec!["ACPI0003", "PNP0C09"]
        );
        assert_eq!(
            resolution
                .preserve_acpi_ids()
                .iter()
                .map(AcpiId::as_str)
                .collect::<Vec<_>>(),
            vec!["PRP0001"]
        );
    }

    #[test]
    fn feature_acpi_id_resolution_emits_acpi_ownerships() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                acpi_ids: vec![String::from("PNP0C09")],
                remove_acpi_ids: vec![String::from("ACPI0003")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                acpi_ids: vec![String::from("PRP0001")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let ownerships = FeatureAcpiIdResolution::from_profile(&profile)
            .unwrap()
            .ownerships()
            .unwrap();

        assert_eq!(
            ownerships
                .iter()
                .map(FeatureOwnership::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "owned_solely_by_removed_feature:bluetooth:acpi_id:ACPI0003",
                "owned_solely_by_removed_feature:bluetooth:acpi_id:PNP0C09",
                "shared_with_live_feature:netfilter:acpi_id:PRP0001",
            ]
        );
    }

    #[test]
    fn feature_acpi_id_resolution_rejects_invalid_acpi_id() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bad"),
            FeatureIntentConfig {
                acpi_ids: vec![String::from("pnp0c09")],
                ..FeatureIntentConfig::default()
            },
        );

        let err = FeatureAcpiIdResolution::from_profile(&profile)
            .unwrap_err()
            .to_string();

        assert!(err.contains("ACPI ID must use uppercase ASCII"));
    }
}
