use super::*;
use crate::edit_reason::EditReason;
use std::path::PathBuf;

#[test]
fn test_logical_lines_joins_backslash_continuations() {
    let logical = logical_lines("obj-y += foo.o \\\n bar.o\nobj-y += baz.o\n");

    assert_eq!(logical.len(), 2);
    assert_eq!(logical[0].start_line, 1);
    assert_eq!(
        logical[0].original,
        vec![String::from("obj-y += foo.o \\"), String::from(" bar.o")]
    );
    assert_eq!(logical[0].joined, "obj-y += foo.o  bar.o");
    assert_eq!(logical[1].start_line, 3);
    assert_eq!(logical[1].joined, "obj-y += baz.o");
}

#[test]
fn test_protected_make_logical_line_starts_marks_recipes_and_define_blocks() {
    let logical = logical_lines(concat!(
        "define macro_body\n",
        "obj-y += protected.o\n",
        "endef\n",
        "all:\n",
        "\tobj-y += recipe.o\n",
        "obj-y += real.o\n",
    ));
    let protected = protected_make_logical_line_starts(&logical);

    assert!(protected.contains(&1));
    assert!(protected.contains(&2));
    assert!(protected.contains(&3));
    assert!(protected.contains(&5));
    assert!(!protected.contains(&4));
    assert!(!protected.contains(&6));
}

#[test]
fn test_parse_make_assignment_strips_comment_and_supports_operators() {
    assert_eq!(
        parse_make_assignment("obj-y += foo.o bar.o # trailing note"),
        Some(("obj-y", "+=", "foo.o bar.o"))
    );
    assert_eq!(
        parse_make_assignment("foo := bar"),
        Some(("foo", ":=", "bar"))
    );
    assert_eq!(parse_make_assignment("# comment only"), None);
}

#[test]
fn test_parse_make_assignment_ignores_recipe_lines() {
    assert_eq!(parse_make_assignment("\tobj-y += foo.o"), None);
}

#[test]
fn test_parse_kbuild_assignment_forms() {
    assert_eq!(
        parse_kbuild_assignment("obj-y += foo.o"),
        Some(KbuildAssignment {
            lhs: "obj-y",
            op: "+=",
            rhs: "foo.o",
            kind: KbuildAssignmentKind::ObjList(ObjListKind::BuiltIn),
        })
    );
    assert_eq!(
        parse_kbuild_assignment("obj-m += foo.o"),
        Some(KbuildAssignment {
            lhs: "obj-m",
            op: "+=",
            rhs: "foo.o",
            kind: KbuildAssignmentKind::ObjList(ObjListKind::Module),
        })
    );
    assert_eq!(
        parse_kbuild_assignment("obj-$(CONFIG_FOO) += foo.o"),
        Some(KbuildAssignment {
            lhs: "obj-$(CONFIG_FOO)",
            op: "+=",
            rhs: "foo.o",
            kind: KbuildAssignmentKind::ObjList(ObjListKind::Config("FOO")),
        })
    );
    assert_eq!(
        parse_kbuild_assignment("foo-y += a.o b.o"),
        Some(KbuildAssignment {
            lhs: "foo-y",
            op: "+=",
            rhs: "a.o b.o",
            kind: KbuildAssignmentKind::CompositeMembers(CompositeKind::BuiltIn {
                target: "foo",
            }),
        })
    );
    assert_eq!(
        parse_kbuild_assignment("foo-$(CONFIG_BAR) += bar.o"),
        Some(KbuildAssignment {
            lhs: "foo-$(CONFIG_BAR)",
            op: "+=",
            rhs: "bar.o",
            kind: KbuildAssignmentKind::CompositeMembers(CompositeKind::Config {
                target: "foo",
                symbol: "BAR",
            }),
        })
    );
    assert_eq!(
        parse_kbuild_assignment("subdir-y += dir/"),
        Some(KbuildAssignment {
            lhs: "subdir-y",
            op: "+=",
            rhs: "dir/",
            kind: KbuildAssignmentKind::SubdirList,
        })
    );
    assert_eq!(
        parse_kbuild_assignment("ccflags-y += -Iinclude"),
        Some(KbuildAssignment {
            lhs: "ccflags-y",
            op: "+=",
            rhs: "-Iinclude",
            kind: KbuildAssignmentKind::CcFlags,
        })
    );
    assert_eq!(
        parse_kbuild_assignment("subdir-ccflags-y += -Iinclude/subdir/"),
        Some(KbuildAssignment {
            lhs: "subdir-ccflags-y",
            op: "+=",
            rhs: "-Iinclude/subdir/",
            kind: KbuildAssignmentKind::CcFlags,
        })
    );
}

