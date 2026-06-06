use super::common::*;

#[test]
fn architecture_doc_describes_module_ownership_and_dependencies() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let doc = std::fs::read_to_string(root.join("docs/architecture.md"))
        .expect("failed to read docs/architecture.md");

    for required in [
        "# kslim architecture",
        "## Module ownership",
        "## Allowed dependency direction",
        "`main.rs`",
        "`core/*`",
        "`cli/*`",
        "`commands/*`",
        "`execution/*`",
        "`state/*`",
        "`plan/*`",
        "`feature/*`",
        "`config/*`",
        "`path/*`",
        "`paths/*`",
        "`generate/*`",
        "`reducer/*`",
        "`output_repo/*`",
        "`removal_manifest/*`",
        "`index/*`",
        "`abi/*`",
        "`hardware/*`",
        "`runtime/*`",
        "`generated/*`",
        "`runtime_registrations.rs` is only the compatibility facade",
        "`device_bindings.rs` is only the compatibility facade",
        "`abi_policy.rs` is only the compatibility facade",
        "`tree_index.rs` is only the compatibility facade",
        "`source_scan/*`",
        "`cpp.rs` and `includes.rs` are compatibility facades",
        "`kconfig/*`, `kbuild/*`",
        "`generate/candidate/*` builds and mutates the private candidate tree",
        "re-exporting `crate::path::*`",
        "must not open the output repo or update `kslim.lock`",
        "`publish.rs` consumes committed output metadata only",
        "`index/*` is read-only and policy-free",
        "`tests/module_boundaries/*` contains executable checks",
    ] {
        assert!(
            doc.contains(required),
            "docs/architecture.md should document ownership/dependency boundary {required}"
        );
    }
}
