use super::common::*;

#[test]
fn exported_symbol_provider_removal_requires_no_live_consumer_proof() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let main = production_source(&root.join("src/main.rs"));
    let exported_symbols = production_source(&root.join("src/exported_symbols.rs"));
    let manifest = format!(
        "{}\n{}",
        production_source(&root.join("src/removal_manifest/model.rs")),
        production_source(&root.join("src/removal_manifest/match_rules.rs"))
    );
    let reducer_report = production_sources(
        &root,
        &[
            "src/reducer/report/json.rs",
            "src/reducer/report/json/schema.rs",
            "src/reducer/report/json/serializer.rs",
            "src/reducer/report/json/escaping.rs",
            "src/reducer/report/json/canonical.rs",
        ],
    );

    assert!(
        main.contains("mod exported_symbols;"),
        "main.rs should register the exported-symbol proof module"
    );

    for required in [
        "ExportedSymbolRemovalProof",
        "prove_removed_exports_have_no_live_consumers",
        "EXPORT_SYMBOL_GPL",
        "exported symbol provider removal requires proof",
        "live consumer",
        "mask_c_comments_and_literals",
    ] {
        assert!(
            exported_symbols.contains(required),
            "exported_symbols.rs should prove removed exports have no live consumers through {required}"
        );
    }

    for required in [
        "removed_exported_symbols",
        "derive_removed_exported_symbol_proofs",
        "prove_removed_exports_have_no_live_consumers",
    ] {
        assert!(
            manifest.contains(required),
            "removal manifest modules should carry exported-symbol removal proof through {required}"
        );
    }

    for required in [
        "removed_exported_symbol_count",
        "removed_exported_symbols",
        "render_removed_exported_symbols_json",
    ] {
        assert!(
            reducer_report.contains(required),
            "reducer reports should expose exported-symbol proof through {required}"
        );
    }
}
