use super::*;
use super::classifier::*;
use crate::edit_reason::DiagnosticClass;
use crate::selftest::{CapturedCommandFailure, SelfTestFailure};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[test]
fn test_parse_missing_header_line_accepts_gcc_missing_header_shape() {
    assert_eq!(
        parse_missing_header_line(
            "drivers/gpu/drm/helper.c:1:10: fatal error: amd/amdgpu_missing.h: No such file or directory"
        ),
        Some(("drivers/gpu/drm/helper.c", 1, "amd/amdgpu_missing.h"))
    );
    assert_eq!(
        parse_missing_header_line(
            "drivers/gpu/drm/helper.c:1:10: fatal error: 'amd/amdgpu_missing.h' file not found"
        ),
        Some(("drivers/gpu/drm/helper.c", 1, "amd/amdgpu_missing.h"))
    );
}

#[test]
fn test_parse_make_missing_target_line_accepts_make_target_shape_but_not_directory_shape() {
    assert_eq!(
        parse_make_missing_target_line(
            "make[3]: *** No rule to make target 'drivers/gpu/drm/amd/amdgpu/amdgpu_drv.o', needed by 'drivers/gpu/drm/built-in.a'.  Stop."
        ),
        Some("drivers/gpu/drm/amd/amdgpu/amdgpu_drv.o")
    );
    assert_eq!(
        parse_make_missing_target_line(
            "make[3]: *** No rule to make target 'drivers/gpu/drm/amd/amdgpu/', needed by 'drivers/gpu/drm/'.  Stop."
        ),
        None
    );
}

#[test]
fn test_parse_make_missing_directory_line_accepts_make_directory_shape_but_not_object_shape() {
    assert_eq!(
        parse_make_missing_directory_line(
            "make[3]: *** No rule to make target 'drivers/gpu/drm/amd/amdgpu/', needed by 'drivers/gpu/drm/'.  Stop."
        ),
        Some("drivers/gpu/drm/amd/amdgpu/")
    );
    assert_eq!(
        parse_make_missing_directory_line(
            "make[3]: *** No rule to make target 'drivers/gpu/drm/amd/amdgpu/amdgpu_drv.o', needed by 'drivers/gpu/drm/built-in.a'.  Stop."
        ),
        None
    );
}

#[test]
fn test_parse_missing_kconfig_source_message_accepts_selftest_shape() {
    assert_eq!(
        parse_missing_kconfig_source_message(
            "selftest failed: drivers/gpu/drm/Kconfig:12 references missing Kconfig source 'drivers/gpu/drm/amd/amdgpu/Kconfig'"
        ),
        Some((
            "drivers/gpu/drm/Kconfig",
            12,
            "drivers/gpu/drm/amd/amdgpu/Kconfig"
        ))
    );
}

