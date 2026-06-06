use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureResolvedPublicHeaderKind {
    RemoveRoot,
    ExplicitRemovePath,
    PreserveRoot,
}

#[allow(dead_code)]
impl FeatureResolvedPublicHeaderKind {
    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::RemoveRoot => "remove_root_public_header",
            Self::ExplicitRemovePath => "explicit_remove_public_header",
            Self::PreserveRoot => "preserve_root_public_header",
        }
    }

    pub(crate) const fn ownership_kind(self) -> FeatureOwnershipKind {
        FeatureOwnershipKind::PublicAbiSurface
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
pub(crate) struct FeatureResolvedPublicHeader {
    feature: FeatureId,
    header: HeaderPath,
    kind: FeatureResolvedPublicHeaderKind,
}

#[allow(dead_code)]
impl FeatureResolvedPublicHeader {
    pub(crate) fn new(
        feature: FeatureId,
        header: HeaderPath,
        kind: FeatureResolvedPublicHeaderKind,
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

    pub(crate) fn kind(&self) -> FeatureResolvedPublicHeaderKind {
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
            FeatureOwnershipSubject::new(format!("public_header:{}", self.header.as_str()))?,
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FeaturePublicHeaderResolution {
    headers: Vec<FeatureResolvedPublicHeader>,
}

#[allow(dead_code)]
impl FeaturePublicHeaderResolution {
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

    pub(crate) fn new(headers: impl IntoIterator<Item = FeatureResolvedPublicHeader>) -> Self {
        let mut headers = headers.into_iter().collect::<Vec<_>>();
        headers.sort_by_key(|header| header.stable_key());
        headers.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Self { headers }
    }

    pub(crate) fn headers(&self) -> &[FeatureResolvedPublicHeader] {
        &self.headers
    }

    pub(crate) fn header_count(&self) -> usize {
        self.headers.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.headers.is_empty()
    }

    pub(crate) fn remove_public_headers(&self) -> Vec<HeaderPath> {
        sorted_headers_for_kind(&self.headers, FeatureResolvedPublicHeaderKind::is_removal)
    }

    pub(crate) fn preserve_public_headers(&self) -> Vec<HeaderPath> {
        sorted_headers_for_kind(
            &self.headers,
            FeatureResolvedPublicHeaderKind::is_preservation,
        )
    }

    pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>> {
        let mut ownerships = self
            .headers
            .iter()
            .map(FeatureResolvedPublicHeader::ownership)
            .collect::<Result<Vec<_>>>()?;
        ownerships.sort_by_key(|ownership| ownership.stable_key());
        ownerships.dedup_by(|left, right| left.stable_key() == right.stable_key());
        Ok(ownerships)
    }
}

fn headers_from_intent(intent: &FeatureIntent) -> Result<Vec<FeatureResolvedPublicHeader>> {
    let mut headers = Vec::new();
    match intent.action {
        FeatureIntentAction::Remove => {
            for root in &intent.roots {
                headers.extend(public_header_from_path(
                    intent.id.clone(),
                    root.as_relative_kernel_path(),
                    FeatureResolvedPublicHeaderKind::RemoveRoot,
                )?);
            }
            for path in &intent.remove_paths {
                headers.extend(public_header_from_path(
                    intent.id.clone(),
                    path,
                    FeatureResolvedPublicHeaderKind::ExplicitRemovePath,
                )?);
            }
        }
        FeatureIntentAction::Preserve => {
            for root in &intent.roots {
                headers.extend(public_header_from_path(
                    intent.id.clone(),
                    root.as_relative_kernel_path(),
                    FeatureResolvedPublicHeaderKind::PreserveRoot,
                )?);
            }
        }
    }
    Ok(headers)
}

fn public_header_from_path(
    feature: FeatureId,
    path: &RelativeKernelPath,
    kind: FeatureResolvedPublicHeaderKind,
) -> Result<Vec<FeatureResolvedPublicHeader>> {
    if !is_public_header_path(path.as_path()) {
        return Ok(Vec::new());
    }
    Ok(vec![FeatureResolvedPublicHeader::new(
        feature,
        HeaderPath::new(path.as_path().to_string_lossy().into_owned())?,
        kind,
    )])
}

fn is_public_header_path(path: &Path) -> bool {
    has_header_extension(path)
        && (path.starts_with("include/linux") || path.starts_with("include/net"))
}

fn has_header_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension == "h")
}

fn sorted_headers_for_kind(
    headers: &[FeatureResolvedPublicHeader],
    matches_kind: impl Fn(FeatureResolvedPublicHeaderKind) -> bool,
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
    fn feature_public_header_resolution_resolves_roots_to_public_headers() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                roots: vec![
                    String::from("include/linux/bluetooth.h"),
                    String::from("drivers/bluetooth/private.h"),
                    String::from("include/uapi/linux/bluetooth.h"),
                ],
                remove_paths: vec![
                    String::from("include/net/bluetooth.h"),
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
                    String::from("include/net/netfilter.h"),
                    String::from("net/netfilter/nf_conntrack_core.h"),
                    String::from("include/uapi/linux/netfilter.h"),
                ],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let resolution = FeaturePublicHeaderResolution::from_profile(&profile).unwrap();

        assert_eq!(resolution.header_count(), 3);
        assert!(!resolution.is_empty());
        assert_eq!(
            resolution
                .headers()
                .iter()
                .map(FeatureResolvedPublicHeader::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "explicit_remove_public_header:bluetooth:include/net/bluetooth.h",
                "preserve_root_public_header:netfilter:include/net/netfilter.h",
                "remove_root_public_header:bluetooth:include/linux/bluetooth.h",
            ]
        );
        assert_eq!(
            resolution
                .remove_public_headers()
                .iter()
                .map(HeaderPath::as_str)
                .collect::<Vec<_>>(),
            vec!["include/linux/bluetooth.h", "include/net/bluetooth.h"]
        );
        assert_eq!(
            resolution
                .preserve_public_headers()
                .iter()
                .map(HeaderPath::as_str)
                .collect::<Vec<_>>(),
            vec!["include/net/netfilter.h"]
        );
    }

    #[test]
    fn feature_public_header_resolution_emits_public_abi_ownerships() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            FeatureIntentConfig {
                roots: vec![String::from("include/linux/bluetooth.h")],
                configs: vec![String::from("BT")],
                ..FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                roots: vec![String::from("include/net/netfilter.h")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );

        let ownerships = FeaturePublicHeaderResolution::from_profile(&profile)
            .unwrap()
            .ownerships()
            .unwrap();

        assert_eq!(
            ownerships
                .iter()
                .map(FeatureOwnership::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "public_abi_surface:bluetooth:public_header:include/linux/bluetooth.h",
                "public_abi_surface:netfilter:public_header:include/net/netfilter.h",
            ]
        );
    }

    #[test]
    fn feature_public_header_resolution_rejects_invalid_public_header_path() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bad"),
            FeatureIntentConfig {
                roots: vec![String::from("include/linux/public header.h")],
                configs: vec![String::from("BAD")],
                ..FeatureIntentConfig::default()
            },
        );

        let err = FeaturePublicHeaderResolution::from_profile(&profile)
            .unwrap_err()
            .to_string();

        assert!(err.contains("header path contains whitespace"));
    }
}
