//! Shared foundation helpers.
//!
//! `core` owns crate-wide primitives that do not depend on product
//! subsystems: error types, result aliases, stable identifiers, fingerprint
//! hashing, and stable line serialization helpers.

mod error;
mod fingerprint;
mod result;
mod serialization;
mod stable_id;

#[allow(unused_imports)]
pub(crate) use error::KslimError;
#[allow(unused_imports)]
pub(crate) use fingerprint::{prefixed_sha256_hex, sha256_hex};
#[allow(unused_imports)]
pub(crate) use result::{KslimResult, StdResult};
#[allow(unused_imports)]
pub(crate) use serialization::{
    append_stable_key_value_line, bool_token, escape_stable_value,
};
#[allow(unused_imports)]
pub(crate) use stable_id::stable_id;

