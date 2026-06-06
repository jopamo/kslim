use anyhow::{Context, Result};
use std::path::{Component, Path, PathBuf};

use crate::config::KslimConfig;

struct Fixture {
    path: &'static str,
    contents: &'static str,
}

const FIXTURES: &[Fixture] = &[
    Fixture {
        path: "README.md",
        contents: r#"# kslim fuzz fixtures

Deterministic seed corpus for parser/scanner fuzzing and reducer regression fixtures.

These files are intentionally small, weird, and stable. They are not generated
from the host environment.
"#,
    },
    Fixture {
        path: "config/profile.toml",
        contents: r#"[profile]
name = "fuzz"
description = "Malformed and adversarial reducer seed profile"

[base]
ref = "fuzz-base"

[slim]
remove_paths = ["drivers/fuzz"]
remove_configs = ["FUZZ_DRIVER"]
"#,
    },
    Fixture {
        path: "cpp/fake-includes.c",
        contents: r##"#include "live.h"
const char *fake = "#include <removed/uapi.h>";
/* #include "commented.h" */
#if defined(CONFIG_REMOVED)
#include "dead.h"
#else
#include "live-again.h"
#endif
"##,
    },
    Fixture {
        path: "cpp/nested-branches.c",
        contents: r#"#if IS_ENABLED(CONFIG_FUZZ)
# if defined(CONFIG_REMOVED) && (CONFIG_LEVEL > 2)
int removed(void);
# elif defined(CONFIG_LIVE)
int live(void);
# endif
#endif
"#,
    },
    Fixture {
        path: "kbuild/Makefile.multiline",
        contents: r#"obj-$(CONFIG_FUZZ) += \
	fuzz-core.o \
	# comment in continuation
	fuzz-extra.o
fuzz-core-y := main.o helper.o
"#,
    },
    Fixture {
        path: "kbuild/Makefile.shell-fragment",
        contents: r#"define gen-rule
obj-y += should-not-parse.o
endef
quiet_cmd_fuzz = FUZZ $@
cmd_fuzz = printf 'obj-y += fake.o\n' > $@
"#,
    },
    Fixture {
        path: "kconfig/malformed.Kconfig",
        contents: r#"menu "Fuzz"
config FUZZ_DRIVER
	tristate "Fuzz driver"
	depends on BROKEN && (
"#,
    },
    Fixture {
        path: "kconfig/nested-if.Kconfig",
        contents: r#"if NET
if BT || RFKILL
config FUZZ_NESTED
	tristate "Nested fuzz"
	depends on (A && B) || (C && !D)
endif
endif
"#,
    },
    Fixture {
        path: "kconfig/unsupported-expression.Kconfig",
        contents: r#"config FUZZ_EXPR
	bool "Unsupported expression seed"
	depends on FOO = "bar" || BAZ >= 7
	select LIVE_SYMBOL if REMOVED && m
"#,
    },
    Fixture {
        path: "manifests/slim.toml",
        contents: r#"remove_paths = [
  "drivers/fuzz",
  "include/uapi/linux/fuzz.h",
]
remove_configs = ["FUZZ_DRIVER"]

[set_defaults]
FUZZ_DRIVER = "n"
"#,
    },
    Fixture {
        path: "metadata/reducer-report.json",
        contents: r#"{
  "schema_version": 1,
  "normalized_removal_manifest": {
    "removed_config_symbols": ["FUZZ_DRIVER"],
    "preserved_config_symbols": [],
    "default_overrides": {}
  }
}
"#,
    },
    Fixture {
        path: "modules/export-live-consumer.c",
        contents: r#"int removed_provider(void) { return 0; }
EXPORT_SYMBOL_GPL(removed_provider);

int live_consumer(void) { return removed_provider(); }
"#,
    },
];

pub(crate) struct FuzzFixtureResult {
    pub(crate) output_dir: PathBuf,
    pub(crate) files: Vec<String>,
}

pub(crate) fn write_fixtures(
    project_root: &Path,
    config: &KslimConfig,
    requested_output: &str,
) -> Result<FuzzFixtureResult> {
    let output_dir = resolve_fixture_output_dir(project_root, requested_output)?;
    reject_output_repo_target(project_root, &output_dir, Path::new(&config.output.path))?;

    for fixture in FIXTURES {
        let path = output_dir.join(fixture.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        std::fs::write(&path, fixture.contents)
            .with_context(|| format!("failed to write {}", path.display()))?;
    }

    let files = FIXTURES
        .iter()
        .map(|fixture| fixture.path.to_string())
        .collect();
    Ok(FuzzFixtureResult { output_dir, files })
}

fn resolve_fixture_output_dir(project_root: &Path, requested_output: &str) -> Result<PathBuf> {
    let requested_output = requested_output.trim();
    if requested_output.is_empty() {
        anyhow::bail!("fuzz fixture output path must not be empty");
    }
    let requested = PathBuf::from(requested_output);
    let output = if requested.is_absolute() {
        requested
    } else {
        project_root.join(requested)
    };
    normalize_without_parent_components("fuzz fixture output", &output)
}

fn reject_output_repo_target(
    project_root: &Path,
    output_dir: &Path,
    output_repo: &Path,
) -> Result<()> {
    let output_repo = if output_repo.is_absolute() {
        output_repo.to_path_buf()
    } else {
        project_root.join(output_repo)
    };
    let output_repo = normalize_without_parent_components("configured output repo", &output_repo)?;
    if output_dir == output_repo || output_dir.starts_with(&output_repo) {
        anyhow::bail!(
            "refusing to write fuzz fixtures inside configured output repo {}",
            output_repo.display()
        );
    }
    Ok(())
}

fn normalize_without_parent_components(label: &str, path: &Path) -> Result<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => {
                anyhow::bail!("{label} must not contain '..': {}", path.display());
            }
        }
    }
    if normalized.as_os_str().is_empty() {
        anyhow::bail!("{label} must not be empty");
    }
    Ok(normalized)
}
