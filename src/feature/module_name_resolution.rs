use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureResolvedModuleNameKind {
    RemoveModuleNameRoot,
    ExplicitRemoveModuleName,
    PreserveModuleNameRoot,
}

#[allow(dead_code)]
impl FeatureResolvedModuleNameKind {
    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::RemoveModuleNameRoot => "remove_module_name_root",
            Self::ExplicitRemoveModuleName => "explicit_remove_module_name",
            Self::PreserveModuleNameRoot => "preserve_module_name_root",
        }
    }

    pub(crate) const fn ownership_kind(self) -> FeatureOwnershipKind {
        match self {
            Self::RemoveModuleNameRoot | Self::ExplicitRemoveModuleName => {
                FeatureOwnershipKind::OwnedSolelyByRemovedFeature
            }
            Self::PreserveModuleNameRoot => FeatureOwnershipKind::SharedWithLiveFeature,
        }
    }

    pub(crate) const fn is_removal(self) -> bool {
        matches!(
            self,
            Self::RemoveModuleNameRoot | Self::ExplicitRemoveModuleName
        )
    }

    pub(crate) const fn is_preservation(self) -> bool {
        matches!(self, Self::PreserveModuleNameRoot)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureResolvedModuleName {
    feature: FeatureId,
    module: ModuleName,
    kind: FeatureResolvedModuleNameKind,
}

#[allow(dead_code)]
impl FeatureResolvedModuleName {
    pub(crate) fn new(
        feature: FeatureId,
        module: ModuleName,
        kind: FeatureResolvedModuleNameKind,
    ) -> Self {
        Self {
            feature,
            module,
            kind,
        }
    }

    pub(crate) fn feature(&self) -> &FeatureId {
        &self.feature
    }

    pub(crate) fn module(&self) -> &ModuleName {
        &self.module
    }

    pub(crate) fn kind(&self) -> FeatureResolvedModuleNameKind {
        self.kind
    }

    pub(crate) fn stable_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.kind.stable_name(),
            self.feature.as_str(),
            self.module.as_str()
        )
    }

    pub(crate) fn ownership(&self) -> Result<FeatureOwnership> {
        Ok(FeatureOwnership::new(
            self.kind.ownership_kind(),
            self.feature.clone(),
            FeatureOwnershipSubject::new(format!("module_name:{}", self.module.as_str()))?,
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FeatureModuleNameResolution {
    modules: Vec<FeatureResolvedModuleName>,
}

#[allow(dead_code)]
impl FeatureModuleNameResolution {
    pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self> {
        let graph = FeatureGraph::from_profile(profile)?;
        Ok(Self::from_graph(&graph))
    }

    pub(crate) fn from_graph(graph: &FeatureGraph) -> Self {
        let mut modules = Vec::new();
        for node in graph.nodes() {
            modules.extend(modules_from_intent(node.intent()));
        }
        Self::new(modules)
    }

    pub(crate) fn new(modules: impl IntoIterator<Item = FeatureResolvedModuleName>) -> Self {
        let mut modules = modules.into_iter().collect::<Vec<_>>();
        modules.sort_by_key(|module| module.stable_key());
        modules.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Self { modules }
    }

    pub(crate) fn modules(&self) -> &[FeatureResolvedModuleName] {
        &self.modules
    }

    pub(crate) fn module_count(&self) -> usize {
        self.modules.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    pub(crate) fn remove_module_names(&self) -> Vec<ModuleName> {
        sorted_modules_for_kind(&self.modules, FeatureResolvedModuleNameKind::is_removal)
    }

    pub(crate) fn preserve_module_names(&self) -> Vec<ModuleName> {
        sorted_modules_for_kind(
            &self.modules,
            FeatureResolvedModuleNameKind::is_preservation,
        )
    }

    pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>> {
        let mut ownerships = self
            .modules
            .iter()
            .map(FeatureResolvedModuleName::ownership)
            .collect::<Result<Vec<_>>>()?;
        ownerships.sort_by_key(|ownership| ownership.stable_key());
        ownerships.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Ok(ownerships)
    }
}

fn modules_from_intent(intent: &FeatureIntent) -> Vec<FeatureResolvedModuleName> {
    let mut modules = Vec::new();
    match intent.action {
        FeatureIntentAction::Remove => {
            modules.extend(intent.module_names.iter().cloned().map(|module| {
                FeatureResolvedModuleName::new(
                    intent.id.clone(),
                    module,
                    FeatureResolvedModuleNameKind::RemoveModuleNameRoot,
                )
            }));
            let explicit_modules = intent.remove_module_names.iter().cloned();
            modules.extend(explicit_modules.map(|module| {
                FeatureResolvedModuleName::new(
                    intent.id.clone(),
                    module,
                    FeatureResolvedModuleNameKind::ExplicitRemoveModuleName,
                )
            }));
        }
        FeatureIntentAction::Preserve => {
            modules.extend(intent.module_names.iter().cloned().map(|module| {
                FeatureResolvedModuleName::new(
                    intent.id.clone(),
                    module,
                    FeatureResolvedModuleNameKind::PreserveModuleNameRoot,
                )
            }));
        }
    }
    modules
}

fn sorted_modules_for_kind(
    modules: &[FeatureResolvedModuleName],
    matches_kind: impl Fn(FeatureResolvedModuleNameKind) -> bool,
) -> Vec<ModuleName> {
    let mut modules = modules
        .iter()
        .filter(|module| matches_kind(module.kind()))
        .map(|module| module.module().clone())
        .collect::<Vec<_>>();
    modules.sort();
    modules.dedup();
    modules
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_module_name_resolution_resolves_roots_to_module_names() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                module_names: vec![String::from("btusb"), String::from("btusb")],
                remove_module_names: vec![String::from("bt-debug")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                module_names: vec![String::from("nf-conntrack")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let resolution = FeatureModuleNameResolution::from_profile(&profile).unwrap();

        assert_eq!(resolution.module_count(), 3);
        assert!(!resolution.is_empty());
        assert_eq!(
            resolution
                .modules()
                .iter()
                .map(FeatureResolvedModuleName::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicit_remove_module_name:bluetooth:bt_debug",
                "preserve_module_name_root:netfilter:nf_conntrack",
                "remove_module_name_root:bluetooth:btusb",
            ]
        );
        assert_eq!(
            resolution
                .remove_module_names()
                .iter()
                .map(ModuleName::as_str)
                .collect::<Vec<_>>(),
            vec!["bt_debug", "btusb"]
        );
        assert_eq!(
            resolution
                .preserve_module_names()
                .iter()
                .map(ModuleName::as_str)
                .collect::<Vec<_>>(),
            vec!["nf_conntrack"]
        );
    }

    #[test]
    fn feature_module_name_resolution_emits_module_ownerships() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                module_names: vec![String::from("btusb")],
                remove_module_names: vec![String::from("bt-debug")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                module_names: vec![String::from("nf-conntrack")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let ownerships = FeatureModuleNameResolution::from_profile(&profile)
            .unwrap()
            .ownerships()
            .unwrap();

        assert_eq!(
            ownerships
                .iter()
                .map(FeatureOwnership::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "owned_solely_by_removed_feature:bluetooth:module_name:bt_debug",
                "owned_solely_by_removed_feature:bluetooth:module_name:btusb",
                "shared_with_live_feature:netfilter:module_name:nf_conntrack",
            ]
        );
    }

    #[test]
    fn feature_module_name_resolution_rejects_invalid_module_name() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bad"),
            FeatureIntentConfig {
                module_names: vec![String::from("bad.ko")],
                ..FeatureIntentConfig::default()
            },
        );

        let err = FeatureModuleNameResolution::from_profile(&profile)
            .unwrap_err()
            .to_string();

        assert!(err.contains("module name must omit .ko suffix"));
    }
}
