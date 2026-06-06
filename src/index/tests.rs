use super::*;
use super::file_index::{index_path_is_under, is_relative_index_path};
use super::source_index::parse_include_target;
use std::collections::BTreeMap;

#[test]
fn test_tree_index_build_indexes_files_include_sites_kconfig_refs_and_kbuild_refs() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("Kconfig"),
        concat!(
            "source \"drivers/foo/Kconfig\"\n",
            "rsource \"subsys/Kconfig\"\n",
            "config FOO\n",
            "\tdepends on BAR || BAZ\n",
            "\tselect QUX if EXTRA\n",
            "\tdefault y if DEFAULT_GATE\n",
        ),
    )
    .unwrap();
    std::fs::write(root.join("Kconfig.extra"), "menuconfig EXTRA_MENU\n").unwrap();
    std::fs::create_dir_all(root.join("subsys")).unwrap();
    std::fs::write(
        root.join("subsys/Kconfig"),
        "config SUBSYS\n\tbool \"Subsys\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(root.join("drivers/foo/remove")).unwrap();
    std::fs::write(root.join("drivers/foo/remove.c"), "int remove;\n").unwrap();
    std::fs::write(
        root.join("drivers/foo/Makefile"),
        "obj-y += remove.o\nsubdir-y += remove/\n",
    )
    .unwrap();
    std::fs::create_dir_all(root.join("include/linux")).unwrap();
    std::fs::write(root.join("include/linux/kernel.h"), "#define KERNEL 1\n").unwrap();
    std::fs::create_dir_all(root.join("drivers/gpu/drm")).unwrap();
    std::fs::write(
        root.join("drivers/gpu/drm/helper.c"),
        concat!(
            "#include <amd/amdgpu/amdgpu_missing.h>\n",
            "#include \"local.h\"\n",
            "#include <linux/kernel.h>\n",
            "#if defined(CONFIG_DRM_FOO) || defined(DEBUG)\n",
            "int helper;\n",
            "#endif\n",
        ),
    )
    .unwrap();

    let index = TreeIndex::build(root, &()).unwrap();

    assert_tree_index_paths_are_relative(&index);
    assert_eq!(
        index.coverage_stats(),
        TreeIndexCoverageStats {
            files_scanned: 7,
            headers_indexed: 1,
            include_sites_indexed: 3,
            kconfig_files_indexed: 3,
            kconfig_symbols_defined: 3,
            kconfig_symbol_refs_indexed: 5,
            kbuild_files_indexed: 1,
            kbuild_object_refs_indexed: 1,
            cpp_gates_indexed: 1,
            abi_paths_indexed: 1,
            abi_source_refs_indexed: 1,
        }
    );
    assert!(index.contains_file(Path::new("drivers/gpu/drm/helper.c")));
    assert!(index.headers.contains(Path::new("include/linux/kernel.h")));
    assert!(index.abi_paths.contains(&AbiPathFact {
        path: PathBuf::from("include/linux/kernel.h"),
        kind: AbiSurfaceKind::PublicHeader,
    }));
    assert!(index.kconfig_files.contains(Path::new("Kconfig")));
    assert!(index.kconfig_files.contains(Path::new("Kconfig.extra")));
    assert!(index.kconfig_defs.contains(&KconfigDefinition {
        file: PathBuf::from("Kconfig"),
        line: 3,
        symbol: String::from("FOO"),
    }));
    assert!(index.kconfig_defs.contains(&KconfigDefinition {
        file: PathBuf::from("Kconfig.extra"),
        line: 1,
        symbol: String::from("EXTRA_MENU"),
    }));
    assert!(index.kconfig_refs.contains(&KconfigSymbolReference {
        file: PathBuf::from("Kconfig"),
        line: 4,
        directive: String::from("depends_on"),
        symbol: String::from("BAR"),
    }));
    assert!(index.kconfig_refs.contains(&KconfigSymbolReference {
        file: PathBuf::from("Kconfig"),
        line: 5,
        directive: String::from("select"),
        symbol: String::from("EXTRA"),
    }));
    assert!(index.kconfig_refs.contains(&KconfigSymbolReference {
        file: PathBuf::from("Kconfig"),
        line: 6,
        directive: String::from("default"),
        symbol: String::from("DEFAULT_GATE"),
    }));
    assert!(index
        .find_kconfig_source_ref(Path::new("Kconfig"), 1, "drivers/foo/Kconfig")
        .is_some());
    assert!(index.has_kconfig_source_ref(
        Path::new("Kconfig"),
        1,
        "drivers/foo/Kconfig",
        false,
        false
    ));
    assert!(index.has_kconfig_source_ref(
        Path::new("Kconfig"),
        2,
        "subsys/Kconfig",
        false,
        true
    ));
    assert!(index
        .kbuild_files
        .contains(Path::new("drivers/foo/Makefile")));
    assert!(index
        .kbuild_object_providers
        .contains(Path::new("drivers/foo/remove.o")));
    assert!(index.has_include_site(
        Path::new("drivers/gpu/drm/helper.c"),
        1,
        "amd/amdgpu/amdgpu_missing.h"
    ));
    assert!(index.has_include_site(Path::new("drivers/gpu/drm/helper.c"), 2, "local.h"));
    assert!(index.has_include_site(
        Path::new("drivers/gpu/drm/helper.c"),
        3,
        "linux/kernel.h"
    ));
    assert!(index.abi_source_refs.contains(&AbiSourceReference {
        file: PathBuf::from("drivers/gpu/drm/helper.c"),
        line: 3,
        target: PathBuf::from("include/linux/kernel.h"),
        kind: AbiSurfaceKind::PublicHeader,
    }));
    assert!(index
        .cpp_gates_by_symbol
        .get("DRM_FOO")
        .is_some_and(|gates| gates.contains(&CppGate {
            file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 4,
            directive: String::from("if"),
            expression: String::from("defined(CONFIG_DRM_FOO) || defined(DEBUG)"),
        })));
    assert!(index
        .cpp_gates_by_symbol
        .get("DEBUG")
        .is_some_and(|gates| gates.iter().any(|gate| gate.line == 4)));
    assert!(index
        .find_include_site(Path::new("drivers/gpu/drm/helper.c"), "local.h")
        .is_some());
    let object_refs = index.find_kbuild_object_refs("drivers/foo/remove.o");
    assert_eq!(object_refs.len(), 1);
    assert_eq!(object_refs[0].file, PathBuf::from("drivers/foo/Makefile"));
    assert_eq!(object_refs[0].line, 1);
    assert_eq!(object_refs[0].assignment_lhs, "obj-y");
    assert_eq!(object_refs[0].object, "remove.o");
    assert_eq!(
        object_refs[0].resolved_path,
        PathBuf::from("drivers/foo/remove.o")
    );
    assert!(index.has_kbuild_object_ref(
        Path::new("drivers/foo/Makefile"),
        1,
        "obj-y",
        "remove.o",
        Path::new("drivers/foo/remove.o")
    ));
    let directory_refs = index.find_kbuild_directory_refs("drivers/foo/remove/");
    assert_eq!(directory_refs.len(), 1);
    assert_eq!(
        directory_refs[0].file,
        PathBuf::from("drivers/foo/Makefile")
    );
    assert_eq!(directory_refs[0].line, 2);
    assert_eq!(directory_refs[0].assignment_lhs, "subdir-y");
    assert_eq!(directory_refs[0].directory, "remove/");
    assert_eq!(
        directory_refs[0].resolved_paths,
        vec![PathBuf::from("drivers/foo/remove"), PathBuf::from("remove"),]
    );
    assert!(index.has_kbuild_directory_ref(
        Path::new("drivers/foo/Makefile"),
        2,
        "subdir-y",
        "remove/",
        Path::new("drivers/foo/remove")
    ));
}

