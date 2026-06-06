use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureResolvedKunitSuiteKind {
    RemoveKunitSuiteRoot,
    ExplicitRemoveKunitSuite,
    PreserveKunitSuiteRoot,
}

#[allow(dead_code)]
impl FeatureResolvedKunitSuiteKind {
    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::RemoveKunitSuiteRoot => "remove_kunit_suite_root",
            Self::ExplicitRemoveKunitSuite => "explicit_remove_kunit_suite",
            Self::PreserveKunitSuiteRoot => "preserve_kunit_suite_root",
        }
    }

    pub(crate) const fn ownership_kind(self) -> FeatureOwnershipKind {
        match self {
            Self::RemoveKunitSuiteRoot | Self::ExplicitRemoveKunitSuite => {
                FeatureOwnershipKind::OwnedSolelyByRemovedFeature
            }
            Self::PreserveKunitSuiteRoot => FeatureOwnershipKind::SharedWithLiveFeature,
        }
    }

    pub(crate) const fn is_removal(self) -> bool {
        matches!(
            self,
            Self::RemoveKunitSuiteRoot | Self::ExplicitRemoveKunitSuite
        )
    }

    pub(crate) const fn is_preservation(self) -> bool {
        matches!(self, Self::PreserveKunitSuiteRoot)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureResolvedKunitSuite {
    feature: FeatureId,
    suite: KunitSuite,
    kind: FeatureResolvedKunitSuiteKind,
}

#[allow(dead_code)]
impl FeatureResolvedKunitSuite {
    pub(crate) fn new(
        feature: FeatureId,
        suite: KunitSuite,
        kind: FeatureResolvedKunitSuiteKind,
    ) -> Self {
        Self {
            feature,
            suite,
            kind,
        }
    }

    pub(crate) fn feature(&self) -> &FeatureId {
        &self.feature
    }

    pub(crate) fn suite(&self) -> &KunitSuite {
        &self.suite
    }

    pub(crate) fn kind(&self) -> FeatureResolvedKunitSuiteKind {
        self.kind
    }

    pub(crate) fn stable_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.kind.stable_name(),
            self.feature.as_str(),
            self.suite.as_str()
        )
    }

    pub(crate) fn ownership(&self) -> Result<FeatureOwnership> {
        Ok(FeatureOwnership::new(
            self.kind.ownership_kind(),
            self.feature.clone(),
            FeatureOwnershipSubject::new(format!("kunit_suite:{}", self.suite.as_str()))?,
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FeatureKunitSuiteResolution {
    suites: Vec<FeatureResolvedKunitSuite>,
}

#[allow(dead_code)]
impl FeatureKunitSuiteResolution {
    pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self> {
        let graph = FeatureGraph::from_profile(profile)?;
        Ok(Self::from_graph(&graph))
    }

    pub(crate) fn from_graph(graph: &FeatureGraph) -> Self {
        let mut suites = Vec::new();
        for node in graph.nodes() {
            suites.extend(kunit_suites_from_intent(node.intent()));
        }
        Self::new(suites)
    }

    pub(crate) fn new(suites: impl IntoIterator<Item = FeatureResolvedKunitSuite>) -> Self {
        let mut suites = suites.into_iter().collect::<Vec<_>>();
        suites.sort_by_key(|suite| suite.stable_key());
        suites.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Self { suites }
    }

    pub(crate) fn suites(&self) -> &[FeatureResolvedKunitSuite] {
        &self.suites
    }

    pub(crate) fn suite_count(&self) -> usize {
        self.suites.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.suites.is_empty()
    }

    pub(crate) fn remove_kunit_suites(&self) -> Vec<KunitSuite> {
        sorted_suites_for_kind(&self.suites, FeatureResolvedKunitSuiteKind::is_removal)
    }

    pub(crate) fn preserve_kunit_suites(&self) -> Vec<KunitSuite> {
        sorted_suites_for_kind(&self.suites, FeatureResolvedKunitSuiteKind::is_preservation)
    }

    pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>> {
        let mut ownerships = self
            .suites
            .iter()
            .map(FeatureResolvedKunitSuite::ownership)
            .collect::<Result<Vec<_>>>()?;
        ownerships.sort_by_key(|ownership| ownership.stable_key());
        ownerships.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Ok(ownerships)
    }
}

fn kunit_suites_from_intent(intent: &FeatureIntent) -> Vec<FeatureResolvedKunitSuite> {
    let mut suites = Vec::new();
    match intent.action {
        FeatureIntentAction::Remove => {
            suites.extend(intent.kunit_suites.iter().cloned().map(|suite| {
                FeatureResolvedKunitSuite::new(
                    intent.id.clone(),
                    suite,
                    FeatureResolvedKunitSuiteKind::RemoveKunitSuiteRoot,
                )
            }));
            let explicit_suites = intent.remove_kunit_suites.iter().cloned();
            suites.extend(explicit_suites.map(|suite| {
                FeatureResolvedKunitSuite::new(
                    intent.id.clone(),
                    suite,
                    FeatureResolvedKunitSuiteKind::ExplicitRemoveKunitSuite,
                )
            }));
        }
        FeatureIntentAction::Preserve => {
            suites.extend(intent.kunit_suites.iter().cloned().map(|suite| {
                FeatureResolvedKunitSuite::new(
                    intent.id.clone(),
                    suite,
                    FeatureResolvedKunitSuiteKind::PreserveKunitSuiteRoot,
                )
            }));
        }
    }
    suites
}

fn sorted_suites_for_kind(
    suites: &[FeatureResolvedKunitSuite],
    matches_kind: impl Fn(FeatureResolvedKunitSuiteKind) -> bool,
) -> Vec<KunitSuite> {
    let mut suites = suites
        .iter()
        .filter(|suite| matches_kind(suite.kind()))
        .map(|suite| suite.suite().clone())
        .collect::<Vec<_>>();
    suites.sort();
    suites.dedup();
    suites
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_kunit_suite_resolution_resolves_roots_to_suites() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                kunit_suites: vec![String::from("bt_test"), String::from("bt_test")],
                remove_kunit_suites: vec![String::from("btusb-test")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                kunit_suites: vec![String::from("nf_conntrack_test")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let resolution = FeatureKunitSuiteResolution::from_profile(&profile).unwrap();

        assert_eq!(resolution.suite_count(), 3);
        assert!(!resolution.is_empty());
        assert_eq!(
            resolution
                .suites()
                .iter()
                .map(FeatureResolvedKunitSuite::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicit_remove_kunit_suite:bluetooth:btusb-test",
                "preserve_kunit_suite_root:netfilter:nf_conntrack_test",
                "remove_kunit_suite_root:bluetooth:bt_test",
            ]
        );
        assert_eq!(
            resolution
                .remove_kunit_suites()
                .iter()
                .map(KunitSuite::as_str)
                .collect::<Vec<_>>(),
            vec!["bt_test", "btusb-test"]
        );
        assert_eq!(
            resolution
                .preserve_kunit_suites()
                .iter()
                .map(KunitSuite::as_str)
                .collect::<Vec<_>>(),
            vec!["nf_conntrack_test"]
        );
    }

    #[test]
    fn feature_kunit_suite_resolution_emits_suite_ownerships() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                kunit_suites: vec![String::from("bt_test")],
                remove_kunit_suites: vec![String::from("btusb-test")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                kunit_suites: vec![String::from("nf_conntrack_test")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let ownerships = FeatureKunitSuiteResolution::from_profile(&profile)
            .unwrap()
            .ownerships()
            .unwrap();

        assert_eq!(
            ownerships
                .iter()
                .map(FeatureOwnership::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "owned_solely_by_removed_feature:bluetooth:kunit_suite:bt_test",
                "owned_solely_by_removed_feature:bluetooth:kunit_suite:btusb-test",
                "shared_with_live_feature:netfilter:kunit_suite:nf_conntrack_test",
            ]
        );
    }

    #[test]
    fn feature_kunit_suite_resolution_rejects_invalid_suite() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bad"),
            FeatureIntentConfig {
                kunit_suites: vec![String::from("bad suite")],
                ..FeatureIntentConfig::default()
            },
        );

        let err = FeatureKunitSuiteResolution::from_profile(&profile)
            .unwrap_err()
            .to_string();

        assert!(err.contains("KUnit suite contains whitespace"));
    }
}
