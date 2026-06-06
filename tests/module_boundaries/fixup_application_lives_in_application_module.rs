use super::common::*;

#[test]
fn fixup_application_lives_in_application_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixups = production_source(&root.join("src/fixups.rs"));
    let application = production_source(&root.join("src/fixups/application.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod application;",
        "pub(in crate::fixups) use application::{",
        "remove_missing_header_include",
        "remove_missing_kconfig_source",
        "remove_stale_kbuild_directory_reference",
        "remove_stale_kbuild_object_reference",
        "validate_fixup_result",
    ] {
        assert!(
            fixups.contains(required),
            "src/fixups.rs should expose fixup application helper {required}"
        );
    }

    for required in [
        "pub(in crate::fixups) fn validate_fixup_result(",
        "pub(in crate::fixups) fn remove_missing_header_include(",
        "pub(in crate::fixups) fn remove_stale_kbuild_directory_reference(",
        "pub(in crate::fixups) fn remove_stale_kbuild_object_reference(",
        "pub(in crate::fixups) fn remove_missing_kconfig_source(",
        "fn remove_kbuild_assignment_token(",
        "pub(in crate::fixups) fn write_proven_fixup_rewrite(",
        "sort_edit_records(&mut edits)",
        "validate_fixup_result(pass_name, index, diagnostic, &result)?;",
        "write_verified_rewrite(root, path, content, &result.edits, pass_name)?;",
    ] {
        assert!(
            application.contains(required),
            "src/fixups/application.rs should own proof-gated application detail {required}"
        );
    }

    for forbidden in [
        "\nfn validate_fixup_result(",
        "\nfn remove_missing_header_include(",
        "\nfn remove_kbuild_assignment_token(",
        "\nfn write_proven_fixup_rewrite(",
    ] {
        assert!(
            !fixups.contains(forbidden),
            "src/fixups.rs should not keep fixup application helper body {forbidden}"
        );
    }

    for required in ["`src/fixups/application.rs`", "Fixup application"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document the fixup application split {required}"
        );
    }
}