#[test]
fn test_tree_index_full_build_is_deterministic() {
    let entries = [
        (
            "Kconfig",
            "source \"z/Kconfig\"\nconfig ROOT\n\tselect ZED if COMMON\n",
        ),
        ("z/Kconfig", "config ZED\n\tdepends on COMMON\n"),
        (
            "drivers/foo/Makefile",
            "obj-y += foo.o\nsubdir-y += child/\n",
        ),
        (
            "drivers/foo/foo.c",
            "#include <linux/foo.h>\n#if defined(CONFIG_ROOT) && defined(CONFIG_ZED)\n#endif\n",
        ),
        ("drivers/foo/foo.h", "#define FOO 1\n"),
        ("drivers/foo/child/bar.c", "int bar;\n"),
    ];
    let tmp_a = tempfile::tempdir().unwrap();
    let tmp_b = tempfile::tempdir().unwrap();
    write_tree_entries(tmp_a.path(), &entries);
    let reversed = entries.iter().rev().copied().collect::<Vec<_>>();
    write_tree_entries(tmp_b.path(), &reversed);

    let first = TreeIndex::build(tmp_a.path(), &()).unwrap();
    let second = TreeIndex::build(tmp_a.path(), &()).unwrap();
    let reordered = TreeIndex::build(tmp_b.path(), &()).unwrap();

    assert_eq!(first, second);
    assert_eq!(first.coverage_stats(), second.coverage_stats());
    assert_eq!(first, reordered);
}

