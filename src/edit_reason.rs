use sha2::{Digest, Sha256};
use std::path::Path;
use std::path::PathBuf;

pub type RelativePathBuf = PathBuf;
pub type TextSpan = LineRange;

mod proof_source;
mod reason;
mod render;
mod serialization;
mod validation;

#[allow(unused_imports)]
pub use proof_source::{
    DiagnosticId, EditProofSource, EditProofSourceKind, IndexKind, KconfigSolverKey, ReferenceKind,
    RemovalKey,
};
pub use reason::{DiagnosticClass, EditReason};
pub use render::{
    grouped_edit_record_refs_by_reason, sort_edit_records, sorted_edit_record_refs,
};
pub(in crate::edit_reason) use render::{bounded_edit_content, payload_token};
pub use serialization::{
    proof_source_kind_for_reason_key, validate_reported_no_speculative_fallout_edit,
    validate_reported_proof_source_payload_for_reason,
};
#[allow(unused_imports)]
pub use validation::{
    ensure_edit_records_for_mutation, validate_edit_records, validate_edit_records_with_policy,
    validate_no_speculative_fallout_edit_records, validate_reasoned_edit_records,
    write_verified_rewrite, EditValidationPolicy,
};
pub(in crate::edit_reason) use validation::{
    validate_non_empty_payload, validate_non_empty_payload_path, validate_relative_edit_path,
};

const MAX_EDIT_CONTENT_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EditKind {
    RemovePath,
    RemoveLine,
    RemoveBlock,
    RewriteLine,
    RewriteBlock,
}

impl EditKind {
    pub fn from_change(line_range: Option<LineRange>, after: &str) -> Self {
        match line_range {
            None => Self::RemovePath,
            Some(range) if after.is_empty() && range.is_single_line() => Self::RemoveLine,
            Some(_) if after.is_empty() => Self::RemoveBlock,
            Some(range) if range.is_single_line() => Self::RewriteLine,
            Some(_) => Self::RewriteBlock,
        }
    }

