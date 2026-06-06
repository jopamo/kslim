use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;

static CLI_NO_NETWORK: AtomicBool = AtomicBool::new(false);
static CLI_OFFLINE: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EndpointKind {
    Local,
    Network,
}

pub(crate) fn configure_cli(no_network: bool, offline: bool) {
    CLI_NO_NETWORK.store(no_network || offline, Ordering::Relaxed);
    CLI_OFFLINE.store(offline, Ordering::Relaxed);
}

pub(crate) fn cli_no_network() -> bool {
    CLI_NO_NETWORK.load(Ordering::Relaxed)
}

pub(crate) fn cli_offline() -> bool {
    CLI_OFFLINE.load(Ordering::Relaxed)
}

pub(crate) fn require_local_upstream_url(url: &str) -> Result<()> {
    if endpoint_kind(url)? == EndpointKind::Network {
        anyhow::bail!(
            "upstream.url must point to an existing local git tree; network-backed upstream '{}' is not allowed",
            url
        );
    }
    Ok(())
}

pub(crate) fn require_cli_no_network_endpoint(label: &str, value: &str) -> Result<()> {
    if !CLI_NO_NETWORK.load(Ordering::Relaxed) {
        return Ok(());
    }
    if endpoint_kind(value)? == EndpointKind::Network {
        anyhow::bail!(
            "network access is disabled by --no-network; {} must be a local path or local file URL, got '{}'",
            label,
            value
        );
    }
    Ok(())
}

fn endpoint_kind(value: &str) -> Result<EndpointKind> {
    let value = value.trim();
    if value.is_empty() {
        anyhow::bail!("network endpoint must not be empty");
    }
    if let Some((scheme, rest)) = value.split_once("://") {
        return if scheme.eq_ignore_ascii_case("file") && local_file_url_authority(rest) {
            Ok(EndpointKind::Local)
        } else {
            Ok(EndpointKind::Network)
        };
    }
    if looks_like_scp_remote(value) {
        Ok(EndpointKind::Network)
    } else {
        Ok(EndpointKind::Local)
    }
}

fn local_file_url_authority(rest: &str) -> bool {
    let authority = rest.split('/').next().unwrap_or("");
    authority.is_empty() || authority.eq_ignore_ascii_case("localhost")
}

fn looks_like_scp_remote(value: &str) -> bool {
    let Some(colon) = value.find(':') else {
        return false;
    };
    if value[..colon].is_empty() {
        return false;
    }
    match value.find('/') {
        Some(slash) => colon < slash,
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_kind_treats_urls_and_scp_syntax_as_network() {
        assert_eq!(
            endpoint_kind("https://example.com/linux.git").unwrap(),
            EndpointKind::Network
        );
        assert_eq!(
            endpoint_kind("ssh://example.com/linux.git").unwrap(),
            EndpointKind::Network
        );
        assert_eq!(
            endpoint_kind("git@example.com:linux.git").unwrap(),
            EndpointKind::Network
        );
        assert_eq!(
            endpoint_kind("example.com:linux.git").unwrap(),
            EndpointKind::Network
        );
    }

    #[test]
    fn test_endpoint_kind_allows_local_paths_and_local_file_urls() {
        assert_eq!(
            endpoint_kind("/srv/linux.git").unwrap(),
            EndpointKind::Local
        );
        assert_eq!(
            endpoint_kind("./mirror:with-colon.git").unwrap(),
            EndpointKind::Local
        );
        assert_eq!(
            endpoint_kind("file:///srv/linux.git").unwrap(),
            EndpointKind::Local
        );
        assert_eq!(
            endpoint_kind("file://localhost/srv/linux.git").unwrap(),
            EndpointKind::Local
        );
        assert_eq!(
            endpoint_kind("file://builder/srv/linux.git").unwrap(),
            EndpointKind::Network
        );
    }
}
