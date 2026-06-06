use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

use crate::generated::is_generated_include_header_path;
use crate::hardware::DeviceBindingRemovalProof;
use crate::exported_symbols::ExportedSymbolRemovalProof;
use crate::model::{HeaderPath, KbuildObject};
use crate::path_policy::{normalized_relative_path_covers, path_is_normalized_tree_root};
use crate::runtime::RuntimeRegistrationRemovalProof;

use super::validate::is_public_header_path;

pub(super) fn derive_removed_headers(
    root: Option<&Path>,
    removed_paths: &BTreeSet<PathBuf>,
    removed_dirs: &BTreeSet<PathBuf>,
    removed_files: &BTreeSet<PathBuf>,
    generated_include_roots: &BTreeSet<PathBuf>,
) -> Result<BTreeSet<HeaderPath>> {
    let mut headers = BTreeSet::new();

    for file in removed_files {
        insert_removed_header_if_allowed(
            file,
            removed_paths,
            generated_include_roots,
            &mut headers,
        )?;
    }

    let Some(root) = root else {
        return Ok(headers);
    };

    for dir in removed_dirs {
        let absolute_dir = root.join(dir);
        if !absolute_dir.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&absolute_dir).follow_links(false) {
            let entry = entry.with_context(|| {
                format!("failed to walk removed header directory {}", dir.display())
            })?;
            if !entry.file_type().is_file() {
                continue;
            }
            let relative = entry.path().strip_prefix(root).with_context(|| {
                format!(
                    "failed to derive root-relative header path for {}",
                    entry.path().display()
                )
            })?;
            insert_removed_header_if_allowed(
                relative,
                removed_paths,
                generated_include_roots,
                &mut headers,
            )?;
        }
    }

    Ok(headers)
}

pub(super) fn derive_removed_public_headers(
    removed_headers: &BTreeSet<HeaderPath>,
) -> BTreeSet<HeaderPath> {
    removed_headers
        .iter()
        .filter(|header| is_public_header_path(header.as_path()))
        .cloned()
        .collect()
}

pub(super) fn derive_removed_kconfig_sources(
    root: Option<&Path>,
    removed_paths: &BTreeSet<PathBuf>,
    removed_dirs: &BTreeSet<PathBuf>,
    removed_files: &BTreeSet<PathBuf>,
) -> Result<BTreeSet<PathBuf>> {
    let mut sources = BTreeSet::new();

    for path in removed_paths {
        if root.is_none() && is_kconfig_source_path(path) {
            sources.insert(path.clone());
        }
    }
    for file in removed_files {
        if is_kconfig_source_path(file) {
            sources.insert(file.clone());
        }
    }

    let Some(root) = root else {
        return Ok(sources);
    };

    for dir in removed_dirs {
        let absolute_dir = root.join(dir);
        if !absolute_dir.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&absolute_dir).follow_links(false) {
            let entry = entry.with_context(|| {
                format!("failed to walk removed Kconfig directory {}", dir.display())
            })?;
            if !entry.file_type().is_file() {
                continue;
            }
            let relative = entry.path().strip_prefix(root).with_context(|| {
                format!(
                    "failed to derive root-relative Kconfig source path for {}",
                    entry.path().display()
                )
            })?;
            if is_kconfig_source_path(relative) {
                sources.insert(relative.to_path_buf());
            }
        }
    }

    for source in
        removed_kconfig_source_references(root, removed_paths, removed_dirs, removed_files)?
    {
        sources.insert(source);
    }

    Ok(sources)
}

fn removed_kconfig_source_references(
    root: &Path,
    removed_paths: &BTreeSet<PathBuf>,
    removed_dirs: &BTreeSet<PathBuf>,
    removed_files: &BTreeSet<PathBuf>,
) -> Result<BTreeSet<PathBuf>> {
    let mut sources = BTreeSet::new();

    for path in kconfig_source_scan_files(root) {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read Kconfig source file {}", path.display()))?;
        let current_dir = path.parent().unwrap_or(root);
        for line in content.lines() {
            let Some(source) = crate::kconfig::parse_kconfig_source(line) else {
                continue;
            };
            let Some(relative) =
                resolve_static_kconfig_source_reference(root, current_dir, &source)
            else {
                continue;
            };
            if removed_kconfig_source_target_is_manifest_removed(
                &relative,
                removed_paths,
                removed_dirs,
                removed_files,
            ) {
                sources.insert(relative);
            }
        }
    }

    Ok(sources)
}

