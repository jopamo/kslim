use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
pub(crate) enum ReducerStage {
    #[default]
    #[serde(rename = "build_manifest")]
    BuildManifest,
    #[serde(rename = "build_initial_index")]
    BuildInitialIndex,
    #[serde(rename = "prune_declared_paths")]
    PruneDeclaredPaths,
    #[serde(rename = "rebuild_full_index")]
    RebuildFullIndex,
    #[serde(rename = "rewrite_kconfig")]
    RewriteKconfig,
    #[serde(rename = "rebuild_kconfig_index")]
    RebuildKconfigIndex,
    #[serde(rename = "rewrite_kbuild")]
    RewriteKbuild,
    #[serde(rename = "rebuild_kbuild_index")]
    RebuildKbuildIndex,
    #[serde(rename = "fold_preprocessor")]
    FoldPreprocessor,
    #[serde(rename = "rebuild_c_header_index")]
    RebuildCHeaderIndex,
    #[serde(rename = "rewrite_includes")]
    RewriteIncludes,
    #[serde(rename = "run_selftests")]
    RunSelftests,
    #[serde(rename = "classify_diagnostics")]
    ClassifyDiagnostics,
    #[serde(rename = "apply_fixups")]
    ApplyFixups,
    #[serde(rename = "reindex_and_repeat")]
    ReindexAndRepeat,
}

impl ReducerStage {
    #[allow(dead_code)]
    pub(crate) const ALL: [Self; 15] = [
        Self::BuildManifest,
        Self::BuildInitialIndex,
        Self::PruneDeclaredPaths,
        Self::RebuildFullIndex,
        Self::RewriteKconfig,
        Self::RebuildKconfigIndex,
        Self::RewriteKbuild,
        Self::RebuildKbuildIndex,
        Self::FoldPreprocessor,
        Self::RebuildCHeaderIndex,
        Self::RewriteIncludes,
        Self::RunSelftests,
        Self::ClassifyDiagnostics,
        Self::ApplyFixups,
        Self::ReindexAndRepeat,
    ];

    pub(crate) const fn as_str(self) -> &'static str {
        self.stable_name()
    }

    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::BuildManifest => "build_manifest",
            Self::BuildInitialIndex => "build_initial_index",
            Self::PruneDeclaredPaths => "prune_declared_paths",
            Self::RebuildFullIndex => "rebuild_full_index",
            Self::RewriteKconfig => "rewrite_kconfig",
            Self::RebuildKconfigIndex => "rebuild_kconfig_index",
            Self::RewriteKbuild => "rewrite_kbuild",
            Self::RebuildKbuildIndex => "rebuild_kbuild_index",
            Self::FoldPreprocessor => "fold_preprocessor",
            Self::RebuildCHeaderIndex => "rebuild_c_header_index",
            Self::RewriteIncludes => "rewrite_includes",
            Self::RunSelftests => "run_selftests",
            Self::ClassifyDiagnostics => "classify_diagnostics",
            Self::ApplyFixups => "apply_fixups",
            Self::ReindexAndRepeat => "reindex_and_repeat",
        }
    }

    pub(crate) const fn description(self) -> &'static str {
        match self {
            Self::BuildManifest => "build RemovalManifest",
            Self::BuildInitialIndex => "build initial TreeIndex",
            Self::PruneDeclaredPaths => "prune declared paths",
            Self::RebuildFullIndex => "rebuild full index",
            Self::RewriteKconfig => "rewrite Kconfig",
            Self::RebuildKconfigIndex => "rebuild Kconfig index",
            Self::RewriteKbuild => "rewrite kbuild",
            Self::RebuildKbuildIndex => "rebuild kbuild index",
            Self::FoldPreprocessor => "fold preprocessor branches",
            Self::RebuildCHeaderIndex => "rebuild C/header index",
            Self::RewriteIncludes => "rewrite/report include sites",
            Self::RunSelftests => "run selected builds/tests",
            Self::ClassifyDiagnostics => "classify diagnostics",
            Self::ApplyFixups => "apply deterministic fixers",
            Self::ReindexAndRepeat => "reindex and repeat until stable or pass limit reached",
        }
    }
}

