//! Security policy entrypoints.
//!
//! This module owns fail-closed security policy decisions so implementation
//! modules can call a narrow boundary instead of embedding compatibility or
//! downgrade rules.

#![allow(dead_code, unused_imports)]

mod command;
mod filesystem;
mod network;
mod policy;
mod report_safety;
mod resource;

pub(crate) use command::CommandPolicy;
pub(crate) use filesystem::{
    contains_parent_traversal, is_absolute_path_like, normalized_relative_path_covers,
    path_contains_parent_traversal, path_is_absolute_like, path_is_empty_like,
    path_is_normalized_tree_root,
};
pub(crate) use network::{
    cli_no_network, cli_offline, configure_cli, require_cli_no_network_endpoint,
    require_local_upstream_url,
};
pub(crate) use policy::validate_security_config;
pub(crate) use report_safety::{
    find_host_specific_absolute_path_marker, is_host_specific_absolute_path,
    is_reproducible_timestamp, raw_log_file_marker, raw_log_marker, temporary_path_markers,
    timestamp_markers, validate_report_text_has_no_host_absolute_paths,
    validate_report_text_has_no_raw_logs, validate_report_text_has_no_temporary_paths,
    validate_reproducible_timestamp, TEMPORARY_PATH_ERROR_HINT,
};
pub(crate) use resource::ResourcePolicy;
