//! Stable digest helpers.

use sha2::{Digest, Sha256};

pub(crate) fn sha256_hex(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    hex::encode(hasher.finalize())
}

#[allow(dead_code)]
pub(crate) fn prefixed_sha256_hex(prefix: &str, value: &str) -> String {
    format!("{prefix}-{}", sha256_hex(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_hex_is_stable_for_identical_input() {
        assert_eq!(sha256_hex("kslim"), sha256_hex("kslim"));
        assert_ne!(sha256_hex("kslim"), sha256_hex("kforge"));
        assert_eq!(
            prefixed_sha256_hex("fingerprint", "kslim"),
            format!("fingerprint-{}", sha256_hex("kslim"))
        );
    }
}