#[test]
fn test_classified_diagnostic_file_returns_primary_path_context() {
    assert_eq!(
        ClassifiedDiagnostic::MissingHeader {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 1,
            header: String::from("amd/amdgpu_missing.h"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
        .file(),
        Some(Path::new("drivers/gpu/drm/helper.c"))
    );
    assert_eq!(
        ClassifiedDiagnostic::MissingKconfigSource {
            kconfig_file: PathBuf::from("drivers/gpu/drm/Kconfig"),
            line: 12,
            source: String::from("drivers/gpu/drm/amd/amdgpu/Kconfig"),
        }
        .file(),
        Some(Path::new("drivers/gpu/drm/Kconfig"))
    );
    assert_eq!(
        ClassifiedDiagnostic::MissingMakeTarget {
            target: String::from("drivers/gpu/drm/amd/amdgpu/amdgpu_drv.o"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
        .file(),
        Some(Path::new("drivers/gpu/drm/amd/amdgpu/amdgpu_drv.o"))
    );
    assert_eq!(
        ClassifiedDiagnostic::MissingMakeDirectory {
            path: String::from("drivers/gpu/drm/amd/amdgpu/"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
        .file(),
        Some(Path::new("drivers/gpu/drm/amd/amdgpu/"))
    );
    assert_eq!(
        ClassifiedDiagnostic::UndeclaredIdentifier {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 7,
            symbol: String::from("amdgpu_magic"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
        .file(),
        Some(Path::new("drivers/gpu/drm/helper.c"))
    );
    assert_eq!(
        ClassifiedDiagnostic::ImplicitDeclaration {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 7,
            symbol: String::from("amdgpu_magic"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
        .file(),
        Some(Path::new("drivers/gpu/drm/helper.c"))
    );
    assert_eq!(
        ClassifiedDiagnostic::UndefinedReference {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            symbol: String::from("amdgpu_magic"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
        .file(),
        Some(Path::new("drivers/gpu/drm/helper.c"))
    );
    assert_eq!(ClassifiedDiagnostic::Unknown.file(), None);
}

#[test]
fn test_classified_diagnostic_line_returns_primary_line_context() {
    assert_eq!(
        ClassifiedDiagnostic::MissingHeader {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 1,
            header: String::from("amd/amdgpu_missing.h"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
        .line(),
        Some(1)
    );
    assert_eq!(
        ClassifiedDiagnostic::MissingKconfigSource {
            kconfig_file: PathBuf::from("drivers/gpu/drm/Kconfig"),
            line: 12,
            source: String::from("drivers/gpu/drm/amd/amdgpu/Kconfig"),
        }
        .line(),
        Some(12)
    );
    assert_eq!(
        ClassifiedDiagnostic::UndeclaredIdentifier {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 7,
            symbol: String::from("amdgpu_magic"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
        .line(),
        Some(7)
    );
    assert_eq!(
        ClassifiedDiagnostic::ImplicitDeclaration {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 7,
            symbol: String::from("amdgpu_magic"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
        .line(),
        Some(7)
    );
    assert_eq!(
        ClassifiedDiagnostic::MissingMakeTarget {
            target: String::from("drivers/gpu/drm/amd/amdgpu/amdgpu_drv.o"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
        .line(),
        None
    );
    assert_eq!(
        ClassifiedDiagnostic::MissingMakeDirectory {
            path: String::from("drivers/gpu/drm/amd/amdgpu/"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
        .line(),
        None
    );
    assert_eq!(
        ClassifiedDiagnostic::UndefinedReference {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            symbol: String::from("amdgpu_magic"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
        .line(),
        None
    );
    assert_eq!(ClassifiedDiagnostic::Unknown.line(), None);
}

#[test]
fn test_classified_diagnostic_build_target_returns_primary_build_target_context() {
    assert_eq!(
        ClassifiedDiagnostic::MissingHeader {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 1,
            header: String::from("amd/amdgpu_missing.h"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
        .build_target(),
        Some("modules")
    );
    assert_eq!(
        ClassifiedDiagnostic::MissingMakeTarget {
            target: String::from("drivers/gpu/drm/amd/amdgpu/amdgpu_drv.o"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
        .build_target(),
        Some("modules")
    );
    assert_eq!(
        ClassifiedDiagnostic::MissingMakeDirectory {
            path: String::from("drivers/gpu/drm/amd/amdgpu/"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
        .build_target(),
        Some("modules")
    );
    assert_eq!(
        ClassifiedDiagnostic::UndeclaredIdentifier {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 7,
            symbol: String::from("amdgpu_magic"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
        .build_target(),
        Some("modules")
    );
    assert_eq!(
        ClassifiedDiagnostic::ImplicitDeclaration {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 7,
            symbol: String::from("amdgpu_magic"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
        .build_target(),
        Some("modules")
    );
    assert_eq!(
        ClassifiedDiagnostic::UndefinedReference {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            symbol: String::from("amdgpu_magic"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
        .build_target(),
        Some("modules")
    );
    assert_eq!(
        ClassifiedDiagnostic::MissingKconfigSource {
            kconfig_file: PathBuf::from("drivers/gpu/drm/Kconfig"),
            line: 12,
            source: String::from("drivers/gpu/drm/amd/amdgpu/Kconfig"),
        }
        .build_target(),
        None
    );
    assert_eq!(ClassifiedDiagnostic::Unknown.build_target(), None);
}

#[test]
fn test_classified_diagnostic_arch_and_config_return_primary_arch_and_config_context() {
    assert_eq!(
        ClassifiedDiagnostic::MissingHeader {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 1,
            header: String::from("amd/amdgpu_missing.h"),
            build_target: Some(String::from("modules")),
            arch: Some(String::from("arm64")),
            config: Some(String::from("defconfig")),
        }
        .arch(),
        Some("arm64")
    );
    assert_eq!(
        ClassifiedDiagnostic::MissingHeader {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 1,
            header: String::from("amd/amdgpu_missing.h"),
            build_target: Some(String::from("modules")),
            arch: Some(String::from("arm64")),
            config: Some(String::from("defconfig")),
        }
        .config(),
        Some("defconfig")
    );
    assert_eq!(
        ClassifiedDiagnostic::MissingMakeTarget {
            target: String::from("drivers/gpu/drm/amd/amdgpu/amdgpu_drv.o"),
            build_target: Some(String::from("modules")),
            arch: Some(String::from("arm64")),
            config: Some(String::from("defconfig")),
        }
        .arch(),
        Some("arm64")
    );
    assert_eq!(
        ClassifiedDiagnostic::MissingMakeDirectory {
            path: String::from("drivers/gpu/drm/amd/amdgpu/"),
            build_target: Some(String::from("modules")),
            arch: Some(String::from("arm64")),
            config: Some(String::from("defconfig")),
        }
        .config(),
        Some("defconfig")
    );
    assert_eq!(
        ClassifiedDiagnostic::UndeclaredIdentifier {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 7,
            symbol: String::from("amdgpu_magic"),
            build_target: Some(String::from("modules")),
            arch: Some(String::from("arm64")),
            config: Some(String::from("defconfig")),
        }
        .arch(),
        Some("arm64")
    );
    assert_eq!(
        ClassifiedDiagnostic::ImplicitDeclaration {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 7,
            symbol: String::from("amdgpu_magic"),
            build_target: Some(String::from("modules")),
            arch: Some(String::from("arm64")),
            config: Some(String::from("defconfig")),
        }
        .config(),
        Some("defconfig")
    );
    assert_eq!(
        ClassifiedDiagnostic::UndefinedReference {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            symbol: String::from("amdgpu_magic"),
            build_target: Some(String::from("modules")),
            arch: Some(String::from("arm64")),
            config: Some(String::from("defconfig")),
        }
        .arch(),
        Some("arm64")
    );
    assert_eq!(
        ClassifiedDiagnostic::MissingKconfigSource {
            kconfig_file: PathBuf::from("drivers/gpu/drm/Kconfig"),
            line: 12,
            source: String::from("drivers/gpu/drm/amd/amdgpu/Kconfig"),
        }
        .arch(),
        None
    );
    assert_eq!(ClassifiedDiagnostic::Unknown.config(), None);
}

#[test]
fn test_classified_diagnostic_subject_returns_primary_symbol_header_or_object() {
    assert_eq!(
        ClassifiedDiagnostic::MissingHeader {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 1,
            header: String::from("amd/amdgpu_missing.h"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
        .subject(),
        Some("amd/amdgpu_missing.h")
    );
    assert_eq!(
        ClassifiedDiagnostic::MissingKconfigSource {
            kconfig_file: PathBuf::from("drivers/gpu/drm/Kconfig"),
            line: 12,
            source: String::from("drivers/gpu/drm/amd/amdgpu/Kconfig"),
        }
        .subject(),
        Some("drivers/gpu/drm/amd/amdgpu/Kconfig")
    );
    assert_eq!(
        ClassifiedDiagnostic::MissingMakeTarget {
            target: String::from("drivers/gpu/drm/amd/amdgpu/amdgpu_drv.o"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
        .subject(),
        Some("drivers/gpu/drm/amd/amdgpu/amdgpu_drv.o")
    );
    assert_eq!(
        ClassifiedDiagnostic::MissingMakeDirectory {
            path: String::from("drivers/gpu/drm/amd/amdgpu/"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
        .subject(),
        Some("drivers/gpu/drm/amd/amdgpu/")
    );
    assert_eq!(
        ClassifiedDiagnostic::UndeclaredIdentifier {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 7,
            symbol: String::from("amdgpu_magic"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
        .subject(),
        Some("amdgpu_magic")
    );
    assert_eq!(
        ClassifiedDiagnostic::ImplicitDeclaration {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 7,
            symbol: String::from("amdgpu_magic"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
        .subject(),
        Some("amdgpu_magic")
    );
    assert_eq!(
        ClassifiedDiagnostic::UndefinedReference {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            symbol: String::from("amdgpu_magic"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
        .subject(),
        Some("amdgpu_magic")
    );
    assert_eq!(ClassifiedDiagnostic::Unknown.subject(), None);
}

#[test]
fn test_classify_selftest_failure_returns_unknown_for_unrecognized_command_output() {
    let failure = SelfTestFailure::Command {
        details: CapturedCommandFailure {
            command: "make".to_string(),
            target: Some("modules".to_string()),
            arch: Some("arm64".to_string()),
            config: Some("defconfig".to_string()),
            stdout: String::new(),
            stderr: String::from("totally unrecognized failure output\n"),
            exit_status: Some(2),
            elapsed: Duration::ZERO,
        },
    };

    let classified = classify_selftest_failure(Path::new("/tmp/tree"), &failure);
    assert_eq!(classified, ClassifiedDiagnostic::Unknown);
    assert_eq!(classified.class(), DiagnosticClass::Unknown);
    assert_eq!(classified.file(), None);
    assert_eq!(classified.line(), None);
    assert_eq!(classified.subject(), None);
    assert_eq!(classified.build_target(), None);
    assert_eq!(classified.arch(), None);
    assert_eq!(classified.config(), None);
}

#[test]
fn test_classify_selftest_failure_returns_unknown_for_unrecognized_builtin_failure() {
    let failure = SelfTestFailure::BuiltIn {
        check: "makefiles",
        message: String::from("selftest failed: something unexpected"),
    };

    let classified = classify_selftest_failure(Path::new("/tmp/tree"), &failure);
    assert_eq!(classified, ClassifiedDiagnostic::Unknown);
    assert_eq!(classified.class(), DiagnosticClass::Unknown);
}

#[test]
fn test_parse_gcc_undeclared_identifier_line_accepts_gcc_shape() {
    assert_eq!(
        parse_gcc_undeclared_identifier_line(
            "drivers/gpu/drm/helper.c:7:2: error: ‘amdgpu_magic’ undeclared (first use in this function)"
        ),
        Some(("drivers/gpu/drm/helper.c", 7, "amdgpu_magic"))
    );
    assert_eq!(
        parse_gcc_undeclared_identifier_line(
            "drivers/gpu/drm/helper.c:7:2: error: 'amdgpu_magic' undeclared (first use in this function)"
        ),
        Some(("drivers/gpu/drm/helper.c", 7, "amdgpu_magic"))
    );
}

#[test]
fn test_parse_clang_undeclared_identifier_line_accepts_clang_shapes() {
    assert_eq!(
        parse_clang_undeclared_identifier_line(
            "drivers/gpu/drm/helper.c:7:2: error: use of undeclared identifier 'amdgpu_magic'"
        ),
        Some(("drivers/gpu/drm/helper.c", 7, "amdgpu_magic"))
    );
    assert_eq!(
        parse_clang_undeclared_identifier_line(
            "drivers/gpu/drm/helper.c:7:2: error: use of undeclared identifier 'amdgpu_magic'; did you mean 'amdgpu_magic2'?"
        ),
        Some(("drivers/gpu/drm/helper.c", 7, "amdgpu_magic"))
    );
}

#[test]
fn test_parse_gcc_implicit_declaration_line_accepts_error_and_warning_shapes() {
    assert_eq!(
        parse_gcc_implicit_declaration_line(
            "drivers/gpu/drm/helper.c:7:2: error: implicit declaration of function ‘amdgpu_magic’ [-Werror=implicit-function-declaration]"
        ),
        Some(("drivers/gpu/drm/helper.c", 7, "amdgpu_magic"))
    );
    assert_eq!(
        parse_gcc_implicit_declaration_line(
            "drivers/gpu/drm/helper.c:7:2: warning: implicit declaration of function 'amdgpu_magic' [-Wimplicit-function-declaration]"
        ),
        Some(("drivers/gpu/drm/helper.c", 7, "amdgpu_magic"))
    );
}

#[test]
fn test_parse_clang_implicit_declaration_line_accepts_error_and_warning_shapes() {
    assert_eq!(
        parse_clang_implicit_declaration_line(
            "drivers/gpu/drm/helper.c:7:2: error: call to undeclared function 'amdgpu_magic'; ISO C99 and later do not support implicit function declarations [-Wimplicit-function-declaration]"
        ),
        Some(("drivers/gpu/drm/helper.c", 7, "amdgpu_magic"))
    );
    assert_eq!(
        parse_clang_implicit_declaration_line(
            "drivers/gpu/drm/helper.c:7:2: warning: call to undeclared function 'amdgpu_magic'; did you mean 'amdgpu_magic2'? [-Wimplicit-function-declaration]"
        ),
        Some(("drivers/gpu/drm/helper.c", 7, "amdgpu_magic"))
    );
}

#[test]
fn test_parse_gcc_undefined_reference_line_accepts_direct_and_ld_prefixed_shapes() {
    assert_eq!(
        parse_gcc_undefined_reference_line(
            "drivers/gpu/drm/helper.c:(.text+0x10): undefined reference to `amdgpu_magic'"
        ),
        Some(("drivers/gpu/drm/helper.c", "amdgpu_magic"))
    );
    assert_eq!(
        parse_gcc_undefined_reference_line(
            "/usr/bin/ld: /tmp/tree/drivers/gpu/drm/helper.c:(.text+0x10): undefined reference to `amdgpu_magic'"
        ),
        Some(("/tmp/tree/drivers/gpu/drm/helper.c", "amdgpu_magic"))
    );
}

#[test]
fn test_classify_selftest_failure_recognizes_gcc_missing_header() {
    let failure = SelfTestFailure::KernelBuild {
        label: "build".to_string(),
        output_dir: PathBuf::from("out"),
        details: CapturedCommandFailure {
            command: "make".to_string(),
            target: Some("modules".to_string()),
            arch: Some("arm64".to_string()),
            config: Some("defconfig".to_string()),
            stdout: String::new(),
            stderr: String::from(
                "drivers/gpu/drm/helper.c:1:10: fatal error: amd/amdgpu_missing.h: No such file or directory\n",
            ),
            exit_status: Some(2),
            elapsed: Duration::ZERO,
        },
    };

    let classified = classify_selftest_failure(Path::new("/tmp/tree"), &failure);
    assert_eq!(
        classified,
        ClassifiedDiagnostic::MissingHeader {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 1,
            header: String::from("amd/amdgpu_missing.h"),
            build_target: Some(String::from("modules")),
            arch: Some(String::from("arm64")),
            config: Some(String::from("defconfig")),
        }
    );
}

#[test]
fn test_classify_selftest_failure_recognizes_clang_missing_header() {
    let failure = SelfTestFailure::KernelBuild {
        label: "build".to_string(),
        output_dir: PathBuf::from("out"),
        details: CapturedCommandFailure {
            command: "clang".to_string(),
            target: Some("modules".to_string()),
            arch: None,
            config: Some("defconfig".to_string()),
            stdout: String::new(),
            stderr: String::from(
                "drivers/gpu/drm/helper.c:1:10: fatal error: 'amd/amdgpu_missing.h' file not found\n",
            ),
            exit_status: Some(1),
            elapsed: Duration::ZERO,
        },
    };

    let classified = classify_selftest_failure(Path::new("/tmp/tree"), &failure);
    assert_eq!(
        classified,
        ClassifiedDiagnostic::MissingHeader {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 1,
            header: String::from("amd/amdgpu_missing.h"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
    );
}

#[test]
fn test_classify_selftest_failure_recognizes_make_missing_target() {
    let failure = SelfTestFailure::KernelBuild {
        label: "build".to_string(),
        output_dir: PathBuf::from("out"),
        details: CapturedCommandFailure {
            command: "make".to_string(),
            target: Some("modules".to_string()),
            arch: None,
            config: Some("defconfig".to_string()),
            stdout: String::new(),
            stderr: String::from(
                "make[3]: *** No rule to make target 'drivers/gpu/drm/amd/amdgpu/amdgpu_drv.o', needed by 'drivers/gpu/drm/built-in.a'.  Stop.\n",
            ),
            exit_status: Some(2),
            elapsed: Duration::ZERO,
        },
    };

    let classified = classify_selftest_failure(Path::new("/tmp/tree"), &failure);
    assert_eq!(
        classified,
        ClassifiedDiagnostic::MissingMakeTarget {
            target: String::from("drivers/gpu/drm/amd/amdgpu/amdgpu_drv.o"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
    );
}

#[test]
fn test_classify_selftest_failure_recognizes_make_missing_directory() {
    let failure = SelfTestFailure::KernelBuild {
        label: "build".to_string(),
        output_dir: PathBuf::from("out"),
        details: CapturedCommandFailure {
            command: "make".to_string(),
            target: Some("modules".to_string()),
            arch: None,
            config: Some("defconfig".to_string()),
            stdout: String::new(),
            stderr: String::from(
                "make[3]: *** No rule to make target 'drivers/gpu/drm/amd/amdgpu/', needed by 'drivers/gpu/drm/'.  Stop.\n",
            ),
            exit_status: Some(2),
            elapsed: Duration::ZERO,
        },
    };

    let classified = classify_selftest_failure(Path::new("/tmp/tree"), &failure);
    assert_eq!(
        classified,
        ClassifiedDiagnostic::MissingMakeDirectory {
            path: String::from("drivers/gpu/drm/amd/amdgpu/"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
    );
}

#[test]
fn test_classify_selftest_failure_recognizes_missing_kconfig_source_builtin() {
    let failure = SelfTestFailure::BuiltIn {
        check: "kconfig-sources",
        message: String::from(
            "selftest failed: drivers/gpu/drm/Kconfig:12 references missing Kconfig source 'drivers/gpu/drm/amd/amdgpu/Kconfig'",
        ),
    };

    let classified = classify_selftest_failure(Path::new("/tmp/tree"), &failure);
    assert_eq!(
        classified,
        ClassifiedDiagnostic::MissingKconfigSource {
            kconfig_file: PathBuf::from("drivers/gpu/drm/Kconfig"),
            line: 12,
            source: String::from("drivers/gpu/drm/amd/amdgpu/Kconfig"),
        }
    );
}

#[test]
fn test_classify_selftest_failure_recognizes_gcc_undeclared_identifier() {
    let failure = SelfTestFailure::KernelBuild {
        label: "build".to_string(),
        output_dir: PathBuf::from("out"),
        details: CapturedCommandFailure {
            command: "make".to_string(),
            target: Some("modules".to_string()),
            arch: None,
            config: Some("defconfig".to_string()),
            stdout: String::new(),
            stderr: String::from(
                "drivers/gpu/drm/helper.c:7:2: error: ‘amdgpu_magic’ undeclared (first use in this function)\n",
            ),
            exit_status: Some(2),
            elapsed: Duration::ZERO,
        },
    };

    let classified = classify_selftest_failure(Path::new("/tmp/tree"), &failure);
    assert_eq!(
        classified,
        ClassifiedDiagnostic::UndeclaredIdentifier {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 7,
            symbol: String::from("amdgpu_magic"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
    );
}

#[test]
fn test_classify_selftest_failure_recognizes_clang_undeclared_identifier() {
    let failure = SelfTestFailure::KernelBuild {
        label: "build".to_string(),
        output_dir: PathBuf::from("out"),
        details: CapturedCommandFailure {
            command: "clang".to_string(),
            target: Some("modules".to_string()),
            arch: None,
            config: Some("defconfig".to_string()),
            stdout: String::new(),
            stderr: String::from(
                "drivers/gpu/drm/helper.c:7:2: error: use of undeclared identifier 'amdgpu_magic'\n",
            ),
            exit_status: Some(1),
            elapsed: Duration::ZERO,
        },
    };

    let classified = classify_selftest_failure(Path::new("/tmp/tree"), &failure);
    assert_eq!(
        classified,
        ClassifiedDiagnostic::UndeclaredIdentifier {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 7,
            symbol: String::from("amdgpu_magic"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
    );
}

#[test]
fn test_classify_selftest_failure_recognizes_gcc_implicit_declaration() {
    let failure = SelfTestFailure::KernelBuild {
        label: "build".to_string(),
        output_dir: PathBuf::from("out"),
        details: CapturedCommandFailure {
            command: "make".to_string(),
            target: Some("modules".to_string()),
            arch: None,
            config: Some("defconfig".to_string()),
            stdout: String::new(),
            stderr: String::from(
                "drivers/gpu/drm/helper.c:7:2: error: implicit declaration of function ‘amdgpu_magic’ [-Werror=implicit-function-declaration]\n",
            ),
            exit_status: Some(2),
            elapsed: Duration::ZERO,
        },
    };

    let classified = classify_selftest_failure(Path::new("/tmp/tree"), &failure);
    assert_eq!(
        classified,
        ClassifiedDiagnostic::ImplicitDeclaration {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 7,
            symbol: String::from("amdgpu_magic"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
    );
}

#[test]
fn test_classify_selftest_failure_recognizes_clang_implicit_declaration() {
    let failure = SelfTestFailure::KernelBuild {
        label: "build".to_string(),
        output_dir: PathBuf::from("out"),
        details: CapturedCommandFailure {
            command: "clang".to_string(),
            target: Some("modules".to_string()),
            arch: None,
            config: Some("defconfig".to_string()),
            stdout: String::new(),
            stderr: String::from(
                "drivers/gpu/drm/helper.c:7:2: error: call to undeclared function 'amdgpu_magic'; ISO C99 and later do not support implicit function declarations [-Wimplicit-function-declaration]\n",
            ),
            exit_status: Some(1),
            elapsed: Duration::ZERO,
        },
    };

    let classified = classify_selftest_failure(Path::new("/tmp/tree"), &failure);
    assert_eq!(
        classified,
        ClassifiedDiagnostic::ImplicitDeclaration {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 7,
            symbol: String::from("amdgpu_magic"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
    );
}

#[test]
fn test_classify_selftest_failure_recognizes_gcc_undefined_reference() {
    let failure = SelfTestFailure::KernelBuild {
        label: "build".to_string(),
        output_dir: PathBuf::from("out"),
        details: CapturedCommandFailure {
            command: "make".to_string(),
            target: Some("modules".to_string()),
            arch: None,
            config: Some("defconfig".to_string()),
            stdout: String::new(),
            stderr: String::from(
                "drivers/gpu/drm/helper.c:(.text+0x10): undefined reference to `amdgpu_magic'\n",
            ),
            exit_status: Some(2),
            elapsed: Duration::ZERO,
        },
    };

    let classified = classify_selftest_failure(Path::new("/tmp/tree"), &failure);
    assert_eq!(
        classified,
        ClassifiedDiagnostic::UndefinedReference {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            symbol: String::from("amdgpu_magic"),
            build_target: Some(String::from("modules")),
            arch: None,
            config: Some(String::from("defconfig")),
        }
    );
}

#[test]
fn test_classify_selftest_failure_normalizes_absolute_gcc_missing_header_path() {
    let root = Path::new("/tmp/tree");
    let failure = SelfTestFailure::Command {
        details: CapturedCommandFailure {
            command: "cc".to_string(),
            target: None,
            arch: None,
            config: None,
            stdout: String::new(),
            stderr: String::from(
                "/tmp/tree/drivers/gpu/drm/helper.c:1:10: fatal error: linux/missing.h: No such file or directory\n",
            ),
            exit_status: Some(1),
            elapsed: Duration::ZERO,
        },
    };

    let classified = classify_selftest_failure(root, &failure);
    assert_eq!(
        classified,
        ClassifiedDiagnostic::MissingHeader {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 1,
            header: String::from("linux/missing.h"),
            build_target: None,
            arch: None,
            config: None,
        }
    );
}

#[test]
fn test_classify_selftest_failure_normalizes_absolute_missing_kconfig_source_path() {
    let root = Path::new("/tmp/tree");
    let failure = SelfTestFailure::BuiltIn {
        check: "kconfig-sources",
        message: String::from(
            "selftest failed: /tmp/tree/drivers/gpu/drm/Kconfig:12 references missing Kconfig source 'drivers/gpu/drm/amd/amdgpu/Kconfig'",
        ),
    };

    let classified = classify_selftest_failure(root, &failure);
    assert_eq!(
        classified,
        ClassifiedDiagnostic::MissingKconfigSource {
            kconfig_file: PathBuf::from("drivers/gpu/drm/Kconfig"),
            line: 12,
            source: String::from("drivers/gpu/drm/amd/amdgpu/Kconfig"),
        }
    );
}

#[test]
fn test_classify_selftest_failure_normalizes_absolute_clang_missing_header_path() {
    let root = Path::new("/tmp/tree");
    let failure = SelfTestFailure::Command {
        details: CapturedCommandFailure {
            command: "clang".to_string(),
            target: None,
            arch: None,
            config: None,
            stdout: String::new(),
            stderr: String::from(
                "/tmp/tree/drivers/gpu/drm/helper.c:1:10: fatal error: 'linux/missing.h' file not found\n",
            ),
            exit_status: Some(1),
            elapsed: Duration::ZERO,
        },
    };

    let classified = classify_selftest_failure(root, &failure);
    assert_eq!(
        classified,
        ClassifiedDiagnostic::MissingHeader {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 1,
            header: String::from("linux/missing.h"),
            build_target: None,
            arch: None,
            config: None,
        }
    );
}

#[test]
fn test_classify_selftest_failure_normalizes_absolute_gcc_undeclared_identifier_path() {
    let root = Path::new("/tmp/tree");
    let failure = SelfTestFailure::Command {
        details: CapturedCommandFailure {
            command: "cc".to_string(),
            target: None,
            arch: None,
            config: None,
            stdout: String::new(),
            stderr: String::from(
                "/tmp/tree/drivers/gpu/drm/helper.c:7:2: error: ‘amdgpu_magic’ undeclared (first use in this function)\n",
            ),
            exit_status: Some(1),
            elapsed: Duration::ZERO,
        },
    };

    let classified = classify_selftest_failure(root, &failure);
    assert_eq!(
        classified,
        ClassifiedDiagnostic::UndeclaredIdentifier {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 7,
            symbol: String::from("amdgpu_magic"),
            build_target: None,
            arch: None,
            config: None,
        }
    );
}

#[test]
fn test_classify_selftest_failure_normalizes_absolute_clang_undeclared_identifier_path() {
    let root = Path::new("/tmp/tree");
    let failure = SelfTestFailure::Command {
        details: CapturedCommandFailure {
            command: "clang".to_string(),
            target: None,
            arch: None,
            config: None,
            stdout: String::new(),
            stderr: String::from(
                "/tmp/tree/drivers/gpu/drm/helper.c:7:2: error: use of undeclared identifier 'amdgpu_magic'\n",
            ),
            exit_status: Some(1),
            elapsed: Duration::ZERO,
        },
    };

    let classified = classify_selftest_failure(root, &failure);
    assert_eq!(
        classified,
        ClassifiedDiagnostic::UndeclaredIdentifier {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 7,
            symbol: String::from("amdgpu_magic"),
            build_target: None,
            arch: None,
            config: None,
        }
    );
}

#[test]
fn test_classify_selftest_failure_normalizes_absolute_gcc_implicit_declaration_path() {
    let root = Path::new("/tmp/tree");
    let failure = SelfTestFailure::Command {
        details: CapturedCommandFailure {
            command: "cc".to_string(),
            target: None,
            arch: None,
            config: None,
            stdout: String::new(),
            stderr: String::from(
                "/tmp/tree/drivers/gpu/drm/helper.c:7:2: error: implicit declaration of function 'amdgpu_magic' [-Werror=implicit-function-declaration]\n",
            ),
            exit_status: Some(1),
            elapsed: Duration::ZERO,
        },
    };

    let classified = classify_selftest_failure(root, &failure);
    assert_eq!(
        classified,
        ClassifiedDiagnostic::ImplicitDeclaration {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 7,
            symbol: String::from("amdgpu_magic"),
            build_target: None,
            arch: None,
            config: None,
        }
    );
}

#[test]
fn test_classify_selftest_failure_normalizes_absolute_clang_implicit_declaration_path() {
    let root = Path::new("/tmp/tree");
    let failure = SelfTestFailure::Command {
        details: CapturedCommandFailure {
            command: "clang".to_string(),
            target: None,
            arch: None,
            config: None,
            stdout: String::new(),
            stderr: String::from(
                "/tmp/tree/drivers/gpu/drm/helper.c:7:2: error: call to undeclared function 'amdgpu_magic'; ISO C99 and later do not support implicit function declarations [-Wimplicit-function-declaration]\n",
            ),
            exit_status: Some(1),
            elapsed: Duration::ZERO,
        },
    };

    let classified = classify_selftest_failure(root, &failure);
    assert_eq!(
        classified,
        ClassifiedDiagnostic::ImplicitDeclaration {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 7,
            symbol: String::from("amdgpu_magic"),
            build_target: None,
            arch: None,
            config: None,
        }
    );
}

#[test]
fn test_classify_selftest_failure_normalizes_absolute_gcc_undefined_reference_path() {
    let root = Path::new("/tmp/tree");
    let failure = SelfTestFailure::Command {
        details: CapturedCommandFailure {
            command: "ld".to_string(),
            target: None,
            arch: None,
            config: None,
            stdout: String::new(),
            stderr: String::from(
                "/usr/bin/ld: /tmp/tree/drivers/gpu/drm/helper.c:(.text+0x10): undefined reference to `amdgpu_magic'\n",
            ),
            exit_status: Some(1),
            elapsed: Duration::ZERO,
        },
    };

    let classified = classify_selftest_failure(root, &failure);
    assert_eq!(
        classified,
        ClassifiedDiagnostic::UndefinedReference {
            source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
            symbol: String::from("amdgpu_magic"),
            build_target: None,
            arch: None,
            config: None,
        }
    );
}
