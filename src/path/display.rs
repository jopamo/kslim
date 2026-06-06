//! Display normalization for typed paths that cross config/report boundaries.

use std::path::Path;

pub(crate) fn path_to_config_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