#[test]
fn test_composite_objects_detects_composite_assignment_forms() {
    let lines = vec![
        LogicalLine {
            start_line: 1,
            original: vec![String::from("foo-y += a.o b.o")],
            joined: String::from("foo-y += a.o b.o"),
        },
        LogicalLine {
            start_line: 2,
            original: vec![String::from("bar-$(CONFIG_BAZ) += c.o")],
            joined: String::from("bar-$(CONFIG_BAZ) += c.o"),
        },
        LogicalLine {
            start_line: 3,
            original: vec![String::from("qux-objs := d.o")],
            joined: String::from("qux-objs := d.o"),
        },
        LogicalLine {
            start_line: 4,
            original: vec![String::from("subdir-y += drivers/")],
            joined: String::from("subdir-y += drivers/"),
        },
        LogicalLine {
            start_line: 5,
            original: vec![String::from("ccflags-y += -Iinclude")],
            joined: String::from("ccflags-y += -Iinclude"),
        },
    ];

    let composite = composite_objects(&lines);

    assert!(composite.contains("foo.o"));
    assert!(composite.contains("bar.o"));
    assert!(composite.contains("qux.o"));
    assert!(!composite.contains("subdir.o"));
    assert!(!composite.contains("ccflags.o"));
}

#[test]
fn test_is_build_graph_assignment_matches_graph_variables_only() {
    assert!(is_build_graph_assignment("obj-y"));
    assert!(is_build_graph_assignment("obj-m"));
    assert!(is_build_graph_assignment("obj-$(CONFIG_FOO)"));
    assert!(is_build_graph_assignment("foo-y"));
    assert!(is_build_graph_assignment("bar-$(CONFIG_BAZ)"));
    assert!(is_build_graph_assignment("subdir-y"));
    assert!(is_build_graph_assignment("ccflags-y"));
    assert!(is_build_graph_assignment("lib-y"));
    assert!(!is_build_graph_assignment("ARCH_PROCESSED"));
}

#[test]
fn test_make_dir_candidates_returns_sorted_current_and_root_relative_candidates() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let current_dir = root.join("zeta");

    let candidates = make_dir_candidates(root, &current_dir, "alpha/");

    assert_eq!(
        candidates,
        vec![PathBuf::from("alpha"), PathBuf::from("zeta/alpha")]
    );
}

#[test]
fn test_include_path_candidates_returns_sorted_current_and_root_relative_candidates() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let current_dir = root.join("zeta");

    let candidates = include_path_candidates(root, &current_dir, "alpha");

    assert_eq!(
        candidates,
        vec![PathBuf::from("alpha"), PathBuf::from("zeta/alpha")]
    );
}

