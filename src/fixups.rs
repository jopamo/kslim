//! Deterministic reducer fixups driven by classified diagnostics.
//!
//! The current fixer surface is intentionally narrow. It handles missing
//! header diagnostics plus stale kbuild directory/object references when the
//! reducer can prove the edit from removal truth plus a read-only tree index.
//! Broader symbol diagnostics are classified but intentionally not rewritten.

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::diagnostics::ClassifiedDiagnostic;
use crate::prune::RemovalAccounting;

mod application;
mod classification;
mod planning;
mod reindex;
mod report;

pub(in crate::fixups) use application::{
    remove_missing_header_include, remove_missing_kconfig_source,
    remove_stale_kbuild_directory_reference, remove_stale_kbuild_object_reference,
    validate_fixup_result,
};
pub(in crate::fixups) use classification::{
    classified_diagnostic_proof, symbol_fallout_rejection_reason,
};
pub(in crate::fixups) use planning::available_fixups;
pub(in crate::fixups) use reindex::{
    build_fixup_index, is_tree_index_truth, proof_matches_tree_index,
};
pub use report::{AppliedFixup, FixupAttempt, FixupProof, FixupResult, SkippedFixup};

pub fn apply_classified_fixup(
    root: &Path,
    removal: &RemovalAccounting,
    diagnostic: &ClassifiedDiagnostic,
) -> Result<FixupAttempt> {
    if let Some(reason) = symbol_fallout_rejection_reason(diagnostic) {
        return Ok(FixupAttempt::Skipped(SkippedFixup {
            fixer_name: None,
            diagnostic: diagnostic.clone(),
            reason,
        }));
    }

    let index = build_fixup_index(root)?;
    for fixup in available_fixups() {
        if !fixup.applies(diagnostic) {
            continue;
        }
        let Some(result) = fixup.apply(root, &index, removal, diagnostic)? else {
            return Ok(FixupAttempt::Skipped(SkippedFixup {
                fixer_name: Some(fixup.name()),
                diagnostic: diagnostic.clone(),
                reason: String::from("applicable fixup found no remaining proven site"),
            }));
        };
        validate_fixup_result(fixup.name(), &index, diagnostic, &result)?;
        return Ok(FixupAttempt::Applied(AppliedFixup {
            fixer_name: fixup.name(),
            diagnostic: diagnostic.clone(),
            edits: result.edits,
            proof_sources: result.proof_sources,
        }));
    }

    Ok(FixupAttempt::Skipped(SkippedFixup {
        fixer_name: None,
        diagnostic: diagnostic.clone(),
        reason: String::from("no deterministic fixer matched this diagnostic"),
    }))
}

pub(in crate::fixups) fn manifest_proven_removed_header_path(
    root: &Path,
    source_dir: &Path,
    header: &str,
    removal: &RemovalAccounting,
) -> Option<PathBuf> {
    let candidates = [
        normalize_relative(&source_dir.join(header)),
        normalize_relative(&root.join(header)),
    ];

    candidates.iter().find_map(|candidate| {
        let relative = candidate.strip_prefix(root).unwrap_or(candidate.as_path());
        if !candidate.exists()
            && (removal.removed_files.iter().any(|path| path == relative)
                || removal
                    .removed_dirs
                    .iter()
                    .any(|dir| relative.starts_with(dir)))
        {
            return Some(relative.to_path_buf());
        }
        None
    })
}

fn normalize_relative(path: &Path) -> PathBuf {
    crate::kbuild::normalize_relative(path)
}

pub(in crate::fixups) fn normalize_directory_subject(path: &str) -> Option<PathBuf> {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }

    Some(normalize_relative(Path::new(trimmed)))
}

pub(in crate::fixups) fn normalize_object_subject(path: &str) -> Option<PathBuf> {
    let trimmed = path.trim();
    if trimmed.is_empty() || !trimmed.ends_with(".o") {
        return None;
    }

    Some(normalize_relative(Path::new(trimmed)))
}

pub(in crate::fixups) fn manifest_proven_removed_directory_path(
    root: &Path,
    path: &str,
    removal: &RemovalAccounting,
) -> Option<PathBuf> {
    let normalized = normalize_directory_subject(path)?;
    let candidate = root.join(&normalized);
    if candidate.exists() {
        return None;
    }

    removal
        .removed_dirs
        .iter()
        .chain(removal.empty_parents_cleaned.iter())
        .find(|removed_dir| {
            let removed_dir = removed_dir.as_path();
            normalized == removed_dir || normalized.starts_with(removed_dir)
        })
        .cloned()
}

pub(in crate::fixups) fn manifest_proven_removed_object_path(
    root: &Path,
    path: &str,
    removal: &RemovalAccounting,
) -> Option<PathBuf> {
    let normalized = normalize_object_subject(path)?;
    if root.join(&normalized).exists() {
        return None;
    }

    if let Some(removed_dir) = removal
        .removed_dirs
        .iter()
        .find(|removed_dir| normalized.starts_with(removed_dir.as_path()))
    {
        return Some(removed_dir.clone());
    }

    removal
        .removed_files
        .iter()
        .find(|removed_file| {
            removed_file.as_path() == normalized
                || removed_object_provider_path(removed_file)
                    .is_some_and(|provider| provider == normalized)
        })
        .cloned()
}

