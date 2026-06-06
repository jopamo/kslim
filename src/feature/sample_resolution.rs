use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureResolvedSampleKind {
    RemoveSampleRoot,
    ExplicitRemoveSample,
    PreserveSampleRoot,
}

#[allow(dead_code)]
impl FeatureResolvedSampleKind {
    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::RemoveSampleRoot => "remove_sample_root",
            Self::ExplicitRemoveSample => "explicit_remove_sample",
            Self::PreserveSampleRoot => "preserve_sample_root",
        }
    }

    pub(crate) const fn ownership_kind(self) -> FeatureOwnershipKind {
        match self {
            Self::RemoveSampleRoot | Self::ExplicitRemoveSample => {
                FeatureOwnershipKind::OwnedSolelyByRemovedFeature
            }
            Self::PreserveSampleRoot => FeatureOwnershipKind::SharedWithLiveFeature,
        }
    }

    pub(crate) const fn is_removal(self) -> bool {
        matches!(self, Self::RemoveSampleRoot | Self::ExplicitRemoveSample)
    }

    pub(crate) const fn is_preservation(self) -> bool {
        matches!(self, Self::PreserveSampleRoot)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureResolvedSample {
    feature: FeatureId,
    path: SamplePath,
    kind: FeatureResolvedSampleKind,
}

#[allow(dead_code)]
impl FeatureResolvedSample {
    pub(crate) fn new(
        feature: FeatureId,
        path: SamplePath,
        kind: FeatureResolvedSampleKind,
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

    pub(crate) fn path(&self) -> &SamplePath {
        &self.path
    }

    pub(crate) fn kind(&self) -> FeatureResolvedSampleKind {
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
            FeatureOwnershipSubject::new(format!("sample_path:{}", self.path.as_str()))?,
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FeatureSampleResolution {
    paths: Vec<FeatureResolvedSample>,
}

#[allow(dead_code)]
impl FeatureSampleResolution {
    pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self> {
        let graph = FeatureGraph::from_profile(profile)?;
        Ok(Self::from_graph(&graph))
    }

    pub(crate) fn from_graph(graph: &FeatureGraph) -> Self {
        let mut paths = Vec::new();
        for node in graph.nodes() {
            paths.extend(sample_paths_from_intent(node.intent()));
        }
        Self::new(paths)
    }

    pub(crate) fn new(paths: impl IntoIterator<Item = FeatureResolvedSample>) -> Self {
        let mut paths = paths.into_iter().collect::<Vec<_>>();
        paths.sort_by_key(|path| path.stable_key());
        paths.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Self { paths }
    }

    pub(crate) fn paths(&self) -> &[FeatureResolvedSample] {
        &self.paths
    }

    pub(crate) fn path_count(&self) -> usize {
        self.paths.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub(crate) fn remove_samples(&self) -> Vec<SamplePath> {
        sorted_paths_for_kind(&self.paths, FeatureResolvedSampleKind::is_removal)
    }

    pub(crate) fn preserve_samples(&self) -> Vec<SamplePath> {
        sorted_paths_for_kind(&self.paths, FeatureResolvedSampleKind::is_preservation)
    }

    pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>> {
        let mut ownerships = self
            .paths
            .iter()
            .map(FeatureResolvedSample::ownership)
            .collect::<Result<Vec<_>>>()?;
        ownerships.sort_by_key(|ownership| ownership.stable_key());
        ownerships.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Ok(ownerships)
    }
}

fn sample_paths_from_intent(intent: &FeatureIntent) -> Vec<FeatureResolvedSample> {
    let mut paths = Vec::new();
    match intent.action {
        FeatureIntentAction::Remove => {
            paths.extend(intent.samples.iter().cloned().map(|path| {
                FeatureResolvedSample::new(
                    intent.id.clone(),
                    path,
                    FeatureResolvedSampleKind::RemoveSampleRoot,
                )
            }));
            let explicit_paths = intent.remove_samples.iter().cloned();
            paths.extend(explicit_paths.map(|path| {
                FeatureResolvedSample::new(
                    intent.id.clone(),
                    path,
                    FeatureResolvedSampleKind::ExplicitRemoveSample,
                )
            }));
        }
        FeatureIntentAction::Preserve => {
            paths.extend(intent.samples.iter().cloned().map(|path| {
                FeatureResolvedSample::new(
                    intent.id.clone(),
                    path,
                    FeatureResolvedSampleKind::PreserveSampleRoot,
                )
            }));
        }
    }
    paths
}

fn sorted_paths_for_kind(
    paths: &[FeatureResolvedSample],
    matches_kind: impl Fn(FeatureResolvedSampleKind) -> bool,
) -> Vec<SamplePath> {
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
    fn feature_sample_resolution_resolves_roots_to_samples() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                samples: vec![String::from("samples/bpf"), String::from("samples/bpf")],
                remove_samples: vec![String::from("samples/hidraw")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                samples: vec![String::from("samples/kobject")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let resolution = FeatureSampleResolution::from_profile(&profile).unwrap();

        assert_eq!(resolution.path_count(), 3);
        assert!(!resolution.is_empty());
        assert_eq!(
            resolution
                .paths()
                .iter()
                .map(FeatureResolvedSample::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicit_remove_sample:bluetooth:samples/hidraw",
                "preserve_sample_root:netfilter:samples/kobject",
                "remove_sample_root:bluetooth:samples/bpf",
            ]
        );
        assert_eq!(
            resolution
                .remove_samples()
                .iter()
                .map(SamplePath::as_str)
                .collect::<Vec<_>>(),
            vec!["samples/bpf", "samples/hidraw"]
        );
        assert_eq!(
            resolution
                .preserve_samples()
                .iter()
                .map(SamplePath::as_str)
                .collect::<Vec<_>>(),
            vec!["samples/kobject"]
        );
    }

    #[test]
    fn feature_sample_resolution_emits_sample_ownerships() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                samples: vec![String::from("samples/bpf")],
                remove_samples: vec![String::from("samples/hidraw")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                samples: vec![String::from("samples/kobject")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let ownerships = FeatureSampleResolution::from_profile(&profile)
            .unwrap()
            .ownerships()
            .unwrap();

        assert_eq!(
            ownerships
                .iter()
                .map(FeatureOwnership::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "owned_solely_by_removed_feature:bluetooth:sample_path:samples/bpf",
                "owned_solely_by_removed_feature:bluetooth:sample_path:samples/hidraw",
                "shared_with_live_feature:netfilter:sample_path:samples/kobject",
            ]
        );
    }

    #[test]
    fn feature_sample_resolution_rejects_invalid_sample_path() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bad"),
            FeatureIntentConfig {
                samples: vec![String::from("tools/perf")],
                ..FeatureIntentConfig::default()
            },
        );

        let err = FeatureSampleResolution::from_profile(&profile)
            .unwrap_err()
            .to_string();

        assert!(err.contains("sample path must be under samples"));
    }
}
