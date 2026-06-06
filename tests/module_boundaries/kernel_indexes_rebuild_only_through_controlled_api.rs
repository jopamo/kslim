use super::common::*;

#[test]
fn kernel_indexes_rebuild_only_through_controlled_api() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let pipeline = production_source(&root.join("src/reducer/pipeline.rs"));

    for required in [
        "TreeIndexRebuildDomain",
        "TreeIndexMutatingPass",
        "rebuild_after_mutating_pass(",
        "TreeIndexMutatingPass::DeclaredPrune",
        "TreeIndexMutatingPass::KconfigRewrite",
        "TreeIndexMutatingPass::KbuildRewrite",
        "TreeIndexMutatingPass::CppFold",
        "TreeIndexMutatingPass::IncludeRewrite",
    ] {
        assert!(
            pipeline.contains(required),
            "reducer/pipeline.rs should rebuild indexes only through controlled mutating-pass API item {required}"
        );
    }

    let mut sources = Vec::new();
    collect_rust_sources(&root.join("src"), &mut sources);
    for source in sources {
        let relative = repo_relative_path(root, &source);
        if relative == "src/index/mod.rs"
            || relative == "src/index/tests.rs"
            || relative == "src/reducer/pipeline.rs"
        {
            continue;
        }
        let production = production_source(&source);
        for forbidden in [
            ".rebuild_all(",
            ".rebuild_kconfig(",
            ".rebuild_kbuild(",
            ".rebuild_c_family(",
            "rebuild_after_mutating_pass(",
            "TreeIndexMutatingPass::",
            "TreeIndexRebuildDomain::",
        ] {
            assert!(
                !production.contains(forbidden),
                "{relative} should not rebuild kernel-domain indexes directly; mutating passes must go through reducer/pipeline controlled API, found {forbidden}"
            );
        }
    }
}