fn kconfig_source_scan_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
    {
        if is_kconfig_source_path(entry.path()) {
            files.push(entry.into_path());
        }
    }
    files.sort();
    files
}

fn resolve_static_kconfig_source_reference(
    root: &Path,
    current_dir: &Path,
    source: &crate::kconfig::KconfigSource,
) -> Option<PathBuf> {
    if source.path.contains('$') {
        return None;
    }
    let source_path = Path::new(&source.path);
    if source_path.is_absolute() {
        return None;
    }

    let primary = if source.relative {
        current_dir.join(source_path)
    } else {
        root.join(source_path)
    };
    let fallback = if source.relative {
        root.join(source_path)
    } else {
        current_dir.join(source_path)
    };

    [primary, fallback].into_iter().find_map(|candidate| {
        let candidate = normalize_path_components(&candidate);
        if !candidate.exists() {
            return None;
        }
        root_relative_normalized_path(root, &candidate)
    })
}

fn root_relative_normalized_path(root: &Path, path: &Path) -> Option<PathBuf> {
    let root = normalize_path_components(root);
    let path = normalize_path_components(path);
    let relative = path.strip_prefix(&root).ok()?;
    (!relative.as_os_str().is_empty()).then(|| relative.to_path_buf())
}

fn removed_kconfig_source_target_is_manifest_removed(
    path: &Path,
    removed_paths: &BTreeSet<PathBuf>,
    removed_dirs: &BTreeSet<PathBuf>,
    removed_files: &BTreeSet<PathBuf>,
) -> bool {
    removed_files.contains(path)
        || removed_paths.contains(path)
        || removed_dirs
            .iter()
            .any(|dir| normalized_relative_path_covers(dir, path))
}

fn is_kconfig_source_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "Kconfig" || name.starts_with("Kconfig."))
}

fn normalize_path_components(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(part) => out.push(part),
            Component::RootDir | Component::Prefix(_) => {
                out = PathBuf::from(component.as_os_str());
            }
        }
    }

    out
}

pub(super) fn derive_removed_kbuild_objects(
    root: Option<&Path>,
    removed_paths: &BTreeSet<PathBuf>,
    removed_dirs: &BTreeSet<PathBuf>,
    removed_files: &BTreeSet<PathBuf>,
) -> Result<BTreeSet<KbuildObject>> {
    let mut removed_object_paths = BTreeSet::new();
    let mut objects = BTreeSet::new();

    for path in removed_paths {
        if root.is_none() {
            insert_kbuild_object_provider_path(path, &mut removed_object_paths);
        }
    }
    for file in removed_files {
        insert_kbuild_object_provider_path(file, &mut removed_object_paths);
    }

    if let Some(root) = root {
        for dir in removed_dirs {
            let absolute_dir = root.join(dir);
            if !absolute_dir.is_dir() {
                continue;
            }
            for entry in WalkDir::new(&absolute_dir).follow_links(false) {
                let entry = entry.with_context(|| {
                    format!("failed to walk removed kbuild directory {}", dir.display())
                })?;
                if !entry.file_type().is_file() {
                    continue;
                }
                let relative = entry.path().strip_prefix(root).with_context(|| {
                    format!(
                        "failed to derive root-relative kbuild object path for {}",
                        entry.path().display()
                    )
                })?;
                insert_kbuild_object_provider_path(relative, &mut removed_object_paths);
            }
        }
    }

    for path in &removed_object_paths {
        objects.insert(kbuild_object_path_string(path, false)?);
    }
    for dir in removed_dirs {
        if path_is_normalized_tree_root(dir) {
            continue;
        }
        objects.insert(kbuild_object_path_string(dir, true)?);
    }

    let Some(root) = root else {
        return Ok(objects);
    };

    let kbuild_index = crate::kbuild::build_kbuild_index(root)
        .with_context(|| "failed to build kbuild index for removal manifest")?;
    insert_removed_kbuild_directory_refs(root, &kbuild_index, removed_dirs, &mut objects)?;
    for target in removed_composite_kbuild_object_targets(
        root,
        &kbuild_index,
        removed_dirs,
        &removed_object_paths,
    ) {
        objects.insert(kbuild_object_path_string(&target, false)?);
    }

    Ok(objects)
}

