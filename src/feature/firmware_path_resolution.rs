use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureResolvedFirmwarePathKind {
    RemoveFirmwarePathRoot,
    ExplicitRemoveFirmwarePath,
    PreserveFirmwarePathRoot,
}

#[allow(dead_code)]
impl FeatureResolvedFirmwarePathKind {
    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::RemoveFirmwarePathRoot => "remove_firmware_path_root",
            Self::ExplicitRemoveFirmwarePath => "explicit_remove_firmware_path",
            Self::PreserveFirmwarePathRoot => "preserve_firmware_path_root",
        }
    }

    pub(crate) const fn ownership_kind(self) -> FeatureOwnershipKind {
        match self {
            Self::RemoveFirmwarePathRoot | Self::ExplicitRemoveFirmwarePath => {
                FeatureOwnershipKind::OwnedSolelyByRemovedFeature
            }
            Self::PreserveFirmwarePathRoot => FeatureOwnershipKind::SharedWithLiveFeature,
        }
    }

    pub(crate) const fn is_removal(self) -> bool {
        matches!(
            self,
            Self::RemoveFirmwarePathRoot | Self::ExplicitRemoveFirmwarePath
        )
    }

    pub(crate) const fn is_preservation(self) -> bool {
        matches!(self, Self::PreserveFirmwarePathRoot)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureResolvedFirmwarePath {
    feature: FeatureId,
    path: FirmwarePath,
    kind: FeatureResolvedFirmwarePathKind,
}

#[allow(dead_code)]
impl FeatureResolvedFirmwarePath {
    pub(crate) fn new(
        feature: FeatureId,
        path: FirmwarePath,
        kind: FeatureResolvedFirmwarePathKind,
    ) -> Self {
        Self {
            feature,
            path,
            kind,
        }
    }

    pub(crate) fn feature(&self) -> &FeatureId {
        &self.feature
    }

    pub(crate) fn path(&self) -> &FirmwarePath {
        &self.path
    }

    pub(crate) fn kind(&self) -> FeatureResolvedFirmwarePathKind {
        self.kind
    }

    pub(crate) fn stable_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.kind.stable_name(),
            self.feature.as_str(),
            self.path.as_str()
        )
    }

    pub(crate) fn ownership(&self) -> Result<FeatureOwnership> {
        Ok(FeatureOwnership::new(
            self.kind.ownership_kind(),
            self.feature.clone(),
            FeatureOwnershipSubject::new(format!("firmware_path:{}", self.path.as_str()))?,
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FeatureFirmwarePathResolution {
    paths: Vec<FeatureResolvedFirmwarePath>,
}

#[allow(dead_code)]
impl FeatureFirmwarePathResolution {
    pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self> {
        let graph = FeatureGraph::from_profile(profile)?;
        Ok(Self::from_graph(&graph))
    }

    pub(crate) fn from_graph(graph: &FeatureGraph) -> Self {
        let mut paths = Vec::new();
        for node in graph.nodes() {
            paths.extend(firmware_paths_from_intent(node.intent()));
        }
        Self::new(paths)
    }

    pub(crate) fn new(paths: impl IntoIterator<Item = FeatureResolvedFirmwarePath>) -> Self {
        let mut paths = paths.into_iter().collect::<Vec<_>>();
        paths.sort_by_key(|path| path.stable_key());
        paths.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Self { paths }
    }

    pub(crate) fn paths(&self) -> &[FeatureResolvedFirmwarePath] {
        &self.paths
    }

    pub(crate) fn path_count(&self) -> usize {
        self.paths.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub(crate) fn remove_firmware_paths(&self) -> Vec<FirmwarePath> {
        sorted_paths_for_kind(&self.paths, FeatureResolvedFirmwarePathKind::is_removal)
    }

    pub(crate) fn preserve_firmware_paths(&self) -> Vec<FirmwarePath> {
        sorted_paths_for_kind(
            &self.paths,
            FeatureResolvedFirmwarePathKind::is_preservation,
        )
    }

    pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>> {
        let mut ownerships = self
            .paths
            .iter()
            .map(FeatureResolvedFirmwarePath::ownership)
            .collect::<Result<Vec<_>>>()?;
        ownerships.sort_by_key(|ownership| ownership.stable_key());
        ownerships.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Ok(ownerships)
    }
}

fn firmware_paths_from_intent(intent: &FeatureIntent) -> Vec<FeatureResolvedFirmwarePath> {
    let mut paths = Vec::new();
    match intent.action {
        FeatureIntentAction::Remove => {
            paths.extend(intent.firmware_paths.iter().cloned().map(|path| {
                FeatureResolvedFirmwarePath::new(
                    intent.id.clone(),
                    path,
                    FeatureResolvedFirmwarePathKind::RemoveFirmwarePathRoot,
                )
            }));
            let explicit_paths = intent.remove_firmware_paths.iter().cloned();
            paths.extend(explicit_paths.map(|path| {
                FeatureResolvedFirmwarePath::new(
                    intent.id.clone(),
                    path,
                    FeatureResolvedFirmwarePathKind::ExplicitRemoveFirmwarePath,
                )
            }));
        }
        FeatureIntentAction::Preserve => {
            paths.extend(intent.firmware_paths.iter().cloned().map(|path| {
                FeatureResolvedFirmwarePath::new(
                    intent.id.clone(),
                    path,
                    FeatureResolvedFirmwarePathKind::PreserveFirmwarePathRoot,
                )
            }));
        }
    }
    paths
}

fn sorted_paths_for_kind(
    paths: &[FeatureResolvedFirmwarePath],
    matches_kind: impl Fn(FeatureResolvedFirmwarePathKind) -> bool,
) -> Vec<FirmwarePath> {
    let mut paths = paths
        .iter()
        .filter(|path| matches_kind(path.kind()))
        .map(|path| path.path().clone())
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_firmware_path_resolution_resolves_roots_to_firmware_paths() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                firmware_paths: vec![
                    String::from("amdgpu/polaris10_mc.bin"),
                    String::from("amdgpu/polaris10_mc.bin"),
                ],
                remove_firmware_paths: vec![String::from("iwlwifi-7260-17.ucode")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                firmware_paths: vec![String::from("qcom/venus-5.2/venus.mbn")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let resolution = FeatureFirmwarePathResolution::from_profile(&profile).unwrap();

        assert_eq!(resolution.path_count(), 3);
        assert!(!resolution.is_empty());
        assert_eq!(
            resolution
                .paths()
                .iter()
                .map(FeatureResolvedFirmwarePath::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicit_remove_firmware_path:bluetooth:iwlwifi-7260-17.ucode",
                "preserve_firmware_path_root:netfilter:qcom/venus-5.2/venus.mbn",
                "remove_firmware_path_root:bluetooth:amdgpu/polaris10_mc.bin",
            ]
        );
        assert_eq!(
            resolution
                .remove_firmware_paths()
                .iter()
                .map(FirmwarePath::as_str)
                .collect::<Vec<_>>(),
            vec!["amdgpu/polaris10_mc.bin", "iwlwifi-7260-17.ucode"]
        );
        assert_eq!(
            resolution
                .preserve_firmware_paths()
                .iter()
                .map(FirmwarePath::as_str)
                .collect::<Vec<_>>(),
            vec!["qcom/venus-5.2/venus.mbn"]
        );
    }

    #[test]
    fn feature_firmware_path_resolution_emits_firmware_ownerships() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                firmware_paths: vec![String::from("amdgpu/polaris10_mc.bin")],
                remove_firmware_paths: vec![String::from("iwlwifi-7260-17.ucode")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                firmware_paths: vec![String::from("qcom/venus-5.2/venus.mbn")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let ownerships = FeatureFirmwarePathResolution::from_profile(&profile)
            .unwrap()
            .ownerships()
            .unwrap();

        assert_eq!(
            ownerships
                .iter()
                .map(FeatureOwnership::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "owned_solely_by_removed_feature:bluetooth:firmware_path:amdgpu/polaris10_mc.bin",
                "owned_solely_by_removed_feature:bluetooth:firmware_path:iwlwifi-7260-17.ucode",
                "shared_with_live_feature:netfilter:firmware_path:qcom/venus-5.2/venus.mbn",
            ]
        );
    }

    #[test]
    fn feature_firmware_path_resolution_rejects_invalid_firmware_path() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bad"),
            FeatureIntentConfig {
                firmware_paths: vec![String::from("../firmware/foo.bin")],
                ..FeatureIntentConfig::default()
            },
        );

        let err = FeatureFirmwarePathResolution::from_profile(&profile)
            .unwrap_err()
            .to_string();

        assert!(err.contains("must not contain '..'"));
    }
}
