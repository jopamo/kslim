//! Crate-wide error model.

use std::fmt;

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) enum KslimError {
    NotManaged(String),
    OutputExists(String),
    ProfileNotFound(String),
    RefNotFound(String),
    GitNotFound,
    UpstreamNotInitialized(String),
}

impl fmt::Display for KslimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KslimError::NotManaged(p) => write!(f, "output path {p} is not managed by kslim"),
            KslimError::OutputExists(p) => {
                write!(f, "output path {p} already exists and is not empty")
            }
            KslimError::ProfileNotFound(p) => write!(f, "profile {p} not found"),
            KslimError::RefNotFound(r) => write!(f, "upstream ref {r} not found"),
            KslimError::GitNotFound => write!(f, "git is required but not found in PATH"),
            KslimError::UpstreamNotInitialized(p) => write!(
                f,
                "upstream repository at {p} is not accessible (run `kslim upstream sync` first)"
            ),
        }
    }
}

impl std::error::Error for KslimError {}