pub(super) fn derive_removed_exported_symbol_proofs(
    root: Option<&Path>,
    removed_paths: &BTreeSet<PathBuf>,
    removed_dirs: &BTreeSet<PathBuf>,
    removed_files: &BTreeSet<PathBuf>,
) -> Result<BTreeSet<ExportedSymbolRemovalProof>> {
    let Some(root) = root else {
        return Ok(BTreeSet::new());
    };

    crate::exported_symbols::prove_removed_exports_have_no_live_consumers(
        root,
        removed_paths,
        removed_dirs,
        removed_files,
    )
}

pub(super) fn derive_removed_device_binding_proofs(
    root: Option<&Path>,
    removed_paths: &BTreeSet<PathBuf>,
    removed_dirs: &BTreeSet<PathBuf>,
    removed_files: &BTreeSet<PathBuf>,
) -> Result<BTreeSet<DeviceBindingRemovalProof>> {
    let Some(root) = root else {
        return Ok(BTreeSet::new());
    };

    crate::hardware::prove_removed_device_bindings_have_no_live_references(
        root,
        removed_paths,
        removed_dirs,
        removed_files,
    )
}

pub(super) fn derive_removed_runtime_registration_proofs(
    root: Option<&Path>,
    removed_paths: &BTreeSet<PathBuf>,
    removed_dirs: &BTreeSet<PathBuf>,
    removed_files: &BTreeSet<PathBuf>,
) -> Result<BTreeSet<RuntimeRegistrationRemovalProof>> {
    let Some(root) = root else {
        return Ok(BTreeSet::new());
    };

    crate::runtime::prove_removed_runtime_registrations_have_no_live_entry_points(
        root,
        removed_paths,
        removed_dirs,
        removed_files,
    )
}

fn insert_kbuild_object_provider_path(path: &Path, out: &mut BTreeSet<PathBuf>) {
    if let Some(provider) = kbuild_object_provider_path(path) {
        out.insert(provider);
    }
}

fn kbuild_object_provider_path(path: &Path) -> Option<PathBuf> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("c") | Some("S") => Some(path.with_extension("o")),
        _ => None,
    }
}

fn insert_removed_kbuild_directory_refs(
    root: &Path,
    kbuild_index: &crate::kbuild::KbuildIndex,
    removed_dirs: &BTreeSet<PathBuf>,
    objects: &mut BTreeSet<KbuildObject>,
) -> Result<()> {
    for reference in &kbuild_index.directory_references {
        let current_dir = root.join(reference.file.parent().unwrap_or(Path::new("")));
        for candidate in
            crate::kbuild::make_dir_candidates(root, &current_dir, &reference.directory)
        {
            if removed_dirs
                .iter()
                .any(|dir| normalized_relative_path_covers(dir, &candidate))
            {
                objects.insert(kbuild_object_path_string(&candidate, true)?);
            }
        }
    }

    Ok(())
}

fn removed_composite_kbuild_object_targets(
    root: &Path,
    kbuild_index: &crate::kbuild::KbuildIndex,
    removed_dirs: &BTreeSet<PathBuf>,
    removed_object_paths: &BTreeSet<PathBuf>,
) -> BTreeSet<PathBuf> {
    let mut members_by_target = BTreeMap::<PathBuf, Vec<PathBuf>>::new();
    for member in &kbuild_index.composite_object_members {
        let Some(member_path) = resolve_kbuild_object_token(root, &member.file, &member.member)
        else {
            continue;
        };
        members_by_target
            .entry(member.target.clone())
            .or_default()
            .push(member_path);
    }

    let mut stale_targets = BTreeSet::new();
    loop {
        let mut changed = false;
        for (target, members) in &members_by_target {
            if stale_targets.contains(target)
                || live_direct_kbuild_object_provider_exists(
                    root,
                    target,
                    removed_dirs,
                    removed_object_paths,
                )
            {
                continue;
            }
            if !members.is_empty()
                && members.iter().all(|member| {
                    removed_kbuild_object_path_matches(member, removed_dirs, removed_object_paths)
                        || stale_targets.contains(member)
                })
            {
                stale_targets.insert(target.clone());
                changed = true;
            }
        }

        if !changed {
            return stale_targets;
        }
    }
}

