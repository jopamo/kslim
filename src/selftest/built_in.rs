use std::path::Path;

use crate::kbuild::{
    composite_objects, has_object_provider, is_build_graph_assignment, logical_lines,
    make_dir_candidates, makefiles, parse_make_assignment, protected_make_logical_line_starts,
};
use crate::kconfig::{kconfig_files, parse_kconfig_source, resolve_kconfig_source};

use super::SelfTestFailure;

pub(super) fn validate_kconfig_sources(root: &Path) -> std::result::Result<(), SelfTestFailure> {
    for path in kconfig_files(root) {
        let content = std::fs::read_to_string(&path).map_err(|err| SelfTestFailure::BuiltIn {
            check: "kconfig-sources",
            message: format!("failed to read {}: {}", path.display(), err),
        })?;
        let current_dir = path.parent().unwrap_or(root);

        for (idx, line) in content.lines().enumerate() {
            let Some(source) = parse_kconfig_source(line) else {
                continue;
            };
            if source.optional || source.path.contains('$') {
                continue;
            }
            if resolve_kconfig_source(root, current_dir, &source).is_none() {
                return Err(SelfTestFailure::BuiltIn {
                    check: "kconfig-sources",
                    message: format!(
                        "selftest failed: {}:{} references missing Kconfig source '{}'",
                        path.display(),
                        idx + 1,
                        source.path
                    ),
                });
            }
        }
    }

    Ok(())
}

pub(super) fn validate_makefiles(root: &Path) -> std::result::Result<(), SelfTestFailure> {
    for path in makefiles(root) {
        let content = std::fs::read_to_string(&path).map_err(|err| SelfTestFailure::BuiltIn {
            check: "makefiles",
            message: format!("failed to read {}: {}", path.display(), err),
        })?;
        let lines = logical_lines(&content);
        let protected_lines = protected_make_logical_line_starts(&lines);
        let composite_objects = composite_objects(&lines);
        let current_dir = path.parent().unwrap_or(root);

        for logical in &lines {
            if protected_lines.contains(&logical.start_line) {
                continue;
            }
            let Some((lhs, _, rhs)) = parse_make_assignment(&logical.joined) else {
                continue;
            };
            if !is_build_graph_assignment(lhs) {
                continue;
            }

            for token in rhs.split_whitespace() {
                if !should_validate_make_token(token) {
                    continue;
                }

                if token.ends_with('/') {
                    if !make_dir_exists(root, current_dir, token) {
                        return Err(SelfTestFailure::BuiltIn {
                            check: "makefiles",
                            message: format!(
                                "selftest failed: {}:{} references missing directory '{}'",
                                path.display(),
                                logical.start_line,
                                token
                            ),
                        });
                    }
                    continue;
                }

                if token.ends_with(".o")
                    && !has_object_provider(current_dir, token, &composite_objects)
                {
                    return Err(SelfTestFailure::BuiltIn {
                        check: "makefiles",
                        message: format!(
                            "selftest failed: {}:{} references object '{}' without a source or composite rule",
                            path.display(),
                            logical.start_line,
                            token
                        ),
                    });
                }
            }
        }
    }

    Ok(())
}

fn should_validate_make_token(token: &str) -> bool {
    if token.is_empty() || token == "\\" {
        return false;
    }
    if token.starts_with('-')
        || token.starts_with('/')
        || token.contains('$')
        || token.contains('%')
        || token.contains(':')
    {
        return false;
    }
    token.ends_with('/') || token.ends_with(".o")
}

fn make_dir_exists(root: &Path, current_dir: &Path, token: &str) -> bool {
    make_dir_candidates(root, current_dir, token)
        .into_iter()
        .any(|relative| root.join(relative).exists())
}
