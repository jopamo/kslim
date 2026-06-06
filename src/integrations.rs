use anyhow::{Context, Result};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::config::{IntegrationsConfig, RtlmqIntegrationConfig};

pub fn apply(root: &Path, tree: &Path, integrations: &IntegrationsConfig) -> Result<()> {
    if let Some(rtlmq) = &integrations.rtlmq {
        apply_rtlmq(root, tree, rtlmq)?;
    }
    Ok(())
}

fn apply_rtlmq(root: &Path, tree: &Path, rtlmq: &RtlmqIntegrationConfig) -> Result<()> {
    let source = resolve_from_root(root, &rtlmq.source);
    let tests_source = match rtlmq.tests_source.as_deref() {
        Some(path) => {
            let resolved = resolve_from_root(root, path);
            require_dir(&resolved, "rtlmq tests source")?;
            Some(resolved)
        }
        None => None,
    };

    let realtek_dir = tree.join("drivers/net/ethernet/realtek");
    let target_dir = realtek_dir.join("rtlmq");
    let parent_kconfig = realtek_dir.join("Kconfig");
    let parent_makefile = realtek_dir.join("Makefile");

    require_dir(tree, "kernel tree")?;
    require_file(&tree.join("Kconfig"), "kernel tree Kconfig")?;
    require_file(&tree.join("Makefile"), "kernel tree Makefile")?;
    require_dir(&realtek_dir, "Realtek driver directory")?;
    require_file(&parent_kconfig, "Realtek parent Kconfig")?;
    require_file(&parent_makefile, "Realtek parent Makefile")?;
    require_dir(&source, "rtlmq source")?;
    require_file(&source.join("Makefile"), "rtlmq Makefile")?;
    require_file(&source.join("Kconfig"), "rtlmq Kconfig")?;

    if target_dir.exists() {
        std::fs::remove_dir_all(&target_dir)
            .with_context(|| format!("failed to remove {}", target_dir.display()))?;
    }
    std::fs::create_dir_all(&target_dir)
        .with_context(|| format!("failed to create {}", target_dir.display()))?;

    copy_file(&source.join("Makefile"), &target_dir.join("Makefile"))?;
    copy_file(&source.join("Kconfig"), &target_dir.join("Kconfig"))?;
    copy_top_level_sources(&source, &target_dir)?;

    if let Some(tests) = tests_source.as_deref().filter(|path| path.is_dir()) {
        copy_top_level_sources(tests, &target_dir)?;
        for name in [".kunitconfig", "TESTING.rst"] {
            let src = tests.join(name);
            if src.exists() {
                copy_file(&src, &target_dir.join(name))?;
            }
        }

        let selftest_src = tests.join("selftests/drivers/net/rtlmq");
        if selftest_src.is_dir() {
            let selftest_parent = tree.join("tools/testing/selftests/drivers/net");
            require_dir(&selftest_parent, "selftests parent directory")?;
            let selftest_dst = selftest_parent.join("rtlmq");
            if selftest_dst.exists() {
                std::fs::remove_dir_all(&selftest_dst)
                    .with_context(|| format!("failed to remove {}", selftest_dst.display()))?;
            }
            copy_tree(&selftest_src, &selftest_dst)?;
        }
    }

    let scripts_src = source.join("scripts");
    if scripts_src.is_dir() {
        let scripts_dst = target_dir.join("scripts");
        if scripts_dst.exists() {
            std::fs::remove_dir_all(&scripts_dst)
                .with_context(|| format!("failed to remove {}", scripts_dst.display()))?;
        }
        copy_tree(&scripts_src, &scripts_dst)?;
        ensure_shell_scripts_executable(&scripts_dst)?;
    }

    insert_line_before_first_anchor(
        &parent_kconfig,
        r#"source "drivers/net/ethernet/realtek/rtlmq/Kconfig""#,
        &["config RTASE", "endif # NET_VENDOR_REALTEK"],
    )?;
    insert_line_before_first_anchor(
        &parent_makefile,
        "obj-$(CONFIG_RTLMQ) += rtlmq/",
        &["obj-$(CONFIG_RTASE) += rtase/"],
    )?;

    log::info!(
        "integrations: installed rtlmq from {} into {}",
        source.display(),
        target_dir.display()
    );

    Ok(())
}

fn resolve_from_root(root: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn require_dir(path: &Path, what: &str) -> Result<()> {
    if !path.is_dir() {
        anyhow::bail!("missing {}: {}", what, path.display());
    }
    Ok(())
}

fn require_file(path: &Path, what: &str) -> Result<()> {
    if !path.is_file() {
        anyhow::bail!("missing {}: {}", what, path.display());
    }
    Ok(())
}

fn copy_top_level_sources(src_dir: &Path, dst_dir: &Path) -> Result<()> {
    for entry in std::fs::read_dir(src_dir)
        .with_context(|| format!("failed to read {}", src_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_file() {
            continue;
        }
        let ext = path.extension().and_then(OsStr::to_str);
        if !matches!(ext, Some("c" | "h")) {
            continue;
        }
        copy_file(&path, &dst_dir.join(entry.file_name()))?;
    }
    Ok(())
}

fn copy_tree(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst).with_context(|| format!("failed to create {}", dst.display()))?;

    for entry in
        std::fs::read_dir(src).with_context(|| format!("failed to read {}", src.display()))?
    {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_tree(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            copy_file(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

fn copy_file(src: &Path, dst: &Path) -> Result<()> {
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    std::fs::copy(src, dst)
        .with_context(|| format!("failed to copy {} to {}", src.display(), dst.display()))?;
    let perms = std::fs::metadata(src)
        .with_context(|| format!("failed to stat {}", src.display()))?
        .permissions();
    std::fs::set_permissions(dst, perms)
        .with_context(|| format!("failed to set permissions on {}", dst.display()))?;
    Ok(())
}

#[cfg(unix)]
fn ensure_shell_scripts_executable(root: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
    {
        if entry.path().extension().and_then(OsStr::to_str) != Some("sh") {
            continue;
        }
        let mut perms = std::fs::metadata(entry.path())?.permissions();
        perms.set_mode(perms.mode() | 0o111);
        std::fs::set_permissions(entry.path(), perms)?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn ensure_shell_scripts_executable(_root: &Path) -> Result<()> {
    Ok(())
}

fn insert_line_before_first_anchor(path: &Path, wanted: &str, anchors: &[&str]) -> Result<()> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();

    if lines.iter().any(|line| line == wanted) {
        return Ok(());
    }

    let insert_at = lines
        .iter()
        .position(|line| anchors.iter().any(|anchor| line == anchor))
        .unwrap_or(lines.len());
    lines.insert(insert_at, wanted.to_string());

    let mut out = lines.join("\n");
    out.push('\n');
    std::fs::write(path, out).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}