#[test]
fn test_build_kbuild_index_collects_providers_refs_dirs_gates_and_include_flags() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::create_dir_all(root.join("drivers/foo/child")).unwrap();
    std::fs::create_dir_all(root.join("drivers/foo/vendor")).unwrap();
    std::fs::create_dir_all(root.join("include/linux")).unwrap();

    for source in [
        "drivers/foo/foo.c",
        "drivers/foo/module.c",
        "drivers/foo/vendor.c",
        "drivers/foo/a.c",
        "drivers/foo/b.c",
        "drivers/foo/c.c",
    ] {
        std::fs::write(root.join(source), "int x;\n").unwrap();
    }
    std::fs::write(root.join("drivers/foo/shipped.o_shipped"), "binary").unwrap();

    std::fs::write(
        root.join("drivers/foo/Makefile"),
        concat!(
            "obj-y += foo.o\n",
            "obj-m += module.o\n",
            "obj-$(CONFIG_VENDOR) += vendor.o vendor/\n",
            "foo-y += a.o b.o\n",
            "foo-$(CONFIG_BAR) += c.o\n",
            "subdir-y += child/\n",
            "ccflags-y += -Iinclude/linux -Iarch/x86/include\n",
            "subdir-ccflags-y += -Iinclude/subdir/\n",
        ),
    )
    .unwrap();

    let index = build_kbuild_index(root).unwrap();

    assert!(index
        .object_providers
        .contains(&PathBuf::from("drivers/foo/foo.o")));
    assert!(index
        .object_providers
        .contains(&PathBuf::from("drivers/foo/module.o")));
    assert!(index
        .object_providers
        .contains(&PathBuf::from("drivers/foo/vendor.o")));
    assert!(index
        .object_providers
        .contains(&PathBuf::from("drivers/foo/shipped.o")));

    assert!(index.object_references.iter().any(|reference| {
        reference.file == PathBuf::from("drivers/foo/Makefile")
            && reference.assignment_lhs == "obj-y"
            && reference.object == "foo.o"
    }));
    assert!(index
        .object_references
        .iter()
        .any(|reference| { reference.assignment_lhs == "foo-y" && reference.object == "a.o" }));
    assert!(index.object_references.iter().any(|reference| {
        reference.assignment_lhs == "foo-$(CONFIG_BAR)" && reference.object == "c.o"
    }));

    assert!(index.composite_object_members.iter().any(|member| {
        member.target == PathBuf::from("drivers/foo/foo.o") && member.member == "a.o"
    }));
    assert!(index.composite_object_members.iter().any(|member| {
        member.target == PathBuf::from("drivers/foo/foo.o") && member.member == "c.o"
    }));

    assert!(index.directory_references.iter().any(|reference| {
        reference.assignment_lhs == "obj-$(CONFIG_VENDOR)" && reference.directory == "vendor/"
    }));
    assert!(index.directory_references.iter().any(|reference| {
        reference.assignment_lhs == "subdir-y" && reference.directory == "child/"
    }));

    assert!(index.config_gated_references.iter().any(|reference| {
        reference.symbol == "VENDOR" && reference.reference == "vendor.o"
    }));
    assert!(index
        .config_gated_references
        .iter()
        .any(|reference| { reference.symbol == "VENDOR" && reference.reference == "vendor/" }));
    assert!(index
        .config_gated_references
        .iter()
        .any(|reference| { reference.symbol == "BAR" && reference.reference == "c.o" }));

    assert!(index.include_path_flags.iter().any(|flag| {
        flag.file == PathBuf::from("drivers/foo/Makefile")
            && flag.flag == "-Iinclude/linux"
            && flag.include_path == "include/linux"
    }));
    assert!(index.include_path_flags.iter().any(|flag| {
        flag.flag == "-Iarch/x86/include" && flag.include_path == "arch/x86/include"
    }));
    assert!(index.include_path_flags.iter().any(|flag| {
        flag.flag == "-Iinclude/subdir/" && flag.include_path == "include/subdir/"
    }));
}

#[test]
fn test_rewrite_makefiles_removes_refs_to_removed_directories() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("drivers/foo/keep")).unwrap();
    std::fs::create_dir_all(root.join("drivers/foo/remove")).unwrap();
    std::fs::write(
        root.join("drivers/foo/Makefile"),
        "obj-y += keep/ remove/\n",
    )
    .unwrap();

    let (removed, edits) =
        rewrite_makefiles(root, &[], &[PathBuf::from("drivers/foo/remove")]).unwrap();

    assert_eq!(removed, 1);
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
        "obj-y += keep/\n"
    );
    assert_eq!(edits.len(), 1);
    assert!(matches!(
        edits[0].reason,
        EditReason::RemovedKbuildRef { ref reference } if reference == "remove/"
    ));
}

