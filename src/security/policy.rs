use anyhow::Result;

use crate::config::SecurityConfig;

pub(crate) fn validate_security_config(config: &SecurityConfig) -> Result<()> {
    if config
        .compatibility_mode
        .as_deref()
        .is_some_and(|mode| mode.trim().is_empty())
    {
        anyhow::bail!("security.compatibility_mode must not be empty when specified");
    }
    if config.allow_network {
        anyhow::bail!(
            "security.allow_network is not supported; use local read-only upstream inputs until security planning lands"
        );
    }
    if !config.reject_host_paths_in_committed_metadata {
        anyhow::bail!(
            "security.reject_host_paths_in_committed_metadata cannot be disabled; host paths must not enter committed metadata"
        );
    }
    if !config.reject_temp_paths_in_committed_metadata {
        anyhow::bail!(
            "security.reject_temp_paths_in_committed_metadata cannot be disabled; temporary paths must remain attempt metadata"
        );
    }
    if !config.reject_raw_logs_in_committed_metadata {
        anyhow::bail!(
            "security.reject_raw_logs_in_committed_metadata cannot be disabled; raw logs must remain attempt metadata or CI artifacts"
        );
    }
    if !config.fail_on_policy_violation {
        anyhow::bail!(
            "security.fail_on_policy_violation cannot be disabled; security policy must fail closed"
        );
    }
    if !config.is_default() {
        anyhow::bail!(
            "security config is parsed but not yet supported; security policy is fixed and fail-closed until security planning lands"
        );
    }
    Ok(())
}
