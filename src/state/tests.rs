use super::*;
use crate::config::{self, KernelBuildConfig, RtlmqIntegrationConfig, SlimConfig};

fn test_resolved_base() -> ResolvedBase {
    ResolvedBase {
        upstream: String::from("linux"),
        url: String::from("/tmp/linux.git"),
        r#ref: String::from("v1.0"),
        commit: String::from("deadbeef"),
        resolved_at: String::from("2026-01-01T00:00:00Z"),
    }
}

fn test_commit_result() -> SuccessfulCommitResult {
    SuccessfulCommitResult {
        committed: true,
        branch: String::from("kslim/v1.0/default"),
        tag: String::from("kslim-v1.0-default-r1"),
        output_commit: String::from("deadbeef"),
    }
}

#[path = "tests_requested.rs"]
mod requested;
#[path = "tests_resolved.rs"]
mod resolved;
#[path = "tests_candidate.rs"]
mod candidate;
#[path = "tests_published.rs"]
mod published;