#[test]
fn test_rewrite_makefiles_removes_subdir_refs_to_removed_directories_and_simplifies_empty_assignment(
) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("drivers/foo/remove")).unwrap();
    std::fs::write(root.join("drivers/foo/Kbuild"), "subdir-y += remove/\n").unwrap();

    let (removed, edits) =
        rewrite_makefiles(root, &[], &[PathBuf::from("drivers/foo/remove")]).unwrap();

    assert_eq!(removed, 1);
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/Kbuild")).unwrap(),
        "# kslim: removed stale make refs from subdir-y\n"
    );
    assert_eq!(edits.len(), 1);
    assert!(matches!(
        edits[0].reason,
        EditReason::RemovedKbuildRef { ref reference } if reference == "remove/"
    ));
}

#[test]
fn test_rewrite_makefiles_removes_obj_m_refs_to_removed_object_provider() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(root.join("drivers/foo/keep.c"), "int keep;\n").unwrap();
    std::fs::write(root.join("drivers/foo/remove.c"), "int remove;\n").unwrap();
    std::fs::write(
        root.join("drivers/foo/Makefile"),
        "obj-m += keep.o remove.o\n",
    )
    .unwrap();

    let (removed, edits) =
        rewrite_makefiles(root, &[PathBuf::from("drivers/foo/remove.c")], &[]).unwrap();

    assert_eq!(removed, 1);
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
        "obj-m += keep.o\n"
    );
    assert_eq!(edits.len(), 1);
    assert!(matches!(
        edits[0].reason,
        EditReason::RemovedKbuildRef { ref reference } if reference == "remove.o"
    ));
}

#[test]
fn test_rewrite_makefiles_keeps_live_config_gated_composite_target_on_unrelated_removal() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("Documentation")).unwrap();
    std::fs::write(root.join("Documentation/.gitignore"), "tmp\n").unwrap();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(root.join("drivers/foo/helper.c"), "int helper;\n").unwrap();
    std::fs::write(
        root.join("drivers/foo/Makefile"),
        "foo-$(CONFIG_LIVE) += helper.o\nobj-$(CONFIG_LIVE) += foo.o\n",
    )
    .unwrap();

    let original = std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap();
    let (removed, edits) =
        rewrite_makefiles(root, &[PathBuf::from("Documentation/.gitignore")], &[]).unwrap();

    assert_eq!(removed, 0);
    assert!(edits.is_empty());
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
        original
    );
}

#[test]
fn test_rewrite_makefiles_keeps_live_multi_composite_targets_on_unrelated_removal() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("Documentation")).unwrap();
    std::fs::write(root.join("Documentation/.gitignore"), "tmp\n").unwrap();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(root.join("drivers/foo/one.c"), "int one;\n").unwrap();
    std::fs::write(root.join("drivers/foo/two.c"), "int two;\n").unwrap();
    std::fs::write(
        root.join("drivers/foo/Makefile"),
        concat!(
            "foo-$(CONFIG_LIVE) += one.o\n",
            "bar-$(CONFIG_LIVE) += two.o\n",
            "obj-$(CONFIG_LIVE) += foo.o bar.o\n",
        ),
    )
    .unwrap();

    let original = std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap();
    let (removed, edits) =
        rewrite_makefiles(root, &[PathBuf::from("Documentation/.gitignore")], &[]).unwrap();

    assert_eq!(removed, 0);
    assert!(edits.is_empty());
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
        original
    );
}

#[test]
fn test_rewrite_makefiles_removes_composite_member_refs_to_removed_object_provider() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(root.join("drivers/foo/live.c"), "int live;\n").unwrap();
    std::fs::write(root.join("drivers/foo/remove.c"), "int remove;\n").unwrap();
    std::fs::write(
        root.join("drivers/foo/Kbuild"),
        "foo-y += live.o remove.o\nobj-y += foo.o\n",
    )
    .unwrap();

    let (removed, edits) =
        rewrite_makefiles(root, &[PathBuf::from("drivers/foo/remove.c")], &[]).unwrap();

    assert_eq!(removed, 1);
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/Kbuild")).unwrap(),
        "foo-y += live.o\nobj-y += foo.o\n"
    );
    assert_eq!(edits.len(), 1);
    assert!(matches!(
        edits[0].reason,
        EditReason::RemovedKbuildRef { ref reference } if reference == "remove.o"
    ));
}

