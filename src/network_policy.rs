//! Compatibility facade for network security policy.
//!
//! New network trust-boundary ownership lives in `crate::security::network`;
//! this module preserves existing `crate::network_policy::*` call sites while
//! migration proceeds.

pub(crate) use crate::security::{
    cli_no_network, cli_offline, configure_cli, require_cli_no_network_endpoint,
    require_local_upstream_url,
};
