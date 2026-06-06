//! Shared path trust-boundary helpers.
//!
//! Relative kernel-tree inputs must not accept host absolute paths or parent
//! traversal in either native Unix form or common foreign/URI forms that
//! `Path::components()` / `Path::is_absolute()` do not classify the same way on
//! this platform.

use std::path::{Component, Path};

pub(crate) fn path_is_empty_like(path: &Path) -> bool {
    path.as_os_str().is_empty() || path.to_str().is_some_and(|value| value.trim().is_empty())
}

pub(crate) fn path_is_absolute_like(path: &Path) -> bool {
    path.is_absolute() || path.to_str().is_some_and(is_absolute_path_like)
}

pub(crate) fn is_absolute_path_like(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() {
        return false;
    }

    Path::new(value).is_absolute()
        || is_file_url_absolute_path_like(value)
        || is_windows_absolute_path_like(value)
}

pub(crate) fn path_contains_parent_traversal(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component, Component::ParentDir))
        || path.to_str().is_some_and(contains_parent_traversal)
}

pub(crate) fn contains_parent_traversal(value: &str) -> bool {
    value.split(['/', '\\']).any(|component| component == "..")
}

pub(crate) fn path_is_normalized_tree_root(path: &Path) -> bool {
    path == Path::new(".")
}

pub(crate) fn normalized_relative_path_covers(parent: &Path, child: &Path) -> bool {
    path_is_normalized_tree_root(parent) || child == parent || child.starts_with(parent)
}

fn is_file_url_absolute_path_like(value: &str) -> bool {
    value
        .strip_prefix("file:")
        .is_some_and(|path| path.starts_with('/') || path.starts_with('\\'))
}

fn is_windows_absolute_path_like(value: &str) -> bool {
    let bytes = value.as_bytes();
    if is_windows_unc_absolute_path_like(value) {
        return true;
    }
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

fn is_windows_unc_absolute_path_like(value: &str) -> bool {
    if !(value.starts_with("\\\\") || value.starts_with("//")) {
        return false;
    }
    let rest = &value[2..];
    if rest.starts_with("?\\") || rest.starts_with("?/") {
        return true;
    }

    let mut components = rest
        .split(['\\', '/'])
        .filter(|component| !component.is_empty());
    components.next().is_some() && components.next().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_absolute_path_like_detects_native_file_url_and_windows_forms() {
        assert!(is_absolute_path_like("/tmp/linux/drivers/foo.c"));
        assert!(is_absolute_path_like("file:///tmp/linux/drivers/foo.c"));
        assert!(is_absolute_path_like("C:/linux/drivers/foo.c"));
        assert!(is_absolute_path_like(r"C:\linux\drivers\foo.c"));
        assert!(is_absolute_path_like(r"\\server\share\linux"));
        assert!(is_absolute_path_like("//server/share/linux"));
        assert!(is_absolute_path_like(r"\\?\C:\linux\drivers\foo.c"));
    }

    #[test]
    fn test_absolute_path_like_allows_relative_kernel_paths() {
        assert!(!is_absolute_path_like("drivers/foo.c"));
        assert!(!is_absolute_path_like("./drivers/foo.c"));
        assert!(!is_absolute_path_like("drivers/foo:C.o"));
        assert!(!is_absolute_path_like(""));
    }

    #[test]
    fn test_empty_path_like_detects_empty_and_whitespace_only_paths() {
        assert!(path_is_empty_like(Path::new("")));
        assert!(path_is_empty_like(Path::new(" ")));
        assert!(path_is_empty_like(Path::new("\t")));
        assert!(!path_is_empty_like(Path::new("drivers/foo.c")));
        assert!(!path_is_empty_like(Path::new("drivers/foo bar.c")));
    }

    #[test]
    fn test_parent_traversal_detects_native_and_foreign_separators() {
        assert!(contains_parent_traversal("../drivers/foo.c"));
        assert!(contains_parent_traversal("drivers/../foo.c"));
        assert!(contains_parent_traversal(r"..\drivers\foo.c"));
        assert!(contains_parent_traversal(r"drivers\..\foo.c"));
        assert!(path_contains_parent_traversal(Path::new(
            "drivers/../foo.c"
        )));
        assert!(path_contains_parent_traversal(Path::new(
            r"drivers\..\foo.c"
        )));
    }

    #[test]
    fn test_parent_traversal_allows_non_traversal_dots() {
        assert!(!contains_parent_traversal("drivers/foo..c"));
        assert!(!contains_parent_traversal("drivers/.../foo.c"));
        assert!(!contains_parent_traversal("drivers/.hidden/foo.c"));
        assert!(!contains_parent_traversal(""));
    }

    #[test]
    fn test_normalized_tree_root_covers_all_relative_paths() {
        assert!(path_is_normalized_tree_root(Path::new(".")));
        assert!(!path_is_normalized_tree_root(Path::new("drivers")));
        assert!(normalized_relative_path_covers(
            Path::new("."),
            Path::new("drivers/foo.c")
        ));
        assert!(normalized_relative_path_covers(
            Path::new("drivers"),
            Path::new("drivers/foo.c")
        ));
        assert!(!normalized_relative_path_covers(
            Path::new("drivers"),
            Path::new("include/linux/foo.h")
        ));
    }

    #[test]
    fn test_normalized_relative_path_covers_component_boundaries_only() {
        assert!(normalized_relative_path_covers(
            Path::new("drivers/gpu"),
            Path::new("drivers/gpu/amdgpu/foo.c")
        ));
        assert!(normalized_relative_path_covers(
            Path::new("drivers/gpu"),
            Path::new("drivers/gpu")
        ));
        assert!(!normalized_relative_path_covers(
            Path::new("drivers/gpu"),
            Path::new("drivers/gpu-next/foo.c")
        ));
        assert!(!normalized_relative_path_covers(
            Path::new("drivers/gpu"),
            Path::new("drivers/gpu2/foo.c")
        ));
    }
}