#[test]
fn test_rewrite_makefiles_removes_stale_composite_targets_emptied_by_member_removal() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(root.join("drivers/foo/remove.c"), "int remove;\n").unwrap();
    std::fs::write(
        root.join("drivers/foo/Makefile"),
        "foo-y += remove.o\nobj-y += foo.o\n",
    )
    .unwrap();

    let (removed, edits) =
        rewrite_makefiles(root, &[PathBuf::from("drivers/foo/remove.c")], &[]).unwrap();

    assert_eq!(removed, 2);
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
        "# kslim: removed stale make refs from foo-y\n# kslim: removed stale make refs from obj-y\n"
    );
    assert_eq!(edits.len(), 2);
    assert!(edits.iter().any(|edit| matches!(
        edit.reason,
        EditReason::RemovedKbuildRef { ref reference } if reference == "remove.o"
    )));
    assert!(edits.iter().any(|edit| matches!(
        edit.reason,
        EditReason::RemovedKbuildRef { ref reference } if reference == "foo.o"
    )));
}

#[test]
fn test_rewrite_makefiles_removes_obj_refs_gated_by_removed_config_symbol() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("drivers/foo/keep")).unwrap();
    std::fs::write(root.join("drivers/foo/helper.c"), "int helper;\n").unwrap();
    std::fs::write(
        root.join("drivers/foo/Makefile"),
        "obj-$(CONFIG_REMOVED) += keep/ helper.o\n",
    )
    .unwrap();

    let (removed, edits) =
        rewrite_makefiles_with_removed_configs(root, &[], &[], &[String::from("REMOVED")])
            .unwrap();

    assert_eq!(removed, 2);
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
        "# kslim: removed stale make refs from obj-$(CONFIG_REMOVED)\n"
    );
    assert_eq!(edits.len(), 2);
    assert!(edits.iter().any(|edit| matches!(
        edit.reason,
        EditReason::RemovedKbuildRef { ref reference } if reference == "keep/"
    )));
    assert!(edits.iter().any(|edit| matches!(
        edit.reason,
        EditReason::RemovedKbuildRef { ref reference } if reference == "helper.o"
    )));
}

#[test]
fn test_rewrite_makefiles_removes_composite_members_gated_by_removed_config_symbol() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(root.join("drivers/foo/helper.c"), "int helper;\n").unwrap();
    std::fs::write(
        root.join("drivers/foo/Kbuild"),
        "foo-$(CONFIG_REMOVED) += helper.o\nobj-y += foo.o\n",
    )
    .unwrap();

    let (removed, edits) =
        rewrite_makefiles_with_removed_configs(root, &[], &[], &[String::from("REMOVED")])
            .unwrap();

    assert_eq!(removed, 2);
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/Kbuild")).unwrap(),
        "# kslim: removed stale make refs from foo-$(CONFIG_REMOVED)\n# kslim: removed stale make refs from obj-y\n"
    );
    assert_eq!(edits.len(), 2);
    assert!(edits.iter().any(|edit| matches!(
        edit.reason,
        EditReason::RemovedKbuildRef { ref reference } if reference == "helper.o"
    )));
    assert!(edits.iter().any(|edit| matches!(
        edit.reason,
        EditReason::RemovedKbuildRef { ref reference } if reference == "foo.o"
    )));
}

#[test]
fn test_rewrite_makefiles_removes_stale_ccflags_include_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("drivers/foo/include")).unwrap();
    std::fs::create_dir_all(root.join("drivers/foo/headers")).unwrap();
    std::fs::write(
        root.join("drivers/foo/Makefile"),
        "ccflags-y += -Iinclude -Iheaders -Werror\n",
    )
    .unwrap();
    std::fs::remove_dir_all(root.join("drivers/foo/include")).unwrap();

    let (removed, edits) =
        rewrite_makefiles(root, &[], &[PathBuf::from("drivers/foo/include")]).unwrap();

    assert_eq!(removed, 1);
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
        "ccflags-y += -Iheaders -Werror\n"
    );
    assert_eq!(edits.len(), 1);
    assert!(matches!(
        edits[0].reason,
        EditReason::RemovedKbuildRef { ref reference }
            if reference == "-Iinclude"
    ));
}

