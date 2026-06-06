use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureResolvedKselftestTargetKind {
    RemoveKselftestTargetRoot,
    ExplicitRemoveKselftestTarget,
    PreserveKselftestTargetRoot,
}

#[allow(dead_code)]
impl FeatureResolvedKselftestTargetKind {
    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::RemoveKselftestTargetRoot => "remove_kselftest_target_root",
            Self::ExplicitRemoveKselftestTarget => "explicit_remove_kselftest_target",
            Self::PreserveKselftestTargetRoot => "preserve_kselftest_target_root",
        }
    }

    pub(crate) const fn ownership_kind(self) -> FeatureOwnershipKind {
        match self {
            Self::RemoveKselftestTargetRoot | Self::ExplicitRemoveKselftestTarget => {
                FeatureOwnershipKind::OwnedSolelyByRemovedFeature
            }
            Self::PreserveKselftestTargetRoot => FeatureOwnershipKind::SharedWithLiveFeature,
        }
    }

    pub(crate) const fn is_removal(self) -> bool {
        matches!(
            self,
            Self::RemoveKselftestTargetRoot | Self::ExplicitRemoveKselftestTarget
        )
    }

    pub(crate) const fn is_preservation(self) -> bool {
        matches!(self, Self::PreserveKselftestTargetRoot)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureResolvedKselftestTarget {
    feature: FeatureId,
    target: KselftestTarget,
    kind: FeatureResolvedKselftestTargetKind,
}

#[allow(dead_code)]
impl FeatureResolvedKselftestTarget {
    pub(crate) fn new(
        feature: FeatureId,
        target: KselftestTarget,
        kind: FeatureResolvedKselftestTargetKind,
    ) -> Self {
        Self {
            feature,
            target,
            kind,
        }
    }

    pub(crate) fn feature(&self) -> &FeatureId {
        &self.feature
    }

    pub(crate) fn target(&self) -> &KselftestTarget {
        &self.target
    }

    pub(crate) fn kind(&self) -> FeatureResolvedKselftestTargetKind {
        self.kind
    }

    pub(crate) fn stable_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.kind.stable_name(),
            self.feature.as_str(),
            self.target.as_str()
        )
    }

    pub(crate) fn ownership(&self) -> Result<FeatureOwnership> {
        Ok(FeatureOwnership::new(
            self.kind.ownership_kind(),
            self.feature.clone(),
            FeatureOwnershipSubject::new(format!("kselftest_target:{}", self.target.as_str()))?,
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FeatureKselftestTargetResolution {
    targets: Vec<FeatureResolvedKselftestTarget>,
}

#[allow(dead_code)]
impl FeatureKselftestTargetResolution {
    pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self> {
        let graph = FeatureGraph::from_profile(profile)?;
        Ok(Self::from_graph(&graph))
    }

    pub(crate) fn from_graph(graph: &FeatureGraph) -> Self {
        let mut targets = Vec::new();
        for node in graph.nodes() {
            targets.extend(kselftest_targets_from_intent(node.intent()));
        }
        Self::new(targets)
    }

    pub(crate) fn new(targets: impl IntoIterator<Item = FeatureResolvedKselftestTarget>) -> Self {
        let mut targets = targets.into_iter().collect::<Vec<_>>();
        targets.sort_by_key(|target| target.stable_key());
        targets.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Self { targets }
    }

    pub(crate) fn targets(&self) -> &[FeatureResolvedKselftestTarget] {
        &self.targets
    }

    pub(crate) fn target_count(&self) -> usize {
        self.targets.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }

    pub(crate) fn remove_kselftest_targets(&self) -> Vec<KselftestTarget> {
        sorted_targets_for_kind(
            &self.targets,
            FeatureResolvedKselftestTargetKind::is_removal,
        )
    }

    pub(crate) fn preserve_kselftest_targets(&self) -> Vec<KselftestTarget> {
        sorted_targets_for_kind(
            &self.targets,
            FeatureResolvedKselftestTargetKind::is_preservation,
        )
    }

    pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>> {
        let mut ownerships = self
            .targets
            .iter()
            .map(FeatureResolvedKselftestTarget::ownership)
            .collect::<Result<Vec<_>>>()?;
        ownerships.sort_by_key(|ownership| ownership.stable_key());
        ownerships.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Ok(ownerships)
    }
}

fn kselftest_targets_from_intent(intent: &FeatureIntent) -> Vec<FeatureResolvedKselftestTarget> {
    let mut targets = Vec::new();
    match intent.action {
        FeatureIntentAction::Remove => {
            targets.extend(intent.kselftest_targets.iter().cloned().map(|target| {
                FeatureResolvedKselftestTarget::new(
                    intent.id.clone(),
                    target,
                    FeatureResolvedKselftestTargetKind::RemoveKselftestTargetRoot,
                )
            }));
            let explicit_targets = intent.remove_kselftest_targets.iter().cloned();
            targets.extend(explicit_targets.map(|target| {
                FeatureResolvedKselftestTarget::new(
                    intent.id.clone(),
                    target,
                    FeatureResolvedKselftestTargetKind::ExplicitRemoveKselftestTarget,
                )
            }));
        }
        FeatureIntentAction::Preserve => {
            targets.extend(intent.kselftest_targets.iter().cloned().map(|target| {
                FeatureResolvedKselftestTarget::new(
                    intent.id.clone(),
                    target,
                    FeatureResolvedKselftestTargetKind::PreserveKselftestTargetRoot,
                )
            }));
        }
    }
    targets
}

fn sorted_targets_for_kind(
    targets: &[FeatureResolvedKselftestTarget],
    matches_kind: impl Fn(FeatureResolvedKselftestTargetKind) -> bool,
) -> Vec<KselftestTarget> {
    let mut targets = targets
        .iter()
        .filter(|target| matches_kind(target.kind()))
        .map(|target| target.target().clone())
        .collect::<Vec<_>>();
    targets.sort();
    targets.dedup();
    targets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_kselftest_target_resolution_resolves_roots_to_targets() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                kselftest_targets: vec![String::from("net"), String::from("net")],
                remove_kselftest_targets: vec![String::from("bpf")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                kselftest_targets: vec![String::from("drivers/net")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let resolution = FeatureKselftestTargetResolution::from_profile(&profile).unwrap();

        assert_eq!(resolution.target_count(), 3);
        assert!(!resolution.is_empty());
        assert_eq!(
            resolution
                .targets()
                .iter()
                .map(FeatureResolvedKselftestTarget::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicit_remove_kselftest_target:bluetooth:bpf",
                "preserve_kselftest_target_root:netfilter:drivers/net",
                "remove_kselftest_target_root:bluetooth:net",
            ]
        );
        assert_eq!(
            resolution
                .remove_kselftest_targets()
                .iter()
                .map(KselftestTarget::as_str)
                .collect::<Vec<_>>(),
            vec!["bpf", "net"]
        );
        assert_eq!(
            resolution
                .preserve_kselftest_targets()
                .iter()
                .map(KselftestTarget::as_str)
                .collect::<Vec<_>>(),
            vec!["drivers/net"]
        );
    }

    #[test]
    fn feature_kselftest_target_resolution_emits_target_ownerships() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                kselftest_targets: vec![String::from("net")],
                remove_kselftest_targets: vec![String::from("bpf")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                kselftest_targets: vec![String::from("drivers/net")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let ownerships = FeatureKselftestTargetResolution::from_profile(&profile)
            .unwrap()
            .ownerships()
            .unwrap();

        assert_eq!(
            ownerships
                .iter()
                .map(FeatureOwnership::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "owned_solely_by_removed_feature:bluetooth:kselftest_target:bpf",
                "owned_solely_by_removed_feature:bluetooth:kselftest_target:net",
                "shared_with_live_feature:netfilter:kselftest_target:drivers/net",
            ]
        );
    }

    #[test]
    fn feature_kselftest_target_resolution_rejects_invalid_target() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bad"),
            FeatureIntentConfig {
                kselftest_targets: vec![String::from("bad target")],
                ..FeatureIntentConfig::default()
            },
        );

        let err = FeatureKselftestTargetResolution::from_profile(&profile)
            .unwrap_err()
            .to_string();

        assert!(err.contains("kselftest target contains whitespace"));
    }
}
