//! Compatibility facade for frozen generate plans.
//!
//! Frozen-plan verification lives in `crate::plan`; this module preserves
//! existing generate-local call sites while migration proceeds.

pub(crate) use crate::plan::{
    ensure_tree_matches_frozen_base, load_frozen_plan, write_frozen_plan_for_request,
    FrozenPlanInputs,
};