pub(in crate::fixups) fn manifest_proven_missing_kconfig_source_path(
    root: &Path,
    source_ref: &crate::tree_index::KconfigSourceReference,
    removal: &RemovalAccounting,
) -> Option<PathBuf> {
    let kconfig_dir = root.join(source_ref.file.parent().unwrap_or(Path::new("")));
    let primary = if source_ref.relative {
        normalize_relative(&kconfig_dir.join(&source_ref.source))
    } else {
        normalize_relative(&root.join(&source_ref.source))
    };
    let fallback = if source_ref.relative {
        normalize_relative(&root.join(&source_ref.source))
    } else {
        normalize_relative(&kconfig_dir.join(&source_ref.source))
    };

    [primary, fallback].into_iter().find_map(|candidate| {
        let relative = candidate.strip_prefix(root).unwrap_or(candidate.as_path());
        if candidate.exists() {
            return None;
        }

        if let Some(removed_file) = removal
            .removed_files
            .iter()
            .find(|removed_file| removed_file.as_path() == relative)
        {
            return Some(removed_file.clone());
        }

        removal
            .removed_dirs
            .iter()
            .find(|removed_dir| relative.starts_with(removed_dir.as_path()))
            .cloned()
    })
}

fn removed_object_provider_path(path: &Path) -> Option<PathBuf> {
    if path.extension().and_then(|ext| ext.to_str()) == Some("o") {
        return Some(path.to_path_buf());
    }

    if is_source_like(path) {
        return Some(path.with_extension("o"));
    }

    let text = path.to_string_lossy();
    let stem = text.strip_suffix(".o_shipped")?;
    Some(PathBuf::from(format!("{stem}.o")))
}

