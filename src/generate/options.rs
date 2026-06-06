use super::frozen_plan::FrozenPlanInputs;

/// Options for the generate command.
pub struct GenerateOptions {
    pub dry_run: bool,
    pub deep_dry_run: bool,
    pub report_only: bool,
    pub keep_temp: bool,
    pub max_fixup_passes: Option<usize>,
    pub matrix: Option<String>,
    pub offline: bool,
    pub frozen_plan: Option<FrozenPlanInputs>,
    pub force: bool,
    pub base_ref: Option<String>,
    pub feature: Option<String>,
    pub remove_feature: Option<String>,
    pub preserve_feature: Option<String>,
    pub arch: Option<String>,
    pub primary_arch: Option<String>,
    pub secondary_arch: Option<String>,
    pub safety: Option<String>,
    pub strict: bool,
    pub no_strict: bool,
    pub run_selftests: bool,
}

impl GenerateOptions {
    pub(crate) fn normalized_base_ref_for_request(&self) -> Option<String> {
        self.base_ref
            .as_deref()
            .map(|base_ref| base_ref.trim().to_string())
    }

    pub(crate) fn normalized_feature_for_request(&self) -> Option<String> {
        self.feature
            .as_deref()
            .map(|feature| feature.trim().to_string())
    }

    pub(crate) fn normalized_remove_feature_for_request(&self) -> Option<String> {
        self.remove_feature
            .as_deref()
            .map(|feature| feature.trim().to_string())
    }

    pub(crate) fn normalized_preserve_feature_for_request(&self) -> Option<String> {
        self.preserve_feature
            .as_deref()
            .map(|feature| feature.trim().to_string())
    }

    pub(crate) fn normalized_arch_for_request(&self) -> Option<String> {
        self.arch.as_deref().map(|arch| arch.trim().to_string())
    }

    pub(crate) fn normalized_primary_arch_for_request(&self) -> Option<String> {
        self.primary_arch
            .as_deref()
            .map(|arch| arch.trim().to_string())
    }

    pub(crate) fn normalized_secondary_arch_for_request(&self) -> Option<String> {
        self.secondary_arch
            .as_deref()
            .map(|arch| arch.trim().to_string())
    }

    pub(crate) fn normalized_safety_for_request(&self) -> Option<String> {
        self.safety
            .as_deref()
            .map(|safety| safety.trim().to_string())
    }

    pub(crate) fn normalized_matrix_for_request(&self) -> Option<String> {
        self.matrix
            .as_deref()
            .map(|matrix| matrix.trim().to_ascii_lowercase())
    }
}
