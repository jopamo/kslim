use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureResolvedExportedSymbolKind {
    RemoveExportedSymbolRoot,
    ExplicitRemoveExportedSymbol,
    PreserveExportedSymbolRoot,
}

#[allow(dead_code)]
impl FeatureResolvedExportedSymbolKind {
    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::RemoveExportedSymbolRoot => "remove_exported_symbol_root",
            Self::ExplicitRemoveExportedSymbol => "explicit_remove_exported_symbol",
            Self::PreserveExportedSymbolRoot => "preserve_exported_symbol_root",
        }
    }

    pub(crate) const fn ownership_kind(self) -> FeatureOwnershipKind {
        match self {
            Self::RemoveExportedSymbolRoot | Self::ExplicitRemoveExportedSymbol => {
                FeatureOwnershipKind::OwnedSolelyByRemovedFeature
            }
            Self::PreserveExportedSymbolRoot => FeatureOwnershipKind::SharedWithLiveFeature,
        }
    }

    pub(crate) const fn is_removal(self) -> bool {
        matches!(
            self,
            Self::RemoveExportedSymbolRoot | Self::ExplicitRemoveExportedSymbol
        )
    }

    pub(crate) const fn is_preservation(self) -> bool {
        matches!(self, Self::PreserveExportedSymbolRoot)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureResolvedExportedSymbol {
    feature: FeatureId,
    symbol: ExportedSymbol,
    kind: FeatureResolvedExportedSymbolKind,
}

#[allow(dead_code)]
impl FeatureResolvedExportedSymbol {
    pub(crate) fn new(
        feature: FeatureId,
        symbol: ExportedSymbol,
        kind: FeatureResolvedExportedSymbolKind,
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

    pub(crate) fn symbol(&self) -> &ExportedSymbol {
        &self.symbol
    }

    pub(crate) fn kind(&self) -> FeatureResolvedExportedSymbolKind {
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
            FeatureOwnershipSubject::new(format!("exported_symbol:{}", self.symbol.as_str()))?,
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FeatureExportedSymbolResolution {
    symbols: Vec<FeatureResolvedExportedSymbol>,
}

#[allow(dead_code)]
impl FeatureExportedSymbolResolution {
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

    pub(crate) fn new(symbols: impl IntoIterator<Item = FeatureResolvedExportedSymbol>) -> Self {
        let mut symbols = symbols.into_iter().collect::<Vec<_>>();
        symbols.sort_by_key(|symbol| symbol.stable_key());
        symbols.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Self { symbols }
    }

    pub(crate) fn symbols(&self) -> &[FeatureResolvedExportedSymbol] {
        &self.symbols
    }

    pub(crate) fn symbol_count(&self) -> usize {
        self.symbols.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    pub(crate) fn remove_exported_symbols(&self) -> Vec<ExportedSymbol> {
        sorted_symbols_for_kind(&self.symbols, FeatureResolvedExportedSymbolKind::is_removal)
    }

    pub(crate) fn preserve_exported_symbols(&self) -> Vec<ExportedSymbol> {
        sorted_symbols_for_kind(
            &self.symbols,
            FeatureResolvedExportedSymbolKind::is_preservation,
        )
    }

    pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>> {
        let mut ownerships = self
            .symbols
            .iter()
            .map(FeatureResolvedExportedSymbol::ownership)
            .collect::<Result<Vec<_>>>()?;
        ownerships.sort_by_key(|ownership| ownership.stable_key());
        ownerships.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Ok(ownerships)
    }
}

fn symbols_from_intent(intent: &FeatureIntent) -> Vec<FeatureResolvedExportedSymbol> {
    let mut symbols = Vec::new();
    match intent.action {
        FeatureIntentAction::Remove => {
            symbols.extend(intent.exported_symbols.iter().cloned().map(|symbol| {
                FeatureResolvedExportedSymbol::new(
                    intent.id.clone(),
                    symbol,
                    FeatureResolvedExportedSymbolKind::RemoveExportedSymbolRoot,
                )
            }));
            let explicit_symbols = intent.remove_exported_symbols.iter().cloned();
            symbols.extend(explicit_symbols.map(|symbol| {
                FeatureResolvedExportedSymbol::new(
                    intent.id.clone(),
                    symbol,
                    FeatureResolvedExportedSymbolKind::ExplicitRemoveExportedSymbol,
                )
            }));
        }
        FeatureIntentAction::Preserve => {
            symbols.extend(intent.exported_symbols.iter().cloned().map(|symbol| {
                FeatureResolvedExportedSymbol::new(
                    intent.id.clone(),
                    symbol,
                    FeatureResolvedExportedSymbolKind::PreserveExportedSymbolRoot,
                )
            }));
        }
    }
    symbols
}

fn sorted_symbols_for_kind(
    symbols: &[FeatureResolvedExportedSymbol],
    matches_kind: impl Fn(FeatureResolvedExportedSymbolKind) -> bool,
) -> Vec<ExportedSymbol> {
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
    fn feature_exported_symbol_resolution_resolves_roots_to_exported_symbols() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                exported_symbols: vec![
                    String::from("bt_sock_register"),
                    String::from("bt_sock_register"),
                ],
                remove_exported_symbols: vec![String::from("bt_debugfs_init")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                exported_symbols: vec![String::from("nf_register_net_hook")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let resolution = FeatureExportedSymbolResolution::from_profile(&profile).unwrap();

        assert_eq!(resolution.symbol_count(), 3);
        assert!(!resolution.is_empty());
        assert_eq!(
            resolution
                .symbols()
                .iter()
                .map(FeatureResolvedExportedSymbol::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicit_remove_exported_symbol:bluetooth:bt_debugfs_init",
                "preserve_exported_symbol_root:netfilter:nf_register_net_hook",
                "remove_exported_symbol_root:bluetooth:bt_sock_register",
            ]
        );
        assert_eq!(
            resolution
                .remove_exported_symbols()
                .iter()
                .map(ExportedSymbol::as_str)
                .collect::<Vec<_>>(),
            vec!["bt_debugfs_init", "bt_sock_register"]
        );
        assert_eq!(
            resolution
                .preserve_exported_symbols()
                .iter()
                .map(ExportedSymbol::as_str)
                .collect::<Vec<_>>(),
            vec!["nf_register_net_hook"]
        );
    }

    #[test]
    fn feature_exported_symbol_resolution_emits_symbol_ownerships() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                exported_symbols: vec![String::from("bt_sock_register")],
                remove_exported_symbols: vec![String::from("bt_debugfs_init")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                exported_symbols: vec![String::from("nf_register_net_hook")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let ownerships = FeatureExportedSymbolResolution::from_profile(&profile)
            .unwrap()
            .ownerships()
            .unwrap();

        assert_eq!(
            ownerships
                .iter()
                .map(FeatureOwnership::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "owned_solely_by_removed_feature:bluetooth:exported_symbol:bt_debugfs_init",
                "owned_solely_by_removed_feature:bluetooth:exported_symbol:bt_sock_register",
                "shared_with_live_feature:netfilter:exported_symbol:nf_register_net_hook",
            ]
        );
    }

    #[test]
    fn feature_exported_symbol_resolution_rejects_invalid_symbol() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bad"),
            FeatureIntentConfig {
                exported_symbols: vec![String::from("1bad_symbol")],
                ..FeatureIntentConfig::default()
            },
        );

        let err = FeatureExportedSymbolResolution::from_profile(&profile)
            .unwrap_err()
            .to_string();

        assert!(err.contains("exported symbol contains invalid characters"));
    }
}
