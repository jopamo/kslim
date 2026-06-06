use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureResolvedToolKind {
    RemoveToolRoot,
    ExplicitRemoveTool,
    PreserveToolRoot,
}

#[allow(dead_code)]
impl FeatureResolvedToolKind {
    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::RemoveToolRoot => "remove_tool_root",
            Self::ExplicitRemoveTool => "explicit_remove_tool",
            Self::PreserveToolRoot => "preserve_tool_root",
        }
    }

    pub(crate) const fn ownership_kind(self) -> FeatureOwnershipKind {
        match self {
            Self::RemoveToolRoot | Self::ExplicitRemoveTool => {
                FeatureOwnershipKind::OwnedSolelyByRemovedFeature
            }
            Self::PreserveToolRoot => FeatureOwnershipKind::SharedWithLiveFeature,
        }
    }

    pub(crate) const fn is_removal(self) -> bool {
        matches!(self, Self::RemoveToolRoot | Self::ExplicitRemoveTool)
    }

    pub(crate) const fn is_preservation(self) -> bool {
        matches!(self, Self::PreserveToolRoot)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureResolvedTool {
    feature: FeatureId,
    path: ToolPath,
    kind: FeatureResolvedToolKind,
}

#[allow(dead_code)]
impl FeatureResolvedTool {
    pub(crate) fn new(feature: FeatureId, path: ToolPath, kind: FeatureResolvedToolKind) -> Self {
        Self {
            feature,
            path,
            kind,
        }
    }

    pub(crate) fn feature(&self) -> &FeatureId {
        &self.feature
    }

    pub(crate) fn path(&self) -> &ToolPath {
        &self.path
    }

    pub(crate) fn kind(&self) -> FeatureResolvedToolKind {
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
            FeatureOwnershipSubject::new(format!("tool_path:{}", self.path.as_str()))?,
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FeatureToolResolution {
    paths: Vec<FeatureResolvedTool>,
}

#[allow(dead_code)]
impl FeatureToolResolution {
    pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self> {
        let graph = FeatureGraph::from_profile(profile)?;
        Ok(Self::from_graph(&graph))
    }

    pub(crate) fn from_graph(graph: &FeatureGraph) -> Self {
        let mut paths = Vec::new();
        for node in graph.nodes() {
            paths.extend(tool_paths_from_intent(node.intent()));
        }
        Self::new(paths)
    }

    pub(crate) fn new(paths: impl IntoIterator<Item = FeatureResolvedTool>) -> Self {
        let mut paths = paths.into_iter().collect::<Vec<_>>();
        paths.sort_by_key(|path| path.stable_key());
        paths.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Self { paths }
    }

    pub(crate) fn paths(&self) -> &[FeatureResolvedTool] {
        &self.paths
    }

    pub(crate) fn path_count(&self) -> usize {
        self.paths.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub(crate) fn remove_tools(&self) -> Vec<ToolPath> {
        sorted_paths_for_kind(&self.paths, FeatureResolvedToolKind::is_removal)
    }

    pub(crate) fn preserve_tools(&self) -> Vec<ToolPath> {
        sorted_paths_for_kind(&self.paths, FeatureResolvedToolKind::is_preservation)
    }

    pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>> {
        let mut ownerships = self
            .paths
            .iter()
            .map(FeatureResolvedTool::ownership)
            .collect::<Result<Vec<_>>>()?;
        ownerships.sort_by_key(|ownership| ownership.stable_key());
        ownerships.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Ok(ownerships)
    }
}

fn tool_paths_from_intent(intent: &FeatureIntent) -> Vec<FeatureResolvedTool> {
    let mut paths = Vec::new();
    match intent.action {
        FeatureIntentAction::Remove => {
            paths.extend(intent.tools.iter().cloned().map(|path| {
                FeatureResolvedTool::new(
                    intent.id.clone(),
                    path,
                    FeatureResolvedToolKind::RemoveToolRoot,
                )
            }));
            let explicit_paths = intent.remove_tools.iter().cloned();
            paths.extend(explicit_paths.map(|path| {
                FeatureResolvedTool::new(
                    intent.id.clone(),
                    path,
                    FeatureResolvedToolKind::ExplicitRemoveTool,
                )
            }));
        }
        FeatureIntentAction::Preserve => {
            paths.extend(intent.tools.iter().cloned().map(|path| {
                FeatureResolvedTool::new(
                    intent.id.clone(),
                    path,
                    FeatureResolvedToolKind::PreserveToolRoot,
                )
            }));
        }
    }
    paths
}

fn sorted_paths_for_kind(
    paths: &[FeatureResolvedTool],
    matches_kind: impl Fn(FeatureResolvedToolKind) -> bool,
) -> Vec<ToolPath> {
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
    fn feature_tool_resolution_resolves_roots_to_tools() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                tools: vec![String::from("tools/perf"), String::from("tools/perf")],
                remove_tools: vec![String::from("tools/objtool")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                tools: vec![String::from("tools/testing/selftests/netfilter")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let resolution = FeatureToolResolution::from_profile(&profile).unwrap();

        assert_eq!(resolution.path_count(), 3);
        assert!(!resolution.is_empty());
        assert_eq!(
            resolution
                .paths()
                .iter()
                .map(FeatureResolvedTool::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicit_remove_tool:bluetooth:tools/objtool",
                "preserve_tool_root:netfilter:tools/testing/selftests/netfilter",
                "remove_tool_root:bluetooth:tools/perf",
            ]
        );
        assert_eq!(
            resolution
                .remove_tools()
                .iter()
                .map(ToolPath::as_str)
                .collect::<Vec<_>>(),
            vec!["tools/objtool", "tools/perf"]
        );
        assert_eq!(
            resolution
                .preserve_tools()
                .iter()
                .map(ToolPath::as_str)
                .collect::<Vec<_>>(),
            vec!["tools/testing/selftests/netfilter"]
        );
    }

    #[test]
    fn feature_tool_resolution_emits_tool_ownerships() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                tools: vec![String::from("tools/perf")],
                remove_tools: vec![String::from("tools/objtool")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                tools: vec![String::from("tools/testing/selftests/netfilter")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let ownerships = FeatureToolResolution::from_profile(&profile)
            .unwrap()
            .ownerships()
            .unwrap();

        assert_eq!(
            ownerships
                .iter()
                .map(FeatureOwnership::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "owned_solely_by_removed_feature:bluetooth:tool_path:tools/objtool",
                "owned_solely_by_removed_feature:bluetooth:tool_path:tools/perf",
                "shared_with_live_feature:netfilter:tool_path:tools/testing/selftests/netfilter",
            ]
        );
    }

    #[test]
    fn feature_tool_resolution_rejects_invalid_tool_path() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bad"),
            FeatureIntentConfig {
                tools: vec![String::from("Documentation/tools.rst")],
                ..FeatureIntentConfig::default()
            },
        );

        let err = FeatureToolResolution::from_profile(&profile)
            .unwrap_err()
            .to_string();

        assert!(err.contains("tool path must be under tools"));
    }
}
