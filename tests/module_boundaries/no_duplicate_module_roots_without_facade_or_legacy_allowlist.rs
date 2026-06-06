use super::common::*;

const LEGACY_DUPLICATE_MODULE_ROOTS: &[(&str, &str)] = &[
    (
        "src/config/model.rs",
        "config schema structs still sit beside model validators until the config model split lands",
    ),
    (
        "src/edit_reason.rs",
        "edit record model and legacy tests still share root after render helper extraction",
    ),
    (
        "src/fixups.rs",
        "fixup orchestration manifest proof helpers and legacy tests still share root after report extraction",
    ),
    (
        "src/generate/publish.rs",
        "publication orchestration still shares this root with stage declarations before output publication is split",
    ),
    (
        "src/generate/verify.rs",
        "candidate verification orchestration still shares this root before verification checks are fully split",
    ),
    (
        "src/generate.rs",
        "generate command orchestration still shares the legacy root until lifecycle modules own all behavior",
    ),
    (
        "src/kconfig/ast.rs",
        "kconfig document AST and parser helpers still share this root before parser and AST modules split",
    ),
];

#[test]
fn no_duplicate_module_roots_without_facade_or_legacy_allowlist() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut sources = Vec::new();
    collect_rust_sources(&root.join("src"), &mut sources);

    let legacy = LEGACY_DUPLICATE_MODULE_ROOTS
        .iter()
        .map(|(relative, reason)| (*relative, *reason))
        .collect::<BTreeMap<_, _>>();

    let mut duplicate_pairs = BTreeSet::new();
    let mut violations = Vec::new();

    for source in sources {
        if source.file_name().and_then(|name| name.to_str()) == Some("mod.rs") {
            continue;
        }

        let Some(module_dir) = sibling_module_dir(&source) else {
            continue;
        };
        if !module_dir.is_dir() {
            continue;
        }

        let relative = repo_relative_path(root, &source);
        let pair = duplicate_pair_label(root, &source, &module_dir);
        duplicate_pairs.insert(pair.clone());

        if is_module_only_facade(&production_source(&source))
            || legacy.contains_key(relative.as_str())
        {
            continue;
        }

        violations.push(pair);
    }

    assert!(
        violations.is_empty(),
        "duplicate module-root files and module directories are forbidden unless the root file is a module-only facade equivalent to mod.rs, or the pair is explicitly listed as legacy debt. Violations: {violations:#?}"
    );

    for (relative, reason) in LEGACY_DUPLICATE_MODULE_ROOTS {
        assert!(
            reason.split_whitespace().count() >= 8,
            "{relative} needs a concrete legacy duplicate module-root allowlist reason"
        );

        let source = root.join(relative);
        let module_dir =
            sibling_module_dir(&source).expect("legacy duplicate module root should have a stem");
        let pair = duplicate_pair_label(root, &source, &module_dir);

        assert!(
            duplicate_pairs.contains(&pair),
            "{relative} is listed as legacy duplicate module-root debt but no longer has a matching module directory"
        );
        assert!(
            !is_module_only_facade(&production_source(&source)),
            "{relative} is now a module-only facade; remove it from the legacy duplicate module-root allowlist"
        );
    }

    let workflow = production_source(&root.join(".github/workflows/source-size.yml"));
    for required in [
        "duplicate module-root files",
        "no_duplicate_module_roots_without_facade_or_legacy_allowlist",
    ] {
        assert!(
            workflow.contains(required),
            "CI should run the duplicate module-root guard through {required}"
        );
    }

    let doc = production_source(&root.join("docs/file-size-policy.md"));
    for required in [
        "## Duplicate module-root guard",
        "duplicate module-root file",
        "module-only facade",
        "legacy allowlist",
    ] {
        assert!(
            doc.contains(required),
            "docs/file-size-policy.md should document duplicate module-root guard {required}"
        );
    }
}

fn sibling_module_dir(source: &Path) -> Option<PathBuf> {
    source
        .file_stem()
        .map(|stem| source.with_file_name(stem))
}

fn duplicate_pair_label(root: &Path, source: &Path, module_dir: &Path) -> String {
    format!(
        "{} + {}/",
        repo_relative_path(root, source),
        repo_relative_path(root, module_dir)
    )
}

fn is_module_only_facade(source: &str) -> bool {
    let statements = facade_statements(source);
    !statements.is_empty()
        && statements
            .iter()
            .all(|statement| is_facade_statement(statement))
}

fn facade_statements(source: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut in_block_comment = false;
    let mut in_attribute = false;

    for raw_line in source.lines() {
        let line = uncommented_line(raw_line, &mut in_block_comment);
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if in_attribute {
            if trimmed.contains(']') {
                in_attribute = false;
            }
            continue;
        }
        if trimmed.starts_with("#[") {
            if !trimmed.contains(']') {
                in_attribute = true;
            }
            continue;
        }
        if trimmed.starts_with('#') {
            continue;
        }

        current.push_str(trimmed);
        current.push(' ');

        if trimmed.ends_with(';') {
            statements.push(current.trim().to_string());
            current.clear();
        }
    }

    if !current.trim().is_empty() {
        statements.push(current.trim().to_string());
    }

    statements
}

fn uncommented_line(raw_line: &str, in_block_comment: &mut bool) -> String {
    let mut uncommented = String::new();
    let mut chars = raw_line.chars().peekable();

    while let Some(ch) = chars.next() {
        if *in_block_comment {
            if ch == '*' && chars.peek() == Some(&'/') {
                chars.next();
                *in_block_comment = false;
            }
            continue;
        }

        if ch == '/' {
            match chars.peek() {
                Some('/') => break,
                Some('*') => {
                    chars.next();
                    *in_block_comment = true;
                }
                _ => uncommented.push(ch),
            }
        } else {
            uncommented.push(ch);
        }
    }

    uncommented
}

fn is_facade_statement(statement: &str) -> bool {
    let statement = statement.trim();
    if is_module_or_use_statement(statement) {
        return true;
    }

    let Some(rest) = statement.strip_prefix("pub") else {
        return false;
    };
    let Some(first) = rest.chars().next() else {
        return false;
    };
    if first != '(' && !first.is_whitespace() {
        return false;
    }

    let rest = rest.trim_start();
    let rest = if rest.starts_with('(') {
        let Some(visibility_end) = rest.find(')') else {
            return false;
        };
        rest[visibility_end + 1..].trim_start()
    } else {
        rest
    };

    is_module_or_use_statement(rest)
}

fn is_module_or_use_statement(statement: &str) -> bool {
    statement.starts_with("mod ") || statement.starts_with("use ")
}
