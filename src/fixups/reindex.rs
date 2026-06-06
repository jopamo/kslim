//! Read-only tree-index rebuilding and proof matching for fixups.
//!
//! Fixups re-read the reduced candidate tree before applying a diagnostic
//! repair. This module owns that fresh index construction and the exact proof
//! checks that tie planned edits back to indexed source facts.

use anyhow::Result;
use std::path::Path;

use crate::tree_index::TreeIndex;

use super::FixupProof;

pub(in crate::fixups) fn build_fixup_index(root: &Path) -> Result<TreeIndex> {
    TreeIndex::build(root, &())
}

pub(in crate::fixups) fn is_tree_index_truth(proof: &FixupProof) -> bool {
    matches!(
        proof,
        FixupProof::TreeIndexIncludeSite { .. }
            | FixupProof::TreeIndexKbuildDirectoryRef { .. }
            | FixupProof::TreeIndexKbuildObjectRef { .. }
            | FixupProof::TreeIndexKconfigSourceRef { .. }
    )
}

pub(in crate::fixups) fn proof_matches_tree_index(proof: &FixupProof, index: &TreeIndex) -> bool {
    match proof {
        FixupProof::TreeIndexIncludeSite { file, line, target } => {
            index.has_include_site(file, *line, target)
        }
        FixupProof::TreeIndexKbuildDirectoryRef {
            file,
            line,
            assignment_lhs,
            directory,
            resolved_path,
        } => index.has_kbuild_directory_ref(file, *line, assignment_lhs, directory, resolved_path),
        FixupProof::TreeIndexKbuildObjectRef {
            file,
            line,
            assignment_lhs,
            object,
            resolved_path,
        } => index.has_kbuild_object_ref(file, *line, assignment_lhs, object, resolved_path),
        FixupProof::TreeIndexKconfigSourceRef {
            file,
            line,
            source,
            optional,
            relative,
        } => index.has_kconfig_source_ref(file, *line, source, *optional, *relative),
        FixupProof::ManifestPath { .. } | FixupProof::ClassifiedDiagnostic { .. } => false,
    }
}
