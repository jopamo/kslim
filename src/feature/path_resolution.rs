use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureResolvedPathKind {
    RemoveRoot,
    ExplicitRemovePath,
    PreserveRoot,
}

#[allow(dead_code)]
impl FeatureResolvedPathKind {
    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::RemoveRoot => "remove_root",
            Self::ExplicitRemovePath => "explicit_remove_path",
            Self::PreserveRoot => "preserve_root",
        }
    }

    pub(crate) const fn ownership_kind(self) -> FeatureOwnershipKind {
        match self {
            Self::RemoveRoot | Self::ExplicitRemovePath => FeatureOwnershipKind::ExplicitlyRemoved,
            Self::PreserveRoot => FeatureOwnershipKind::ExplicitlyPreserved,
        }
    }

    pub(crate) const fn is_removal(self) -> bool {
        matches!(self, Self::RemoveRoot | Self::ExplicitRemovePath)
    }

    pub(crate) const fn is_preservation(self) -> bool {
        matches!(self, Self::PreserveRoot)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureResolvedPath {
    feature: FeatureId,
    path: RelativeKernelPath,
    kind: FeatureResolvedPathKind,
}

#[allow(dead_code)]
impl FeatureResolvedPath {
    pub(crate) fn new(
        feature: FeatureId,
        path: RelativeKernelPath,
        kind: FeatureResolvedPathKind,
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

    pub(crate) fn path(&self) -> &RelativeKernelPath {
        &self.path
    }

    pub(crate) fn kind(&self) -> FeatureResolvedPathKind {
        self.kind
    }

    pub(crate) fn stable_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.kind.stable_name(),
            self.feature.as_str(),
            self.path.as_path().to_string_lossy()
        )
    }

    pub(crate) fn ownership(&self) -> Result<FeatureOwnership> {
        Ok(FeatureOwnership::new(
            self.kind.ownership_kind(),
            self.feature.clone(),
            FeatureOwnershipSubject::new(format!(
                "path:{}",
                self.path.as_path().to_string_lossy()
            ))?,
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FeaturePathResolution {
    paths: Vec<FeatureResolvedPath>,
}

#[allow(dead_code)]
impl FeaturePathResolution {
    pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self> {
        let graph = FeatureGraph::from_profile(profile)?;
        Ok(Self::from_graph(&graph))
    }

    pub(crate) fn from_graph(graph: &FeatureGraph) -> Self {
        let mut paths = Vec::new();
        for node in graph.nodes() {
            paths.extend(paths_from_intent(node.intent()));
        }
        Self::new(paths)
    }

    pub(crate) fn new(paths: impl IntoIterator<Item = FeatureResolvedPath>) -> Self {
        let mut paths = paths.into_iter().collect::<Vec<_>>();
        paths.sort_by(|left, right| left.stable_key().cmp(&right.stable_key()));
        paths.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Self { paths }
    }

    pub(crate) fn paths(&self) -> &[FeatureResolvedPath] {
        &self.paths
    }

    pub(crate) fn path_count(&self) -> usize {
        self.paths.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub(crate) fn remove_paths(&self) -> Vec<RelativeKernelPath> {
        sorted_paths_for_kind(&self.paths, FeatureResolvedPathKind::is_removal)
    }

    pub(crate) fn preserve_paths(&self) -> Vec<RelativeKernelPath> {
        sorted_paths_for_kind(&self.paths, FeatureResolvedPathKind::is_preservation)
    }

    pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>> {
        let mut ownerships = self
            .paths
            .iter()
            .map(FeatureResolvedPath::ownership)
            .collect::<Result<Vec<_>>>()?;
        ownerships.sort_by(|left, right| left.stable_key().cmp(&right.stable_key()));
        ownerships.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Ok(ownerships)
    }
}

fn paths_from_intent(intent: &FeatureIntent) -> Vec<FeatureResolvedPath> {
    let mut paths = Vec::new();
    match intent.action {
        FeatureIntentAction::Remove => {
            paths.extend(intent.roots.iter().map(|root| {
                FeatureResolvedPath::new(
                    intent.id.clone(),
                    root.as_relative_kernel_path().clone(),
                    FeatureResolvedPathKind::RemoveRoot,
                )
            }));
            paths.extend(intent.remove_paths.iter().cloned().map(|path| {
                FeatureResolvedPath::new(
                    intent.id.clone(),
                    path,
                    FeatureResolvedPathKind::ExplicitRemovePath,
                )
            }));
        }
        FeatureIntentAction::Preserve => {
            paths.extend(intent.roots.iter().map(|root| {
                FeatureResolvedPath::new(
                    intent.id.clone(),
                    root.as_relative_kernel_path().clone(),
                    FeatureResolvedPathKind::PreserveRoot,
                )
            }));
        }
    }
    paths
}

fn sorted_paths_for_kind(
    paths: &[FeatureResolvedPath],
    matches_kind: impl Fn(FeatureResolvedPathKind) -> bool,
) -> Vec<RelativeKernelPath> {
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
    fn feature_path_resolution_resolves_roots_to_paths() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                roots: vec![
                    String::from("net/bluetooth"),
                    String::from("drivers/bluetooth"),
                ],
                remove_paths: vec![String::from("drivers/bluetooth/btusb.c")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                roots: vec![String::from("net/netfilter")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let resolution = FeaturePathResolution::from_profile(&profile).unwrap();

        assert_eq!(resolution.path_count(), 4);
        assert!(!resolution.is_empty());
        assert_eq!(
            resolution
                .paths()
                .iter()
                .map(FeatureResolvedPath::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicit_remove_path:bluetooth:drivers/bluetooth/btusb.c",
                "preserve_root:netfilter:net/netfilter",
                "remove_root:bluetooth:drivers/bluetooth",
                "remove_root:bluetooth:net/bluetooth",
            ]
        );
        assert_eq!(
            resolution
                .remove_paths()
                .iter()
                .map(|path| path.as_path().to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            vec![
                "drivers/bluetooth",
                "drivers/bluetooth/btusb.c",
                "net/bluetooth",
            ]
        );
        assert_eq!(
            resolution
                .preserve_paths()
                .iter()
                .map(|path| path.as_path().to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            vec!["net/netfilter"]
        );
    }

    #[test]
    fn feature_path_resolution_emits_path_ownerships() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                roots: vec![String::from("drivers/bluetooth")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                roots: vec![String::from("net/netfilter")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let ownerships = FeaturePathResolution::from_profile(&profile)
            .unwrap()
            .ownerships()
            .unwrap();

        assert_eq!(
            ownerships
                .iter()
                .map(FeatureOwnership::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicitly_preserved:netfilter:path:net/netfilter",
                "explicitly_removed:bluetooth:path:drivers/bluetooth",
            ]
        );
    }

    #[test]
    fn feature_path_resolution_rejects_invalid_feature_intent() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from(" "),
            FeatureIntentConfig {
                roots: vec![String::from("drivers/bluetooth")],
                ..FeatureIntentConfig::default()
            },
        );

        let err = FeaturePathResolution::from_profile(&profile).unwrap_err();

        assert!(format!("{err:#}").contains("feature name must not be empty"));
    }
}
