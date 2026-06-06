//! Read-only tree indexing for reducer passes.
//!
//! This module will index files, headers, include sites, Kconfig graph facts,
//! kbuild graph facts, and supported preprocessor gates.
//! It must not mutate the indexed tree or decide reducer policy.

use anyhow::Result;
use std::path::{Path, PathBuf};

mod abi_index;
mod file_index;
mod kbuild_index;
mod kconfig_index;
mod query;
mod source_index;

use abi_index::{abi_path_fact, abi_source_reference_from_include_site};
use file_index::{
    ensure_index_text_not_host_absolute_path, ensure_relative_index_path, existing_touched_files,
    index_path_is_under, indexed_tree_files, is_header_path, normalize_touched_paths,
    relative_path_under_root,
};
use kbuild_index::build_kbuild_domain;
use kconfig_index::{kconfig_files_from_indexed_files, scan_kconfig_file};
use source_index::{scan_c_family_file, unique_cpp_gate_count};

#[allow(unused_imports)]
pub use abi_index::{
    AbiPathFact, AbiPathIndex, AbiSourceReference, AbiSourceReferenceIndex, AbiSurfaceKind,
};
pub use file_index::{FileIndex, HeaderIndex};
pub use kbuild_index::{
    KbuildDirectoryReference, KbuildDirectoryReferenceIndex, KbuildFileIndex,
    KbuildObjectProviderIndex, KbuildObjectReference, KbuildObjectReferenceIndex,
};
#[allow(unused_imports)]
pub use kconfig_index::{
    KconfigDefinition, KconfigDefinitionIndex, KconfigFileIndex, KconfigReferenceIndex,
    KconfigSourceIndex, KconfigSourceReference, KconfigSymbolReference,
};
#[allow(unused_imports)]
pub use source_index::{CppGate, CppGateIndex, IncludeSite, IncludeSiteIndex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TreeIndexRebuildDomain {
    All,
    Kconfig,
    Kbuild,
    CFamily,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TreeIndexMutatingPass {
    DeclaredPrune,
    KconfigRewrite,
    KbuildRewrite,
    CppFold,
    IncludeRewrite,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TreeIndexCoverageStats {
    pub files_scanned: usize,
    pub headers_indexed: usize,
    pub include_sites_indexed: usize,
    pub kconfig_files_indexed: usize,
    pub kconfig_symbols_defined: usize,
    pub kconfig_symbol_refs_indexed: usize,
    pub kbuild_files_indexed: usize,
    pub kbuild_object_refs_indexed: usize,
    pub cpp_gates_indexed: usize,
    pub abi_paths_indexed: usize,
    pub abi_source_refs_indexed: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TreeIndex {
    pub files: FileIndex,
    pub headers: HeaderIndex,
    pub include_sites: IncludeSiteIndex,
    pub kconfig_files: KconfigFileIndex,
    pub kconfig_defs: KconfigDefinitionIndex,
    pub kconfig_refs: KconfigReferenceIndex,
    pub kconfig_sources: KconfigSourceIndex,
    pub kbuild_files: KbuildFileIndex,
    pub kbuild_object_providers: KbuildObjectProviderIndex,
    pub kbuild_object_refs: KbuildObjectReferenceIndex,
    pub kbuild_dir_refs: KbuildDirectoryReferenceIndex,
    pub cpp_gates_by_symbol: CppGateIndex,
    pub abi_paths: AbiPathIndex,
    pub abi_source_refs: AbiSourceReferenceIndex,
}

impl TreeIndex {
    pub fn build<M>(root: &Path, manifest: &M) -> Result<Self> {
        let mut index = Self::default();
        index.rebuild_all(root, manifest)?;
        Ok(index)
    }

    pub(crate) fn rebuild_after_mutating_pass<M>(
        &mut self,
        root: &Path,
        manifest: &M,
        domain: TreeIndexRebuildDomain,
        touched: &[PathBuf],
        _pass: TreeIndexMutatingPass,
    ) -> Result<()> {
        match domain {
            TreeIndexRebuildDomain::All => self.rebuild_all(root, manifest),
            TreeIndexRebuildDomain::Kconfig => self.rebuild_kconfig(root, touched),
            TreeIndexRebuildDomain::Kbuild => self.rebuild_kbuild(root, touched),
            TreeIndexRebuildDomain::CFamily => self.rebuild_c_family(root, touched),
        }
    }

    fn rebuild_all<M>(&mut self, root: &Path, _manifest: &M) -> Result<()> {
        *self = Self::build_fresh(root)?;
        Ok(())
    }

    fn rebuild_kconfig(&mut self, root: &Path, touched: &[PathBuf]) -> Result<()> {
        self.rebuild_touched(root, touched)
    }

    fn rebuild_kbuild(&mut self, root: &Path, touched: &[PathBuf]) -> Result<()> {
        self.rebuild_touched(root, touched)
    }

    fn rebuild_c_family(&mut self, root: &Path, touched: &[PathBuf]) -> Result<()> {
        self.rebuild_touched(root, touched)
    }

    #[allow(dead_code)]
    pub fn coverage_stats(&self) -> TreeIndexCoverageStats {
        TreeIndexCoverageStats {
            files_scanned: self.files.len(),
            headers_indexed: self.headers.len(),
            include_sites_indexed: self.include_sites.len(),
            kconfig_files_indexed: self.kconfig_files.len(),
            kconfig_symbols_defined: self.kconfig_defs.len(),
            kconfig_symbol_refs_indexed: self.kconfig_refs.len(),
            kbuild_files_indexed: self.kbuild_files.len(),
            kbuild_object_refs_indexed: self.kbuild_object_refs.len(),
            cpp_gates_indexed: unique_cpp_gate_count(&self.cpp_gates_by_symbol),
            abi_paths_indexed: self.abi_paths.len(),
            abi_source_refs_indexed: self.abi_source_refs.len(),
        }
    }

    fn rebuild_touched(&mut self, root: &Path, touched: &[PathBuf]) -> Result<()> {
        if touched.is_empty() {
            *self = Self::build_fresh(root)?;
            return Ok(());
        }

        let touched = normalize_touched_paths(root, touched)?;
        for path in &touched {
            self.remove_entries_under(path);
        }
        for (relative, path) in existing_touched_files(root, &touched)? {
            self.add_common_file(&relative);
            self.add_source_file_facts(scan_c_family_file(&relative, &path));
            self.add_kconfig_file_facts(scan_kconfig_file(&relative, &path));
        }
        self.rebuild_full_kbuild_domain(root)?;
        self.validate_invariants()?;
        Ok(())
    }

    fn build_fresh(root: &Path) -> Result<Self> {
        let mut index = Self::default();

        for (relative, path) in indexed_tree_files(root)? {
            index.add_common_file(&relative);
            index.add_source_file_facts(scan_c_family_file(&relative, &path));
        }

        for path in kconfig_files_from_indexed_files(root, &index.files) {
            let relative = relative_path_under_root(root, &path)?;
            index.add_kconfig_file_facts(scan_kconfig_file(&relative, &path));
        }

        index.rebuild_full_kbuild_domain(root)?;
        index.validate_invariants()?;
        Ok(index)
    }

    fn add_common_file(&mut self, relative: &Path) {
        self.files.insert(relative.to_path_buf());
        if is_header_path(relative) {
            self.headers.insert(relative.to_path_buf());
        }
        if let Some(fact) = abi_path_fact(relative) {
            self.abi_paths.insert(fact);
        }
    }

    fn add_kconfig_file_facts(&mut self, facts: kconfig_index::KconfigFileFacts) {
        self.kconfig_files.extend(facts.files);
        self.kconfig_defs.extend(facts.definitions);
        self.kconfig_refs.extend(facts.references);
        self.kconfig_sources.extend(facts.sources);
    }

    fn add_source_file_facts(&mut self, facts: source_index::SourceFileFacts) {
        for site in facts.include_sites {
            if let Some(reference) = abi_source_reference_from_include_site(&site) {
                self.abi_source_refs.insert(reference);
            }
            self.include_sites.insert(site);
        }
        for (symbol, gates) in facts.cpp_gates_by_symbol {
            self.cpp_gates_by_symbol
                .entry(symbol)
                .or_default()
                .extend(gates);
        }
    }

    fn rebuild_full_kbuild_domain(&mut self, root: &Path) -> Result<()> {
        let facts = build_kbuild_domain(root)?;
        self.kbuild_files = facts.files;
        self.kbuild_object_providers = facts.object_providers;
        self.kbuild_object_refs = facts.object_refs;
        self.kbuild_dir_refs = facts.directory_refs;
        Ok(())
    }

    fn remove_entries_under(&mut self, base: &Path) {
        self.files.retain(|path| !index_path_is_under(path, base));
        self.headers.retain(|path| !index_path_is_under(path, base));
        self.include_sites
            .retain(|site| !index_path_is_under(&site.file, base));
        self.kconfig_files
            .retain(|path| !index_path_is_under(path, base));
        self.kconfig_defs
            .retain(|definition| !index_path_is_under(&definition.file, base));
        self.kconfig_refs
            .retain(|reference| !index_path_is_under(&reference.file, base));
        self.kconfig_sources
            .retain(|source| !index_path_is_under(&source.file, base));
        self.kbuild_files
            .retain(|path| !index_path_is_under(path, base));
        self.kbuild_object_providers
            .retain(|path| !index_path_is_under(path, base));
        self.kbuild_object_refs
            .retain(|reference| !index_path_is_under(&reference.file, base));
        self.kbuild_dir_refs
            .retain(|reference| !index_path_is_under(&reference.file, base));
        self.abi_paths
            .retain(|fact| !index_path_is_under(&fact.path, base));
        self.abi_source_refs
            .retain(|reference| !index_path_is_under(&reference.file, base));
        for gates in self.cpp_gates_by_symbol.values_mut() {
            gates.retain(|gate| !index_path_is_under(&gate.file, base));
        }
        self.cpp_gates_by_symbol
            .retain(|_, gates| !gates.is_empty());
    }

    fn validate_invariants(&self) -> Result<()> {
        for path in &self.files {
            ensure_relative_index_path(path)?;
        }
        for path in &self.headers {
            ensure_relative_index_path(path)?;
        }
        for site in &self.include_sites {
            ensure_relative_index_path(&site.file)?;
            ensure_index_text_not_host_absolute_path(&site.target)?;
        }
        for path in &self.kconfig_files {
            ensure_relative_index_path(path)?;
        }
        for definition in &self.kconfig_defs {
            ensure_relative_index_path(&definition.file)?;
        }
        for reference in &self.kconfig_refs {
            ensure_relative_index_path(&reference.file)?;
        }
        for source in &self.kconfig_sources {
            ensure_relative_index_path(&source.file)?;
            ensure_index_text_not_host_absolute_path(&source.source)?;
        }
        for path in &self.kbuild_files {
            ensure_relative_index_path(path)?;
        }
        for path in &self.kbuild_object_providers {
            ensure_relative_index_path(path)?;
        }
        for reference in &self.kbuild_object_refs {
            ensure_relative_index_path(&reference.file)?;
            ensure_index_text_not_host_absolute_path(&reference.object)?;
            ensure_relative_index_path(&reference.resolved_path)?;
        }
        for reference in &self.kbuild_dir_refs {
            ensure_relative_index_path(&reference.file)?;
            ensure_index_text_not_host_absolute_path(&reference.directory)?;
            for resolved in &reference.resolved_paths {
                ensure_relative_index_path(resolved)?;
            }
        }
        for gates in self.cpp_gates_by_symbol.values() {
            for gate in gates {
                ensure_relative_index_path(&gate.file)?;
            }
        }
        for fact in &self.abi_paths {
            ensure_relative_index_path(&fact.path)?;
        }
        for reference in &self.abi_source_refs {
            ensure_relative_index_path(&reference.file)?;
            ensure_relative_index_path(&reference.target)?;
        }
        Ok(())
    }

}

#[cfg(test)]
mod tests;
