use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureResolvedKconfigKind {
    RemoveConfigRoot,
    ExplicitRemoveConfig,
    PreserveConfigRoot,
}

#[allow(dead_code)]
impl FeatureResolvedKconfigKind {
    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::RemoveConfigRoot => "remove_config_root",
            Self::ExplicitRemoveConfig => "explicit_remove_config",
            Self::PreserveConfigRoot => "preserve_config_root",
        }
    }

    pub(crate) const fn ownership_kind(self) -> FeatureOwnershipKind {
        match self {
            Self::RemoveConfigRoot | Self::ExplicitRemoveConfig => {
                FeatureOwnershipKind::ExplicitlyRemoved
            }
            Self::PreserveConfigRoot => FeatureOwnershipKind::ExplicitlyPreserved,
        }
    }

    pub(crate) const fn is_removal(self) -> bool {
        matches!(self, Self::RemoveConfigRoot | Self::ExplicitRemoveConfig)
    }

    pub(crate) const fn is_preservation(self) -> bool {
        matches!(self, Self::PreserveConfigRoot)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureResolvedKconfig {
    feature: FeatureId,
    symbol: KconfigSymbol,
    kind: FeatureResolvedKconfigKind,
}

#[allow(dead_code)]
impl FeatureResolvedKconfig {
    pub(crate) fn new(
        feature: FeatureId,
        symbol: KconfigSymbol,
        kind: FeatureResolvedKconfigKind,
    ) -> Self {
        Self {
            feature,
            symbol,
            kind,
        }
    }

    pub(crate) fn feature(&self) -> &FeatureId {
        &self.feature
    }

    pub(crate) fn symbol(&self) -> &KconfigSymbol {
        &self.symbol
    }

    pub(crate) fn kind(&self) -> FeatureResolvedKconfigKind {
        self.kind
    }

    pub(crate) fn stable_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.kind.stable_name(),
            self.feature.as_str(),
            self.symbol.as_str()
        )
    }

    pub(crate) fn ownership(&self) -> Result<FeatureOwnership> {
        Ok(FeatureOwnership::new(
            self.kind.ownership_kind(),
            self.feature.clone(),
            FeatureOwnershipSubject::new(format!("kconfig:{}", self.symbol.as_str()))?,
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FeatureKconfigResolution {
    symbols: Vec<FeatureResolvedKconfig>,
}

#[allow(dead_code)]
impl FeatureKconfigResolution {
    pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self> {
        let graph = FeatureGraph::from_profile(profile)?;
        Ok(Self::from_graph(&graph))
    }

    pub(crate) fn from_graph(graph: &FeatureGraph) -> Self {
        let mut symbols = Vec::new();
        for node in graph.nodes() {
            symbols.extend(symbols_from_intent(node.intent()));
        }
        Self::new(symbols)
    }

    pub(crate) fn new(symbols: impl IntoIterator<Item = FeatureResolvedKconfig>) -> Self {
        let mut symbols = symbols.into_iter().collect::<Vec<_>>();
        symbols.sort_by_key(|symbol| symbol.stable_key());
        symbols.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Self { symbols }
    }

    pub(crate) fn symbols(&self) -> &[FeatureResolvedKconfig] {
        &self.symbols
    }

    pub(crate) fn symbol_count(&self) -> usize {
        self.symbols.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    pub(crate) fn remove_configs(&self) -> Vec<KconfigSymbol> {
        sorted_symbols_for_kind(&self.symbols, FeatureResolvedKconfigKind::is_removal)
    }

    pub(crate) fn preserve_configs(&self) -> Vec<KconfigSymbol> {
        sorted_symbols_for_kind(&self.symbols, FeatureResolvedKconfigKind::is_preservation)
    }

    pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>> {
        let mut ownerships = self
            .symbols
            .iter()
            .map(FeatureResolvedKconfig::ownership)
            .collect::<Result<Vec<_>>>()?;
        ownerships.sort_by_key(|ownership| ownership.stable_key());
        ownerships.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Ok(ownerships)
    }
}

fn symbols_from_intent(intent: &FeatureIntent) -> Vec<FeatureResolvedKconfig> {
    let mut symbols = Vec::new();
    match intent.action {
        FeatureIntentAction::Remove => {
            symbols.extend(intent.configs.iter().cloned().map(|symbol| {
                FeatureResolvedKconfig::new(
                    intent.id.clone(),
                    symbol,
                    FeatureResolvedKconfigKind::RemoveConfigRoot,
                )
            }));
            symbols.extend(intent.remove_configs.iter().cloned().map(|symbol| {
                FeatureResolvedKconfig::new(
                    intent.id.clone(),
                    symbol,
                    FeatureResolvedKconfigKind::ExplicitRemoveConfig,
                )
            }));
        }
        FeatureIntentAction::Preserve => {
            symbols.extend(intent.configs.iter().cloned().map(|symbol| {
                FeatureResolvedKconfig::new(
                    intent.id.clone(),
                    symbol,
                    FeatureResolvedKconfigKind::PreserveConfigRoot,
                )
            }));
        }
    }
    symbols
}

fn sorted_symbols_for_kind(
    symbols: &[FeatureResolvedKconfig],
    matches_kind: impl Fn(FeatureResolvedKconfigKind) -> bool,
) -> Vec<KconfigSymbol> {
    let mut symbols = symbols
        .iter()
        .filter(|symbol| matches_kind(symbol.kind()))
        .map(|symbol| symbol.symbol().clone())
        .collect::<Vec<_>>();
    symbols.sort();
    symbols.dedup();
    symbols
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_kconfig_resolution_resolves_roots_to_kconfig_symbols() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                configs: vec![String::from("BT"), String::from("BT_HCIBTUSB")],
                remove_configs: vec![String::from("BT_DEBUG")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let resolution = FeatureKconfigResolution::from_profile(&profile).unwrap();

        assert_eq!(resolution.symbol_count(), 4);
        assert!(!resolution.is_empty());
        assert_eq!(
            resolution
                .symbols()
                .iter()
                .map(FeatureResolvedKconfig::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicit_remove_config:bluetooth:BT_DEBUG",
                "preserve_config_root:netfilter:NETFILTER",
                "remove_config_root:bluetooth:BT",
                "remove_config_root:bluetooth:BT_HCIBTUSB",
            ]
        );
        assert_eq!(
            resolution
                .remove_configs()
                .iter()
                .map(|symbol| symbol.as_str())
                .collect::<Vec<_>>(),
            vec!["BT", "BT_DEBUG", "BT_HCIBTUSB"]
        );
        assert_eq!(
            resolution
                .preserve_configs()
                .iter()
                .map(|symbol| symbol.as_str())
                .collect::<Vec<_>>(),
            vec!["NETFILTER"]
        );
    }

    #[test]
    fn feature_kconfig_resolution_emits_symbol_ownerships() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let ownerships = FeatureKconfigResolution::from_profile(&profile)
            .unwrap()
            .ownerships()
            .unwrap();

        assert_eq!(
            ownerships
                .iter()
                .map(FeatureOwnership::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicitly_preserved:netfilter:kconfig:NETFILTER",
                "explicitly_removed:bluetooth:kconfig:BT",
            ]
        );
    }

    #[test]
    fn feature_kconfig_resolution_rejects_invalid_feature_intent() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                configs: vec![String::from("BT DEBUG")],
                ..FeatureIntentConfig::default()
            },
        );

        let err = FeatureKconfigResolution::from_profile(&profile).unwrap_err();

        assert!(format!("{err:#}").contains("Kconfig symbol contains invalid characters"));
    }
}
