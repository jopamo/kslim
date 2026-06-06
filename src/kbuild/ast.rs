//! Kbuild logical-line and assignment AST types.
//!
//! This module owns the parsed, borrowed value shapes consumed by kbuild
//! parsing, indexing, and rewrite code.

#[derive(Debug, Clone)]
pub(crate) struct LogicalLine {
    pub start_line: usize,
    pub original: Vec<String>,
    pub joined: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KbuildAssignment<'a> {
    pub lhs: &'a str,
    pub op: &'a str,
    pub rhs: &'a str,
    pub kind: KbuildAssignmentKind<'a>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KbuildAssignmentKind<'a> {
    ObjList(ObjListKind<'a>),
    CompositeMembers(CompositeKind<'a>),
    SubdirList,
    CcFlags,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ObjListKind<'a> {
    BuiltIn,
    Module,
    Config(&'a str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompositeKind<'a> {
    BuiltIn { target: &'a str },
    Module { target: &'a str },
    Config { target: &'a str, symbol: &'a str },
    Objs { target: &'a str },
}

impl<'a> CompositeKind<'a> {
    pub(in crate::kbuild) fn target(&self) -> &'a str {
        match self {
            Self::BuiltIn { target }
            | Self::Module { target }
            | Self::Config { target, .. }
            | Self::Objs { target } => target,
        }
    }
}
