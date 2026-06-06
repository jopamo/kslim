mod load;
mod model;
mod source_map;
mod templates;
mod validate;
pub use crate::abi::AbiPolicyConfig;
pub use load::{
    insert_profile_feature_selection_cli_overrides, insert_profile_strictness_cli_overrides,
    list_profiles, load_kslim_config, load_profile, normalize_arch_name, normalize_feature_name,
    normalize_profile_name, require_known_profile, select_profile_features,
    ProfileFeatureSelection,
};
pub(crate) use load::{load_kslim_config_file_with_source_map, load_profile_with_source_map};
pub use model::*;
pub use source_map::*;
pub use templates::{
    amdgpu_prune_profile_template, default_kslim_config, default_profile_config,
    default_publish_config, kernel_build_iteration_guide,
};
pub use validate::{validate_config, validate_profile};
#[cfg(test)]
mod profile_validation_tests;
#[cfg(test)]
mod tests;
