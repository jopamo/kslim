use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureResolvedDeviceCompatibleKind {
    RemoveDeviceCompatibleRoot,
    ExplicitRemoveDeviceCompatible,
    PreserveDeviceCompatibleRoot,
}

#[allow(dead_code)]
impl FeatureResolvedDeviceCompatibleKind {
    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::RemoveDeviceCompatibleRoot => "remove_device_compatible_root",
            Self::ExplicitRemoveDeviceCompatible => "explicit_remove_device_compatible",
            Self::PreserveDeviceCompatibleRoot => "preserve_device_compatible_root",
        }
    }

    pub(crate) const fn ownership_kind(self) -> FeatureOwnershipKind {
        match self {
            Self::RemoveDeviceCompatibleRoot | Self::ExplicitRemoveDeviceCompatible => {
                FeatureOwnershipKind::OwnedSolelyByRemovedFeature
            }
            Self::PreserveDeviceCompatibleRoot => FeatureOwnershipKind::SharedWithLiveFeature,
        }
    }

    pub(crate) const fn is_removal(self) -> bool {
        matches!(
            self,
            Self::RemoveDeviceCompatibleRoot | Self::ExplicitRemoveDeviceCompatible
        )
    }

    pub(crate) const fn is_preservation(self) -> bool {
        matches!(self, Self::PreserveDeviceCompatibleRoot)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureResolvedDeviceCompatible {
    feature: FeatureId,
    compatible: DeviceCompatible,
    kind: FeatureResolvedDeviceCompatibleKind,
}

#[allow(dead_code)]
impl FeatureResolvedDeviceCompatible {
    pub(crate) fn new(
        feature: FeatureId,
        compatible: DeviceCompatible,
        kind: FeatureResolvedDeviceCompatibleKind,
    ) -> Self {
        Self {
            feature,
            compatible,
            kind,
        }
    }

    pub(crate) fn feature(&self) -> &FeatureId {
        &self.feature
    }

    pub(crate) fn compatible(&self) -> &DeviceCompatible {
        &self.compatible
    }

    pub(crate) fn kind(&self) -> FeatureResolvedDeviceCompatibleKind {
        self.kind
    }

    pub(crate) fn stable_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.kind.stable_name(),
            self.feature.as_str(),
            self.compatible.as_str()
        )
    }

    pub(crate) fn ownership(&self) -> Result<FeatureOwnership> {
        Ok(FeatureOwnership::new(
            self.kind.ownership_kind(),
            self.feature.clone(),
            FeatureOwnershipSubject::new(format!(
                "device_compatible:{}",
                self.compatible.as_str()
            ))?,
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FeatureDeviceCompatibleResolution {
    compatibles: Vec<FeatureResolvedDeviceCompatible>,
}

#[allow(dead_code)]
impl FeatureDeviceCompatibleResolution {
    pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self> {
        let graph = FeatureGraph::from_profile(profile)?;
        Ok(Self::from_graph(&graph))
    }

    pub(crate) fn from_graph(graph: &FeatureGraph) -> Self {
        let mut compatibles = Vec::new();
        for node in graph.nodes() {
            compatibles.extend(compatibles_from_intent(node.intent()));
        }
        Self::new(compatibles)
    }

    pub(crate) fn new(
        compatibles: impl IntoIterator<Item = FeatureResolvedDeviceCompatible>,
    ) -> Self {
        let mut compatibles = compatibles.into_iter().collect::<Vec<_>>();
        compatibles.sort_by_key(|compatible| compatible.stable_key());
        compatibles.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Self { compatibles }
    }

    pub(crate) fn compatibles(&self) -> &[FeatureResolvedDeviceCompatible] {
        &self.compatibles
    }

    pub(crate) fn compatible_count(&self) -> usize {
        self.compatibles.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.compatibles.is_empty()
    }

    pub(crate) fn remove_device_compatibles(&self) -> Vec<DeviceCompatible> {
        sorted_compatibles_for_kind(
            &self.compatibles,
            FeatureResolvedDeviceCompatibleKind::is_removal,
        )
    }

    pub(crate) fn preserve_device_compatibles(&self) -> Vec<DeviceCompatible> {
        sorted_compatibles_for_kind(
            &self.compatibles,
            FeatureResolvedDeviceCompatibleKind::is_preservation,
        )
    }

    pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>> {
        let mut ownerships = self
            .compatibles
            .iter()
            .map(FeatureResolvedDeviceCompatible::ownership)
            .collect::<Result<Vec<_>>>()?;
        ownerships.sort_by_key(|ownership| ownership.stable_key());
        ownerships.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Ok(ownerships)
    }
}

fn compatibles_from_intent(intent: &FeatureIntent) -> Vec<FeatureResolvedDeviceCompatible> {
    let mut compatibles = Vec::new();
    match intent.action {
        FeatureIntentAction::Remove => {
            compatibles.extend(intent.device_compatibles.iter().cloned().map(|compatible| {
                FeatureResolvedDeviceCompatible::new(
                    intent.id.clone(),
                    compatible,
                    FeatureResolvedDeviceCompatibleKind::RemoveDeviceCompatibleRoot,
                )
            }));
            let explicit_compatibles = intent.remove_device_compatibles.iter().cloned();
            compatibles.extend(explicit_compatibles.map(|compatible| {
                FeatureResolvedDeviceCompatible::new(
                    intent.id.clone(),
                    compatible,
                    FeatureResolvedDeviceCompatibleKind::ExplicitRemoveDeviceCompatible,
                )
            }));
        }
        FeatureIntentAction::Preserve => {
            compatibles.extend(intent.device_compatibles.iter().cloned().map(|compatible| {
                FeatureResolvedDeviceCompatible::new(
                    intent.id.clone(),
                    compatible,
                    FeatureResolvedDeviceCompatibleKind::PreserveDeviceCompatibleRoot,
                )
            }));
        }
    }
    compatibles
}

fn sorted_compatibles_for_kind(
    compatibles: &[FeatureResolvedDeviceCompatible],
    matches_kind: impl Fn(FeatureResolvedDeviceCompatibleKind) -> bool,
) -> Vec<DeviceCompatible> {
    let mut compatibles = compatibles
        .iter()
        .filter(|compatible| matches_kind(compatible.kind()))
        .map(|compatible| compatible.compatible().clone())
        .collect::<Vec<_>>();
    compatibles.sort();
    compatibles.dedup();
    compatibles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_device_compatible_resolution_resolves_roots_to_device_compatibles() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                device_compatibles: vec![
                    String::from("qcom,ipq8064"),
                    String::from("qcom,ipq8064"),
                ],
                remove_device_compatibles: vec![String::from("vendor,removed-device")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                device_compatibles: vec![String::from("brcm,bcm2835-aux-uart")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let resolution = FeatureDeviceCompatibleResolution::from_profile(&profile).unwrap();

        assert_eq!(resolution.compatible_count(), 3);
        assert!(!resolution.is_empty());
        assert_eq!(
            resolution
                .compatibles()
                .iter()
                .map(FeatureResolvedDeviceCompatible::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicit_remove_device_compatible:bluetooth:vendor,removed-device",
                "preserve_device_compatible_root:netfilter:brcm,bcm2835-aux-uart",
                "remove_device_compatible_root:bluetooth:qcom,ipq8064",
            ]
        );
        assert_eq!(
            resolution
                .remove_device_compatibles()
                .iter()
                .map(DeviceCompatible::as_str)
                .collect::<Vec<_>>(),
            vec!["qcom,ipq8064", "vendor,removed-device"]
        );
        assert_eq!(
            resolution
                .preserve_device_compatibles()
                .iter()
                .map(DeviceCompatible::as_str)
                .collect::<Vec<_>>(),
            vec!["brcm,bcm2835-aux-uart"]
        );
    }

    #[test]
    fn feature_device_compatible_resolution_emits_compatible_ownerships() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                device_compatibles: vec![String::from("qcom,ipq8064")],
                remove_device_compatibles: vec![String::from("vendor,removed-device")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                device_compatibles: vec![String::from("brcm,bcm2835-aux-uart")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let ownerships = FeatureDeviceCompatibleResolution::from_profile(&profile)
            .unwrap()
            .ownerships()
            .unwrap();

        assert_eq!(
            ownerships
                .iter()
                .map(FeatureOwnership::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "owned_solely_by_removed_feature:bluetooth:device_compatible:qcom,ipq8064",
                "owned_solely_by_removed_feature:bluetooth:device_compatible:vendor,removed-device",
                "shared_with_live_feature:netfilter:device_compatible:brcm,bcm2835-aux-uart",
            ]
        );
    }

    #[test]
    fn feature_device_compatible_resolution_rejects_invalid_compatible() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bad"),
            FeatureIntentConfig {
                device_compatibles: vec![String::from("simple-bus")],
                ..FeatureIntentConfig::default()
            },
        );

        let err = FeatureDeviceCompatibleResolution::from_profile(&profile)
            .unwrap_err()
            .to_string();

        assert!(err.contains("device compatible must use vendor,device form"));
    }
}
