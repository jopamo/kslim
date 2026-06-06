//! Shared result aliases.

#[allow(dead_code)]
pub(crate) type KslimResult<T> = anyhow::Result<T>;

#[allow(dead_code)]
pub(crate) type StdResult<T, E> = std::result::Result<T, E>;

