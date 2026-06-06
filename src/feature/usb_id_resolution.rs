use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureResolvedUsbIdKind {
    RemoveUsbIdRoot,
    ExplicitRemoveUsbId,
    PreserveUsbIdRoot,
}

#[allow(dead_code)]
impl FeatureResolvedUsbIdKind {
    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::RemoveUsbIdRoot => "remove_usb_id_root",
            Self::ExplicitRemoveUsbId => "explicit_remove_usb_id",
            Self::PreserveUsbIdRoot => "preserve_usb_id_root",
        }
    }

    pub(crate) const fn ownership_kind(self) -> FeatureOwnershipKind {
        match self {
            Self::RemoveUsbIdRoot | Self::ExplicitRemoveUsbId => {
                FeatureOwnershipKind::OwnedSolelyByRemovedFeature
            }
            Self::PreserveUsbIdRoot => FeatureOwnershipKind::SharedWithLiveFeature,
        }
    }

    pub(crate) const fn is_removal(self) -> bool {
        matches!(self, Self::RemoveUsbIdRoot | Self::ExplicitRemoveUsbId)
    }

    pub(crate) const fn is_preservation(self) -> bool {
        matches!(self, Self::PreserveUsbIdRoot)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureResolvedUsbId {
    feature: FeatureId,
    id: UsbId,
    kind: FeatureResolvedUsbIdKind,
}

#[allow(dead_code)]
impl FeatureResolvedUsbId {
    pub(crate) fn new(feature: FeatureId, id: UsbId, kind: FeatureResolvedUsbIdKind) -> Self {
        Self { feature, id, kind }
    }

    pub(crate) fn feature(&self) -> &FeatureId {
        &self.feature
    }

    pub(crate) fn id(&self) -> &UsbId {
        &self.id
    }

    pub(crate) fn kind(&self) -> FeatureResolvedUsbIdKind {
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
            FeatureOwnershipSubject::new(format!("usb_id:{}", self.id.as_str()))?,
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FeatureUsbIdResolution {
    ids: Vec<FeatureResolvedUsbId>,
}

#[allow(dead_code)]
impl FeatureUsbIdResolution {
    pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self> {
        let graph = FeatureGraph::from_profile(profile)?;
        Ok(Self::from_graph(&graph))
    }

    pub(crate) fn from_graph(graph: &FeatureGraph) -> Self {
        let mut ids = Vec::new();
        for node in graph.nodes() {
            ids.extend(usb_ids_from_intent(node.intent()));
        }
        Self::new(ids)
    }

    pub(crate) fn new(ids: impl IntoIterator<Item = FeatureResolvedUsbId>) -> Self {
        let mut ids = ids.into_iter().collect::<Vec<_>>();
        ids.sort_by_key(|id| id.stable_key());
        ids.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Self { ids }
    }

    pub(crate) fn ids(&self) -> &[FeatureResolvedUsbId] {
        &self.ids
    }

    pub(crate) fn id_count(&self) -> usize {
        self.ids.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub(crate) fn remove_usb_ids(&self) -> Vec<UsbId> {
        sorted_ids_for_kind(&self.ids, FeatureResolvedUsbIdKind::is_removal)
    }

    pub(crate) fn preserve_usb_ids(&self) -> Vec<UsbId> {
        sorted_ids_for_kind(&self.ids, FeatureResolvedUsbIdKind::is_preservation)
    }

    pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>> {
        let mut ownerships = self
            .ids
            .iter()
            .map(FeatureResolvedUsbId::ownership)
            .collect::<Result<Vec<_>>>()?;
        ownerships.sort_by_key(|ownership| ownership.stable_key());
        ownerships.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Ok(ownerships)
    }
}

fn usb_ids_from_intent(intent: &FeatureIntent) -> Vec<FeatureResolvedUsbId> {
    let mut ids = Vec::new();
    match intent.action {
        FeatureIntentAction::Remove => {
            ids.extend(intent.usb_ids.iter().cloned().map(|id| {
                FeatureResolvedUsbId::new(
                    intent.id.clone(),
                    id,
                    FeatureResolvedUsbIdKind::RemoveUsbIdRoot,
                )
            }));
            let explicit_ids = intent.remove_usb_ids.iter().cloned();
            ids.extend(explicit_ids.map(|id| {
                FeatureResolvedUsbId::new(
                    intent.id.clone(),
                    id,
                    FeatureResolvedUsbIdKind::ExplicitRemoveUsbId,
                )
            }));
        }
        FeatureIntentAction::Preserve => {
            ids.extend(intent.usb_ids.iter().cloned().map(|id| {
                FeatureResolvedUsbId::new(
                    intent.id.clone(),
                    id,
                    FeatureResolvedUsbIdKind::PreserveUsbIdRoot,
                )
            }));
        }
    }
    ids
}

fn sorted_ids_for_kind(
    ids: &[FeatureResolvedUsbId],
    matches_kind: impl Fn(FeatureResolvedUsbIdKind) -> bool,
) -> Vec<UsbId> {
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
    fn feature_usb_id_resolution_resolves_roots_to_usb_ids() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                usb_ids: vec![String::from("0BDA:8153"), String::from("0BDA:8153")],
                remove_usb_ids: vec![String::from("046D:C52B")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                usb_ids: vec![String::from("1D6B:0002")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let resolution = FeatureUsbIdResolution::from_profile(&profile).unwrap();

        assert_eq!(resolution.id_count(), 3);
        assert!(!resolution.is_empty());
        assert_eq!(
            resolution
                .ids()
                .iter()
                .map(FeatureResolvedUsbId::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicit_remove_usb_id:bluetooth:046D:C52B",
                "preserve_usb_id_root:netfilter:1D6B:0002",
                "remove_usb_id_root:bluetooth:0BDA:8153",
            ]
        );
        assert_eq!(
            resolution
                .remove_usb_ids()
                .iter()
                .map(UsbId::as_str)
                .collect::<Vec<_>>(),
            vec!["046D:C52B", "0BDA:8153"]
        );
        assert_eq!(
            resolution
                .preserve_usb_ids()
                .iter()
                .map(UsbId::as_str)
                .collect::<Vec<_>>(),
            vec!["1D6B:0002"]
        );
    }

    #[test]
    fn feature_usb_id_resolution_emits_usb_ownerships() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                usb_ids: vec![String::from("0BDA:8153")],
                remove_usb_ids: vec![String::from("046D:C52B")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                usb_ids: vec![String::from("1D6B:0002")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let ownerships = FeatureUsbIdResolution::from_profile(&profile)
            .unwrap()
            .ownerships()
            .unwrap();

        assert_eq!(
            ownerships
                .iter()
                .map(FeatureOwnership::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "owned_solely_by_removed_feature:bluetooth:usb_id:046D:C52B",
                "owned_solely_by_removed_feature:bluetooth:usb_id:0BDA:8153",
                "shared_with_live_feature:netfilter:usb_id:1D6B:0002",
            ]
        );
    }

    #[test]
    fn feature_usb_id_resolution_rejects_invalid_usb_id() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bad"),
            FeatureIntentConfig {
                usb_ids: vec![String::from("0BDA:815g")],
                ..FeatureIntentConfig::default()
            },
        );

        let err = FeatureUsbIdResolution::from_profile(&profile)
            .unwrap_err()
            .to_string();

        assert!(err.contains("USB ID must use uppercase hexadecimal"));
    }
}
