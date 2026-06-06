//! Fixup planning strategies for classified build diagnostics.
//!
//! This module owns deterministic fixer selection and proof-plan assembly.
//! Rewrite application remains behind proof-gated helpers in the fixups root
//! until the application split lands.

use anyhow::Result;
use std::path::Path;

use crate::diagnostics::ClassifiedDiagnostic;
use crate::prune::RemovalAccounting;
use crate::tree_index::TreeIndex;

use super::{
    classified_diagnostic_proof, manifest_proven_missing_kconfig_source_path,
    manifest_proven_removed_directory_path, manifest_proven_removed_header_path,
    manifest_proven_removed_object_path, normalize_directory_subject, normalize_object_subject,
    remove_missing_header_include, remove_missing_kconfig_source,
    remove_stale_kbuild_directory_reference, remove_stale_kbuild_object_reference, FixupProof,
    FixupResult,
};

pub(in crate::fixups) trait Fixup {
    fn name(&self) -> &'static str;

    fn applies(&self, diagnostic: &ClassifiedDiagnostic) -> bool;

    fn apply(
        &self,
        root: &Path,
        index: &TreeIndex,
        removal: &RemovalAccounting,
        diagnostic: &ClassifiedDiagnostic,
    ) -> Result<Option<FixupResult>>;
}

pub(in crate::fixups) struct MissingHeaderIncludeFixup;
pub(in crate::fixups) struct StaleKbuildDirectoryFixup;
pub(in crate::fixups) struct StaleKbuildObjectFixup;
pub(in crate::fixups) struct MissingKconfigSourceFixup;

impl Fixup for MissingHeaderIncludeFixup {
    fn name(&self) -> &'static str {
        "fixups.remove_missing_header_include"
    }

    fn applies(&self, diagnostic: &ClassifiedDiagnostic) -> bool {
        matches!(diagnostic, ClassifiedDiagnostic::MissingHeader { .. })
    }

    fn apply(
        &self,
        root: &Path,
        index: &TreeIndex,
        removal: &RemovalAccounting,
        diagnostic: &ClassifiedDiagnostic,
    ) -> Result<Option<FixupResult>> {
        let ClassifiedDiagnostic::MissingHeader {
            source_file,
            header,
            ..
        } = diagnostic
        else {
            anyhow::bail!(
                "{} cannot handle diagnostic class {}",
                self.name(),
                diagnostic.class().stable_name()
            );
        };

        let include_site = index
            .find_include_site(source_file, header)
            .map(|site| (site.file.clone(), site.line, site.target.clone()));
        let Some((include_file, include_line, include_target)) = include_site else {
            return Ok(None);
        };

        let source_path = root.join(source_file);
        let source_dir = source_path.parent().unwrap_or(root);
        let manifest_path = manifest_proven_removed_header_path(root, source_dir, header, removal)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "cannot apply missing-header fixup for '{}' because the header is not proven removed",
                    header,
                )
            })?;

        let proof_sources = vec![
            FixupProof::ManifestPath {
                path: manifest_path,
            },
            FixupProof::TreeIndexIncludeSite {
                file: include_file,
                line: include_line,
                target: include_target,
            },
            classified_diagnostic_proof(diagnostic),
        ];
        let edits = remove_missing_header_include(
            root,
            removal,
            source_file,
            header,
            index,
            diagnostic,
            &proof_sources,
        )?;

        Ok(Some(FixupResult::new(edits, proof_sources)))
    }
}

impl Fixup for StaleKbuildDirectoryFixup {
    fn name(&self) -> &'static str {
        "fixups.remove_stale_kbuild_directory_ref"
    }

    fn applies(&self, diagnostic: &ClassifiedDiagnostic) -> bool {
        matches!(
            diagnostic,
            ClassifiedDiagnostic::MissingMakeDirectory { .. }
        )
    }

    fn apply(
        &self,
        root: &Path,
        index: &TreeIndex,
        removal: &RemovalAccounting,
        diagnostic: &ClassifiedDiagnostic,
    ) -> Result<Option<FixupResult>> {
        let ClassifiedDiagnostic::MissingMakeDirectory { path, .. } = diagnostic else {
            anyhow::bail!(
                "{} cannot handle diagnostic class {}",
                self.name(),
                diagnostic.class().stable_name()
            );
        };

        let manifest_path = manifest_proven_removed_directory_path(root, path, removal)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "cannot apply stale-kbuild-directory fixup for '{}' because the directory is not proven removed",
                    path
                )
            })?;
        let resolved_path = normalize_directory_subject(path).ok_or_else(|| {
            anyhow::anyhow!(
                "cannot apply stale-kbuild-directory fixup for '{}' because the path is empty",
                path
            )
        })?;
        let directory_refs = index.find_kbuild_directory_refs(path);
        if directory_refs.is_empty() {
            return Ok(None);
        }
        let [directory_ref] = directory_refs.as_slice() else {
            anyhow::bail!(
                "cannot apply stale-kbuild-directory fixup for '{}' because {} matching tree-index references were found",
                path,
                directory_refs.len()
            );
        };

        let proof_sources = vec![
            FixupProof::ManifestPath {
                path: manifest_path,
            },
            FixupProof::TreeIndexKbuildDirectoryRef {
                file: directory_ref.file.clone(),
                line: directory_ref.line,
                assignment_lhs: directory_ref.assignment_lhs.clone(),
                directory: directory_ref.directory.clone(),
                resolved_path,
            },
            classified_diagnostic_proof(diagnostic),
        ];
        let edits = remove_stale_kbuild_directory_reference(
            root,
            &directory_ref.file,
            directory_ref.line,
            &directory_ref.assignment_lhs,
            &directory_ref.directory,
            index,
            diagnostic,
            &proof_sources,
        )?;

        Ok(Some(FixupResult::new(edits, proof_sources)))
    }
}