#[test]
fn test_tree_index_rebuild_apis_refresh_domain_indexes() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(root.join("Kconfig"), "config OLD\n\tdepends on OLD_DEP\n").unwrap();
    std::fs::write(root.join("Makefile"), "obj-y += old.o\n").unwrap();
    std::fs::write(
        root.join("old.c"),
        "#include <old.h>\n#if defined(CONFIG_OLD_GATE)\n#endif\n",
    )
    .unwrap();
    std::fs::write(root.join("old.h"), "#define OLD 1\n").unwrap();

    let mut index = TreeIndex::build(root, &()).unwrap();
    assert!(index.kconfig_defs.contains(&KconfigDefinition {
        file: PathBuf::from("Kconfig"),
        line: 1,
        symbol: String::from("OLD"),
    }));
    assert!(index.has_kbuild_object_ref(
        Path::new("Makefile"),
        1,
        "obj-y",
        "old.o",
        Path::new("old.o")
    ));
    assert!(index.has_include_site(Path::new("old.c"), 1, "old.h"));
    assert!(index.cpp_gates_by_symbol.contains_key("OLD_GATE"));

    std::fs::write(root.join("Kconfig"), "config NEW\n\tdepends on NEW_DEP\n").unwrap();
    index
        .rebuild_kconfig(root, &[PathBuf::from("Kconfig")])
        .unwrap();
    assert!(!index
        .kconfig_defs
        .iter()
        .any(|definition| definition.symbol == "OLD"));
    assert!(index.kconfig_defs.contains(&KconfigDefinition {
        file: PathBuf::from("Kconfig"),
        line: 1,
        symbol: String::from("NEW"),
    }));
    assert!(index.kconfig_refs.contains(&KconfigSymbolReference {
        file: PathBuf::from("Kconfig"),
        line: 2,
        directive: String::from("depends_on"),
        symbol: String::from("NEW_DEP"),
    }));

    std::fs::write(root.join("Makefile"), "obj-y += new.o\n").unwrap();
    std::fs::write(root.join("new.c"), "int new;\n").unwrap();
    index
        .rebuild_kbuild(root, &[PathBuf::from("Makefile")])
        .unwrap();
    assert!(index.find_kbuild_object_refs("old.o").is_empty());
    assert!(index.has_kbuild_object_ref(
        Path::new("Makefile"),
        1,
        "obj-y",
        "new.o",
        Path::new("new.o")
    ));
    assert!(index.kbuild_object_providers.contains(Path::new("new.o")));

    std::fs::write(
        root.join("old.c"),
        "#include <new.h>\n#if defined(CONFIG_NEW_GATE)\n#endif\n",
    )
    .unwrap();
    index
        .rebuild_c_family(root, &[PathBuf::from("old.c")])
        .unwrap();
    assert!(index
        .find_include_site(Path::new("old.c"), "old.h")
        .is_none());
    assert!(index.has_include_site(Path::new("old.c"), 1, "new.h"));
    assert!(!index.cpp_gates_by_symbol.contains_key("OLD_GATE"));
    assert!(index.cpp_gates_by_symbol.contains_key("NEW_GATE"));

    std::fs::remove_file(root.join("new.c")).unwrap();
    index
        .rebuild_c_family(root, &[PathBuf::from("new.c")])
        .unwrap();
    assert!(!index.contains_file(Path::new("new.c")));
}

