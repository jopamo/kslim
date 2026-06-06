use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureResolvedRuntimeRegistrationKind {
    RemoveRuntimeRegistrationRoot,
    ExplicitRemoveRuntimeRegistration,
    PreserveRuntimeRegistrationRoot,
}

#[allow(dead_code)]
impl FeatureResolvedRuntimeRegistrationKind {
    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::RemoveRuntimeRegistrationRoot => "remove_runtime_registration_root",
            Self::ExplicitRemoveRuntimeRegistration => "explicit_remove_runtime_registration",
            Self::PreserveRuntimeRegistrationRoot => "preserve_runtime_registration_root",
        }
    }

    pub(crate) const fn ownership_kind(self) -> FeatureOwnershipKind {
        match self {
            Self::RemoveRuntimeRegistrationRoot | Self::ExplicitRemoveRuntimeRegistration => {
                FeatureOwnershipKind::OwnedSolelyByRemovedFeature
            }
            Self::PreserveRuntimeRegistrationRoot => FeatureOwnershipKind::SharedWithLiveFeature,
        }
    }

    pub(crate) const fn is_removal(self) -> bool {
        matches!(
            self,
            Self::RemoveRuntimeRegistrationRoot | Self::ExplicitRemoveRuntimeRegistration
        )
    }

    pub(crate) const fn is_preservation(self) -> bool {
        matches!(self, Self::PreserveRuntimeRegistrationRoot)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureResolvedRuntimeRegistration {
    feature: FeatureId,
    runtime_registration: RuntimeRegistrationSurface,
    kind: FeatureResolvedRuntimeRegistrationKind,
}

#[allow(dead_code)]
impl FeatureResolvedRuntimeRegistration {
    pub(crate) fn new(
        feature: FeatureId,
        runtime_registration: RuntimeRegistrationSurface,
        kind: FeatureResolvedRuntimeRegistrationKind,
    ) -> Self {
        Self {
            feature,
            runtime_registration,
            kind,
        }
    }

    pub(crate) fn feature(&self) -> &FeatureId {
        &self.feature
    }

    pub(crate) fn runtime_registration(&self) -> &RuntimeRegistrationSurface {
        &self.runtime_registration
    }

    pub(crate) fn kind(&self) -> FeatureResolvedRuntimeRegistrationKind {
        self.kind
    }

    pub(crate) fn stable_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.kind.stable_name(),
            self.feature.as_str(),
            self.runtime_registration.as_str()
        )
    }

    pub(crate) fn ownership(&self) -> Result<FeatureOwnership> {
        Ok(FeatureOwnership::new(
            self.kind.ownership_kind(),
            self.feature.clone(),
            FeatureOwnershipSubject::new(format!(
                "runtime_registration_surface:{}",
                self.runtime_registration.as_str()
            ))?,
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FeatureRuntimeRegistrationResolution {
    runtime_registrations: Vec<FeatureResolvedRuntimeRegistration>,
}

#[allow(dead_code)]
impl FeatureRuntimeRegistrationResolution {
    pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self> {
        let graph = FeatureGraph::from_profile(profile)?;
        Ok(Self::from_graph(&graph))
    }

    pub(crate) fn from_graph(graph: &FeatureGraph) -> Self {
        let mut runtime_registrations = Vec::new();
        for node in graph.nodes() {
            runtime_registrations.extend(runtime_registrations_from_intent(node.intent()));
        }
        Self::new(runtime_registrations)
    }

    pub(crate) fn new(
        runtime_registrations: impl IntoIterator<Item = FeatureResolvedRuntimeRegistration>,
    ) -> Self {
        let mut runtime_registrations = runtime_registrations.into_iter().collect::<Vec<_>>();
        runtime_registrations.sort_by_key(|runtime_registration| runtime_registration.stable_key());
        runtime_registrations.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Self {
            runtime_registrations,
        }
    }

    pub(crate) fn runtime_registrations(&self) -> &[FeatureResolvedRuntimeRegistration] {
        &self.runtime_registrations
    }

    pub(crate) fn runtime_registration_count(&self) -> usize {
        self.runtime_registrations.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.runtime_registrations.is_empty()
    }

    pub(crate) fn remove_runtime_registrations(&self) -> Vec<RuntimeRegistrationSurface> {
        sorted_runtime_registrations_for_kind(
            &self.runtime_registrations,
            FeatureResolvedRuntimeRegistrationKind::is_removal,
        )
    }

    pub(crate) fn preserve_runtime_registrations(&self) -> Vec<RuntimeRegistrationSurface> {
        sorted_runtime_registrations_for_kind(
            &self.runtime_registrations,
            FeatureResolvedRuntimeRegistrationKind::is_preservation,
        )
    }

    pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>> {
        let mut ownerships = self
            .runtime_registrations
            .iter()
            .map(FeatureResolvedRuntimeRegistration::ownership)
            .collect::<Result<Vec<_>>>()?;
        ownerships.sort_by_key(|ownership| ownership.stable_key());
        ownerships.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Ok(ownerships)
    }
}

fn runtime_registrations_from_intent(
    intent: &FeatureIntent,
) -> Vec<FeatureResolvedRuntimeRegistration> {
    let mut runtime_registrations = Vec::new();
    match intent.action {
        FeatureIntentAction::Remove => {
            runtime_registrations.extend(intent.runtime_registrations.iter().cloned().map(
                |runtime_registration| {
                    FeatureResolvedRuntimeRegistration::new(
                        intent.id.clone(),
                        runtime_registration,
                        FeatureResolvedRuntimeRegistrationKind::RemoveRuntimeRegistrationRoot,
                    )
                },
            ));
            let explicit_runtime_registrations =
                intent.remove_runtime_registrations.iter().cloned();
            runtime_registrations.extend(explicit_runtime_registrations.map(
                |runtime_registration| {
                    FeatureResolvedRuntimeRegistration::new(
                        intent.id.clone(),
                        runtime_registration,
                        FeatureResolvedRuntimeRegistrationKind::ExplicitRemoveRuntimeRegistration,
                    )
                },
            ));
        }
        FeatureIntentAction::Preserve => {
            runtime_registrations.extend(intent.runtime_registrations.iter().cloned().map(
                |runtime_registration| {
                    FeatureResolvedRuntimeRegistration::new(
                        intent.id.clone(),
                        runtime_registration,
                        FeatureResolvedRuntimeRegistrationKind::PreserveRuntimeRegistrationRoot,
                    )
                },
            ));
        }
    }
    runtime_registrations
}

fn sorted_runtime_registrations_for_kind(
    runtime_registrations: &[FeatureResolvedRuntimeRegistration],
    matches_kind: impl Fn(FeatureResolvedRuntimeRegistrationKind) -> bool,
) -> Vec<RuntimeRegistrationSurface> {
    let mut runtime_registrations = runtime_registrations
        .iter()
        .filter(|runtime_registration| matches_kind(runtime_registration.kind()))
        .map(|runtime_registration| runtime_registration.runtime_registration().clone())
        .collect::<Vec<_>>();
    runtime_registrations.sort();
    runtime_registrations.dedup();
    runtime_registrations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_runtime_registration_resolution_resolves_roots_to_runtime_registrations() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                runtime_registrations: vec![
                    String::from("module_init:bt_init"),
                    String::from("module_init:bt_init"),
                ],
                remove_runtime_registrations: vec![String::from(
                    "module_platform_driver:btusb_driver",
                )],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                runtime_registrations: vec![String::from(
                    "module_init:nf_conntrack_standalone_init",
                )],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let resolution = FeatureRuntimeRegistrationResolution::from_profile(&profile).unwrap();

        assert_eq!(resolution.runtime_registration_count(), 3);
        assert!(!resolution.is_empty());
        assert_eq!(
            resolution
                .runtime_registrations()
                .iter()
                .map(FeatureResolvedRuntimeRegistration::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicit_remove_runtime_registration:bluetooth:module_platform_driver:btusb_driver",
                "preserve_runtime_registration_root:netfilter:module_init:nf_conntrack_standalone_init",
                "remove_runtime_registration_root:bluetooth:module_init:bt_init",
            ]
        );
        assert_eq!(
            resolution
                .remove_runtime_registrations()
                .iter()
                .map(RuntimeRegistrationSurface::as_str)
                .collect::<Vec<_>>(),
            vec!["module_init:bt_init", "module_platform_driver:btusb_driver"]
        );
        assert_eq!(
            resolution
                .preserve_runtime_registrations()
                .iter()
                .map(RuntimeRegistrationSurface::as_str)
                .collect::<Vec<_>>(),
            vec!["module_init:nf_conntrack_standalone_init"]
        );
    }

    #[test]
    fn feature_runtime_registration_resolution_emits_runtime_registration_ownerships() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                runtime_registrations: vec![String::from("module_init:bt_init")],
                remove_runtime_registrations: vec![String::from(
                    "module_platform_driver:btusb_driver",
                )],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                runtime_registrations: vec![String::from(
                    "module_init:nf_conntrack_standalone_init",
                )],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let ownerships = FeatureRuntimeRegistrationResolution::from_profile(&profile)
            .unwrap()
            .ownerships()
            .unwrap();

        assert_eq!(
            ownerships
                .iter()
                .map(FeatureOwnership::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "owned_solely_by_removed_feature:bluetooth:runtime_registration_surface:module_init:bt_init",
                "owned_solely_by_removed_feature:bluetooth:runtime_registration_surface:module_platform_driver:btusb_driver",
                "shared_with_live_feature:netfilter:runtime_registration_surface:module_init:nf_conntrack_standalone_init",
            ]
        );
    }

    #[test]
    fn feature_runtime_registration_resolution_rejects_invalid_runtime_registration() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bad"),
            FeatureIntentConfig {
                runtime_registrations: vec![String::from("module_init:1bad")],
                ..FeatureIntentConfig::default()
            },
        );

        let err = FeatureRuntimeRegistrationResolution::from_profile(&profile)
            .unwrap_err()
            .to_string();

        assert!(err.contains("runtime registration entry point contains invalid characters"));
    }
}
