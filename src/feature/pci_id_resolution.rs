use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureResolvedPciIdKind {
    RemovePciIdRoot,
    ExplicitRemovePciId,
    PreservePciIdRoot,
}

#[allow(dead_code)]
impl FeatureResolvedPciIdKind {
    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::RemovePciIdRoot => "remove_pci_id_root",
            Self::ExplicitRemovePciId => "explicit_remove_pci_id",
            Self::PreservePciIdRoot => "preserve_pci_id_root",
        }
    }

    pub(crate) const fn ownership_kind(self) -> FeatureOwnershipKind {
        match self {
            Self::RemovePciIdRoot | Self::ExplicitRemovePciId => {
                FeatureOwnershipKind::OwnedSolelyByRemovedFeature
            }
            Self::PreservePciIdRoot => FeatureOwnershipKind::SharedWithLiveFeature,
        }
    }

    pub(crate) const fn is_removal(self) -> bool {
        matches!(self, Self::RemovePciIdRoot | Self::ExplicitRemovePciId)
    }

    pub(crate) const fn is_preservation(self) -> bool {
        matches!(self, Self::PreservePciIdRoot)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureResolvedPciId {
    feature: FeatureId,
    id: PciId,
    kind: FeatureResolvedPciIdKind,
}

#[allow(dead_code)]
impl FeatureResolvedPciId {
    pub(crate) fn new(feature: FeatureId, id: PciId, kind: FeatureResolvedPciIdKind) -> Self {
        Self { feature, id, kind }
    }

    pub(crate) fn feature(&self) -> &FeatureId {
        &self.feature
    }

    pub(crate) fn id(&self) -> &PciId {
        &self.id
    }

    pub(crate) fn kind(&self) -> FeatureResolvedPciIdKind {
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
            FeatureOwnershipSubject::new(format!("pci_id:{}", self.id.as_str()))?,
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FeaturePciIdResolution {
    ids: Vec<FeatureResolvedPciId>,
}

#[allow(dead_code)]
impl FeaturePciIdResolution {
    pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self> {
        let graph = FeatureGraph::from_profile(profile)?;
        Ok(Self::from_graph(&graph))
    }

    pub(crate) fn from_graph(graph: &FeatureGraph) -> Self {
        let mut ids = Vec::new();
        for node in graph.nodes() {
            ids.extend(pci_ids_from_intent(node.intent()));
        }
        Self::new(ids)
    }

    pub(crate) fn new(ids: impl IntoIterator<Item = FeatureResolvedPciId>) -> Self {
        let mut ids = ids.into_iter().collect::<Vec<_>>();
        ids.sort_by_key(|id| id.stable_key());
        ids.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Self { ids }
    }

    pub(crate) fn ids(&self) -> &[FeatureResolvedPciId] {
        &self.ids
    }

    pub(crate) fn id_count(&self) -> usize {
        self.ids.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub(crate) fn remove_pci_ids(&self) -> Vec<PciId> {
        sorted_ids_for_kind(&self.ids, FeatureResolvedPciIdKind::is_removal)
    }

    pub(crate) fn preserve_pci_ids(&self) -> Vec<PciId> {
        sorted_ids_for_kind(&self.ids, FeatureResolvedPciIdKind::is_preservation)
    }

    pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>> {
        let mut ownerships = self
            .ids
            .iter()
            .map(FeatureResolvedPciId::ownership)
            .collect::<Result<Vec<_>>>()?;
        ownerships.sort_by_key(|ownership| ownership.stable_key());
        ownerships.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Ok(ownerships)
    }
}

fn pci_ids_from_intent(intent: &FeatureIntent) -> Vec<FeatureResolvedPciId> {
    let mut ids = Vec::new();
    match intent.action {
        FeatureIntentAction::Remove => {
            ids.extend(intent.pci_ids.iter().cloned().map(|id| {
                FeatureResolvedPciId::new(
                    intent.id.clone(),
                    id,
                    FeatureResolvedPciIdKind::RemovePciIdRoot,
                )
            }));
            let explicit_ids = intent.remove_pci_ids.iter().cloned();
            ids.extend(explicit_ids.map(|id| {
                FeatureResolvedPciId::new(
                    intent.id.clone(),
                    id,
                    FeatureResolvedPciIdKind::ExplicitRemovePciId,
                )
            }));
        }
        FeatureIntentAction::Preserve => {
            ids.extend(intent.pci_ids.iter().cloned().map(|id| {
                FeatureResolvedPciId::new(
                    intent.id.clone(),
                    id,
                    FeatureResolvedPciIdKind::PreservePciIdRoot,
                )
            }));
        }
    }
    ids
}

fn sorted_ids_for_kind(
    ids: &[FeatureResolvedPciId],
    matches_kind: impl Fn(FeatureResolvedPciIdKind) -> bool,
) -> Vec<PciId> {
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
    fn feature_pci_id_resolution_resolves_roots_to_pci_ids() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                pci_ids: vec![String::from("8086:1572"), String::from("8086:1572")],
                remove_pci_ids: vec![String::from("10EC:8168")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                pci_ids: vec![String::from("1AF4:1000")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let resolution = FeaturePciIdResolution::from_profile(&profile).unwrap();

        assert_eq!(resolution.id_count(), 3);
        assert!(!resolution.is_empty());
        assert_eq!(
            resolution
                .ids()
                .iter()
                .map(FeatureResolvedPciId::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicit_remove_pci_id:bluetooth:10EC:8168",
                "preserve_pci_id_root:netfilter:1AF4:1000",
                "remove_pci_id_root:bluetooth:8086:1572",
            ]
        );
        assert_eq!(
            resolution
                .remove_pci_ids()
                .iter()
                .map(PciId::as_str)
                .collect::<Vec<_>>(),
            vec!["10EC:8168", "8086:1572"]
        );
        assert_eq!(
            resolution
                .preserve_pci_ids()
                .iter()
                .map(PciId::as_str)
                .collect::<Vec<_>>(),
            vec!["1AF4:1000"]
        );
    }

    #[test]
    fn feature_pci_id_resolution_emits_pci_ownerships() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                pci_ids: vec![String::from("8086:1572")],
                remove_pci_ids: vec![String::from("10EC:8168")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                pci_ids: vec![String::from("1AF4:1000")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let ownerships = FeaturePciIdResolution::from_profile(&profile)
            .unwrap()
            .ownerships()
            .unwrap();

        assert_eq!(
            ownerships
                .iter()
                .map(FeatureOwnership::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "owned_solely_by_removed_feature:bluetooth:pci_id:10EC:8168",
                "owned_solely_by_removed_feature:bluetooth:pci_id:8086:1572",
                "shared_with_live_feature:netfilter:pci_id:1AF4:1000",
            ]
        );
    }

    #[test]
    fn feature_pci_id_resolution_rejects_invalid_pci_id() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bad"),
            FeatureIntentConfig {
                pci_ids: vec![String::from("8086:157g")],
                ..FeatureIntentConfig::default()
            },
        );

        let err = FeaturePciIdResolution::from_profile(&profile)
            .unwrap_err()
            .to_string();

        assert!(err.contains("PCI ID must use uppercase hexadecimal"));
    }
}