fn resolve_kbuild_object_token(root: &Path, file: &Path, token: &str) -> Option<PathBuf> {
    if token.is_empty()
        || !token.ends_with(".o")
        || token.starts_with('/')
        || token.contains('$')
        || token.contains('%')
        || token.contains(':')
    {
        return None;
    }

    let current_dir = root.join(file.parent().unwrap_or(Path::new("")));
    root_relative_normalized_path(root, &current_dir.join(token))
}

fn live_direct_kbuild_object_provider_exists(
    root: &Path,
    object: &Path,
    removed_dirs: &BTreeSet<PathBuf>,
    removed_object_paths: &BTreeSet<PathBuf>,
) -> bool {
    if removed_kbuild_object_path_matches(object, removed_dirs, removed_object_paths) {
        return false;
    }

    ["c", "S"].iter().any(|extension| {
        let source = object.with_extension(extension);
        root.join(&source).is_file()
            && !removed_dirs
                .iter()
                .any(|dir| normalized_relative_path_covers(dir, &source))
            && !kbuild_object_provider_path(&source)
                .is_some_and(|provider| removed_object_paths.contains(&provider))
    })
}

fn removed_kbuild_object_path_matches(
    object: &Path,
    removed_dirs: &BTreeSet<PathBuf>,
    removed_object_paths: &BTreeSet<PathBuf>,
) -> bool {
    removed_object_paths.contains(object)
        || removed_dirs
            .iter()
            .any(|dir| normalized_relative_path_covers(dir, object))
}

fn kbuild_object_path_string(path: &Path, directory: bool) -> Result<KbuildObject> {
    let mut value = relative_path_string(path, "removed kbuild object path")?;
    if directory && !value.ends_with('/') {
        value.push('/');
    }
    KbuildObject::new(value)
}

fn insert_removed_header_if_allowed(
    path: &Path,
    removed_paths: &BTreeSet<PathBuf>,
    generated_include_roots: &BTreeSet<PathBuf>,
    headers: &mut BTreeSet<HeaderPath>,
) -> Result<()> {
    if removed_path_is_derivable_header(path, removed_paths, generated_include_roots) {
        headers.insert(header_path_string(path)?);
    }
    Ok(())
}

fn removed_path_is_derivable_header(
    path: &Path,
    removed_paths: &BTreeSet<PathBuf>,
    generated_include_roots: &BTreeSet<PathBuf>,
) -> bool {
    if !path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext == "h")
    {
        return false;
    }
    if is_public_header_path(path) && !removed_paths.contains(path) {
        return false;
    }
    if is_generated_include_header_path(path) {
        return generated_include_roots
            .iter()
            .any(|root| path == root.as_path() || path.starts_with(root));
    }

    path.starts_with("include/linux")
        || path.starts_with("include/uapi")
        || path.starts_with("include/net")
        || is_arch_include_header_path(path)
        || path.starts_with("drivers")
}

fn is_arch_include_header_path(path: &Path) -> bool {
    let mut components = path.components();
    matches!(
        (
            components.next().and_then(|part| part.as_os_str().to_str()),
            components.next().and_then(|part| part.as_os_str().to_str()),
            components.next().and_then(|part| part.as_os_str().to_str()),
        ),
        (Some("arch"), Some(_arch), Some("include"))
    )
}


fn header_path_string(path: &Path) -> Result<HeaderPath> {
    HeaderPath::new(relative_path_string(path, "removed header path")?)
}

fn relative_path_string(path: &Path, label: &str) -> Result<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => {
                let Some(part) = part.to_str() else {
                    anyhow::bail!("{} contains non-UTF-8 component: {}", label, path.display());
                };
                parts.push(part);
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                anyhow::bail!(
                    "{} must be normalized and relative: {}",
                    label,
                    path.display()
                );
            }
        }
    }

    Ok(parts.join("/"))
}
