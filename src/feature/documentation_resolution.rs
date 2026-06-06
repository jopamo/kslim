use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureResolvedDocumentationKind {
    RemoveDocumentationRoot,
    ExplicitRemoveDocumentation,
    PreserveDocumentationRoot,
}

#[allow(dead_code)]
impl FeatureResolvedDocumentationKind {
    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::RemoveDocumentationRoot => "remove_documentation_root",
            Self::ExplicitRemoveDocumentation => "explicit_remove_documentation",
            Self::PreserveDocumentationRoot => "preserve_documentation_root",
        }
    }

    pub(crate) const fn ownership_kind(self) -> FeatureOwnershipKind {
        match self {
            Self::RemoveDocumentationRoot | Self::ExplicitRemoveDocumentation => {
                FeatureOwnershipKind::OwnedSolelyByRemovedFeature
            }
            Self::PreserveDocumentationRoot => FeatureOwnershipKind::SharedWithLiveFeature,
        }
    }

    pub(crate) const fn is_removal(self) -> bool {
        matches!(
            self,
            Self::RemoveDocumentationRoot | Self::ExplicitRemoveDocumentation
        )
    }

    pub(crate) const fn is_preservation(self) -> bool {
        matches!(self, Self::PreserveDocumentationRoot)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureResolvedDocumentation {
    feature: FeatureId,
    path: DocumentationPath,
    kind: FeatureResolvedDocumentationKind,
}

#[allow(dead_code)]
impl FeatureResolvedDocumentation {
    pub(crate) fn new(
        feature: FeatureId,
        path: DocumentationPath,
        kind: FeatureResolvedDocumentationKind,
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

    pub(crate) fn path(&self) -> &DocumentationPath {
        &self.path
    }

    pub(crate) fn kind(&self) -> FeatureResolvedDocumentationKind {
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
            FeatureOwnershipSubject::new(format!("documentation_path:{}", self.path.as_str()))?,
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FeatureDocumentationResolution {
    paths: Vec<FeatureResolvedDocumentation>,
}

#[allow(dead_code)]
impl FeatureDocumentationResolution {
    pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self> {
        let graph = FeatureGraph::from_profile(profile)?;
        Ok(Self::from_graph(&graph))
    }

    pub(crate) fn from_graph(graph: &FeatureGraph) -> Self {
        let mut paths = Vec::new();
        for node in graph.nodes() {
            paths.extend(documentation_paths_from_intent(node.intent()));
        }
        Self::new(paths)
    }

    pub(crate) fn new(paths: impl IntoIterator<Item = FeatureResolvedDocumentation>) -> Self {
        let mut paths = paths.into_iter().collect::<Vec<_>>();
        paths.sort_by_key(|path| path.stable_key());
        paths.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Self { paths }
    }

    pub(crate) fn paths(&self) -> &[FeatureResolvedDocumentation] {
        &self.paths
    }

    pub(crate) fn path_count(&self) -> usize {
        self.paths.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub(crate) fn remove_docs(&self) -> Vec<DocumentationPath> {
        sorted_paths_for_kind(&self.paths, FeatureResolvedDocumentationKind::is_removal)
    }

    pub(crate) fn preserve_docs(&self) -> Vec<DocumentationPath> {
        sorted_paths_for_kind(
            &self.paths,
            FeatureResolvedDocumentationKind::is_preservation,
        )
    }

    pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>> {
        let mut ownerships = self
            .paths
            .iter()
            .map(FeatureResolvedDocumentation::ownership)
            .collect::<Result<Vec<_>>>()?;
        ownerships.sort_by_key(|ownership| ownership.stable_key());
        ownerships.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Ok(ownerships)
    }
}

fn documentation_paths_from_intent(intent: &FeatureIntent) -> Vec<FeatureResolvedDocumentation> {
    let mut paths = Vec::new();
    match intent.action {
        FeatureIntentAction::Remove => {
            paths.extend(intent.docs.iter().cloned().map(|path| {
                FeatureResolvedDocumentation::new(
                    intent.id.clone(),
                    path,
                    FeatureResolvedDocumentationKind::RemoveDocumentationRoot,
                )
            }));
            let explicit_paths = intent.remove_docs.iter().cloned();
            paths.extend(explicit_paths.map(|path| {
                FeatureResolvedDocumentation::new(
                    intent.id.clone(),
                    path,
                    FeatureResolvedDocumentationKind::ExplicitRemoveDocumentation,
                )
            }));
        }
        FeatureIntentAction::Preserve => {
            paths.extend(intent.docs.iter().cloned().map(|path| {
                FeatureResolvedDocumentation::new(
                    intent.id.clone(),
                    path,
                    FeatureResolvedDocumentationKind::PreserveDocumentationRoot,
                )
            }));
        }
    }
    paths
}

fn sorted_paths_for_kind(
    paths: &[FeatureResolvedDocumentation],
    matches_kind: impl Fn(FeatureResolvedDocumentationKind) -> bool,
) -> Vec<DocumentationPath> {
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
    fn feature_documentation_resolution_resolves_roots_to_docs() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                docs: vec![
                    String::from("Documentation/networking/bluetooth.rst"),
                    String::from("Documentation/networking/bluetooth.rst"),
                ],
                remove_docs: vec![String::from("Documentation/driver-api/btusb.rst")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                docs: vec![String::from(
                    "Documentation/networking/nf_conntrack-sysctl.rst",
                )],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let resolution = FeatureDocumentationResolution::from_profile(&profile).unwrap();

        assert_eq!(resolution.path_count(), 3);
        assert!(!resolution.is_empty());
        assert_eq!(
            resolution
                .paths()
                .iter()
                .map(FeatureResolvedDocumentation::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicit_remove_documentation:bluetooth:Documentation/driver-api/btusb.rst",
                "preserve_documentation_root:netfilter:Documentation/networking/nf_conntrack-sysctl.rst",
                "remove_documentation_root:bluetooth:Documentation/networking/bluetooth.rst",
            ]
        );
        assert_eq!(
            resolution
                .remove_docs()
                .iter()
                .map(DocumentationPath::as_str)
                .collect::<Vec<_>>(),
            vec![
                "Documentation/driver-api/btusb.rst",
                "Documentation/networking/bluetooth.rst",
            ]
        );
        assert_eq!(
            resolution
                .preserve_docs()
                .iter()
                .map(DocumentationPath::as_str)
                .collect::<Vec<_>>(),
            vec!["Documentation/networking/nf_conntrack-sysctl.rst"]
        );
    }

    #[test]
    fn feature_documentation_resolution_emits_documentation_ownerships() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                docs: vec![String::from("Documentation/networking/bluetooth.rst")],
                remove_docs: vec![String::from("Documentation/driver-api/btusb.rst")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                docs: vec![String::from(
                    "Documentation/networking/nf_conntrack-sysctl.rst",
                )],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let ownerships = FeatureDocumentationResolution::from_profile(&profile)
            .unwrap()
            .ownerships()
            .unwrap();

        assert_eq!(
            ownerships
                .iter()
                .map(FeatureOwnership::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "owned_solely_by_removed_feature:bluetooth:documentation_path:Documentation/driver-api/btusb.rst",
                "owned_solely_by_removed_feature:bluetooth:documentation_path:Documentation/networking/bluetooth.rst",
                "shared_with_live_feature:netfilter:documentation_path:Documentation/networking/nf_conntrack-sysctl.rst",
            ]
        );
    }

    #[test]
    fn feature_documentation_resolution_rejects_invalid_documentation_path() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bad"),
            FeatureIntentConfig {
                docs: vec![String::from("drivers/foo/README.rst")],
                ..FeatureIntentConfig::default()
            },
        );

        let err = FeatureDocumentationResolution::from_profile(&profile)
            .unwrap_err()
            .to_string();

        assert!(err.contains("documentation path must be under Documentation"));
    }
}