#[test]
fn test_rewrite_makefiles_preserves_shell_fragments_while_rewriting_build_graph_lines() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("drivers/foo/keep")).unwrap();
    std::fs::create_dir_all(root.join("drivers/foo/remove")).unwrap();
    std::fs::write(
        root.join("drivers/foo/Makefile"),
        concat!(
            "quiet_cmd_check_sha1 = CHKSHA1 $<\n",
            "cmd_check_sha1 = if ! command -v sha1sum >/dev/null; then echo \"warning: cannot check the header due to sha1sum missing\"; exit 0; fi; if [ \"$$(sed -n '$$s:// ::p' $<)\" != \"$$(sed '$$d' $< | sha1sum | sed 's/ .*//')\" ]; then echo \"error: $< has been modified.\" >&2; exit 1; fi; touch $@\n",
            "obj-y += keep/ remove/\n",
        ),
    )
    .unwrap();

    let (removed, edits) =
        rewrite_makefiles(root, &[], &[PathBuf::from("drivers/foo/remove")]).unwrap();

    assert_eq!(removed, 1);
    assert_eq!(edits.len(), 1);
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
        concat!(
            "quiet_cmd_check_sha1 = CHKSHA1 $<\n",
            "cmd_check_sha1 = if ! command -v sha1sum >/dev/null; then echo \"warning: cannot check the header due to sha1sum missing\"; exit 0; fi; if [ \"$$(sed -n '$$s:// ::p' $<)\" != \"$$(sed '$$d' $< | sha1sum | sed 's/ .*//')\" ]; then echo \"error: $< has been modified.\" >&2; exit 1; fi; touch $@\n",
            "obj-y += keep/\n",
        )
    );
}

#[test]
fn test_rewrite_makefiles_preserves_non_build_assignments_while_rewriting_build_graph_lines() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("drivers/foo/keep")).unwrap();
    std::fs::create_dir_all(root.join("drivers/foo/remove")).unwrap();
    std::fs::write(
        root.join("drivers/foo/Makefile"),
        concat!(
            "ARCH_PROCESSED := x86\n",
            "SOME_OTHER_VAR = keep-me\n",
            "obj-y += keep/ remove/\n",
        ),
    )
    .unwrap();

    let (removed, edits) =
        rewrite_makefiles(root, &[], &[PathBuf::from("drivers/foo/remove")]).unwrap();

    assert_eq!(removed, 1);
    assert_eq!(edits.len(), 1);
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
        concat!(
            "ARCH_PROCESSED := x86\n",
            "SOME_OTHER_VAR = keep-me\n",
            "obj-y += keep/\n",
        )
    );
}

#[test]
fn test_rewrite_makefiles_preserves_recipe_lines_that_look_like_build_assignments() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("drivers/foo/keep")).unwrap();
    std::fs::create_dir_all(root.join("drivers/foo/remove")).unwrap();
    std::fs::write(root.join("drivers/foo/remove.c"), "int remove;\n").unwrap();
    std::fs::write(
        root.join("drivers/foo/Makefile"),
        concat!(
            "obj-y += keep/ remove/\n",
            "all:\n",
            "\tobj-y += remove.o\n",
        ),
    )
    .unwrap();

    let (removed, edits) = rewrite_makefiles(
        root,
        &[PathBuf::from("drivers/foo/remove.c")],
        &[PathBuf::from("drivers/foo/remove")],
    )
    .unwrap();

    assert_eq!(removed, 1);
    assert_eq!(edits.len(), 1);
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
        concat!("obj-y += keep/\n", "all:\n", "\tobj-y += remove.o\n",)
    );
}

