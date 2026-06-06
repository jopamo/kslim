use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureResolvedSourceFileKind {
    RemoveRoot,
    ExplicitRemovePath,
    PreserveRoot,
}

#[allow(dead_code)]
impl FeatureResolvedSourceFileKind {
    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::RemoveRoot => "remove_root_source_file",
            Self::ExplicitRemovePath => "explicit_remove_source_file",
            Self::PreserveRoot => "preserve_root_source_file",
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
pub(crate) struct FeatureResolvedSourceFile {
    feature: FeatureId,
    source: SourceFilePath,
    kind: FeatureResolvedSourceFileKind,
}

#[allow(dead_code)]
impl FeatureResolvedSourceFile {
    pub(crate) fn new(
        feature: FeatureId,
        source: SourceFilePath,
        kind: FeatureResolvedSourceFileKind,
    ) -> Self {
        Self {
            feature,
            source,
            kind,
        }
    }

    pub(crate) fn feature(&self) -> &FeatureId {
        &self.feature
    }

    pub(crate) fn source(&self) -> &SourceFilePath {
        &self.source
    }

    pub(crate) fn kind(&self) -> FeatureResolvedSourceFileKind {
        self.kind
    }

    pub(crate) fn stable_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.kind.stable_name(),
            self.feature.as_str(),
            self.source.as_str()
        )
    }

    pub(crate) fn ownership(&self) -> Result<FeatureOwnership> {
        Ok(FeatureOwnership::new(
            self.kind.ownership_kind(),
            self.feature.clone(),
            FeatureOwnershipSubject::new(format!("source:{}", self.source.as_str()))?,
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FeatureSourceResolution {
    sources: Vec<FeatureResolvedSourceFile>,
}

#[allow(dead_code)]
impl FeatureSourceResolution {
    pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self> {
        let graph = FeatureGraph::from_profile(profile)?;
        Self::from_graph(&graph)
    }

    pub(crate) fn from_graph(graph: &FeatureGraph) -> Result<Self> {
        let mut sources = Vec::new();
        for node in graph.nodes() {
            sources.extend(sources_from_intent(node.intent())?);
        }
        Ok(Self::new(sources))
    }

    pub(crate) fn new(sources: impl IntoIterator<Item = FeatureResolvedSourceFile>) -> Self {
        let mut sources = sources.into_iter().collect::<Vec<_>>();
        sources.sort_by_key(|source| source.stable_key());
        sources.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Self { sources }
    }

    pub(crate) fn sources(&self) -> &[FeatureResolvedSourceFile] {
        &self.sources
    }

    pub(crate) fn source_count(&self) -> usize {
        self.sources.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    pub(crate) fn remove_source_files(&self) -> Vec<SourceFilePath> {
        sorted_sources_for_kind(&self.sources, FeatureResolvedSourceFileKind::is_removal)
    }

    pub(crate) fn preserve_source_files(&self) -> Vec<SourceFilePath> {
        sorted_sources_for_kind(
            &self.sources,
            FeatureResolvedSourceFileKind::is_preservation,
        )
    }

    pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>> {
        let mut ownerships = self
            .sources
            .iter()
            .map(FeatureResolvedSourceFile::ownership)
            .collect::<Result<Vec<_>>>()?;
        ownerships.sort_by_key(|ownership| ownership.stable_key());
        ownerships.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Ok(ownerships)
    }
}

fn sources_from_intent(intent: &FeatureIntent) -> Result<Vec<FeatureResolvedSourceFile>> {
    let mut sources = Vec::new();
    match intent.action {
        FeatureIntentAction::Remove => {
            for root in &intent.roots {
                sources.extend(source_file_from_path(
                    intent.id.clone(),
                    root.as_relative_kernel_path(),
                    FeatureResolvedSourceFileKind::RemoveRoot,
                )?);
            }
            for path in &intent.remove_paths {
                sources.extend(source_file_from_path(
                    intent.id.clone(),
                    path,
                    FeatureResolvedSourceFileKind::ExplicitRemovePath,
                )?);
            }
        }
        FeatureIntentAction::Preserve => {
            for root in &intent.roots {
                sources.extend(source_file_from_path(
                    intent.id.clone(),
                    root.as_relative_kernel_path(),
                    FeatureResolvedSourceFileKind::PreserveRoot,
                )?);
            }
        }
    }
    Ok(sources)
}

fn source_file_from_path(
    feature: FeatureId,
    path: &RelativeKernelPath,
    kind: FeatureResolvedSourceFileKind,
) -> Result<Vec<FeatureResolvedSourceFile>> {
    if !is_source_file_path(path.as_path()) {
        return Ok(Vec::new());
    }
    Ok(vec![FeatureResolvedSourceFile::new(
        feature,
        SourceFilePath::new(path.as_path().to_path_buf())?,
        kind,
    )])
}

fn is_source_file_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("c" | "S" | "rs")
    )
}

fn sorted_sources_for_kind(
    sources: &[FeatureResolvedSourceFile],
    matches_kind: impl Fn(FeatureResolvedSourceFileKind) -> bool,
) -> Vec<SourceFilePath> {
    let mut sources = sources
        .iter()
        .filter(|source| matches_kind(source.kind()))
        .map(|source| source.source().clone())
        .collect::<Vec<_>>();
    sources.sort();
    sources.dedup();
    sources
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_source_resolution_resolves_roots_to_source_files() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                roots: vec![
                    String::from("drivers/bluetooth/btusb.c"),
                    String::from("net/bluetooth"),
                ],
                remove_paths: vec![
                    String::from("drivers/bluetooth/btrtl.c"),
                    String::from("drivers/bluetooth/Kconfig"),
                    String::from("drivers/bluetooth/private.h"),
                ],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                roots: vec![
                    String::from("net/netfilter/nf_conntrack_core.c"),
                    String::from("net/netfilter"),
                ],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let resolution = FeatureSourceResolution::from_profile(&profile).unwrap();

        assert_eq!(resolution.source_count(), 3);
        assert!(!resolution.is_empty());
        assert_eq!(
            resolution
                .sources()
                .iter()
                .map(FeatureResolvedSourceFile::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicit_remove_source_file:bluetooth:drivers/bluetooth/btrtl.c",
                "preserve_root_source_file:netfilter:net/netfilter/nf_conntrack_core.c",
                "remove_root_source_file:bluetooth:drivers/bluetooth/btusb.c",
            ]
        );
        assert_eq!(
            resolution
                .remove_source_files()
                .iter()
                .map(SourceFilePath::as_str)
                .collect::<Vec<_>>(),
            vec!["drivers/bluetooth/btrtl.c", "drivers/bluetooth/btusb.c"]
        );
        assert_eq!(
            resolution
                .preserve_source_files()
                .iter()
                .map(SourceFilePath::as_str)
                .collect::<Vec<_>>(),
            vec!["net/netfilter/nf_conntrack_core.c"]
        );
    }

    #[test]
    fn feature_source_resolution_emits_source_file_ownerships() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                roots: vec![String::from("drivers/bluetooth/btusb.c")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                roots: vec![String::from("net/netfilter/nf_conntrack_core.c")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let ownerships = FeatureSourceResolution::from_profile(&profile)
            .unwrap()
            .ownerships()
            .unwrap();

        assert_eq!(
            ownerships
                .iter()
                .map(FeatureOwnership::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicitly_preserved:netfilter:source:net/netfilter/nf_conntrack_core.c",
                "explicitly_removed:bluetooth:source:drivers/bluetooth/btusb.c",
            ]
        );
    }

    #[test]
    fn feature_source_resolution_rejects_invalid_source_file_path() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bad"),
            FeatureIntentConfig {
                roots: vec![String::from("$(obj)/remove.c")],
                configs: vec![String::from("BAD")],
                ..FeatureIntentConfig::default()
            },
        );

        let err = FeatureSourceResolution::from_profile(&profile)
            .unwrap_err()
            .to_string();

        assert!(err.contains("source file path contains unsupported syntax"));
    }
}