fn is_source_like(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext,
                "c" | "S" | "s" | "rs" | "cc" | "cpp" | "cxx" | "m" | "mm"
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::application::write_proven_fixup_rewrite;
    use super::planning::{
        Fixup, MissingHeaderIncludeFixup, MissingKconfigSourceFixup, StaleKbuildDirectoryFixup,
        StaleKbuildObjectFixup,
    };
    use crate::diagnostics::ClassifiedDiagnostic;
    use crate::edit_reason::{
        DiagnosticClass, EditProofSource, EditReason, EditRecord, LineRange,
    };
    use crate::tree_index::TreeIndex;

    fn expect_applied(attempt: FixupAttempt) -> AppliedFixup {
        match attempt {
            FixupAttempt::Applied(applied) => applied,
            FixupAttempt::Skipped(skipped) => {
                panic!("expected applied fixup, got skipped: {:?}", skipped)
            }
        }
    }

    fn expect_skipped(attempt: FixupAttempt) -> SkippedFixup {
        match attempt {
            FixupAttempt::Applied(applied) => {
                panic!("expected skipped fixup, got applied: {:?}", applied)
            }
            FixupAttempt::Skipped(skipped) => skipped,
        }
    }

    #[test]
    fn test_available_fixups_expose_registered_fixup_traits() {
        let fixups = available_fixups();
        assert_eq!(fixups.len(), 4);
        assert_eq!(fixups[0].name(), "fixups.remove_missing_header_include");
        assert_eq!(fixups[1].name(), "fixups.remove_stale_kbuild_directory_ref");
        assert_eq!(fixups[2].name(), "fixups.remove_stale_kbuild_object_ref");
        assert_eq!(fixups[3].name(), "fixups.remove_missing_kconfig_source");
        assert!(fixups[0].applies(&ClassifiedDiagnostic::MissingHeader {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 1,
            header: String::from("amd/amdgpu/amdgpu_missing.h"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }));
        assert!(
            fixups[1].applies(&ClassifiedDiagnostic::MissingMakeDirectory {
                path: String::from("drivers/foo/remove/"),
                build_target: Some(String::from("modules")),
                arch: None,
                config: Some(String::from("defconfig")),
            })
        );
        assert!(fixups[2].applies(&ClassifiedDiagnostic::MissingMakeTarget {
            target: String::from("drivers/foo/remove.o"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }));
        assert!(
            fixups[3].applies(&ClassifiedDiagnostic::MissingKconfigSource {
                kconfig_file: PathBuf::from("Kconfig"),
                line: 1,
                source: String::from("drivers/foo/Kconfig"),
            })
        );
        assert!(!fixups[0].applies(&ClassifiedDiagnostic::Unknown));
        assert!(!fixups[1].applies(&ClassifiedDiagnostic::Unknown));
        assert!(
            !fixups[2].applies(&ClassifiedDiagnostic::MissingMakeTarget {
                target: String::from("drivers/foo/built-in.a"),
                build_target: Some(String::from("modules")),
                arch: None,
                config: Some(String::from("defconfig")),
            })
        );
        assert!(!fixups[3].applies(&ClassifiedDiagnostic::Unknown));
    }

    #[test]
    fn test_apply_classified_fixup_removes_include_for_removed_header() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("drivers/gpu/drm")).unwrap();
        std::fs::write(
            root.join("drivers/gpu/drm/helper.c"),
            "#include <amd/amdgpu/amdgpu_missing.h>\nint helper;\n",
        )
        .unwrap();

        let applied = expect_applied(
            apply_classified_fixup(
                root,
                &RemovalAccounting {
                    removed_files: Vec::new(),
                    removed_dirs: vec![PathBuf::from("drivers/gpu/drm/amd/amdgpu")],
                    removed_config_symbols: Vec::new(),
                    empty_parents_cleaned: Vec::new(),
                    missing_paths: Vec::new(),
                },
                &ClassifiedDiagnostic::MissingHeader {
                    source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
                    line: 1,
                    header: String::from("amd/amdgpu/amdgpu_missing.h"),
                    build_target: Some(String::from("modules")),
                    arch: None,
                    config: Some(String::from("defconfig")),
                },
            )
            .unwrap(),
        );
        let edits = applied.edits;

        assert_eq!(edits.len(), 1);
        let rewritten = std::fs::read_to_string(root.join("drivers/gpu/drm/helper.c")).unwrap();
        assert_eq!(rewritten, "int helper;\n");
    }

    #[test]
    fn test_apply_classified_fixup_is_idempotent_on_missing_header_second_run() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("drivers/gpu/drm")).unwrap();
        std::fs::write(
            root.join("drivers/gpu/drm/helper.c"),
            "#include <amd/amdgpu/amdgpu_missing.h>\nint helper;\n",
        )
        .unwrap();
        let diagnostic = ClassifiedDiagnostic::MissingHeader {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 1,
            header: String::from("amd/amdgpu/amdgpu_missing.h"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        };
        let removal = RemovalAccounting {
            removed_files: Vec::new(),
            removed_dirs: vec![PathBuf::from("drivers/gpu/drm/amd/amdgpu")],
            removed_config_symbols: Vec::new(),
            empty_parents_cleaned: Vec::new(),
            missing_paths: Vec::new(),
        };

        let first = apply_classified_fixup(root, &removal, &diagnostic).unwrap();
        assert!(matches!(first, FixupAttempt::Applied(_)));
        let second = apply_classified_fixup(root, &removal, &diagnostic).unwrap();
        let skipped = expect_skipped(second);
        assert_eq!(
            skipped.fixer_name,
            Some("fixups.remove_missing_header_include")
        );
    }

    #[test]
    fn test_missing_header_fixup_trait_apply_returns_fixup_result() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("drivers/gpu/drm")).unwrap();
        std::fs::write(
            root.join("drivers/gpu/drm/helper.c"),
            "#include <amd/amdgpu/amdgpu_missing.h>\nint helper;\n",
        )
        .unwrap();
        let index = TreeIndex::build(root, &()).unwrap();

        let result = MissingHeaderIncludeFixup
            .apply(
                root,
                &index,
                &RemovalAccounting {
                    removed_files: Vec::new(),
                    removed_dirs: vec![PathBuf::from("drivers/gpu/drm/amd/amdgpu")],
                    removed_config_symbols: Vec::new(),
                    empty_parents_cleaned: Vec::new(),
                    missing_paths: Vec::new(),
                },
                &ClassifiedDiagnostic::MissingHeader {
                    source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
                    line: 1,
                    header: String::from("amd/amdgpu/amdgpu_missing.h"),
                    build_target: Some(String::from("modules")),
                    arch: None,
                    config: Some(String::from("defconfig")),
                },
            )
            .unwrap()
            .unwrap();

        assert_eq!(result.edits.len(), 1);
        assert_eq!(
            result.proof_sources,
            vec![
                FixupProof::ManifestPath {
                    path: PathBuf::from("drivers/gpu/drm/amd/amdgpu/amdgpu_missing.h"),
                },
                FixupProof::TreeIndexIncludeSite {
                    file: PathBuf::from("drivers/gpu/drm/helper.c"),
                    line: 1,
                    target: String::from("amd/amdgpu/amdgpu_missing.h"),
                },
                FixupProof::ClassifiedDiagnostic {
                    class: DiagnosticClass::MissingHeader,
                    file: Some(PathBuf::from("drivers/gpu/drm/helper.c")),
                    line: Some(1),
                    subject: Some(String::from("amd/amdgpu/amdgpu_missing.h")),
                },
            ]
        );
        assert_eq!(
            std::fs::read_to_string(root.join("drivers/gpu/drm/helper.c")).unwrap(),
            "int helper;\n"
        );
    }

    #[test]
    fn test_apply_classified_fixup_removes_stale_kbuild_directory_ref() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
        std::fs::write(
            root.join("drivers/foo/Makefile"),
            "subdir-y += keep/ remove/ # keep comment\n",
        )
        .unwrap();

        let applied = expect_applied(
            apply_classified_fixup(
                root,
                &RemovalAccounting {
                    removed_files: Vec::new(),
                    removed_dirs: vec![PathBuf::from("drivers/foo/remove")],
                    removed_config_symbols: Vec::new(),
                    empty_parents_cleaned: Vec::new(),
                    missing_paths: Vec::new(),
                },
                &ClassifiedDiagnostic::MissingMakeDirectory {
                    path: String::from("drivers/foo/remove/"),
                    build_target: Some(String::from("modules")),
                    arch: None,
                    config: Some(String::from("defconfig")),
                },
            )
            .unwrap(),
        );
        let edits = applied.edits;

        assert_eq!(edits.len(), 1);
        assert_eq!(
            std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
            "subdir-y += keep/ # keep comment\n"
        );
        assert!(matches!(
            edits[0].reason,
            EditReason::BuildDiagnostic {
                class: DiagnosticClass::StaleKbuildDirectoryRef,
            }
        ));
    }

    #[test]
    fn test_apply_classified_fixup_is_idempotent_on_stale_kbuild_directory_second_run() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
        std::fs::write(root.join("drivers/foo/Makefile"), "subdir-y += remove/\n").unwrap();
        let diagnostic = ClassifiedDiagnostic::MissingMakeDirectory {
            path: String::from("drivers/foo/remove/"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        };
        let removal = RemovalAccounting {
            removed_files: Vec::new(),
            removed_dirs: vec![PathBuf::from("drivers/foo/remove")],
            removed_config_symbols: Vec::new(),
            empty_parents_cleaned: Vec::new(),
            missing_paths: Vec::new(),
        };

        let first = apply_classified_fixup(root, &removal, &diagnostic).unwrap();
        assert!(matches!(first, FixupAttempt::Applied(_)));
        let second = apply_classified_fixup(root, &removal, &diagnostic).unwrap();
        let skipped = expect_skipped(second);
        assert_eq!(
            skipped.fixer_name,
            Some("fixups.remove_stale_kbuild_directory_ref")
        );
    }

    #[test]
    fn test_stale_kbuild_directory_fixup_trait_apply_returns_fixup_result() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
        std::fs::write(root.join("drivers/foo/Makefile"), "subdir-y += remove/\n").unwrap();
        let index = TreeIndex::build(root, &()).unwrap();

        let result = StaleKbuildDirectoryFixup
            .apply(
                root,
                &index,
                &RemovalAccounting {
                    removed_files: Vec::new(),
                    removed_dirs: vec![PathBuf::from("drivers/foo/remove")],
                    removed_config_symbols: Vec::new(),
                    empty_parents_cleaned: Vec::new(),
                    missing_paths: Vec::new(),
                },
                &ClassifiedDiagnostic::MissingMakeDirectory {
                    path: String::from("drivers/foo/remove/"),
                    build_target: Some(String::from("modules")),
                    arch: None,
                    config: Some(String::from("defconfig")),
                },
            )
            .unwrap()
            .unwrap();

        assert_eq!(result.edits.len(), 1);
        assert!(result.proof_sources.iter().any(|proof| matches!(
            proof,
            FixupProof::ManifestPath { path }
                if path == Path::new("drivers/foo/remove")
        )));
        assert!(result.proof_sources.iter().any(|proof| matches!(
            proof,
            FixupProof::TreeIndexKbuildDirectoryRef {
                file,
                line,
                assignment_lhs,
                directory,
                resolved_path,
            }
                if file == Path::new("drivers/foo/Makefile")
                    && *line == 1
                    && assignment_lhs == "subdir-y"
                    && directory == "remove/"
                    && resolved_path == Path::new("drivers/foo/remove")
        )));
        assert!(result
            .proof_sources
            .iter()
            .any(
                |proof| proof.matches_diagnostic(&ClassifiedDiagnostic::MissingMakeDirectory {
                    path: String::from("drivers/foo/remove/"),
                    build_target: Some(String::from("modules")),
                    arch: None,
                    config: Some(String::from("defconfig")),
                })
            ));
        assert_eq!(
            std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
            "# kslim: removed stale make refs from subdir-y\n"
        );
    }

    #[test]
    fn test_apply_classified_fixup_removes_stale_kbuild_object_ref() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
        std::fs::write(
            root.join("drivers/foo/Makefile"),
            "obj-y += keep.o remove.o # keep comment\n",
        )
        .unwrap();

        let applied = expect_applied(
            apply_classified_fixup(
                root,
                &RemovalAccounting {
                    removed_files: vec![PathBuf::from("drivers/foo/remove.c")],
                    removed_dirs: Vec::new(),
                    removed_config_symbols: Vec::new(),
                    empty_parents_cleaned: Vec::new(),
                    missing_paths: Vec::new(),
                },
                &ClassifiedDiagnostic::MissingMakeTarget {
                    target: String::from("drivers/foo/remove.o"),
                    build_target: Some(String::from("modules")),
                    arch: None,
                    config: Some(String::from("defconfig")),
                },
            )
            .unwrap(),
        );
        let edits = applied.edits;

        assert_eq!(edits.len(), 1);
        assert_eq!(
            std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
            "obj-y += keep.o # keep comment\n"
        );
        assert!(matches!(
            edits[0].reason,
            EditReason::BuildDiagnostic {
                class: DiagnosticClass::StaleKbuildObjectRef,
            }
        ));
    }

    #[test]
    fn test_apply_classified_fixup_is_idempotent_on_stale_kbuild_object_second_run() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
        std::fs::write(root.join("drivers/foo/Makefile"), "obj-y += remove.o\n").unwrap();
        let diagnostic = ClassifiedDiagnostic::MissingMakeTarget {
            target: String::from("drivers/foo/remove.o"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        };
        let removal = RemovalAccounting {
            removed_files: vec![PathBuf::from("drivers/foo/remove.c")],
            removed_dirs: Vec::new(),
            removed_config_symbols: Vec::new(),
            empty_parents_cleaned: Vec::new(),
            missing_paths: Vec::new(),
        };

        let first = apply_classified_fixup(root, &removal, &diagnostic).unwrap();
        assert!(matches!(first, FixupAttempt::Applied(_)));
        let second = apply_classified_fixup(root, &removal, &diagnostic).unwrap();
        let skipped = expect_skipped(second);
        assert_eq!(
            skipped.fixer_name,
            Some("fixups.remove_stale_kbuild_object_ref")
        );
    }

    #[test]
    fn test_stale_kbuild_object_fixup_trait_apply_returns_fixup_result() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
        std::fs::write(root.join("drivers/foo/Makefile"), "obj-y += remove.o\n").unwrap();
        let index = TreeIndex::build(root, &()).unwrap();

        let result = StaleKbuildObjectFixup
            .apply(
                root,
                &index,
                &RemovalAccounting {
                    removed_files: vec![PathBuf::from("drivers/foo/remove.c")],
                    removed_dirs: Vec::new(),
                    removed_config_symbols: Vec::new(),
                    empty_parents_cleaned: Vec::new(),
                    missing_paths: Vec::new(),
                },
                &ClassifiedDiagnostic::MissingMakeTarget {
                    target: String::from("drivers/foo/remove.o"),
                    build_target: Some(String::from("modules")),
                    arch: None,
                    config: Some(String::from("defconfig")),
                },
            )
            .unwrap()
            .unwrap();

        assert_eq!(result.edits.len(), 1);
        assert!(result.proof_sources.iter().any(|proof| matches!(
            proof,
            FixupProof::ManifestPath { path }
                if path == Path::new("drivers/foo/remove.c")
        )));
        assert!(result.proof_sources.iter().any(|proof| matches!(
            proof,
            FixupProof::TreeIndexKbuildObjectRef {
                file,
                line,
                assignment_lhs,
                object,
                resolved_path,
            }
                if file == Path::new("drivers/foo/Makefile")
                    && *line == 1
                    && assignment_lhs == "obj-y"
                    && object == "remove.o"
                    && resolved_path == Path::new("drivers/foo/remove.o")
        )));
        assert!(result
            .proof_sources
            .iter()
            .any(
                |proof| proof.matches_diagnostic(&ClassifiedDiagnostic::MissingMakeTarget {
                    target: String::from("drivers/foo/remove.o"),
                    build_target: Some(String::from("modules")),
                    arch: None,
                    config: Some(String::from("defconfig")),
                })
            ));
        assert_eq!(
            std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
            "# kslim: removed stale make refs from obj-y\n"
        );
    }

    #[test]
    fn test_apply_classified_fixup_removes_missing_kconfig_source() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
        std::fs::write(
            root.join("Kconfig"),
            "  source \"drivers/foo/Kconfig\" # keep comment\n",
        )
        .unwrap();

        let applied = expect_applied(
            apply_classified_fixup(
                root,
                &RemovalAccounting {
                    removed_files: vec![PathBuf::from("drivers/foo/Kconfig")],
                    removed_dirs: Vec::new(),
                    removed_config_symbols: Vec::new(),
                    empty_parents_cleaned: Vec::new(),
                    missing_paths: Vec::new(),
                },
                &ClassifiedDiagnostic::MissingKconfigSource {
                    kconfig_file: PathBuf::from("Kconfig"),
                    line: 1,
                    source: String::from("drivers/foo/Kconfig"),
                },
            )
            .unwrap(),
        );
        let edits = applied.edits;

        assert_eq!(edits.len(), 1);
        assert_eq!(
            std::fs::read_to_string(root.join("Kconfig")).unwrap(),
            "  # kslim: removed source \"drivers/foo/Kconfig\" # keep comment\n"
        );
        assert!(matches!(
            edits[0].reason,
            EditReason::BuildDiagnostic {
                class: DiagnosticClass::MissingKconfigSource,
            }
        ));
    }

    #[test]
    fn test_apply_classified_fixup_is_idempotent_on_missing_kconfig_source_second_run() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
        std::fs::write(root.join("Kconfig"), "source \"drivers/foo/Kconfig\"\n").unwrap();
        let diagnostic = ClassifiedDiagnostic::MissingKconfigSource {
            kconfig_file: PathBuf::from("Kconfig"),
            line: 1,
            source: String::from("drivers/foo/Kconfig"),
        };
        let removal = RemovalAccounting {
            removed_files: vec![PathBuf::from("drivers/foo/Kconfig")],
            removed_dirs: Vec::new(),
            removed_config_symbols: Vec::new(),
            empty_parents_cleaned: Vec::new(),
            missing_paths: Vec::new(),
        };

        let first = apply_classified_fixup(root, &removal, &diagnostic).unwrap();
        assert!(matches!(first, FixupAttempt::Applied(_)));
        let second = apply_classified_fixup(root, &removal, &diagnostic).unwrap();
        let skipped = expect_skipped(second);
        assert_eq!(
            skipped.fixer_name,
            Some("fixups.remove_missing_kconfig_source")
        );
    }

    #[test]
    fn test_missing_kconfig_source_fixup_trait_apply_returns_fixup_result() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
        std::fs::write(root.join("Kconfig"), "source \"drivers/foo/Kconfig\"\n").unwrap();
        let index = TreeIndex::build(root, &()).unwrap();

        let result = MissingKconfigSourceFixup
            .apply(
                root,
                &index,
                &RemovalAccounting {
                    removed_files: vec![PathBuf::from("drivers/foo/Kconfig")],
                    removed_dirs: Vec::new(),
                    removed_config_symbols: Vec::new(),
                    empty_parents_cleaned: Vec::new(),
                    missing_paths: Vec::new(),
                },
                &ClassifiedDiagnostic::MissingKconfigSource {
                    kconfig_file: PathBuf::from("Kconfig"),
                    line: 1,
                    source: String::from("drivers/foo/Kconfig"),
                },
            )
            .unwrap()
            .unwrap();

        assert_eq!(result.edits.len(), 1);
        assert!(result.proof_sources.iter().any(|proof| matches!(
            proof,
            FixupProof::ManifestPath { path } if path == Path::new("drivers/foo/Kconfig")
        )));
        assert!(result.proof_sources.iter().any(|proof| matches!(
            proof,
            FixupProof::TreeIndexKconfigSourceRef {
                file,
                line,
                source,
                optional,
                relative,
            }
                if file == Path::new("Kconfig")
                    && *line == 1
                    && source == "drivers/foo/Kconfig"
                    && !optional
                    && !relative
        )));
        assert!(result
            .proof_sources
            .iter()
            .any(
                |proof| proof.matches_diagnostic(&ClassifiedDiagnostic::MissingKconfigSource {
                    kconfig_file: PathBuf::from("Kconfig"),
                    line: 1,
                    source: String::from("drivers/foo/Kconfig"),
                })
            ));
        assert_eq!(
            std::fs::read_to_string(root.join("Kconfig")).unwrap(),
            "# kslim: removed source \"drivers/foo/Kconfig\"\n"
        );
    }

    #[test]
    fn test_validate_fixup_result_rejects_missing_manifest_truth_proof() {
        let err = validate_fixup_result(
            "test.fixup",
            &TreeIndex::default(),
            &ClassifiedDiagnostic::MissingHeader {
                source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
                line: 1,
                header: String::from("amd/amdgpu/amdgpu_missing.h"),
                build_target: Some(String::from("modules")),
                arch: None,
                config: Some(String::from("defconfig")),
            },
            &FixupResult::new(
                vec![EditRecord::new(
                    PathBuf::from("drivers/gpu/drm/helper.c"),
                    Some(LineRange { start: 1, end: 1 }),
                    String::from("#include <amd/amdgpu/amdgpu_missing.h>\n"),
                    String::new(),
                    EditReason::BuildDiagnostic {
                        class: DiagnosticClass::MissingHeader,
                    },
                    EditProofSource::ClassifiedDiagnostic {
                        diagnostic_id: DiagnosticClass::MissingHeader.into(),
                    },
                    "test.fixup",
                )],
                vec![FixupProof::ClassifiedDiagnostic {
                    class: DiagnosticClass::MissingHeader,
                    file: Some(PathBuf::from("drivers/gpu/drm/helper.c")),
                    line: Some(1),
                    subject: Some(String::from("amd/amdgpu/amdgpu_missing.h")),
                }],
            ),
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("without manifest truth proof"));
    }

    #[test]
    fn test_validate_fixup_result_rejects_missing_tree_index_truth_proof() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("drivers/gpu/drm")).unwrap();
        std::fs::write(
            root.join("drivers/gpu/drm/helper.c"),
            "#include <amd/amdgpu/amdgpu_missing.h>\nint helper;\n",
        )
        .unwrap();
        let index = TreeIndex::build(root, &()).unwrap();

        let err = validate_fixup_result(
            "test.fixup",
            &index,
            &ClassifiedDiagnostic::MissingHeader {
                source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
                line: 1,
                header: String::from("amd/amdgpu/amdgpu_missing.h"),
                build_target: Some(String::from("modules")),
                arch: None,
                config: Some(String::from("defconfig")),
            },
            &FixupResult::new(
                vec![EditRecord::new(
                    PathBuf::from("drivers/gpu/drm/helper.c"),
                    Some(LineRange { start: 1, end: 1 }),
                    String::from("#include <amd/amdgpu/amdgpu_missing.h>\n"),
                    String::new(),
                    EditReason::BuildDiagnostic {
                        class: DiagnosticClass::MissingHeader,
                    },
                    EditProofSource::ClassifiedDiagnostic {
                        diagnostic_id: DiagnosticClass::MissingHeader.into(),
                    },
                    "test.fixup",
                )],
                vec![
                    FixupProof::ManifestPath {
                        path: PathBuf::from("drivers/gpu/drm/amd/amdgpu/amdgpu_missing.h"),
                    },
                    FixupProof::ClassifiedDiagnostic {
                        class: DiagnosticClass::MissingHeader,
                        file: Some(PathBuf::from("drivers/gpu/drm/helper.c")),
                        line: Some(1),
                        subject: Some(String::from("amd/amdgpu/amdgpu_missing.h")),
                    },
                ],
            ),
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("without tree index truth proof"));
    }

    #[test]
    fn test_validate_fixup_result_rejects_missing_classified_diagnostic_truth_proof() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("drivers/gpu/drm")).unwrap();
        std::fs::write(
            root.join("drivers/gpu/drm/helper.c"),
            "#include <amd/amdgpu/amdgpu_missing.h>\nint helper;\n",
        )
        .unwrap();
        let index = TreeIndex::build(root, &()).unwrap();

        let err = validate_fixup_result(
            "test.fixup",
            &index,
            &ClassifiedDiagnostic::MissingHeader {
                source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
                line: 1,
                header: String::from("amd/amdgpu/amdgpu_missing.h"),
                build_target: Some(String::from("modules")),
                arch: None,
                config: Some(String::from("defconfig")),
            },
            &FixupResult::new(
                vec![EditRecord::new(
                    PathBuf::from("drivers/gpu/drm/helper.c"),
                    Some(LineRange { start: 1, end: 1 }),
                    String::from("#include <amd/amdgpu/amdgpu_missing.h>\n"),
                    String::new(),
                    EditReason::BuildDiagnostic {
                        class: DiagnosticClass::MissingHeader,
                    },
                    EditProofSource::ClassifiedDiagnostic {
                        diagnostic_id: DiagnosticClass::MissingHeader.into(),
                    },
                    "test.fixup",
                )],
                vec![
                    FixupProof::ManifestPath {
                        path: PathBuf::from("drivers/gpu/drm/amd/amdgpu/amdgpu_missing.h"),
                    },
                    FixupProof::TreeIndexIncludeSite {
                        file: PathBuf::from("drivers/gpu/drm/helper.c"),
                        line: 1,
                        target: String::from("amd/amdgpu/amdgpu_missing.h"),
                    },
                ],
            ),
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("without classified diagnostic truth proof"));
    }

    #[test]
    fn test_write_proven_fixup_rewrite_rejects_missing_proof_before_mutation() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("drivers/gpu/drm")).unwrap();
        let path = root.join("drivers/gpu/drm/helper.c");
        let original = "#include <amd/amdgpu/amdgpu_missing.h>\nint helper;\n";
        std::fs::write(&path, original).unwrap();
        let index = TreeIndex::build(root, &()).unwrap();
        let diagnostic = ClassifiedDiagnostic::MissingHeader {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 1,
            header: String::from("amd/amdgpu/amdgpu_missing.h"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        };
        let edits = vec![EditRecord::new(
            PathBuf::from("drivers/gpu/drm/helper.c"),
            Some(LineRange { start: 1, end: 1 }),
            String::from("#include <amd/amdgpu/amdgpu_missing.h>\n"),
            String::new(),
            EditReason::BuildDiagnostic {
                class: DiagnosticClass::MissingHeader,
            },
            EditProofSource::ClassifiedDiagnostic {
                diagnostic_id: DiagnosticClass::MissingHeader.into(),
            },
            "fixups.remove_missing_header_include",
        )];

        let err = write_proven_fixup_rewrite(
            root,
            &path,
            "int helper;\n",
            edits,
            &[classified_diagnostic_proof(&diagnostic)],
            "fixups.remove_missing_header_include",
            &index,
            &diagnostic,
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("without manifest truth proof"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }

    #[test]
    fn test_apply_classified_fixup_refuses_broad_undeclared_identifier_edit() {
        let tmp = tempfile::tempdir().unwrap();

        let applied = apply_classified_fixup(
            tmp.path(),
            &RemovalAccounting::default(),
            &ClassifiedDiagnostic::UndeclaredIdentifier {
                source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
                line: 7,
                symbol: String::from("amdgpu_magic"),
                build_target: Some(String::from("modules")),
                arch: None,
                config: Some(String::from("defconfig")),
            },
        )
        .unwrap();

        let skipped = expect_skipped(applied);
        assert_eq!(skipped.fixer_name, None);
        assert!(skipped.reason.contains("symbol fallout"));
        assert!(skipped
            .reason
            .contains("broad speculative edits are forbidden"));
    }

    #[test]
    fn test_apply_classified_fixup_refuses_non_object_missing_make_target_edit() {
        let tmp = tempfile::tempdir().unwrap();

        let applied = apply_classified_fixup(
            tmp.path(),
            &RemovalAccounting::default(),
            &ClassifiedDiagnostic::MissingMakeTarget {
                target: String::from("drivers/foo/built-in.a"),
                build_target: Some(String::from("modules")),
                arch: None,
                config: Some(String::from("defconfig")),
            },
        )
        .unwrap();

        let skipped = expect_skipped(applied);
        assert_eq!(skipped.fixer_name, None);
    }

    #[test]
    fn test_apply_classified_fixup_rejects_ambiguous_missing_make_target_edit() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
        std::fs::write(root.join("Makefile"), "obj-y += drivers/foo/remove.o\n").unwrap();
        std::fs::write(root.join("drivers/foo/Makefile"), "obj-y += remove.o\n").unwrap();

        let err = apply_classified_fixup(
            root,
            &RemovalAccounting {
                removed_files: vec![PathBuf::from("drivers/foo/remove.c")],
                removed_dirs: Vec::new(),
                removed_config_symbols: Vec::new(),
                empty_parents_cleaned: Vec::new(),
                missing_paths: Vec::new(),
            },
            &ClassifiedDiagnostic::MissingMakeTarget {
                target: String::from("drivers/foo/remove.o"),
                build_target: Some(String::from("modules")),
                arch: None,
                config: Some(String::from("defconfig")),
            },
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("matching tree-index references were found"));
    }

    #[test]
    fn test_apply_classified_fixup_rejects_unproven_missing_kconfig_source_edit() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(
            root.join("Kconfig"),
            "source \"drivers/gpu/drm/amd/amdgpu/Kconfig\"\n",
        )
        .unwrap();

        let err = apply_classified_fixup(
            root,
            &RemovalAccounting::default(),
            &ClassifiedDiagnostic::MissingKconfigSource {
                kconfig_file: PathBuf::from("Kconfig"),
                line: 1,
                source: String::from("drivers/gpu/drm/amd/amdgpu/Kconfig"),
            },
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("not proven removed"));
    }

    #[test]
    fn test_apply_classified_fixup_rejects_ambiguous_missing_make_directory_edit() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
        std::fs::write(root.join("Makefile"), "subdir-y += drivers/foo/remove/\n").unwrap();
        std::fs::write(root.join("drivers/foo/Makefile"), "subdir-y += remove/\n").unwrap();

        let err = apply_classified_fixup(
            root,
            &RemovalAccounting {
                removed_files: Vec::new(),
                removed_dirs: vec![PathBuf::from("drivers/foo/remove")],
                removed_config_symbols: Vec::new(),
                empty_parents_cleaned: Vec::new(),
                missing_paths: Vec::new(),
            },
            &ClassifiedDiagnostic::MissingMakeDirectory {
                path: String::from("drivers/foo/remove/"),
                build_target: Some(String::from("modules")),
                arch: None,
                config: Some(String::from("defconfig")),
            },
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("matching tree-index references were found"));
    }

    #[test]
    fn test_apply_classified_fixup_refuses_broad_implicit_declaration_edit() {
        let tmp = tempfile::tempdir().unwrap();

        let applied = apply_classified_fixup(
            tmp.path(),
            &RemovalAccounting::default(),
            &ClassifiedDiagnostic::ImplicitDeclaration {
                source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
                line: 7,
                symbol: String::from("amdgpu_magic"),
                build_target: Some(String::from("modules")),
                arch: None,
                config: Some(String::from("defconfig")),
            },
        )
        .unwrap();

        let skipped = expect_skipped(applied);
        assert_eq!(skipped.fixer_name, None);
        assert!(skipped.reason.contains("symbol fallout"));
        assert!(skipped
            .reason
            .contains("broad speculative edits are forbidden"));
    }

    #[test]
    fn test_apply_classified_fixup_refuses_broad_undefined_reference_edit() {
        let tmp = tempfile::tempdir().unwrap();

        let applied = apply_classified_fixup(
            tmp.path(),
            &RemovalAccounting::default(),
            &ClassifiedDiagnostic::UndefinedReference {
                source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
                symbol: String::from("amdgpu_magic"),
                build_target: Some(String::from("modules")),
                arch: None,
                config: Some(String::from("defconfig")),
            },
        )
        .unwrap();

        let skipped = expect_skipped(applied);
        assert_eq!(skipped.fixer_name, None);
        assert!(skipped.reason.contains("symbol fallout"));
        assert!(skipped
            .reason
            .contains("broad speculative edits are forbidden"));
    }
}