impl Fixup for StaleKbuildObjectFixup {
    fn name(&self) -> &'static str {
        "fixups.remove_stale_kbuild_object_ref"
    }

    fn applies(&self, diagnostic: &ClassifiedDiagnostic) -> bool {
        matches!(
            diagnostic,
            ClassifiedDiagnostic::MissingMakeTarget { target, .. } if target.ends_with(".o")
        )
    }

    fn apply(
        &self,
        root: &Path,
        index: &TreeIndex,
        removal: &RemovalAccounting,
        diagnostic: &ClassifiedDiagnostic,
    ) -> Result<Option<FixupResult>> {
        let ClassifiedDiagnostic::MissingMakeTarget { target, .. } = diagnostic else {
            anyhow::bail!(
                "{} cannot handle diagnostic class {}",
                self.name(),
                diagnostic.class().stable_name()
            );
        };

        let manifest_path = manifest_proven_removed_object_path(root, target, removal)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "cannot apply stale-kbuild-object fixup for '{}' because the object is not proven removed",
                    target
                )
            })?;
        let resolved_path = normalize_object_subject(target).ok_or_else(|| {
            anyhow::anyhow!(
                "cannot apply stale-kbuild-object fixup for '{}' because the target is not a supported .o path",
                target
            )
        })?;
        let object_refs = index.find_kbuild_object_refs(target);
        if object_refs.is_empty() {
            return Ok(None);
        }
        let [object_ref] = object_refs.as_slice() else {
            anyhow::bail!(
                "cannot apply stale-kbuild-object fixup for '{}' because {} matching tree-index references were found",
                target,
                object_refs.len()
            );
        };

        let proof_sources = vec![
            FixupProof::ManifestPath {
                path: manifest_path,
            },
            FixupProof::TreeIndexKbuildObjectRef {
                file: object_ref.file.clone(),
                line: object_ref.line,
                assignment_lhs: object_ref.assignment_lhs.clone(),
                object: object_ref.object.clone(),
                resolved_path,
            },
            classified_diagnostic_proof(diagnostic),
        ];
        let edits = remove_stale_kbuild_object_reference(
            root,
            &object_ref.file,
            object_ref.line,
            &object_ref.assignment_lhs,
            &object_ref.object,
            index,
            diagnostic,
            &proof_sources,
        )?;

        Ok(Some(FixupResult::new(edits, proof_sources)))
    }
}

impl Fixup for MissingKconfigSourceFixup {
    fn name(&self) -> &'static str {
        "fixups.remove_missing_kconfig_source"
    }

    fn applies(&self, diagnostic: &ClassifiedDiagnostic) -> bool {
        matches!(
            diagnostic,
            ClassifiedDiagnostic::MissingKconfigSource { .. }
        )
    }

    fn apply(
        &self,
        root: &Path,
        index: &TreeIndex,
        removal: &RemovalAccounting,
        diagnostic: &ClassifiedDiagnostic,
    ) -> Result<Option<FixupResult>> {
        let ClassifiedDiagnostic::MissingKconfigSource {
            kconfig_file,
            line,
            source,
        } = diagnostic
        else {
            anyhow::bail!(
                "{} cannot handle diagnostic class {}",
                self.name(),
                diagnostic.class().stable_name()
            );
        };

        let source_ref = index
            .find_kconfig_source_ref(kconfig_file, *line, source)
            .cloned();
        let Some(source_ref) = source_ref else {
            return Ok(None);
        };
        if source_ref.optional || source_ref.source.contains('$') {
            anyhow::bail!(
                "cannot apply missing-Kconfig-source fixup for '{}' because the source directive is optional or dynamic",
                source
            );
        }
        let manifest_path =
            manifest_proven_missing_kconfig_source_path(root, &source_ref, removal).ok_or_else(
                || {
                    anyhow::anyhow!(
                        "cannot apply missing-Kconfig-source fixup for '{}' because the source is not proven removed",
                        source
                    )
                },
            )?;

        let proof_sources = vec![
            FixupProof::ManifestPath {
                path: manifest_path,
            },
            FixupProof::TreeIndexKconfigSourceRef {
                file: source_ref.file.clone(),
                line: source_ref.line,
                source: source_ref.source.clone(),
                optional: source_ref.optional,
                relative: source_ref.relative,
            },
            classified_diagnostic_proof(diagnostic),
        ];
        let edits = remove_missing_kconfig_source(
            root,
            &source_ref.file,
            source_ref.line,
            &source_ref.source,
            index,
            diagnostic,
            &proof_sources,
        )?;

        Ok(Some(FixupResult::new(edits, proof_sources)))
    }
}

pub(in crate::fixups) fn available_fixups() -> [&'static dyn Fixup; 4] {
    static MISSING_HEADER_INCLUDE_FIXUP: MissingHeaderIncludeFixup = MissingHeaderIncludeFixup;
    static STALE_KBUILD_DIRECTORY_FIXUP: StaleKbuildDirectoryFixup = StaleKbuildDirectoryFixup;
    static STALE_KBUILD_OBJECT_FIXUP: StaleKbuildObjectFixup = StaleKbuildObjectFixup;
    static MISSING_KCONFIG_SOURCE_FIXUP: MissingKconfigSourceFixup = MissingKconfigSourceFixup;
    [
        &MISSING_HEADER_INCLUDE_FIXUP,
        &STALE_KBUILD_DIRECTORY_FIXUP,
        &STALE_KBUILD_OBJECT_FIXUP,
        &MISSING_KCONFIG_SOURCE_FIXUP,
    ]
}