    pub fn json_key(self) -> &'static str {
        match self {
            Self::RemovePath => "remove_path",
            Self::RemoveLine => "remove_line",
            Self::RemoveBlock => "remove_block",
            Self::RewriteLine => "rewrite_line",
            Self::RewriteBlock => "rewrite_block",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineRange {
    pub start: usize,
    pub end: usize,
}

impl LineRange {
    pub fn is_valid(self) -> bool {
        self.start >= 1 && self.end >= self.start
    }

    pub fn is_single_line(self) -> bool {
        self.start == self.end
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EditIdempotenceMarker {
    pub value: String,
}

impl EditIdempotenceMarker {
    pub fn as_str(&self) -> &str {
        &self.value
    }

    fn for_record_parts(
        file: &Path,
        line_range: Option<LineRange>,
        before: &str,
        after: &str,
        reason: &EditReason,
        proof_source: &EditProofSource,
        edit_kind: EditKind,
        pass_name: &str,
    ) -> Self {
        let mut hasher = Sha256::new();
        hash_field(&mut hasher, "version", "1");
        hash_field(&mut hasher, "file", &file.to_string_lossy());
        hash_field(&mut hasher, "pass_name", pass_name);
        hash_field(&mut hasher, "edit_kind", edit_kind.json_key());
        match line_range {
            Some(range) => {
                hash_field(&mut hasher, "line_start", &range.start.to_string());
                hash_field(&mut hasher, "line_end", &range.end.to_string());
            }
            None => {
                hash_field(&mut hasher, "line_start", "none");
                hash_field(&mut hasher, "line_end", "none");
            }
        }
        hash_field(&mut hasher, "before", before);
        hash_field(&mut hasher, "after", after);
        hash_field(&mut hasher, "reason_kind", reason.json_key());
        hash_field(&mut hasher, "reason_payload", &reason.payload_label());
        hash_field(&mut hasher, "proof_kind", proof_source.kind().json_key());
        hash_field(&mut hasher, "proof_payload", &proof_source.payload_label());

        Self {
            value: hex::encode(hasher.finalize()),
        }
    }
}

fn hash_field(hasher: &mut Sha256, name: &str, value: &str) {
    hasher.update(name.as_bytes());
    hasher.update(b"\0");
    hasher.update(value.len().to_string().as_bytes());
    hasher.update(b"\0");
    hasher.update(value.as_bytes());
    hasher.update(b"\0");
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditRecord {
    pub path: RelativePathBuf,
    pub pass: String,
    pub reason: EditReason,
    pub span: Option<TextSpan>,
    pub old: Option<String>,
    pub new: Option<String>,
    pub file: PathBuf,
    pub line_range: Option<LineRange>,
    pub before: String,
    pub after: String,
    pub edit_kind: EditKind,
    pub proof_source: EditProofSource,
    pub idempotence_marker: EditIdempotenceMarker,
    pub pass_name: &'static str,
}

impl EditRecord {
    pub fn new(
        file: PathBuf,
        line_range: Option<LineRange>,
        before: String,
        after: String,
        reason: EditReason,
        proof_source: EditProofSource,
        pass_name: &'static str,
    ) -> Self {
        let before = bounded_edit_content(before);
        let after = bounded_edit_content(after);
        let edit_kind = EditKind::from_change(line_range, &after);
        let idempotence_marker = EditIdempotenceMarker::for_record_parts(
            &file,
            line_range,
            &before,
            &after,
            &reason,
            &proof_source,
            edit_kind,
            pass_name,
        );

        Self {
            path: file.clone(),
            pass: pass_name.to_string(),
            span: line_range,
            old: Some(before.clone()),
            new: Some(after.clone()),
            file,
            line_range,
            before,
            after,
            edit_kind,
            reason,
            proof_source,
            idempotence_marker,
            pass_name,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_verified_rewrite_rejects_unproven_write() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("Kconfig");
        std::fs::write(&path, "before\n").unwrap();

        let err = write_verified_rewrite(root, &path, "after\n", &[], "test.pass")
            .unwrap_err()
            .to_string();

        assert!(err.contains("refusing unproven rewrite"));
    }

    #[test]
    fn test_write_verified_rewrite_accepts_matching_proof() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("Kconfig");
        std::fs::write(&path, "before\n").unwrap();

        let edits = vec![EditRecord::new(
            PathBuf::from("Kconfig"),
            Some(LineRange { start: 1, end: 1 }),
            String::from("before\n"),
            String::from("after\n"),
            EditReason::ManifestConfig {
                symbol: String::from("FOO"),
            },
            EditProofSource::removal_manifest_config(String::from("FOO")),
            "test.pass",
        )];

        write_verified_rewrite(root, &path, "after\n", &edits, "test.pass").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "after\n");
    }

    #[test]
    fn test_write_verified_rewrite_rejects_unrecorded_extra_mutation() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("Kconfig");
        std::fs::write(&path, "keep\nbefore\n").unwrap();

        let edits = vec![EditRecord::new(
            PathBuf::from("Kconfig"),
            Some(LineRange { start: 2, end: 2 }),
            String::from("before\n"),
            String::from("after\n"),
            EditReason::ManifestConfig {
                symbol: String::from("FOO"),
            },
            EditProofSource::removal_manifest_config(String::from("FOO")),
            "test.pass",
        )];

        let err =
            write_verified_rewrite(root, &path, "changed\nafter\n", &edits, "test.pass")
                .unwrap_err()
                .to_string();

        assert!(err.contains("unrecorded mutation"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "keep\nbefore\n");
    }

    #[test]
    fn test_diagnostic_class_stable_names_cover_all_variants() {
        let cases = [
            (DiagnosticClass::MissingHeader, "MissingHeader"),
            (
                DiagnosticClass::MissingKconfigSource,
                "MissingKconfigSource",
            ),
            (
                DiagnosticClass::StaleKbuildDirectoryRef,
                "StaleKbuildDirectoryRef",
            ),
            (
                DiagnosticClass::StaleKbuildObjectRef,
                "StaleKbuildObjectRef",
            ),
            (
                DiagnosticClass::DeadConfigGatedCodePath,
                "DeadConfigGatedCodePath",
            ),
            (
                DiagnosticClass::RemovedConfigSymbolUse,
                "RemovedConfigSymbolUse",
            ),
            (
                DiagnosticClass::RemovedHeaderSymbolUse,
                "RemovedHeaderSymbolUse",
            ),
            (DiagnosticClass::UndefinedReference, "UndefinedReference"),
            (DiagnosticClass::Unknown, "Unknown"),
        ];

        for (class, stable_name) in cases {
            assert_eq!(class.stable_name(), stable_name);
            assert_eq!(
                EditReason::BuildDiagnostic {
                    class: class.clone()
                }
                .payload_label(),
                format!("class={stable_name}")
            );
            assert_eq!(DiagnosticId::from_class(class).payload_label(), {
                format!("class={stable_name} key={stable_name}")
            });
        }
    }

    #[test]
    fn test_edit_record_new_populates_required_audit_fields() {
        let edit = EditRecord::new(
            PathBuf::from("drivers/foo/test.c"),
            Some(LineRange { start: 7, end: 7 }),
            String::from("#include <missing.h>\n"),
            String::new(),
            EditReason::BuildDiagnostic {
                class: DiagnosticClass::MissingHeader,
            },
            EditProofSource::ClassifiedDiagnostic {
                diagnostic_id: DiagnosticClass::MissingHeader.into(),
            },
            "fixups.remove_missing_header_include",
        );

        assert_eq!(edit.file, PathBuf::from("drivers/foo/test.c"));
        assert_eq!(edit.pass_name, "fixups.remove_missing_header_include");
        assert_eq!(edit.path, PathBuf::from("drivers/foo/test.c"));
        assert_eq!(edit.pass, "fixups.remove_missing_header_include");
        assert_eq!(edit.span, Some(LineRange { start: 7, end: 7 }));
        assert_eq!(edit.old, Some(String::from("#include <missing.h>\n")));
        assert_eq!(edit.new, Some(String::new()));
        assert_eq!(edit.edit_kind, EditKind::RemoveLine);
        assert!(matches!(
            edit.reason,
            EditReason::BuildDiagnostic {
                class: DiagnosticClass::MissingHeader
            }
        ));
        assert_eq!(
            edit.proof_source.kind(),
            EditProofSourceKind::ClassifiedDiagnostic
        );
        assert_eq!(edit.line_range, Some(LineRange { start: 7, end: 7 }));
        assert_eq!(edit.before, "#include <missing.h>\n");
        assert_eq!(edit.after, "");
        assert_eq!(edit.idempotence_marker.as_str().len(), 64);
        validate_edit_records(&[edit]).unwrap();
    }

    #[test]
    fn test_edit_reason_defines_structured_reason_taxonomy() {
        let reasons = vec![
            (
                EditReason::DeclaredPathPruned,
                "declared_path_pruned",
                EditProofSourceKind::RemovalManifestEntry,
            ),
            (
                EditReason::RemovedKconfigSource,
                "removed_kconfig_source",
                EditProofSourceKind::RemovalManifestEntry,
            ),
            (
                EditReason::RemovedKconfigSymbolEdge,
                "removed_kconfig_symbol_edge",
                EditProofSourceKind::StaleReference,
            ),
            (
                EditReason::RemovedDeadKconfigSymbolDefinition {
                    symbol: String::from("DEAD"),
                },
                "removed_dead_kconfig_symbol_definition",
                EditProofSourceKind::KconfigSolverProof,
            ),
            (
                EditReason::RemovedEmptyKconfigMenu {
                    prompt: String::from("Dead drivers"),
                },
                "removed_empty_kconfig_menu",
                EditProofSourceKind::KconfigSolverProof,
            ),
            (
                EditReason::SimplifiedKconfigExpression,
                "simplified_kconfig_expression",
                EditProofSourceKind::RemovalManifestEntry,
            ),
            (
                EditReason::RemovedKbuildDirectoryRef,
                "removed_kbuild_directory_ref",
                EditProofSourceKind::StaleReference,
            ),
            (
                EditReason::RemovedKbuildObjectRef,
                "removed_kbuild_object_ref",
                EditProofSourceKind::StaleReference,
            ),
            (
                EditReason::RemovedKbuildConfigGatedRef,
                "removed_kbuild_config_gated_ref",
                EditProofSourceKind::StaleReference,
            ),
            (
                EditReason::RemovedKbuildIncludePath,
                "removed_kbuild_include_path",
                EditProofSourceKind::StaleReference,
            ),
            (
                EditReason::FoldedDeadPreprocessorBranch,
                "folded_dead_preprocessor_branch",
                EditProofSourceKind::RemovalManifestEntry,
            ),
            (
                EditReason::RemovedManifestBackedInclude,
                "removed_manifest_backed_include",
                EditProofSourceKind::RemovalManifestEntry,
            ),
            (
                EditReason::RemovedDeadBranchInclude {
                    header: String::from("linux/dead.h"),
                    symbol: String::from("FOO"),
                },
                "removed_dead_branch_include",
                EditProofSourceKind::RemovalManifestEntry,
            ),
            (
                EditReason::ReportedLiveMissingInclude,
                "reported_live_missing_include",
                EditProofSourceKind::TreeIndexEntry,
            ),
            (
                EditReason::DiagnosticMissingHeaderFixup,
                "diagnostic_missing_header_fixup",
                EditProofSourceKind::ClassifiedDiagnostic,
            ),
            (
                EditReason::DiagnosticStaleKbuildDirFixup,
                "diagnostic_stale_kbuild_dir_fixup",
                EditProofSourceKind::ClassifiedDiagnostic,
            ),
            (
                EditReason::DiagnosticStaleKbuildObjectFixup,
                "diagnostic_stale_kbuild_object_fixup",
                EditProofSourceKind::ClassifiedDiagnostic,
            ),
            (
                EditReason::DiagnosticMissingKconfigSourceFixup,
                "diagnostic_missing_kconfig_source_fixup",
                EditProofSourceKind::ClassifiedDiagnostic,
            ),
            (
                EditReason::DiagnosticPreprocessorRefoldFixup,
                "diagnostic_preprocessor_refold_fixup",
                EditProofSourceKind::ClassifiedDiagnostic,
            ),
        ];

        for (reason, key, proof_kind) in reasons {
            assert_eq!(reason.json_key(), key);
            assert_eq!(reason.proof_source_kind(), proof_kind);
            assert!(!reason.payload_label().is_empty());
            reason.validate_reasoned_payload().unwrap();
        }

        assert!(
            EditProofSource::removal_manifest_path(PathBuf::from("drivers/foo"))
                .matches_reason(&EditReason::DeclaredPathPruned)
        );
        assert!(
            EditProofSource::removal_manifest_kconfig_source(PathBuf::from("drivers/foo/Kconfig"))
                .matches_reason(&EditReason::RemovedKconfigSource)
        );
        assert!(
            EditProofSource::stale_kbuild_reference(String::from("foo.o"))
                .matches_reason(&EditReason::RemovedKbuildObjectRef)
        );
        assert!(
            EditProofSource::kconfig_solver_unreachable_symbol_definition(
                String::from("DEAD"),
                PathBuf::from("Kconfig"),
                3,
            )
            .matches_reason(&EditReason::RemovedDeadKconfigSymbolDefinition {
                symbol: String::from("DEAD"),
            })
        );
        assert!(
            EditProofSource::removal_manifest_config(String::from("FOO")).matches_reason(
                &EditReason::RemovedDeadBranchInclude {
                    header: String::from("linux/dead.h"),
                    symbol: String::from("FOO"),
                }
            )
        );
        assert!(EditProofSource::ClassifiedDiagnostic {
            diagnostic_id: DiagnosticClass::MissingHeader.into()
        }
        .matches_reason(&EditReason::DiagnosticMissingHeaderFixup));
    }

    #[test]
    fn test_reason_key_lookup_covers_structured_edit_reasons() {
        let reasons = vec![
            EditReason::DeclaredPathPruned,
            EditReason::RemovedKconfigSource,
            EditReason::RemovedKconfigSymbolEdge,
            EditReason::RemovedDeadKconfigSymbolDefinition {
                symbol: String::from("DEAD"),
            },
            EditReason::RemovedEmptyKconfigMenu {
                prompt: String::from("Dead drivers"),
            },
            EditReason::SimplifiedKconfigExpression,
            EditReason::RemovedKbuildDirectoryRef,
            EditReason::RemovedKbuildObjectRef,
            EditReason::RemovedKbuildConfigGatedRef,
            EditReason::RemovedKbuildIncludePath,
            EditReason::FoldedDeadPreprocessorBranch,
            EditReason::RemovedManifestBackedInclude,
            EditReason::RemovedDeadBranchInclude {
                header: String::from("linux/dead.h"),
                symbol: String::from("FOO"),
            },
            EditReason::ReportedLiveMissingInclude,
            EditReason::DiagnosticMissingHeaderFixup,
            EditReason::DiagnosticStaleKbuildDirFixup,
            EditReason::DiagnosticStaleKbuildObjectFixup,
            EditReason::DiagnosticMissingKconfigSourceFixup,
            EditReason::DiagnosticPreprocessorRefoldFixup,
            EditReason::ManifestPath {
                path: PathBuf::from("drivers/foo"),
            },
            EditReason::ManifestConfig {
                symbol: String::from("FOO"),
            },
            EditReason::RemovedKbuildRef {
                reference: String::from("foo.o"),
            },
            EditReason::RemovedHeader {
                header: String::from("linux/foo.h"),
            },
            EditReason::SimplifiedTristateExpr {
                symbol: String::from("FOO"),
            },
            EditReason::BuildDiagnostic {
                class: DiagnosticClass::MissingHeader,
            },
        ];

        for reason in reasons {
            assert_eq!(
                proof_source_kind_for_reason_key(reason.json_key()),
                Some(reason.proof_source_kind()),
                "missing structured proof-kind mapping for {}",
                reason.json_key()
            );
        }
        assert_eq!(
            proof_source_kind_for_reason_key("unstructured_reason"),
            None
        );
    }

    #[test]
    fn test_edit_proof_source_defines_structured_sources() {
        let proof_sources = vec![
            (
                EditProofSource::RemovalManifest {
                    key: RemovalKey::Path(PathBuf::from("drivers/foo")),
                },
                EditProofSourceKind::RemovalManifestEntry,
                EditReason::DeclaredPathPruned,
                "path=drivers/foo",
            ),
            (
                EditProofSource::removal_manifest_kconfig_source(PathBuf::from(
                    "drivers/foo/Kconfig",
                )),
                EditProofSourceKind::RemovalManifestEntry,
                EditReason::RemovedKconfigSource,
                "kconfig_source=drivers/foo/Kconfig",
            ),
            (
                EditProofSource::TreeIndex {
                    index_kind: IndexKind::IncludeSite,
                    key: String::from("drivers/foo/live.c:7:missing.h"),
                },
                EditProofSourceKind::TreeIndexEntry,
                EditReason::ReportedLiveMissingInclude,
                "index_kind=include_site key=drivers/foo/live.c:7:missing.h",
            ),
            (
                EditProofSource::kconfig_solver_unreachable_symbol_definition(
                    String::from("DEAD"),
                    PathBuf::from("Kconfig"),
                    3,
                ),
                EditProofSourceKind::KconfigSolverProof,
                EditReason::RemovedDeadKconfigSymbolDefinition {
                    symbol: String::from("DEAD"),
                },
                "solver=unreachable_symbol_definition symbol=DEAD file=Kconfig line=3",
            ),
            (
                EditProofSource::kconfig_solver_empty_menu(
                    String::from("Dead drivers"),
                    PathBuf::from("Kconfig"),
                    7,
                ),
                EditProofSourceKind::KconfigSolverProof,
                EditReason::RemovedEmptyKconfigMenu {
                    prompt: String::from("Dead drivers"),
                },
                "solver=empty_menu prompt=Dead%20drivers file=Kconfig line=7",
            ),
            (
                EditProofSource::StaleReference {
                    reference_kind: ReferenceKind::KbuildObjectRef,
                    key: String::from("drivers/foo/remove.o"),
                },
                EditProofSourceKind::StaleReference,
                EditReason::RemovedKbuildObjectRef,
                "reference_kind=kbuild_object_ref key=drivers/foo/remove.o",
            ),
            (
                EditProofSource::ClassifiedDiagnostic {
                    diagnostic_id: DiagnosticId {
                        class: DiagnosticClass::MissingHeader,
                        key: String::from("drivers/foo/live.c:7:missing.h"),
                    },
                },
                EditProofSourceKind::ClassifiedDiagnostic,
                EditReason::DiagnosticMissingHeaderFixup,
                "class=MissingHeader key=drivers/foo/live.c:7:missing.h",
            ),
        ];

        for (proof_source, kind, reason, label) in proof_sources {
            assert_eq!(proof_source.kind(), kind);
            assert_eq!(proof_source.payload_label(), label);
            assert!(proof_source.matches_reason(&reason));
            proof_source.validate_reasoned_payload().unwrap();
        }

        let empty_tree_key = EditProofSource::TreeIndex {
            index_kind: IndexKind::Header,
            key: String::new(),
        };
        assert!(empty_tree_key.validate_reasoned_payload().is_err());

        let unknown_diagnostic = EditProofSource::ClassifiedDiagnostic {
            diagnostic_id: DiagnosticId::from_class(DiagnosticClass::Unknown),
        };
        assert!(unknown_diagnostic.validate_reasoned_payload().is_err());
    }

    #[test]
    fn test_sorted_edit_record_refs_orders_by_pass_path_span_and_kind() {
        fn remove_path(path: &str) -> EditRecord {
            EditRecord::new(
                PathBuf::from(path),
                None,
                String::from("before\n"),
                String::new(),
                EditReason::ManifestPath {
                    path: PathBuf::from(path),
                },
                EditProofSource::removal_manifest_path(PathBuf::from(path)),
                "prune.remove_path",
            )
        }

        fn stale_kbuild_edit(
            path: &str,
            line_range: LineRange,
            after: &str,
            pass_name: &'static str,
            reference: &str,
        ) -> EditRecord {
            EditRecord::new(
                PathBuf::from(path),
                Some(line_range),
                format!("{reference}\n"),
                after.to_string(),
                EditReason::RemovedKbuildRef {
                    reference: reference.to_string(),
                },
                EditProofSource::stale_kbuild_reference(reference.to_string()),
                pass_name,
            )
        }

        let edits = vec![
            stale_kbuild_edit(
                "a.c",
                LineRange { start: 1, end: 1 },
                "",
                "cpp.fold_removed_config_branches",
                "CONFIG_REMOVED",
            ),
            stale_kbuild_edit(
                "drivers/Makefile",
                LineRange { start: 9, end: 9 },
                "# kslim: removed stale make refs from obj-y\n",
                "prune.rewrite_makefiles",
                "late.o",
            ),
            remove_path("z/removed.c"),
            stale_kbuild_edit(
                "drivers/Makefile",
                LineRange { start: 2, end: 2 },
                "# kslim: removed stale make refs from obj-y\n",
                "prune.rewrite_makefiles",
                "rewrite.o",
            ),
            stale_kbuild_edit(
                "drivers/Makefile",
                LineRange { start: 2, end: 2 },
                "",
                "prune.rewrite_makefiles",
                "remove.o",
            ),
            remove_path("a/removed.c"),
        ];

        let sorted = sorted_edit_record_refs(&edits);
        let actual = sorted
            .into_iter()
            .map(|edit| {
                (
                    edit.pass_name,
                    edit.file.clone(),
                    edit.line_range.map(|range| (range.start, range.end)),
                    edit.edit_kind,
                    edit.before.clone(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            actual,
            vec![
                (
                    "prune.remove_path",
                    PathBuf::from("a/removed.c"),
                    None,
                    EditKind::RemovePath,
                    String::from("before\n"),
                ),
                (
                    "prune.remove_path",
                    PathBuf::from("z/removed.c"),
                    None,
                    EditKind::RemovePath,
                    String::from("before\n"),
                ),
                (
                    "prune.rewrite_makefiles",
                    PathBuf::from("drivers/Makefile"),
                    Some((2, 2)),
                    EditKind::RemoveLine,
                    String::from("remove.o\n"),
                ),
                (
                    "prune.rewrite_makefiles",
                    PathBuf::from("drivers/Makefile"),
                    Some((2, 2)),
                    EditKind::RewriteLine,
                    String::from("rewrite.o\n"),
                ),
                (
                    "prune.rewrite_makefiles",
                    PathBuf::from("drivers/Makefile"),
                    Some((9, 9)),
                    EditKind::RewriteLine,
                    String::from("late.o\n"),
                ),
                (
                    "cpp.fold_removed_config_branches",
                    PathBuf::from("a.c"),
                    Some((1, 1)),
                    EditKind::RemoveLine,
                    String::from("CONFIG_REMOVED\n"),
                ),
            ]
        );
    }

    #[test]
    fn test_sort_edit_records_deduplicates_identical_records() {
        fn remove_path(path: &str) -> EditRecord {
            EditRecord::new(
                PathBuf::from(path),
                None,
                String::from("before\n"),
                String::new(),
                EditReason::ManifestPath {
                    path: PathBuf::from(path),
                },
                EditProofSource::removal_manifest_path(PathBuf::from(path)),
                "prune.remove_path",
            )
        }

        let edit_z = remove_path("z/removed.c");
        let edit_a = remove_path("a/removed.c");
        let mut edits = vec![edit_z.clone(), edit_a.clone(), edit_z.clone()];

        sort_edit_records(&mut edits);

        assert_eq!(edits, vec![edit_a.clone(), edit_z.clone()]);

        let unsorted = vec![edit_z.clone(), edit_a.clone(), edit_z];
        let refs = sorted_edit_record_refs(&unsorted);
        let actual = refs
            .into_iter()
            .map(|edit| edit.file.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            vec![PathBuf::from("a/removed.c"), PathBuf::from("z/removed.c")]
        );
    }

    #[test]
    fn test_grouped_edit_record_refs_by_reason_is_deterministic() {
        fn remove_path(path: &str) -> EditRecord {
            EditRecord::new(
                PathBuf::from(path),
                None,
                String::from("before\n"),
                String::new(),
                EditReason::ManifestPath {
                    path: PathBuf::from(path),
                },
                EditProofSource::removal_manifest_path(PathBuf::from(path)),
                "prune.remove_path",
            )
        }

        fn manifest_config(symbol: &str) -> EditRecord {
            EditRecord::new(
                PathBuf::from("Kconfig"),
                Some(LineRange { start: 1, end: 3 }),
                format!("config {symbol}\n\tbool\n"),
                format!("# kslim: removed config {symbol}\n"),
                EditReason::ManifestConfig {
                    symbol: symbol.to_string(),
                },
                EditProofSource::removal_manifest_config(symbol.to_string()),
                "prune.prune_configs",
            )
        }

        fn stale_kbuild(path: &str, reference: &str) -> EditRecord {
            EditRecord::new(
                PathBuf::from(path),
                Some(LineRange { start: 7, end: 7 }),
                format!("obj-y += {reference}\n"),
                String::new(),
                EditReason::RemovedKbuildRef {
                    reference: reference.to_string(),
                },
                EditProofSource::stale_kbuild_reference(reference.to_string()),
                "prune.rewrite_makefiles",
            )
        }

        fn snapshot(edits: &[EditRecord]) -> Vec<(String, Vec<PathBuf>)> {
            grouped_edit_record_refs_by_reason(edits)
                .into_iter()
                .map(|(reason, records)| {
                    (
                        reason.to_string(),
                        records
                            .into_iter()
                            .map(|edit| edit.file.clone())
                            .collect::<Vec<_>>(),
                    )
                })
                .collect()
        }

        let edits = vec![
            stale_kbuild("drivers/z/Makefile", "z.o"),
            remove_path("z/remove.c"),
            manifest_config("REMOVED"),
            stale_kbuild("drivers/a/Makefile", "a.o"),
            remove_path("a/remove.c"),
        ];
        validate_edit_records(&edits).unwrap();

        let reversed = edits.iter().cloned().rev().collect::<Vec<_>>();
        assert_eq!(snapshot(&edits), snapshot(&reversed));
        assert_eq!(
            snapshot(&edits),
            vec![
                (
                    String::from("manifest_config"),
                    vec![PathBuf::from("Kconfig")]
                ),
                (
                    String::from("manifest_path"),
                    vec![PathBuf::from("a/remove.c"), PathBuf::from("z/remove.c")]
                ),
                (
                    String::from("removed_kbuild_ref"),
                    vec![
                        PathBuf::from("drivers/a/Makefile"),
                        PathBuf::from("drivers/z/Makefile"),
                    ]
                ),
            ]
        );
    }

    #[test]
    fn test_edit_record_rejects_competing_proof_source() {
        let mut edit = EditRecord::new(
            PathBuf::from("Kconfig"),
            Some(LineRange { start: 1, end: 1 }),
            String::from("before\n"),
            String::from("after\n"),
            EditReason::ManifestConfig {
                symbol: String::from("FOO"),
            },
            EditProofSource::removal_manifest_config(String::from("FOO")),
            "test.pass",
        );
        edit.proof_source = EditProofSource::stale_kbuild_reference(String::from("foo.o"));

        let err = edit
            .validate_no_competing_proof_sources()
            .unwrap_err()
            .to_string();
        assert!(err.contains("competing proof source"));
        assert!(err.contains("Kconfig"));
    }

    #[test]
    fn test_reported_proof_source_payload_rejects_competing_truth() {
        validate_reported_proof_source_payload_for_reason(
            "manifest_path",
            "path=drivers/remove.c",
            "path=drivers/remove.c",
        )
        .unwrap();

        let err = validate_reported_proof_source_payload_for_reason(
            "manifest_path",
            "path=drivers/remove.c",
            "path=drivers/other.c",
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("conflicts with proof payload"));
        assert!(err.contains("drivers/remove.c"));
        assert!(err.contains("drivers/other.c"));
    }

    #[test]
    fn test_edit_record_rejects_broad_speculative_fallout() {
        let edit = EditRecord::new(
            PathBuf::from("drivers/foo/test.c"),
            Some(LineRange { start: 7, end: 7 }),
            String::from("before\n"),
            String::from("after\n"),
            EditReason::BuildDiagnostic {
                class: DiagnosticClass::UndefinedReference,
            },
            EditProofSource::ClassifiedDiagnostic {
                diagnostic_id: DiagnosticClass::UndefinedReference.into(),
            },
            "test.speculative_fallout",
        );

        let err = edit
            .validate_not_speculative_fallout()
            .unwrap_err()
            .to_string();

        assert!(err.contains("broad speculative fallout edit"));
    }

    #[test]
    fn test_reported_edit_rejects_broad_speculative_fallout() {
        validate_reported_no_speculative_fallout_edit(
            "build_diagnostic",
            "class=MissingHeader",
            "classified_build_diagnostic",
            "class=MissingHeader key=drivers/foo.c:7:missing.h",
        )
        .unwrap();

        let err = validate_reported_no_speculative_fallout_edit(
            "build_diagnostic",
            "class=UndefinedReference",
            "classified_build_diagnostic",
            "class=UndefinedReference key=amdgpu_magic",
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("broad speculative fallout edits are forbidden"));
        assert!(err.contains("UndefinedReference"));
    }

    #[test]
    fn test_strict_edit_validation_policy_rejects_unreasoned_edits() {
        let edit = EditRecord::new(
            PathBuf::from("drivers/foo/test.c"),
            Some(LineRange { start: 7, end: 7 }),
            String::from("before\n"),
            String::from("after\n"),
            EditReason::BuildDiagnostic {
                class: DiagnosticClass::Unknown,
            },
            EditProofSource::ClassifiedDiagnostic {
                diagnostic_id: DiagnosticClass::Unknown.into(),
            },
            "test.unreasoned",
        );

        let err = validate_edit_records_with_policy(
            &[edit.clone()],
            EditValidationPolicy {
                reject_unreasoned_edits: true,
                reject_speculative_fallout_edits: false,
            },
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("unreasoned EditReason"));
        validate_edit_records_with_policy(
            &[edit],
            EditValidationPolicy {
                reject_unreasoned_edits: false,
                reject_speculative_fallout_edits: false,
            },
        )
        .unwrap();
    }

    #[test]
    fn test_strict_edit_validation_policy_rejects_broad_speculative_fallout_edits() {
        let edit = EditRecord::new(
            PathBuf::from("drivers/foo/test.c"),
            Some(LineRange { start: 7, end: 7 }),
            String::from("before\n"),
            String::from("after\n"),
            EditReason::BuildDiagnostic {
                class: DiagnosticClass::UndefinedReference,
            },
            EditProofSource::ClassifiedDiagnostic {
                diagnostic_id: DiagnosticClass::UndefinedReference.into(),
            },
            "test.speculative_fallout",
        );

        let err = validate_edit_records_with_policy(
            &[edit.clone()],
            EditValidationPolicy {
                reject_unreasoned_edits: false,
                reject_speculative_fallout_edits: true,
            },
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("broad speculative fallout edit"));
        validate_edit_records_with_policy(
            &[edit],
            EditValidationPolicy {
                reject_unreasoned_edits: false,
                reject_speculative_fallout_edits: false,
            },
        )
        .unwrap();
    }

    #[test]
    fn test_edit_record_validation_rejects_invalid_idempotence_marker() {
        let mut edit = EditRecord::new(
            PathBuf::from("Kconfig"),
            Some(LineRange { start: 1, end: 1 }),
            String::from("before\n"),
            String::from("after\n"),
            EditReason::ManifestConfig {
                symbol: String::from("FOO"),
            },
            EditProofSource::removal_manifest_config(String::from("FOO")),
            "test.pass",
        );
        edit.after = String::from("tampered\n");
        edit.new = Some(edit.after.clone());

        let err = validate_edit_records(&[edit]).unwrap_err().to_string();
        assert!(err.contains("invalid idempotence marker"));
    }

    #[test]
    fn test_edit_record_validation_rejects_unstable_audit_aliases() {
        let base = EditRecord::new(
            PathBuf::from("Kconfig"),
            Some(LineRange { start: 1, end: 1 }),
            String::from("before\n"),
            String::from("after\n"),
            EditReason::ManifestConfig {
                symbol: String::from("FOO"),
            },
            EditProofSource::removal_manifest_config(String::from("FOO")),
            "test.pass",
        );

        let mut edit = base.clone();
        edit.path = PathBuf::from("Other");
        let err = validate_edit_records(&[edit]).unwrap_err().to_string();
        assert!(err.contains("unstable path alias"));

        let mut edit = base.clone();
        edit.pass = String::from("other.pass");
        let err = validate_edit_records(&[edit]).unwrap_err().to_string();
        assert!(err.contains("unstable pass alias"));

        let mut edit = base.clone();
        edit.span = Some(LineRange { start: 1, end: 2 });
        let err = validate_edit_records(&[edit]).unwrap_err().to_string();
        assert!(err.contains("unstable span alias"));

        let mut edit = base.clone();
        edit.old = None;
        let err = validate_edit_records(&[edit]).unwrap_err().to_string();
        assert!(err.contains("unstable old logical item"));

        let mut edit = base;
        edit.new = None;
        let err = validate_edit_records(&[edit]).unwrap_err().to_string();
        assert!(err.contains("unstable new logical item"));
    }

    #[test]
    fn test_edit_record_validation_rejects_unstable_edit_kind() {
        let mut edit = EditRecord::new(
            PathBuf::from("Kconfig"),
            Some(LineRange { start: 1, end: 1 }),
            String::from("before\n"),
            String::from("after\n"),
            EditReason::ManifestConfig {
                symbol: String::from("FOO"),
            },
            EditProofSource::removal_manifest_config(String::from("FOO")),
            "test.pass",
        );
        edit.edit_kind = EditKind::RewriteBlock;

        let err = validate_edit_records(&[edit]).unwrap_err().to_string();
        assert!(err.contains("unstable edit kind"));
    }

    #[test]
    fn test_edit_record_validation_rejects_invalid_path() {
        let edit = EditRecord::new(
            PathBuf::from("../Kconfig"),
            Some(LineRange { start: 1, end: 1 }),
            String::from("before\n"),
            String::from("after\n"),
            EditReason::ManifestConfig {
                symbol: String::from("FOO"),
            },
            EditProofSource::removal_manifest_config(String::from("FOO")),
            "test.pass",
        );

        let err = validate_edit_records(&[edit]).unwrap_err().to_string();
        assert!(err.contains("normalized and relative"));
    }

    #[test]
    fn test_edit_record_validation_rejects_invalid_reason_pass_pairing() {
        let edit = EditRecord::new(
            PathBuf::from("Kconfig"),
            Some(LineRange { start: 1, end: 1 }),
            String::from("before\n"),
            String::from("after\n"),
            EditReason::ManifestConfig {
                symbol: String::from("FOO"),
            },
            EditProofSource::removal_manifest_config(String::from("FOO")),
            "fixups.remove_missing_header_include",
        );

        let err = validate_edit_records(&[edit]).unwrap_err().to_string();
        assert!(err.contains("not valid for pass"));
    }

    #[test]
    fn test_edit_record_validation_requires_span_for_text_rewrite() {
        let edit = EditRecord::new(
            PathBuf::from("drivers/foo.c"),
            None,
            String::from("before\n"),
            String::from("after\n"),
            EditReason::ManifestPath {
                path: PathBuf::from("drivers/foo.c"),
            },
            EditProofSource::removal_manifest_path(PathBuf::from("drivers/foo.c")),
            "prune.remove_path",
        );

        let err = validate_edit_records(&[edit]).unwrap_err().to_string();
        assert!(err.contains("whole-path deletion with replacement content"));
    }

    #[test]
    fn test_edit_record_new_elides_large_audit_content() {
        let large_before = "x".repeat(MAX_EDIT_CONTENT_BYTES + 1);
        let edit = EditRecord::new(
            PathBuf::from("drivers/large.c"),
            None,
            large_before,
            String::new(),
            EditReason::ManifestPath {
                path: PathBuf::from("drivers/large.c"),
            },
            EditProofSource::removal_manifest_path(PathBuf::from("drivers/large.c")),
            "prune.remove_path",
        );

        assert!(edit.before.starts_with("<kslim: content elided "));
        assert!(edit.before.len() < MAX_EDIT_CONTENT_BYTES);
        assert_eq!(edit.old, Some(edit.before.clone()));
        validate_edit_records(&[edit]).unwrap();
    }

    #[test]
    fn test_mutating_pass_requires_matching_edit_records() {
        let edit = EditRecord::new(
            PathBuf::from("Kconfig"),
            Some(LineRange { start: 1, end: 1 }),
            String::from("before\n"),
            String::from("after\n"),
            EditReason::ManifestConfig {
                symbol: String::from("FOO"),
            },
            EditProofSource::removal_manifest_config(String::from("FOO")),
            "test.pass",
        );

        ensure_edit_records_for_mutation("test.pass", 1, &[edit.clone()]).unwrap();
        ensure_edit_records_for_mutation("test.pass", 0, &[]).unwrap();

        let err = ensure_edit_records_for_mutation("test.pass", 1, &[])
            .unwrap_err()
            .to_string();
        assert!(err.contains("without edit records"));

        let err = ensure_edit_records_for_mutation("other.pass", 1, &[edit])
            .unwrap_err()
            .to_string();
        assert!(err.contains("other.pass"));
    }

    #[test]
    fn test_mutating_pass_requires_one_matching_edit_record_per_mutation() {
        let edit = EditRecord::new(
            PathBuf::from("Kconfig"),
            Some(LineRange { start: 1, end: 1 }),
            String::from("before\n"),
            String::from("after\n"),
            EditReason::ManifestConfig {
                symbol: String::from("FOO"),
            },
            EditProofSource::removal_manifest_config(String::from("FOO")),
            "test.pass",
        );

        let err = ensure_edit_records_for_mutation("test.pass", 2, &[edit])
            .unwrap_err()
            .to_string();

        assert!(err.contains("2 mutation(s)"));
        assert!(err.contains("only 1 matching edit record"));
    }

    #[test]
    fn test_edit_proof_source_kind_maps_all_variants() {
        assert_eq!(
            EditProofSource::removal_manifest_path(PathBuf::from("drivers/foo")).kind(),
            EditProofSourceKind::RemovalManifestEntry
        );
        assert_eq!(
            EditProofSource::removal_manifest_config(String::from("FOO")).kind(),
            EditProofSourceKind::RemovalManifestEntry
        );
        assert_eq!(
            EditProofSource::removal_manifest_header(
                String::from("foo.h"),
                PathBuf::from("drivers/foo/foo.h")
            )
            .kind(),
            EditProofSourceKind::RemovalManifestEntry
        );
        assert_eq!(
            EditProofSource::removal_manifest_kconfig_source(PathBuf::from("drivers/foo/Kconfig"))
                .kind(),
            EditProofSourceKind::RemovalManifestEntry
        );
        assert_eq!(
            EditProofSource::kconfig_solver_unreachable_symbol_definition(
                String::from("DEAD"),
                PathBuf::from("Kconfig"),
                3,
            )
            .kind(),
            EditProofSourceKind::KconfigSolverProof
        );
        assert_eq!(
            EditProofSource::stale_kbuild_reference(String::from("foo.o")).kind(),
            EditProofSourceKind::StaleReference
        );
        assert_eq!(
            EditProofSource::ClassifiedDiagnostic {
                diagnostic_id: DiagnosticClass::MissingHeader.into()
            }
            .kind(),
            EditProofSourceKind::ClassifiedDiagnostic
        );
    }
}