#[test]
fn test_rewrite_makefiles_preserves_define_blocks_that_look_like_build_assignments() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("drivers/foo/keep")).unwrap();
    std::fs::create_dir_all(root.join("drivers/foo/remove")).unwrap();
    std::fs::write(root.join("drivers/foo/remove.c"), "int remove;\n").unwrap();
    std::fs::write(
        root.join("drivers/foo/Makefile"),
        concat!(
            "define macro_body\n",
            "obj-y += remove/\n",
            "foo-y += remove.o\n",
            "endef\n",
            "obj-y += keep/ remove/\n",
        ),
    )
    .unwrap();

    let (removed, edits) = rewrite_makefiles(
        root,
        &[PathBuf::from("drivers/foo/remove.c")],
        &[PathBuf::from("drivers/foo/remove")],
    )
    .unwrap();

    assert_eq!(removed, 1);
    assert_eq!(edits.len(), 1);
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
        concat!(
            "define macro_body\n",
            "obj-y += remove/\n",
            "foo-y += remove.o\n",
            "endef\n",
            "obj-y += keep/\n",
        )
    );
}

#[test]
fn test_rewrite_makefiles_preserves_comments_while_rewriting_build_graph_lines() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("drivers/foo/keep")).unwrap();
    std::fs::create_dir_all(root.join("drivers/foo/remove")).unwrap();
    std::fs::write(
        root.join("drivers/foo/Makefile"),
        concat!(
            "# keep intro comment\n",
            "obj-y += keep/ remove/  # keep trailing comment\n",
            "# keep outro comment\n",
        ),
    )
    .unwrap();

    let (removed, edits) =
        rewrite_makefiles(root, &[], &[PathBuf::from("drivers/foo/remove")]).unwrap();

    assert_eq!(removed, 1);
    assert_eq!(edits.len(), 1);
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
        concat!(
            "# keep intro comment\n",
            "obj-y += keep/  # keep trailing comment\n",
            "# keep outro comment\n",
        )
    );
}

#[test]
fn test_rewrite_makefiles_preserves_multiline_assignment_layout() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("drivers/foo/live")).unwrap();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(root.join("drivers/foo/first.c"), "int first;\n").unwrap();
    std::fs::write(root.join("drivers/foo/remove.c"), "int remove;\n").unwrap();
    std::fs::write(root.join("drivers/foo/second.c"), "int second;\n").unwrap();
    std::fs::write(
        root.join("drivers/foo/Makefile"),
        concat!(
            "ccflags-y += -DKEEP\n",
            "obj-y += first.o \\\n",
            "         remove.o \\\n",
            "         second.o # keep object-list note\n",
            "subdir-y += live/\n",
        ),
    )
    .unwrap();

    let (removed, edits) =
        rewrite_makefiles(root, &[PathBuf::from("drivers/foo/remove.c")], &[]).unwrap();

    assert_eq!(removed, 1);
    assert_eq!(edits.len(), 1);
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
        concat!(
            "ccflags-y += -DKEEP\n",
            "obj-y += first.o \\\n",
            "         second.o # keep object-list note\n",
            "subdir-y += live/\n",
        )
    );
}

#[test]
fn test_rewrite_makefiles_preserves_continuation_when_first_token_drops() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(root.join("drivers/foo/remove.c"), "int remove;\n").unwrap();
    std::fs::write(root.join("drivers/foo/keep.c"), "int keep;\n").unwrap();
    std::fs::write(
        root.join("drivers/foo/Makefile"),
        concat!(
            "obj-y += remove.o \\\n",
            "         keep.o # keep tail note\n",
        ),
    )
    .unwrap();

    let (removed, edits) =
        rewrite_makefiles(root, &[PathBuf::from("drivers/foo/remove.c")], &[]).unwrap();

    assert_eq!(removed, 1);
    assert_eq!(edits.len(), 1);
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
        concat!("obj-y += \\\n", "         keep.o # keep tail note\n",)
    );
}

