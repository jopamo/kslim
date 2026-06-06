use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureResolvedUapiHeaderKind {
    RemoveRoot,
    ExplicitRemovePath,
    PreserveRoot,
}

#[allow(dead_code)]
impl FeatureResolvedUapiHeaderKind {
    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::RemoveRoot => "remove_root_uapi_header",
            Self::ExplicitRemovePath => "explicit_remove_uapi_header",
            Self::PreserveRoot => "preserve_root_uapi_header",
        }
    }

    pub(crate) const fn ownership_kind(self) -> FeatureOwnershipKind {
        FeatureOwnershipKind::PublicUapiSurface
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
pub(crate) struct FeatureResolvedUapiHeader {
    feature: FeatureId,
    header: UapiPath,
    kind: FeatureResolvedUapiHeaderKind,
}

#[allow(dead_code)]
impl FeatureResolvedUapiHeader {
    pub(crate) fn new(
        feature: FeatureId,
        header: UapiPath,
        kind: FeatureResolvedUapiHeaderKind,
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

    pub(crate) fn header(&self) -> &UapiPath {
        &self.header
    }

    pub(crate) fn kind(&self) -> FeatureResolvedUapiHeaderKind {
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
            FeatureOwnershipSubject::new(format!("uapi_header:{}", self.header.as_str()))?,
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FeatureUapiHeaderResolution {
    headers: Vec<FeatureResolvedUapiHeader>,
}

#[allow(dead_code)]
impl FeatureUapiHeaderResolution {
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

    pub(crate) fn new(headers: impl IntoIterator<Item = FeatureResolvedUapiHeader>) -> Self {
        let mut headers = headers.into_iter().collect::<Vec<_>>();
        headers.sort_by_key(|header| header.stable_key());
        headers.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Self { headers }
    }

    pub(crate) fn headers(&self) -> &[FeatureResolvedUapiHeader] {
        &self.headers
    }

    pub(crate) fn header_count(&self) -> usize {
        self.headers.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.headers.is_empty()
    }

    pub(crate) fn remove_uapi_headers(&self) -> Vec<UapiPath> {
        sorted_headers_for_kind(&self.headers, FeatureResolvedUapiHeaderKind::is_removal)
    }

    pub(crate) fn preserve_uapi_headers(&self) -> Vec<UapiPath> {
        sorted_headers_for_kind(
            &self.headers,
            FeatureResolvedUapiHeaderKind::is_preservation,
        )
    }

    pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>> {
        let mut ownerships = self
            .headers
            .iter()
            .map(FeatureResolvedUapiHeader::ownership)
            .collect::<Result<Vec<_>>>()?;
        ownerships.sort_by_key(|ownership| ownership.stable_key());
        ownerships.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Ok(ownerships)
    }
}

fn headers_from_intent(intent: &FeatureIntent) -> Result<Vec<FeatureResolvedUapiHeader>> {
    let mut headers = Vec::new();
    match intent.action {
        FeatureIntentAction::Remove => {
            for root in &intent.roots {
                headers.extend(uapi_header_from_path(
                    intent.id.clone(),
                    root.as_relative_kernel_path(),
                    FeatureResolvedUapiHeaderKind::RemoveRoot,
                )?);
            }
            for path in &intent.remove_paths {
                headers.extend(uapi_header_from_path(
                    intent.id.clone(),
                    path,
                    FeatureResolvedUapiHeaderKind::ExplicitRemovePath,
                )?);
            }
        }
        FeatureIntentAction::Preserve => {
            for root in &intent.roots {
                headers.extend(uapi_header_from_path(
                    intent.id.clone(),
                    root.as_relative_kernel_path(),
                    FeatureResolvedUapiHeaderKind::PreserveRoot,
                )?);
            }
        }
    }
    Ok(headers)
}

fn uapi_header_from_path(
    feature: FeatureId,
    path: &RelativeKernelPath,
    kind: FeatureResolvedUapiHeaderKind,
) -> Result<Vec<FeatureResolvedUapiHeader>> {
    if !is_uapi_like_header_path(path.as_path()) {
        return Ok(Vec::new());
    }
    Ok(vec![FeatureResolvedUapiHeader::new(
        feature,
        UapiPath::new(path.as_path().to_path_buf())?,
        kind,
    )])
}

fn is_uapi_header_path(path: &Path) -> bool {
    has_header_extension(path) && UapiPath::matches_path(path)
}

fn is_uapi_like_header_path(path: &Path) -> bool {
    if is_uapi_header_path(path) {
        return true;
    }
    has_header_extension(path) && raw_uapi_path_parts_match(path)
}

fn raw_uapi_path_parts_match(path: &Path) -> bool {
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

fn has_header_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension == "h")
}

fn sorted_headers_for_kind(
    headers: &[FeatureResolvedUapiHeader],
    matches_kind: impl Fn(FeatureResolvedUapiHeaderKind) -> bool,
) -> Vec<UapiPath> {
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
    fn feature_uapi_header_resolution_resolves_roots_to_uapi_headers() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                roots: vec![
                    String::from("include/uapi/linux/bluetooth.h"),
                    String::from("include/linux/bluetooth.h"),
                    String::from("drivers/bluetooth/private.h"),
                ],
                remove_paths: vec![
                    String::from("arch/x86/include/uapi/asm/bluetooth.h"),
                    String::from("include/generated/uapi/linux/autoconf.h"),
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
                    String::from("include/uapi/linux/netfilter.h"),
                    String::from("include/net/netfilter.h"),
                    String::from("drivers/net/netfilter_private.h"),
                ],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let resolution = FeatureUapiHeaderResolution::from_profile(&profile).unwrap();

        assert_eq!(resolution.header_count(), 4);
        assert!(!resolution.is_empty());
        assert_eq!(
            resolution
                .headers()
                .iter()
                .map(FeatureResolvedUapiHeader::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicit_remove_uapi_header:bluetooth:arch/x86/include/uapi/asm/bluetooth.h",
                "explicit_remove_uapi_header:bluetooth:include/generated/uapi/linux/autoconf.h",
                "preserve_root_uapi_header:netfilter:include/uapi/linux/netfilter.h",
                "remove_root_uapi_header:bluetooth:include/uapi/linux/bluetooth.h",
            ]
        );
        assert_eq!(
            resolution
                .remove_uapi_headers()
                .iter()
                .map(UapiPath::as_str)
                .collect::<Vec<_>>(),
            vec![
                "arch/x86/include/uapi/asm/bluetooth.h",
                "include/generated/uapi/linux/autoconf.h",
                "include/uapi/linux/bluetooth.h",
            ]
        );
        assert_eq!(
            resolution
                .preserve_uapi_headers()
                .iter()
                .map(UapiPath::as_str)
                .collect::<Vec<_>>(),
            vec!["include/uapi/linux/netfilter.h"]
        );
    }

    #[test]
    fn feature_uapi_header_resolution_emits_uapi_ownerships() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                roots: vec![String::from("include/uapi/linux/bluetooth.h")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                roots: vec![String::from("include/uapi/linux/netfilter.h")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let ownerships = FeatureUapiHeaderResolution::from_profile(&profile)
            .unwrap()
            .ownerships()
            .unwrap();

        assert_eq!(
            ownerships
                .iter()
                .map(FeatureOwnership::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "public_uapi_surface:bluetooth:uapi_header:include/uapi/linux/bluetooth.h",
                "public_uapi_surface:netfilter:uapi_header:include/uapi/linux/netfilter.h",
            ]
        );
    }

    #[test]
    fn feature_uapi_header_resolution_rejects_invalid_uapi_header_path() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bad"),
            FeatureIntentConfig {
                roots: vec![String::from("include/uapi/linux/bad header.h")],
                configs: vec![String::from("BAD")],
                ..FeatureIntentConfig::default()
            },
        );

        let err = FeatureUapiHeaderResolution::from_profile(&profile)
            .unwrap_err()
            .to_string();

        assert!(err.contains("UAPI path contains whitespace"));
    }
}