#[test]
fn test_tree_index_incremental_rebuild_matches_full_build_for_touched_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(root.join("Kconfig"), "config OLD\n\tdepends on OLD_DEP\n").unwrap();
    std::fs::write(root.join("Makefile"), "obj-y += old.o\n").unwrap();
    std::fs::write(
        root.join("old.c"),
        "#include <old.h>\n#if defined(CONFIG_OLD_GATE)\n#endif\n",
    )
    .unwrap();
    std::fs::write(root.join("old.h"), "#define OLD 1\n").unwrap();

    let mut incremental = TreeIndex::build(root, &()).unwrap();

    std::fs::write(root.join("Kconfig"), "config NEW\n\tdepends on NEW_DEP\n").unwrap();
    std::fs::write(root.join("Makefile"), "obj-y += new.o\n").unwrap();
    std::fs::write(root.join("new.c"), "int new;\n").unwrap();
    std::fs::write(
        root.join("old.c"),
        "#include <new.h>\n#if defined(CONFIG_NEW_GATE)\n#endif\n",
    )
    .unwrap();
    incremental
        .rebuild_kbuild(
            root,
            &[
                PathBuf::from("Kconfig"),
                PathBuf::from("Makefile"),
                PathBuf::from("new.c"),
                PathBuf::from("old.c"),
            ],
        )
        .unwrap();
    let full = TreeIndex::build(root, &()).unwrap();

    assert_eq!(incremental, full);
}

#[test]
fn test_tree_index_incremental_rebuild_removes_missing_file_and_directory_entries() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo/subdir")).unwrap();
    std::fs::write(
        root.join("drivers/foo/Kconfig"),
        "config FOO\n\tdepends on BAR\n",
    )
    .unwrap();
    std::fs::write(
        root.join("drivers/foo/Makefile"),
        "obj-y += foo.o\nsubdir-y += subdir/\n",
    )
    .unwrap();
    std::fs::write(
        root.join("drivers/foo/foo.c"),
        "#include <foo.h>\n#if defined(CONFIG_FOO)\n#endif\n",
    )
    .unwrap();
    std::fs::write(root.join("drivers/foo/foo.h"), "#define FOO 1\n").unwrap();
    std::fs::write(root.join("drivers/foo/subdir/bar.c"), "int bar;\n").unwrap();

    let mut index = TreeIndex::build(root, &()).unwrap();
    assert!(index.contains_file(Path::new("drivers/foo/foo.c")));
    assert!(index
        .kbuild_object_providers
        .contains(Path::new("drivers/foo/foo.o")));

    std::fs::remove_file(root.join("drivers/foo/foo.c")).unwrap();
    index
        .rebuild_c_family(root, &[PathBuf::from("drivers/foo/foo.c")])
        .unwrap();
    assert!(!index.contains_file(Path::new("drivers/foo/foo.c")));
    assert!(index
        .find_include_site(Path::new("drivers/foo/foo.c"), "foo.h")
        .is_none());
    assert!(!index.cpp_gates_by_symbol.contains_key("FOO"));
    assert!(!index
        .kbuild_object_providers
        .contains(Path::new("drivers/foo/foo.o")));
    assert_eq!(index, TreeIndex::build(root, &()).unwrap());

    std::fs::remove_dir_all(root.join("drivers/foo")).unwrap();
    index
        .rebuild_kconfig(root, &[PathBuf::from("drivers/foo")])
        .unwrap();

    assert_no_index_owner_under(&index, Path::new("drivers/foo"));
    assert_eq!(index, TreeIndex::build(root, &()).unwrap());
}

#[test]
fn test_tree_index_build_does_not_mutate_tree() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(root.join("Kconfig"), "config FOO\n").unwrap();
    std::fs::write(root.join("drivers/foo/Makefile"), "obj-y += foo.o\n").unwrap();
    std::fs::write(root.join("drivers/foo/foo.c"), "#include <linux/foo.h>\n").unwrap();

    let before = tree_file_snapshot(root);
    TreeIndex::build(root, &()).unwrap();
    let after = tree_file_snapshot(root);

    assert_eq!(after, before);
}

#[test]
fn test_tree_index_skips_absolute_reference_literals() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let absolute_header = root.join("outside.h").display().to_string();
    std::fs::write(
        root.join("Kconfig"),
        format!(
            "source \"{}\"\nconfig FOO\n",
            root.join("Kconfig.abs").display()
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("Makefile"),
        "obj-y += /tmp/outside.o\nsubdir-y += /tmp/outside/\n",
    )
    .unwrap();
    std::fs::write(
        root.join("foo.c"),
        format!("#include \"{absolute_header}\"\nint foo;\n"),
    )
    .unwrap();

    let index = TreeIndex::build(root, &()).unwrap();

    assert!(index.include_sites.is_empty());
    assert!(index.kconfig_sources.is_empty());
    assert!(index.kbuild_object_refs.is_empty());
    assert!(index.kbuild_dir_refs.is_empty());
    assert_tree_index_paths_are_relative(&index);
}

