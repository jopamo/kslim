use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureResolvedInitcallKind {
    RemoveInitcallRoot,
    ExplicitRemoveInitcall,
    PreserveInitcallRoot,
}

#[allow(dead_code)]
impl FeatureResolvedInitcallKind {
    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::RemoveInitcallRoot => "remove_initcall_root",
            Self::ExplicitRemoveInitcall => "explicit_remove_initcall",
            Self::PreserveInitcallRoot => "preserve_initcall_root",
        }
    }

    pub(crate) const fn ownership_kind(self) -> FeatureOwnershipKind {
        match self {
            Self::RemoveInitcallRoot | Self::ExplicitRemoveInitcall => {
                FeatureOwnershipKind::OwnedSolelyByRemovedFeature
            }
            Self::PreserveInitcallRoot => FeatureOwnershipKind::SharedWithLiveFeature,
        }
    }

    pub(crate) const fn is_removal(self) -> bool {
        matches!(
            self,
            Self::RemoveInitcallRoot | Self::ExplicitRemoveInitcall
        )
    }

    pub(crate) const fn is_preservation(self) -> bool {
        matches!(self, Self::PreserveInitcallRoot)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureResolvedInitcall {
    feature: FeatureId,
    initcall: Initcall,
    kind: FeatureResolvedInitcallKind,
}

#[allow(dead_code)]
impl FeatureResolvedInitcall {
    pub(crate) fn new(
        feature: FeatureId,
        initcall: Initcall,
        kind: FeatureResolvedInitcallKind,
    ) -> Self {
        Self {
            feature,
            initcall,
            kind,
        }
    }

    pub(crate) fn feature(&self) -> &FeatureId {
        &self.feature
    }

    pub(crate) fn initcall(&self) -> &Initcall {
        &self.initcall
    }

    pub(crate) fn kind(&self) -> FeatureResolvedInitcallKind {
        self.kind
    }

    pub(crate) fn stable_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.kind.stable_name(),
            self.feature.as_str(),
            self.initcall.as_str()
        )
    }

    pub(crate) fn ownership(&self) -> Result<FeatureOwnership> {
        Ok(FeatureOwnership::new(
            self.kind.ownership_kind(),
            self.feature.clone(),
            FeatureOwnershipSubject::new(format!("initcall:{}", self.initcall.as_str()))?,
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FeatureInitcallResolution {
    initcalls: Vec<FeatureResolvedInitcall>,
}

#[allow(dead_code)]
impl FeatureInitcallResolution {
    pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self> {
        let graph = FeatureGraph::from_profile(profile)?;
        Ok(Self::from_graph(&graph))
    }

    pub(crate) fn from_graph(graph: &FeatureGraph) -> Self {
        let mut initcalls = Vec::new();
        for node in graph.nodes() {
            initcalls.extend(initcalls_from_intent(node.intent()));
        }
        Self::new(initcalls)
    }

    pub(crate) fn new(initcalls: impl IntoIterator<Item = FeatureResolvedInitcall>) -> Self {
        let mut initcalls = initcalls.into_iter().collect::<Vec<_>>();
        initcalls.sort_by_key(|initcall| initcall.stable_key());
        initcalls.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Self { initcalls }
    }

    pub(crate) fn initcalls(&self) -> &[FeatureResolvedInitcall] {
        &self.initcalls
    }

    pub(crate) fn initcall_count(&self) -> usize {
        self.initcalls.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.initcalls.is_empty()
    }

    pub(crate) fn remove_initcalls(&self) -> Vec<Initcall> {
        sorted_initcalls_for_kind(&self.initcalls, FeatureResolvedInitcallKind::is_removal)
    }

    pub(crate) fn preserve_initcalls(&self) -> Vec<Initcall> {
        sorted_initcalls_for_kind(
            &self.initcalls,
            FeatureResolvedInitcallKind::is_preservation,
        )
    }

    pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>> {
        let mut ownerships = self
            .initcalls
            .iter()
            .map(FeatureResolvedInitcall::ownership)
            .collect::<Result<Vec<_>>>()?;
        ownerships.sort_by_key(|ownership| ownership.stable_key());
        ownerships.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Ok(ownerships)
    }
}

fn initcalls_from_intent(intent: &FeatureIntent) -> Vec<FeatureResolvedInitcall> {
    let mut initcalls = Vec::new();
    match intent.action {
        FeatureIntentAction::Remove => {
            initcalls.extend(intent.initcalls.iter().cloned().map(|initcall| {
                FeatureResolvedInitcall::new(
                    intent.id.clone(),
                    initcall,
                    FeatureResolvedInitcallKind::RemoveInitcallRoot,
                )
            }));
            let explicit_initcalls = intent.remove_initcalls.iter().cloned();
            initcalls.extend(explicit_initcalls.map(|initcall| {
                FeatureResolvedInitcall::new(
                    intent.id.clone(),
                    initcall,
                    FeatureResolvedInitcallKind::ExplicitRemoveInitcall,
                )
            }));
        }
        FeatureIntentAction::Preserve => {
            initcalls.extend(intent.initcalls.iter().cloned().map(|initcall| {
                FeatureResolvedInitcall::new(
                    intent.id.clone(),
                    initcall,
                    FeatureResolvedInitcallKind::PreserveInitcallRoot,
                )
            }));
        }
    }
    initcalls
}

fn sorted_initcalls_for_kind(
    initcalls: &[FeatureResolvedInitcall],
    matches_kind: impl Fn(FeatureResolvedInitcallKind) -> bool,
) -> Vec<Initcall> {
    let mut initcalls = initcalls
        .iter()
        .filter(|initcall| matches_kind(initcall.kind()))
        .map(|initcall| initcall.initcall().clone())
        .collect::<Vec<_>>();
    initcalls.sort();
    initcalls.dedup();
    initcalls
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_initcall_resolution_resolves_roots_to_initcalls() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                initcalls: vec![String::from("bt_init"), String::from("bt_init")],
                remove_initcalls: vec![String::from("btusb_driver_init")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                initcalls: vec![String::from("nf_conntrack_standalone_init")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let resolution = FeatureInitcallResolution::from_profile(&profile).unwrap();

        assert_eq!(resolution.initcall_count(), 3);
        assert!(!resolution.is_empty());
        assert_eq!(
            resolution
                .initcalls()
                .iter()
                .map(FeatureResolvedInitcall::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicit_remove_initcall:bluetooth:btusb_driver_init",
                "preserve_initcall_root:netfilter:nf_conntrack_standalone_init",
                "remove_initcall_root:bluetooth:bt_init",
            ]
        );
        assert_eq!(
            resolution
                .remove_initcalls()
                .iter()
                .map(Initcall::as_str)
                .collect::<Vec<_>>(),
            vec!["bt_init", "btusb_driver_init"]
        );
        assert_eq!(
            resolution
                .preserve_initcalls()
                .iter()
                .map(Initcall::as_str)
                .collect::<Vec<_>>(),
            vec!["nf_conntrack_standalone_init"]
        );
    }

    #[test]
    fn feature_initcall_resolution_emits_initcall_ownerships() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                initcalls: vec![String::from("bt_init")],
                remove_initcalls: vec![String::from("btusb_driver_init")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                initcalls: vec![String::from("nf_conntrack_standalone_init")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let ownerships = FeatureInitcallResolution::from_profile(&profile)
            .unwrap()
            .ownerships()
            .unwrap();

        assert_eq!(
            ownerships
                .iter()
                .map(FeatureOwnership::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "owned_solely_by_removed_feature:bluetooth:initcall:bt_init",
                "owned_solely_by_removed_feature:bluetooth:initcall:btusb_driver_init",
                "shared_with_live_feature:netfilter:initcall:nf_conntrack_standalone_init",
            ]
        );
    }

    #[test]
    fn feature_initcall_resolution_rejects_invalid_initcall() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bad"),
            FeatureIntentConfig {
                initcalls: vec![String::from("1bad")],
                ..FeatureIntentConfig::default()
            },
        );

        let err = FeatureInitcallResolution::from_profile(&profile)
            .unwrap_err()
            .to_string();

        assert!(err.contains("initcall contains invalid characters"));
    }
}
