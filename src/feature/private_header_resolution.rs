use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureResolvedPrivateHeaderKind {
    RemoveRoot,
    ExplicitRemovePath,
    PreserveRoot,
}

#[allow(dead_code)]
impl FeatureResolvedPrivateHeaderKind {
    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::RemoveRoot => "remove_root_private_header",
            Self::ExplicitRemovePath => "explicit_remove_private_header",
            Self::PreserveRoot => "preserve_root_private_header",
        }
    }

    pub(crate) const fn ownership_kind(self) -> FeatureOwnershipKind {
        match self {
            Self::RemoveRoot | Self::ExplicitRemovePath => FeatureOwnershipKind::ExplicitlyRemoved,
            Self::PreserveRoot => FeatureOwnershipKind::ExplicitlyPreserved,
        }
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
pub(crate) struct FeatureResolvedPrivateHeader {
    feature: FeatureId,
    header: HeaderPath,
    kind: FeatureResolvedPrivateHeaderKind,
}

#[allow(dead_code)]
impl FeatureResolvedPrivateHeader {
    pub(crate) fn new(
        feature: FeatureId,
        header: HeaderPath,
        kind: FeatureResolvedPrivateHeaderKind,
    ) -> Self {
        Self {
            feature,
            header,
            kind,
        }
    }

    pub(crate) fn feature(&self) -> &FeatureId {
        &self.feature
    }

    pub(crate) fn header(&self) -> &HeaderPath {
        &self.header
    }

    pub(crate) fn kind(&self) -> FeatureResolvedPrivateHeaderKind {
        self.kind
    }

    pub(crate) fn stable_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.kind.stable_name(),
            self.feature.as_str(),
            self.header.as_str()
        )
    }

    pub(crate) fn ownership(&self) -> Result<FeatureOwnership> {
        Ok(FeatureOwnership::new(
            self.kind.ownership_kind(),
            self.feature.clone(),
            FeatureOwnershipSubject::new(format!("private_header:{}", self.header.as_str()))?,
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FeaturePrivateHeaderResolution {
    headers: Vec<FeatureResolvedPrivateHeader>,
}

#[allow(dead_code)]
impl FeaturePrivateHeaderResolution {
    pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self> {
        let graph = FeatureGraph::from_profile(profile)?;
        Self::from_graph(&graph)
    }

    pub(crate) fn from_graph(graph: &FeatureGraph) -> Result<Self> {
        let mut headers = Vec::new();
        for node in graph.nodes() {
            headers.extend(headers_from_intent(node.intent())?);
        }
        Ok(Self::new(headers))
    }

    pub(crate) fn new(headers: impl IntoIterator<Item = FeatureResolvedPrivateHeader>) -> Self {
        let mut headers = headers.into_iter().collect::<Vec<_>>();
        headers.sort_by_key(|header| header.stable_key());
        headers.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Self { headers }
    }

    pub(crate) fn headers(&self) -> &[FeatureResolvedPrivateHeader] {
        &self.headers
    }

    pub(crate) fn header_count(&self) -> usize {
        self.headers.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.headers.is_empty()
    }

    pub(crate) fn remove_private_headers(&self) -> Vec<HeaderPath> {
        sorted_headers_for_kind(&self.headers, FeatureResolvedPrivateHeaderKind::is_removal)
    }

    pub(crate) fn preserve_private_headers(&self) -> Vec<HeaderPath> {
        sorted_headers_for_kind(
            &self.headers,
            FeatureResolvedPrivateHeaderKind::is_preservation,
        )
    }

    pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>> {
        let mut ownerships = self
            .headers
            .iter()
            .map(FeatureResolvedPrivateHeader::ownership)
            .collect::<Result<Vec<_>>>()?;
        ownerships.sort_by_key(|ownership| ownership.stable_key());
        ownerships.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Ok(ownerships)
    }
}

fn headers_from_intent(intent: &FeatureIntent) -> Result<Vec<FeatureResolvedPrivateHeader>> {
    let mut headers = Vec::new();
    match intent.action {
        FeatureIntentAction::Remove => {
            for root in &intent.roots {
                headers.extend(private_header_from_path(
                    intent.id.clone(),
                    root.as_relative_kernel_path(),
                    FeatureResolvedPrivateHeaderKind::RemoveRoot,
                )?);
            }
            for path in &intent.remove_paths {
                headers.extend(private_header_from_path(
                    intent.id.clone(),
                    path,
                    FeatureResolvedPrivateHeaderKind::ExplicitRemovePath,
                )?);
            }
        }
        FeatureIntentAction::Preserve => {
            for root in &intent.roots {
                headers.extend(private_header_from_path(
                    intent.id.clone(),
                    root.as_relative_kernel_path(),
                    FeatureResolvedPrivateHeaderKind::PreserveRoot,
                )?);
            }
        }
    }
    Ok(headers)
}

fn private_header_from_path(
    feature: FeatureId,
    path: &RelativeKernelPath,
    kind: FeatureResolvedPrivateHeaderKind,
) -> Result<Vec<FeatureResolvedPrivateHeader>> {
    if !is_private_header_path(path.as_path()) {
        return Ok(Vec::new());
    }
    Ok(vec![FeatureResolvedPrivateHeader::new(
        feature,
        HeaderPath::new(path.as_path().to_string_lossy().into_owned())?,
        kind,
    )])
}

fn is_private_header_path(path: &Path) -> bool {
    has_header_extension(path)
        && !is_public_header_path(path)
        && !is_uapi_like_header_path(path)
        && !is_generated_header_path(path)
}

fn has_header_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension == "h")
}

fn is_public_header_path(path: &Path) -> bool {
    path.starts_with("include/linux") || path.starts_with("include/net")
}

fn is_uapi_like_header_path(path: &Path) -> bool {
    let parts = path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>();

    matches!(
        parts.as_slice(),
        ["include", "uapi", ..]
            | ["include", "generated", "uapi", ..]
            | ["arch", _, "include", "uapi", ..]
            | ["arch", _, "include", "generated", "uapi", ..]
    )
}

fn is_generated_header_path(path: &Path) -> bool {
    let parts = path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>();

    matches!(
        parts.as_slice(),
        ["include", "generated", ..] | ["arch", _, "include", "generated", ..]
    )
}

fn sorted_headers_for_kind(
    headers: &[FeatureResolvedPrivateHeader],
    matches_kind: impl Fn(FeatureResolvedPrivateHeaderKind) -> bool,
) -> Vec<HeaderPath> {
    let mut headers = headers
        .iter()
        .filter(|header| matches_kind(header.kind()))
        .map(|header| header.header().clone())
        .collect::<Vec<_>>();
    headers.sort();
    headers.dedup();
    headers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_private_header_resolution_resolves_roots_to_private_headers() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                roots: vec![
                    String::from("drivers/bluetooth/btusb.h"),
                    String::from("drivers/bluetooth"),
                ],
                remove_paths: vec![
                    String::from("drivers/bluetooth/btrtl.h"),
                    String::from("include/linux/public.h"),
                    String::from("include/net/public.h"),
                    String::from("include/uapi/linux/abi.h"),
                    String::from("include/generated/autoconf.h"),
                    String::from("drivers/bluetooth/btusb.c"),
                ],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                roots: vec![
                    String::from("net/netfilter/nf_conntrack_core.h"),
                    String::from("include/net/netfilter_public.h"),
                    String::from("arch/x86/include/generated/asm/offsets.h"),
                ],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let resolution = FeaturePrivateHeaderResolution::from_profile(&profile).unwrap();

        assert_eq!(resolution.header_count(), 3);
        assert!(!resolution.is_empty());
        assert_eq!(
            resolution
                .headers()
                .iter()
                .map(FeatureResolvedPrivateHeader::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicit_remove_private_header:bluetooth:drivers/bluetooth/btrtl.h",
                "preserve_root_private_header:netfilter:net/netfilter/nf_conntrack_core.h",
                "remove_root_private_header:bluetooth:drivers/bluetooth/btusb.h",
            ]
        );
        assert_eq!(
            resolution
                .remove_private_headers()
                .iter()
                .map(HeaderPath::as_str)
                .collect::<Vec<_>>(),
            vec!["drivers/bluetooth/btrtl.h", "drivers/bluetooth/btusb.h"]
        );
        assert_eq!(
            resolution
                .preserve_private_headers()
                .iter()
                .map(HeaderPath::as_str)
                .collect::<Vec<_>>(),
            vec!["net/netfilter/nf_conntrack_core.h"]
        );
    }

    #[test]
    fn feature_private_header_resolution_emits_header_ownerships() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                roots: vec![String::from("drivers/bluetooth/btusb.h")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                roots: vec![String::from("net/netfilter/nf_conntrack_core.h")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let ownerships = FeaturePrivateHeaderResolution::from_profile(&profile)
            .unwrap()
            .ownerships()
            .unwrap();

        assert_eq!(
            ownerships
                .iter()
                .map(FeatureOwnership::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicitly_preserved:netfilter:private_header:net/netfilter/nf_conntrack_core.h",
                "explicitly_removed:bluetooth:private_header:drivers/bluetooth/btusb.h",
            ]
        );
    }

    #[test]
    fn feature_private_header_resolution_rejects_invalid_private_header_path() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bad"),
            FeatureIntentConfig {
                roots: vec![String::from("drivers/foo/private header.h")],
                configs: vec![String::from("BAD")],
                ..FeatureIntentConfig::default()
            },
        );

        let err = FeaturePrivateHeaderResolution::from_profile(&profile)
            .unwrap_err()
            .to_string();

        assert!(err.contains("header path contains whitespace"));
    }
}