#[test]
fn test_tree_index_parse_include_target_accepts_simple_supported_forms_only() {
    assert_eq!(
        parse_include_target("#include <linux/kernel.h>"),
        Some("linux/kernel.h")
    );
    assert_eq!(
        parse_include_target("#include \"local.h\""),
        Some("local.h")
    );
    assert_eq!(parse_include_target("# include <linux/kernel.h>"), None);
    assert_eq!(parse_include_target("int helper;"), None);
}

fn assert_tree_index_paths_are_relative(index: &TreeIndex) {
    for path in &index.files {
        assert_relative_path(path);
    }
    for path in &index.headers {
        assert_relative_path(path);
    }
    for site in &index.include_sites {
        assert_relative_path(&site.file);
    }
    for path in &index.kconfig_files {
        assert_relative_path(path);
    }
    for definition in &index.kconfig_defs {
        assert_relative_path(&definition.file);
    }
    for reference in &index.kconfig_refs {
        assert_relative_path(&reference.file);
    }
    for source in &index.kconfig_sources {
        assert_relative_path(&source.file);
    }
    for path in &index.kbuild_files {
        assert_relative_path(path);
    }
    for path in &index.kbuild_object_providers {
        assert_relative_path(path);
    }
    for reference in &index.kbuild_object_refs {
        assert_relative_path(&reference.file);
        assert_relative_path(&reference.resolved_path);
    }
    for reference in &index.kbuild_dir_refs {
        assert_relative_path(&reference.file);
        for resolved in &reference.resolved_paths {
            assert_relative_path(resolved);
        }
    }
    for gates in index.cpp_gates_by_symbol.values() {
        for gate in gates {
            assert_relative_path(&gate.file);
        }
    }
    for fact in &index.abi_paths {
        assert_relative_path(&fact.path);
    }
    for reference in &index.abi_source_refs {
        assert_relative_path(&reference.file);
        assert_relative_path(&reference.target);
    }
}

fn assert_no_index_owner_under(index: &TreeIndex, base: &Path) {
    assert!(index
        .files
        .iter()
        .all(|path| !index_path_is_under(path, base)));
    assert!(index
        .headers
        .iter()
        .all(|path| !index_path_is_under(path, base)));
    assert!(index
        .include_sites
        .iter()
        .all(|site| !index_path_is_under(&site.file, base)));
    assert!(index
        .kconfig_files
        .iter()
        .all(|path| !index_path_is_under(path, base)));
    assert!(index
        .kconfig_defs
        .iter()
        .all(|definition| !index_path_is_under(&definition.file, base)));
    assert!(index
        .kconfig_refs
        .iter()
        .all(|reference| !index_path_is_under(&reference.file, base)));
    assert!(index
        .kconfig_sources
        .iter()
        .all(|source| !index_path_is_under(&source.file, base)));
    assert!(index
        .kbuild_files
        .iter()
        .all(|path| !index_path_is_under(path, base)));
    assert!(index
        .kbuild_object_providers
        .iter()
        .all(|path| !index_path_is_under(path, base)));
    assert!(index
        .kbuild_object_refs
        .iter()
        .all(|reference| !index_path_is_under(&reference.file, base)));
    assert!(index
        .kbuild_dir_refs
        .iter()
        .all(|reference| !index_path_is_under(&reference.file, base)));
    assert!(index.cpp_gates_by_symbol.values().all(|gates| {
        gates
            .iter()
            .all(|gate| !index_path_is_under(&gate.file, base))
    }));
    assert!(index
        .abi_paths
        .iter()
        .all(|fact| !index_path_is_under(&fact.path, base)));
    assert!(index
        .abi_source_refs
        .iter()
        .all(|reference| !index_path_is_under(&reference.file, base)));
}

fn assert_relative_path(path: &Path) {
    assert!(
        is_relative_index_path(path),
        "tree index path should be relative: {}",
        path.display()
    );
}

fn write_tree_entries(root: &Path, entries: &[(&str, &str)]) {
    for (relative, content) in entries {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }
}

fn tree_file_snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut snapshot = BTreeMap::new();
    for entry in walkdir::WalkDir::new(root) {
        let entry = entry.unwrap();
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry.path().strip_prefix(root).unwrap().to_path_buf();
        snapshot.insert(relative, std::fs::read(entry.path()).unwrap());
    }
    snapshot
}
