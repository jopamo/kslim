//! Stable identifier construction.

use sha2::{Digest, Sha256};

pub(crate) fn stable_id(kind: &str, fields: &[(&str, &str)]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(kind.as_bytes());
    hasher.update(b"\0");
    for (name, value) in fields {
        hasher.update(name.as_bytes());
        hasher.update(b"\0");
        hasher.update(value.len().to_string().as_bytes());
        hasher.update(b"\0");
        hasher.update(value.as_bytes());
        hasher.update(b"\0");
    }
    format!("{}-{}", kind, hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_id_includes_kind_names_lengths_and_values() {
        let first = stable_id("item", &[("a", "bc")]);
        let second = stable_id("item", &[("ab", "c")]);
        let different_kind = stable_id("other", &[("a", "bc")]);

        assert!(first.starts_with("item-"));
        assert_ne!(first, second);
        assert_ne!(first, different_kind);
    }
}

