use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureResolvedModuleAliasKind {
    RemoveModuleAliasRoot,
    ExplicitRemoveModuleAlias,
    PreserveModuleAliasRoot,
}

#[allow(dead_code)]
impl FeatureResolvedModuleAliasKind {
    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::RemoveModuleAliasRoot => "remove_module_alias_root",
            Self::ExplicitRemoveModuleAlias => "explicit_remove_module_alias",
            Self::PreserveModuleAliasRoot => "preserve_module_alias_root",
        }
    }

    pub(crate) const fn ownership_kind(self) -> FeatureOwnershipKind {
        match self {
            Self::RemoveModuleAliasRoot | Self::ExplicitRemoveModuleAlias => {
                FeatureOwnershipKind::OwnedSolelyByRemovedFeature
            }
            Self::PreserveModuleAliasRoot => FeatureOwnershipKind::SharedWithLiveFeature,
        }
    }

    pub(crate) const fn is_removal(self) -> bool {
        matches!(
            self,
            Self::RemoveModuleAliasRoot | Self::ExplicitRemoveModuleAlias
        )
    }

    pub(crate) const fn is_preservation(self) -> bool {
        matches!(self, Self::PreserveModuleAliasRoot)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureResolvedModuleAlias {
    feature: FeatureId,
    alias: ModuleAlias,
    kind: FeatureResolvedModuleAliasKind,
}

#[allow(dead_code)]
impl FeatureResolvedModuleAlias {
    pub(crate) fn new(
        feature: FeatureId,
        alias: ModuleAlias,
        kind: FeatureResolvedModuleAliasKind,
    ) -> Self {
        Self {
            feature,
            alias,
            kind,
        }
    }

    pub(crate) fn feature(&self) -> &FeatureId {
        &self.feature
    }

    pub(crate) fn alias(&self) -> &ModuleAlias {
        &self.alias
    }

    pub(crate) fn kind(&self) -> FeatureResolvedModuleAliasKind {
        self.kind
    }

    pub(crate) fn stable_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.kind.stable_name(),
            self.feature.as_str(),
            self.alias.as_str()
        )
    }

    pub(crate) fn ownership(&self) -> Result<FeatureOwnership> {
        Ok(FeatureOwnership::new(
            self.kind.ownership_kind(),
            self.feature.clone(),
            FeatureOwnershipSubject::new(format!("module_alias:{}", self.alias.as_str()))?,
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FeatureModuleAliasResolution {
    aliases: Vec<FeatureResolvedModuleAlias>,
}

#[allow(dead_code)]
impl FeatureModuleAliasResolution {
    pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self> {
        let graph = FeatureGraph::from_profile(profile)?;
        Ok(Self::from_graph(&graph))
    }

    pub(crate) fn from_graph(graph: &FeatureGraph) -> Self {
        let mut aliases = Vec::new();
        for node in graph.nodes() {
            aliases.extend(aliases_from_intent(node.intent()));
        }
        Self::new(aliases)
    }

    pub(crate) fn new(aliases: impl IntoIterator<Item = FeatureResolvedModuleAlias>) -> Self {
        let mut aliases = aliases.into_iter().collect::<Vec<_>>();
        aliases.sort_by_key(|alias| alias.stable_key());
        aliases.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Self { aliases }
    }

    pub(crate) fn aliases(&self) -> &[FeatureResolvedModuleAlias] {
        &self.aliases
    }

    pub(crate) fn alias_count(&self) -> usize {
        self.aliases.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.aliases.is_empty()
    }

    pub(crate) fn remove_module_aliases(&self) -> Vec<ModuleAlias> {
        sorted_aliases_for_kind(&self.aliases, FeatureResolvedModuleAliasKind::is_removal)
    }

    pub(crate) fn preserve_module_aliases(&self) -> Vec<ModuleAlias> {
        sorted_aliases_for_kind(
            &self.aliases,
            FeatureResolvedModuleAliasKind::is_preservation,
        )
    }

    pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>> {
        let mut ownerships = self
            .aliases
            .iter()
            .map(FeatureResolvedModuleAlias::ownership)
            .collect::<Result<Vec<_>>>()?;
        ownerships.sort_by_key(|ownership| ownership.stable_key());
        ownerships.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Ok(ownerships)
    }
}

fn aliases_from_intent(intent: &FeatureIntent) -> Vec<FeatureResolvedModuleAlias> {
    let mut aliases = Vec::new();
    match intent.action {
        FeatureIntentAction::Remove => {
            aliases.extend(intent.module_aliases.iter().cloned().map(|alias| {
                FeatureResolvedModuleAlias::new(
                    intent.id.clone(),
                    alias,
                    FeatureResolvedModuleAliasKind::RemoveModuleAliasRoot,
                )
            }));
            let explicit_aliases = intent.remove_module_aliases.iter().cloned();
            aliases.extend(explicit_aliases.map(|alias| {
                FeatureResolvedModuleAlias::new(
                    intent.id.clone(),
                    alias,
                    FeatureResolvedModuleAliasKind::ExplicitRemoveModuleAlias,
                )
            }));
        }
        FeatureIntentAction::Preserve => {
            aliases.extend(intent.module_aliases.iter().cloned().map(|alias| {
                FeatureResolvedModuleAlias::new(
                    intent.id.clone(),
                    alias,
                    FeatureResolvedModuleAliasKind::PreserveModuleAliasRoot,
                )
            }));
        }
    }
    aliases
}

fn sorted_aliases_for_kind(
    aliases: &[FeatureResolvedModuleAlias],
    matches_kind: impl Fn(FeatureResolvedModuleAliasKind) -> bool,
) -> Vec<ModuleAlias> {
    let mut aliases = aliases
        .iter()
        .filter(|alias| matches_kind(alias.kind()))
        .map(|alias| alias.alias().clone())
        .collect::<Vec<_>>();
    aliases.sort();
    aliases.dedup();
    aliases
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_module_alias_resolution_resolves_roots_to_module_aliases() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                module_aliases: vec![
                    String::from("usb:v*p*d*dc*dsc*dp*ic*isc*ip*in*"),
                    String::from("usb:v*p*d*dc*dsc*dp*ic*isc*ip*in*"),
                ],
                remove_module_aliases: vec![String::from("pci:v00008086d00001572sv*sd*bc*sc*i*")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                module_aliases: vec![String::from("of:N*T*Cqcom,ipq8064")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let resolution = FeatureModuleAliasResolution::from_profile(&profile).unwrap();

        assert_eq!(resolution.alias_count(), 3);
        assert!(!resolution.is_empty());
        assert_eq!(
            resolution
                .aliases()
                .iter()
                .map(FeatureResolvedModuleAlias::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicit_remove_module_alias:bluetooth:pci:v00008086d00001572sv*sd*bc*sc*i*",
                "preserve_module_alias_root:netfilter:of:N*T*Cqcom,ipq8064",
                "remove_module_alias_root:bluetooth:usb:v*p*d*dc*dsc*dp*ic*isc*ip*in*",
            ]
        );
        assert_eq!(
            resolution
                .remove_module_aliases()
                .iter()
                .map(ModuleAlias::as_str)
                .collect::<Vec<_>>(),
            vec![
                "pci:v00008086d00001572sv*sd*bc*sc*i*",
                "usb:v*p*d*dc*dsc*dp*ic*isc*ip*in*",
            ]
        );
        assert_eq!(
            resolution
                .preserve_module_aliases()
                .iter()
                .map(ModuleAlias::as_str)
                .collect::<Vec<_>>(),
            vec!["of:N*T*Cqcom,ipq8064"]
        );
    }

    #[test]
    fn feature_module_alias_resolution_emits_alias_ownerships() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                module_aliases: vec![String::from("usb:v*p*d*dc*dsc*dp*ic*isc*ip*in*")],
                remove_module_aliases: vec![String::from("pci:v00008086d00001572sv*sd*bc*sc*i*")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                module_aliases: vec![String::from("of:N*T*Cqcom,ipq8064")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let ownerships = FeatureModuleAliasResolution::from_profile(&profile)
            .unwrap()
            .ownerships()
            .unwrap();

        assert_eq!(
            ownerships
                .iter()
                .map(FeatureOwnership::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "owned_solely_by_removed_feature:bluetooth:module_alias:pci:v00008086d00001572sv*sd*bc*sc*i*",
                "owned_solely_by_removed_feature:bluetooth:module_alias:usb:v*p*d*dc*dsc*dp*ic*isc*ip*in*",
                "shared_with_live_feature:netfilter:module_alias:of:N*T*Cqcom,ipq8064",
            ]
        );
    }

    #[test]
    fn feature_module_alias_resolution_rejects_invalid_module_alias() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bad"),
            FeatureIntentConfig {
                module_aliases: vec![String::from("bad alias")],
                ..FeatureIntentConfig::default()
            },
        );

        let err = FeatureModuleAliasResolution::from_profile(&profile)
            .unwrap_err()
            .to_string();

        assert!(err.contains("module alias must not contain whitespace"));
    }
}
