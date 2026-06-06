use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureResolvedGeneratedArtifactKind {
    RemoveRoot,
    ExplicitRemovePath,
    PreserveRoot,
}

#[allow(dead_code)]
impl FeatureResolvedGeneratedArtifactKind {
    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::RemoveRoot => "remove_root_generated_artifact",
            Self::ExplicitRemovePath => "explicit_remove_generated_artifact",
            Self::PreserveRoot => "preserve_root_generated_artifact",
        }
    }

    pub(crate) const fn ownership_kind(self) -> FeatureOwnershipKind {
        FeatureOwnershipKind::GeneratedByLiveBuild
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
pub(crate) struct FeatureResolvedGeneratedArtifact {
    feature: FeatureId,
    artifact: GeneratedArtifactPath,
    kind: FeatureResolvedGeneratedArtifactKind,
}

#[allow(dead_code)]
impl FeatureResolvedGeneratedArtifact {
    pub(crate) fn new(
        feature: FeatureId,
        artifact: GeneratedArtifactPath,
        kind: FeatureResolvedGeneratedArtifactKind,
    ) -> Self {
        Self {
            feature,
            artifact,
            kind,
        }
    }

    pub(crate) fn feature(&self) -> &FeatureId {
        &self.feature
    }

    pub(crate) fn artifact(&self) -> &GeneratedArtifactPath {
        &self.artifact
    }

    pub(crate) fn kind(&self) -> FeatureResolvedGeneratedArtifactKind {
        self.kind
    }

    pub(crate) fn stable_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.kind.stable_name(),
            self.feature.as_str(),
            self.artifact.as_str()
        )
    }

    pub(crate) fn ownership(&self) -> Result<FeatureOwnership> {
        Ok(FeatureOwnership::new(
            self.kind.ownership_kind(),
            self.feature.clone(),
            FeatureOwnershipSubject::new(format!("generated_artifact:{}", self.artifact.as_str()))?,
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FeatureGeneratedArtifactResolution {
    artifacts: Vec<FeatureResolvedGeneratedArtifact>,
}

#[allow(dead_code)]
impl FeatureGeneratedArtifactResolution {
    pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self> {
        let graph = FeatureGraph::from_profile(profile)?;
        Self::from_graph(&graph)
    }

    pub(crate) fn from_graph(graph: &FeatureGraph) -> Result<Self> {
        let mut artifacts = Vec::new();
        for node in graph.nodes() {
            artifacts.extend(artifacts_from_intent(node.intent())?);
        }
        Ok(Self::new(artifacts))
    }

    pub(crate) fn new(
        artifacts: impl IntoIterator<Item = FeatureResolvedGeneratedArtifact>,
    ) -> Self {
        let mut artifacts = artifacts.into_iter().collect::<Vec<_>>();
        artifacts.sort_by_key(|artifact| artifact.stable_key());
        artifacts.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Self { artifacts }
    }

    pub(crate) fn artifacts(&self) -> &[FeatureResolvedGeneratedArtifact] {
        &self.artifacts
    }

    pub(crate) fn artifact_count(&self) -> usize {
        self.artifacts.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }

    pub(crate) fn remove_generated_artifacts(&self) -> Vec<GeneratedArtifactPath> {
        sorted_artifacts_for_kind(
            &self.artifacts,
            FeatureResolvedGeneratedArtifactKind::is_removal,
        )
    }

    pub(crate) fn preserve_generated_artifacts(&self) -> Vec<GeneratedArtifactPath> {
        sorted_artifacts_for_kind(
            &self.artifacts,
            FeatureResolvedGeneratedArtifactKind::is_preservation,
        )
    }

    pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>> {
        let mut ownerships = self
            .artifacts
            .iter()
            .map(FeatureResolvedGeneratedArtifact::ownership)
            .collect::<Result<Vec<_>>>()?;
        ownerships.sort_by_key(|ownership| ownership.stable_key());
        ownerships.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Ok(ownerships)
    }
}

fn artifacts_from_intent(intent: &FeatureIntent) -> Result<Vec<FeatureResolvedGeneratedArtifact>> {
    let mut artifacts = Vec::new();
    match intent.action {
        FeatureIntentAction::Remove => {
            for root in &intent.roots {
                artifacts.extend(generated_artifact_from_path(
                    intent.id.clone(),
                    root.as_relative_kernel_path(),
                    FeatureResolvedGeneratedArtifactKind::RemoveRoot,
                )?);
            }
            for path in &intent.remove_paths {
                artifacts.extend(generated_artifact_from_path(
                    intent.id.clone(),
                    path,
                    FeatureResolvedGeneratedArtifactKind::ExplicitRemovePath,
                )?);
            }
        }
        FeatureIntentAction::Preserve => {
            for root in &intent.roots {
                artifacts.extend(generated_artifact_from_path(
                    intent.id.clone(),
                    root.as_relative_kernel_path(),
                    FeatureResolvedGeneratedArtifactKind::PreserveRoot,
                )?);
            }
        }
    }
    Ok(artifacts)
}

fn generated_artifact_from_path(
    feature: FeatureId,
    path: &RelativeKernelPath,
    kind: FeatureResolvedGeneratedArtifactKind,
) -> Result<Vec<FeatureResolvedGeneratedArtifact>> {
    if !crate::generated::is_generated_artifact_like_path(path.as_path()) {
        return Ok(Vec::new());
    }
    Ok(vec![FeatureResolvedGeneratedArtifact::new(
        feature,
        GeneratedArtifactPath::new(path.as_path().to_path_buf())?,
        kind,
    )])
}


fn sorted_artifacts_for_kind(
    artifacts: &[FeatureResolvedGeneratedArtifact],
    matches_kind: impl Fn(FeatureResolvedGeneratedArtifactKind) -> bool,
) -> Vec<GeneratedArtifactPath> {
    let mut artifacts = artifacts
        .iter()
        .filter(|artifact| matches_kind(artifact.kind()))
        .map(|artifact| artifact.artifact().clone())
        .collect::<Vec<_>>();
    artifacts.sort();
    artifacts.dedup();
    artifacts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_generated_artifact_resolution_resolves_roots_to_generated_artifacts() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("generated-config"),
            FeatureIntentConfig {
                roots: vec![
                    String::from("include/generated/autoconf.h"),
                    String::from("include/generated/uapi/linux/autoconf.h"),
                    String::from("drivers/bluetooth/btusb.c"),
                ],
                remove_paths: vec![
                    String::from("arch/x86/include/generated/asm/offsets.h"),
                    String::from("include/config/auto.conf"),
                    String::from("modules.order"),
                    String::from("include/uapi/linux/bluetooth.h"),
                ],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("live-generated"),
            FeatureIntentConfig {
                roots: vec![
                    String::from("include/generated/utsrelease.h"),
                    String::from("include/linux/version.h"),
                    String::from("arch/arm64/include/generated/uapi/asm/foo.h"),
                ],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let resolution = FeatureGeneratedArtifactResolution::from_profile(&profile).unwrap();

        assert_eq!(resolution.artifact_count(), 5);
        assert!(!resolution.is_empty());
        assert_eq!(
            resolution
                .artifacts()
                .iter()
                .map(FeatureResolvedGeneratedArtifact::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicit_remove_generated_artifact:generated-config:arch/x86/include/generated/asm/offsets.h",
                "explicit_remove_generated_artifact:generated-config:include/config/auto.conf",
                "explicit_remove_generated_artifact:generated-config:modules.order",
                "preserve_root_generated_artifact:live-generated:include/generated/utsrelease.h",
                "remove_root_generated_artifact:generated-config:include/generated/autoconf.h",
            ]
        );
        assert_eq!(
            resolution
                .remove_generated_artifacts()
                .iter()
                .map(GeneratedArtifactPath::as_str)
                .collect::<Vec<_>>(),
            vec![
                "arch/x86/include/generated/asm/offsets.h",
                "include/config/auto.conf",
                "include/generated/autoconf.h",
                "modules.order",
            ]
        );
        assert_eq!(
            resolution
                .preserve_generated_artifacts()
                .iter()
                .map(GeneratedArtifactPath::as_str)
                .collect::<Vec<_>>(),
            vec!["include/generated/utsrelease.h"]
        );
    }

    #[test]
    fn feature_generated_artifact_resolution_emits_generated_ownerships() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("generated-config"),
            FeatureIntentConfig {
                roots: vec![String::from("include/generated/autoconf.h")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("live-generated"),
            FeatureIntentConfig {
                roots: vec![String::from("include/generated/utsrelease.h")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let ownerships = FeatureGeneratedArtifactResolution::from_profile(&profile)
            .unwrap()
            .ownerships()
            .unwrap();

        assert_eq!(
            ownerships
                .iter()
                .map(FeatureOwnership::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "generated_by_live_build:generated-config:generated_artifact:include/generated/autoconf.h",
                "generated_by_live_build:live-generated:generated_artifact:include/generated/utsrelease.h",
            ]
        );
    }

    #[test]
    fn feature_generated_artifact_resolution_rejects_invalid_generated_artifact_path() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bad"),
            FeatureIntentConfig {
                roots: vec![String::from("include/generated/bad artifact.h")],
                configs: vec![String::from("BAD")],
                ..FeatureIntentConfig::default()
            },
        );

        let err = FeatureGeneratedArtifactResolution::from_profile(&profile)
            .unwrap_err()
            .to_string();

        assert!(err.contains("generated artifact path contains whitespace"));
    }
}
