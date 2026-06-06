use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureResolvedKbuildObjectKind {
    RemoveRootObject,
    RemoveRootDirectory,
    ExplicitRemoveObject,
    ExplicitRemoveDirectory,
    PreserveRootObject,
    PreserveRootDirectory,
}

#[allow(dead_code)]
impl FeatureResolvedKbuildObjectKind {
    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::RemoveRootObject => "remove_root_object",
            Self::RemoveRootDirectory => "remove_root_directory",
            Self::ExplicitRemoveObject => "explicit_remove_object",
            Self::ExplicitRemoveDirectory => "explicit_remove_directory",
            Self::PreserveRootObject => "preserve_root_object",
            Self::PreserveRootDirectory => "preserve_root_directory",
        }
    }

    pub(crate) const fn ownership_kind(self) -> FeatureOwnershipKind {
        match self {
            Self::RemoveRootObject
            | Self::RemoveRootDirectory
            | Self::ExplicitRemoveObject
            | Self::ExplicitRemoveDirectory => FeatureOwnershipKind::ExplicitlyRemoved,
            Self::PreserveRootObject | Self::PreserveRootDirectory => {
                FeatureOwnershipKind::ExplicitlyPreserved
            }
        }
    }

    pub(crate) const fn is_removal(self) -> bool {
        matches!(
            self,
            Self::RemoveRootObject
                | Self::RemoveRootDirectory
                | Self::ExplicitRemoveObject
                | Self::ExplicitRemoveDirectory
        )
    }

    pub(crate) const fn is_preservation(self) -> bool {
        matches!(self, Self::PreserveRootObject | Self::PreserveRootDirectory)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureResolvedKbuildObject {
    feature: FeatureId,
    object: KbuildObject,
    kind: FeatureResolvedKbuildObjectKind,
}

#[allow(dead_code)]
impl FeatureResolvedKbuildObject {
    pub(crate) fn new(
        feature: FeatureId,
        object: KbuildObject,
        kind: FeatureResolvedKbuildObjectKind,
    ) -> Self {
        Self {
            feature,
            object,
            kind,
        }
    }

    pub(crate) fn feature(&self) -> &FeatureId {
        &self.feature
    }

    pub(crate) fn object(&self) -> &KbuildObject {
        &self.object
    }

    pub(crate) fn kind(&self) -> FeatureResolvedKbuildObjectKind {
        self.kind
    }

    pub(crate) fn stable_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.kind.stable_name(),
            self.feature.as_str(),
            self.object.as_str()
        )
    }

    pub(crate) fn ownership(&self) -> Result<FeatureOwnership> {
        Ok(FeatureOwnership::new(
            self.kind.ownership_kind(),
            self.feature.clone(),
            FeatureOwnershipSubject::new(format!("kbuild:{}", self.object.as_str()))?,
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FeatureKbuildResolution {
    objects: Vec<FeatureResolvedKbuildObject>,
}

#[allow(dead_code)]
impl FeatureKbuildResolution {
    pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self> {
        let graph = FeatureGraph::from_profile(profile)?;
        Self::from_graph(&graph)
    }

    pub(crate) fn from_graph(graph: &FeatureGraph) -> Result<Self> {
        let mut objects = Vec::new();
        for node in graph.nodes() {
            objects.extend(objects_from_intent(node.intent())?);
        }
        Ok(Self::new(objects))
    }

    pub(crate) fn new(objects: impl IntoIterator<Item = FeatureResolvedKbuildObject>) -> Self {
        let mut objects = objects.into_iter().collect::<Vec<_>>();
        objects.sort_by_key(|object| object.stable_key());
        objects.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Self { objects }
    }

    pub(crate) fn objects(&self) -> &[FeatureResolvedKbuildObject] {
        &self.objects
    }

    pub(crate) fn object_count(&self) -> usize {
        self.objects.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    pub(crate) fn remove_objects(&self) -> Vec<KbuildObject> {
        sorted_objects_for_kind(&self.objects, FeatureResolvedKbuildObjectKind::is_removal)
    }

    pub(crate) fn preserve_objects(&self) -> Vec<KbuildObject> {
        sorted_objects_for_kind(
            &self.objects,
            FeatureResolvedKbuildObjectKind::is_preservation,
        )
    }

    pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>> {
        let mut ownerships = self
            .objects
            .iter()
            .map(FeatureResolvedKbuildObject::ownership)
            .collect::<Result<Vec<_>>>()?;
        ownerships.sort_by_key(|ownership| ownership.stable_key());
        ownerships.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Ok(ownerships)
    }
}

fn objects_from_intent(intent: &FeatureIntent) -> Result<Vec<FeatureResolvedKbuildObject>> {
    let mut objects = Vec::new();
    match intent.action {
        FeatureIntentAction::Remove => {
            for root in &intent.roots {
                objects.extend(objects_from_path(
                    intent.id.clone(),
                    root.as_relative_kernel_path(),
                    FeatureResolvedKbuildObjectKind::RemoveRootObject,
                    FeatureResolvedKbuildObjectKind::RemoveRootDirectory,
                    true,
                )?);
            }
            for path in &intent.remove_paths {
                objects.extend(objects_from_path(
                    intent.id.clone(),
                    path,
                    FeatureResolvedKbuildObjectKind::ExplicitRemoveObject,
                    FeatureResolvedKbuildObjectKind::ExplicitRemoveDirectory,
                    true,
                )?);
            }
        }
        FeatureIntentAction::Preserve => {
            for root in &intent.roots {
                objects.extend(objects_from_path(
                    intent.id.clone(),
                    root.as_relative_kernel_path(),
                    FeatureResolvedKbuildObjectKind::PreserveRootObject,
                    FeatureResolvedKbuildObjectKind::PreserveRootDirectory,
                    true,
                )?);
            }
        }
    }
    Ok(objects)
}

fn objects_from_path(
    feature: FeatureId,
    path: &RelativeKernelPath,
    object_kind: FeatureResolvedKbuildObjectKind,
    directory_kind: FeatureResolvedKbuildObjectKind,
    directory_by_default: bool,
) -> Result<Vec<FeatureResolvedKbuildObject>> {
    let Some((object, kind)) =
        kbuild_object_for_path(path, object_kind, directory_kind, directory_by_default)?
    else {
        return Ok(Vec::new());
    };
    Ok(vec![FeatureResolvedKbuildObject::new(
        feature, object, kind,
    )])
}

fn kbuild_object_for_path(
    path: &RelativeKernelPath,
    object_kind: FeatureResolvedKbuildObjectKind,
    directory_kind: FeatureResolvedKbuildObjectKind,
    directory_by_default: bool,
) -> Result<Option<(KbuildObject, FeatureResolvedKbuildObjectKind)>> {
    let path = path.as_path();
    if is_kbuild_source_path(path) {
        return Ok(Some((
            KbuildObject::new(path.with_extension("o").to_string_lossy().into_owned())?,
            object_kind,
        )));
    }
    if path.extension().and_then(|extension| extension.to_str()) == Some("o") {
        return Ok(Some((
            KbuildObject::new(path.to_string_lossy().into_owned())?,
            object_kind,
        )));
    }
    if !directory_by_default || path.extension().is_some() || is_kbuild_metadata_file(path) {
        return Ok(None);
    }

    Ok(Some((
        KbuildObject::new(format!("{}/", path.to_string_lossy()))?,
        directory_kind,
    )))
}

fn is_kbuild_source_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("c" | "S")
    )
}

fn is_kbuild_metadata_file(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("Kbuild" | "Kconfig" | "Makefile")
    )
}

fn sorted_objects_for_kind(
    objects: &[FeatureResolvedKbuildObject],
    matches_kind: impl Fn(FeatureResolvedKbuildObjectKind) -> bool,
) -> Vec<KbuildObject> {
    let mut objects = objects
        .iter()
        .filter(|object| matches_kind(object.kind()))
        .map(|object| object.object().clone())
        .collect::<Vec<_>>();
    objects.sort();
    objects.dedup();
    objects
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_kbuild_resolution_resolves_roots_to_kbuild_objects() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                roots: vec![
                    String::from("net/bluetooth"),
                    String::from("drivers/bluetooth/hci_core.c"),
                ],
                remove_paths: vec![
                    String::from("drivers/bluetooth/btusb.c"),
                    String::from("drivers/bluetooth/btintel.o"),
                    String::from("drivers/bluetooth/Kconfig"),
                    String::from("include/net/bluetooth.h"),
                ],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                roots: vec![String::from("net/netfilter")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let resolution = FeatureKbuildResolution::from_profile(&profile).unwrap();

        assert_eq!(resolution.object_count(), 5);
        assert!(!resolution.is_empty());
        assert_eq!(
            resolution
                .objects()
                .iter()
                .map(FeatureResolvedKbuildObject::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicit_remove_object:bluetooth:drivers/bluetooth/btintel.o",
                "explicit_remove_object:bluetooth:drivers/bluetooth/btusb.o",
                "preserve_root_directory:netfilter:net/netfilter/",
                "remove_root_directory:bluetooth:net/bluetooth/",
                "remove_root_object:bluetooth:drivers/bluetooth/hci_core.o",
            ]
        );
        assert_eq!(
            resolution
                .remove_objects()
                .iter()
                .map(|object| object.as_str())
                .collect::<Vec<_>>(),
            vec![
                "drivers/bluetooth/btintel.o",
                "drivers/bluetooth/btusb.o",
                "drivers/bluetooth/hci_core.o",
                "net/bluetooth/",
            ]
        );
        assert_eq!(
            resolution
                .preserve_objects()
                .iter()
                .map(|object| object.as_str())
                .collect::<Vec<_>>(),
            vec!["net/netfilter/"]
        );
    }

    #[test]
    fn feature_kbuild_resolution_emits_object_ownerships() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                roots: vec![String::from("drivers/bluetooth")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                roots: vec![String::from("net/netfilter")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let ownerships = FeatureKbuildResolution::from_profile(&profile)
            .unwrap()
            .ownerships()
            .unwrap();

        assert_eq!(
            ownerships
                .iter()
                .map(FeatureOwnership::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicitly_preserved:netfilter:kbuild:net/netfilter/",
                "explicitly_removed:bluetooth:kbuild:drivers/bluetooth/",
            ]
        );
    }

    #[test]
    fn feature_kbuild_resolution_rejects_invalid_kbuild_object_path() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                roots: vec![String::from("drivers/$bad")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );

        let err = FeatureKbuildResolution::from_profile(&profile).unwrap_err();

        assert!(format!("{err:#}").contains("unsupported make syntax"));
    }
}