impl std::fmt::Display for ReducerStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct StageFixture {
        stage: ReducerStage,
    }

    #[test]
    fn test_reducer_stage_serialization_is_stable() {
        let cases = [
            (ReducerStage::BuildManifest, "build_manifest"),
            (ReducerStage::BuildInitialIndex, "build_initial_index"),
            (ReducerStage::PruneDeclaredPaths, "prune_declared_paths"),
            (ReducerStage::RebuildFullIndex, "rebuild_full_index"),
            (ReducerStage::RewriteKconfig, "rewrite_kconfig"),
            (ReducerStage::RebuildKconfigIndex, "rebuild_kconfig_index"),
            (ReducerStage::RewriteKbuild, "rewrite_kbuild"),
            (ReducerStage::RebuildKbuildIndex, "rebuild_kbuild_index"),
            (ReducerStage::FoldPreprocessor, "fold_preprocessor"),
            (ReducerStage::RebuildCHeaderIndex, "rebuild_c_header_index"),
            (ReducerStage::RewriteIncludes, "rewrite_includes"),
            (ReducerStage::RunSelftests, "run_selftests"),
            (ReducerStage::ClassifyDiagnostics, "classify_diagnostics"),
            (ReducerStage::ApplyFixups, "apply_fixups"),
            (ReducerStage::ReindexAndRepeat, "reindex_and_repeat"),
        ];
        assert_eq!(
            ReducerStage::ALL
                .into_iter()
                .map(ReducerStage::stable_name)
                .collect::<Vec<_>>(),
            cases.iter().map(|(_, value)| *value).collect::<Vec<_>>()
        );
        assert_eq!(
            ReducerStage::ALL.len(),
            cases.len(),
            "every ReducerStage variant must have a pinned serialization value"
        );

        for (stage, value) in cases {
            assert_eq!(stage.as_str(), value);
            assert_eq!(stage.to_string(), value);
            let encoded = toml::to_string(&StageFixture { stage }).unwrap();
            assert_eq!(encoded, format!("stage = \"{}\"\n", value));

            let decoded: StageFixture =
                toml::from_str(&format!("stage = \"{}\"\n", value)).unwrap();
            assert_eq!(decoded.stage, stage);

            let encoded_json = serde_json::to_string(&stage).unwrap();
            assert_eq!(encoded_json, format!("\"{}\"", value));

            let decoded_json: ReducerStage =
                serde_json::from_str(&format!("\"{}\"", value)).unwrap();
            assert_eq!(decoded_json, stage);
        }
    }

    #[test]
    fn test_reducer_stage_descriptions_match_pipeline_order() {
        assert_eq!(
            ReducerStage::ALL
                .into_iter()
                .map(ReducerStage::description)
                .collect::<Vec<_>>(),
            crate::reducer::pipeline::FIXED_REDUCER_PIPELINE.to_vec()
        );
    }

    #[test]
    fn test_reducer_stage_rejects_legacy_or_display_labels() {
        for legacy_stage in [
            "build RemovalManifest",
            "build initial TreeIndex",
            "prune declared paths",
            "rewrite Kconfig",
            "rewrite kbuild",
            "fold preprocessor branches",
            "rewrite/report include sites",
            "run selected builds/tests",
            "apply deterministic fixers",
            "manifest",
            "index",
            "kconfig",
            "kbuild",
            "cpp",
            "include",
            "selftest",
            "fixup",
        ] {
            let decoded =
                toml::from_str::<StageFixture>(&format!("stage = \"{}\"\n", legacy_stage));
            assert!(
                decoded.is_err(),
                "legacy reducer stage alias must not deserialize: {}",
                legacy_stage
            );
        }
    }
}
