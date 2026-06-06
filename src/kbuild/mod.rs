//! kbuild parsing and rewrite support for manifest-driven graph cleanup.
//!
//! The current reducer-owned kbuild surface is line-oriented:
//! - identify stale object and directory references after declared removals
//! - rewrite Makefile and Kbuild assignments with proof-carrying edits

mod ast;
mod object_graph;
mod parser;
mod report;
mod rewrite;

pub(crate) use ast::{
    CompositeKind, KbuildAssignment, KbuildAssignmentKind, LogicalLine, ObjListKind,
};
#[allow(unused_imports)]
pub(crate) use object_graph::{
    build_kbuild_index, has_object_provider, is_build_graph_assignment, make_dir_candidates,
    makefiles, normalize_relative, KbuildCompositeObjectMember, KbuildConfigGatedReference,
    KbuildDirectoryReference, KbuildIncludePathFlag, KbuildIndex, KbuildObjectReference,
};
pub(in crate::kbuild) use object_graph::{
    has_direct_object_provider, include_path_candidates, object_provider_path, relative_to_root,
    relative_to_root_path,
};
pub(crate) use parser::{
    logical_lines, parse_kbuild_assignment, parse_make_assignment,
    protected_make_logical_line_starts,
};
pub(in crate::kbuild) use parser::parse_kbuild_assignment_kind;
pub(crate) use report::{KbuildRewriteReport, KbuildSkippedLine};
#[cfg(test)]
pub(crate) use rewrite::{rewrite_makefiles, rewrite_makefiles_with_removed_configs};
pub(crate) use rewrite::{composite_objects, rewrite_makefiles_report};

#[cfg(test)]
mod tests;
