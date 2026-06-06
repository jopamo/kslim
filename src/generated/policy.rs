//! Generated include-root policy.

use anyhow::Result;
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use crate::path_policy::{path_contains_parent_traversal, path_is_absolute_like, path_is_empty_like};

pub(crate) fn normalize_generated_include_roots(roots: &[PathBuf]) -> Result<BTreeSet<PathBuf>> {
    let mut normalized_roots = BTreeSet::new();

    for root in roots {
        if path_is_empty_like(root) {
            anyhow::bail!("generated include roots must not be empty");
        }
        if path_is_absolute_like(root) {
            anyhow::bail!(
                "generated include roots must be relative to the tree: {}",
                root.display()
            );
        }
        if path_contains_parent_traversal(root) {
            anyhow::bail!(
                "generated include roots must not contain '..': {}",
                root.display()
            );
        }

        let mut normalized = PathBuf::new();
        for component in root.components() {
            match component {
                Component::CurDir => {}
                Component::Normal(part) => normalized.push(part),
                Component::ParentDir => {
                    anyhow::bail!(
                        "generated include roots must not contain '..': {}",
                        root.display()
                    );
                }
                Component::RootDir | Component::Prefix(_) => {
                    anyhow::bail!(
                        "generated include roots must be relative to the tree: {}",
                        root.display()
                    );
                }
            }
        }

        if normalized.as_os_str().is_empty() {
            anyhow::bail!("generated include roots must not resolve to the tree root");
        }
        normalized_roots.insert(normalized);
    }

    Ok(normalized_roots)
}

pub(crate) fn is_generated_include_header_path(path: &Path) -> bool {
    if path.starts_with("include/generated") {
        return true;
    }

    let mut components = path.components();
    matches!(
        (
            components.next().and_then(|part| part.as_os_str().to_str()),
            components.next().and_then(|part| part.as_os_str().to_str()),
            components.next().and_then(|part| part.as_os_str().to_str()),
            components.next().and_then(|part| part.as_os_str().to_str()),
        ),
        (
            Some("arch"),
            Some(_arch),
            Some("include"),
            Some("generated")
        )
    )
}