#[test]
fn test_rewrite_makefiles_preserves_surviving_token_and_line_order() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("drivers/foo/keep")).unwrap();
    std::fs::create_dir_all(root.join("drivers/foo/remove")).unwrap();
    std::fs::create_dir_all(root.join("drivers/foo/child")).unwrap();
    std::fs::write(root.join("drivers/foo/first.c"), "int first;\n").unwrap();
    std::fs::write(root.join("drivers/foo/remove.c"), "int remove;\n").unwrap();
    std::fs::write(root.join("drivers/foo/second.c"), "int second;\n").unwrap();
    std::fs::write(
        root.join("drivers/foo/Makefile"),
        concat!(
            "lib-y += before.o\n",
            "obj-y += first.o remove.o keep/ remove/ second.o\n",
            "subdir-y += child/\n",
        ),
    )
    .unwrap();
    std::fs::write(root.join("drivers/foo/before.c"), "int before;\n").unwrap();

    let (removed, edits) = rewrite_makefiles(
        root,
        &[PathBuf::from("drivers/foo/remove.c")],
        &[PathBuf::from("drivers/foo/remove")],
    )
    .unwrap();

    assert_eq!(removed, 2);
    assert_eq!(edits.len(), 2);
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
        concat!(
            "lib-y += before.o\n",
            "obj-y += first.o keep/ second.o\n",
            "subdir-y += child/\n",
        )
    );
}

#[test]
fn test_rewrite_makefiles_keeps_ambiguous_live_ccflags_include_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("drivers/foo/include")).unwrap();
    std::fs::create_dir_all(root.join("include")).unwrap();
    std::fs::write(
        root.join("drivers/foo/Makefile"),
        "ccflags-y += -Iinclude -Werror\n",
    )
    .unwrap();

    let original = std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap();
    let (removed, edits) =
        rewrite_makefiles(root, &[], &[PathBuf::from("drivers/foo/include")]).unwrap();

    assert_eq!(removed, 0);
    assert!(edits.is_empty());
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
        original
    );
}

#[test]
fn test_rewrite_makefiles_reports_ambiguous_live_ccflags_include_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("drivers/foo/include")).unwrap();
    std::fs::create_dir_all(root.join("include")).unwrap();
    std::fs::write(
        root.join("drivers/foo/Makefile"),
        "ccflags-y += -Iinclude -Werror\n",
    )
    .unwrap();

    let report =
        rewrite_makefiles_report(root, &[], &[PathBuf::from("drivers/foo/include")], &[])
            .unwrap();

    assert_eq!(report.removed_refs, 0);
    assert!(report.edits.is_empty());
    assert_eq!(report.skipped_ambiguous_lines.len(), 1);
    assert_eq!(
        report.skipped_ambiguous_lines[0],
        KbuildSkippedLine {
            file: PathBuf::from("drivers/foo/Makefile"),
            line: 1,
            assignment_lhs: String::from("ccflags-y"),
            reason: String::from(
                "ambiguous include path flag '-Iinclude' resolves to both removed and live paths",
            ),
        }
    );
}

#[test]
fn test_rewrite_makefiles_removes_refs_to_removed_shipped_object_provider() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(root.join("drivers/foo/keep.c"), "int keep;\n").unwrap();
    std::fs::write(root.join("drivers/foo/blob.o_shipped"), "blob").unwrap();
    std::fs::write(
        root.join("drivers/foo/Makefile"),
        "obj-y += keep.o blob.o\n",
    )
    .unwrap();

    let (removed, edits) =
        rewrite_makefiles(root, &[PathBuf::from("drivers/foo/blob.o_shipped")], &[]).unwrap();

    assert_eq!(removed, 1);
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
        "obj-y += keep.o\n"
    );
    assert_eq!(edits.len(), 1);
    assert!(matches!(
        edits[0].reason,
        EditReason::RemovedKbuildRef { ref reference } if reference == "blob.o"
    ));
}

#[test]
fn test_rewrite_makefiles_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("drivers/foo/remove")).unwrap();
    std::fs::write(root.join("drivers/foo/Makefile"), "obj-y += remove/\n").unwrap();

    let (first_removed, first_edits) =
        rewrite_makefiles(root, &[], &[PathBuf::from("drivers/foo/remove")]).unwrap();
    let after_first = std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap();

    let (second_removed, second_edits) =
        rewrite_makefiles(root, &[], &[PathBuf::from("drivers/foo/remove")]).unwrap();
    let after_second = std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap();

    assert_eq!(first_removed, 1);
    assert_eq!(first_edits.len(), 1);
    assert_eq!(second_removed, 0);
    assert!(second_edits.is_empty());
    assert_eq!(after_first, after_second);
}
