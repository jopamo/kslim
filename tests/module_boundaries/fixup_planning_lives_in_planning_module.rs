use super::common::*;

#[test]
fn fixup_planning_lives_in_planning_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixups = production_source(&root.join("src/fixups.rs"));
    let planning = production_source(&root.join("src/fixups/planning.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod planning;",
        "pub(in crate::fixups) use planning::available_fixups;",
    ] {
        assert!(
            fixups.contains(required),
            "src/fixups.rs should expose fixup planning through {required}"
        );
    }

    for required in [
        "pub(in crate::fixups) trait Fixup",
        "struct MissingHeaderIncludeFixup;",
        "struct StaleKbuildDirectoryFixup;",
        "struct StaleKbuildObjectFixup;",
        "struct MissingKconfigSourceFixup;",
        "impl Fixup for MissingHeaderIncludeFixup",
        "impl Fixup for StaleKbuildDirectoryFixup",
        "impl Fixup for StaleKbuildObjectFixup",
        "impl Fixup for MissingKconfigSourceFixup",
        "pub(in crate::fixups) fn available_fixups()",
        "classified_diagnostic_proof(diagnostic)",
        "FixupResult::new(edits, proof_sources)",
    ] {
        assert!(
            planning.contains(required),
            "src/fixups/planning.rs should own fixup planning detail {required}"
        );
    }

    for forbidden in [
        "\npub trait Fixup",
        "\nstruct MissingHeaderIncludeFixup;",
        "\nstruct StaleKbuildDirectoryFixup;",
        "\nstruct StaleKbuildObjectFixup;",
        "\nstruct MissingKconfigSourceFixup;",
        "\nfn available_fixups()",
    ] {
        assert!(
            !fixups.contains(forbidden),
            "src/fixups.rs should not keep fixup planning helper body {forbidden}"
        );
    }

    for required in ["`src/fixups/planning.rs`", "Fixup planning"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document the fixup planning split {required}"
        );
    }
}
