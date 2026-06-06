use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
#[allow(dead_code)]
pub enum GenerateStage {
    #[default]
    #[serde(rename = "resolve")]
    Resolve,
    #[serde(rename = "materialize")]
    Materialize,
    #[serde(rename = "integrate")]
    Integrate,
    #[serde(rename = "prune")]
    Prune,
    #[serde(rename = "reduce")]
    Reduce,
    #[serde(rename = "selftest")]
    Selftest,
    #[serde(rename = "metadata")]
    Metadata,
    #[serde(rename = "commit")]
    Commit,
    #[serde(rename = "publish")]
    Publish,
}

impl GenerateStage {
    #[allow(dead_code)]
    pub(crate) const ALL: [Self; 9] = [
        Self::Resolve,
        Self::Materialize,
        Self::Integrate,
        Self::Prune,
        Self::Reduce,
        Self::Selftest,
        Self::Metadata,
        Self::Commit,
        Self::Publish,
    ];

    pub(crate) fn as_str(self) -> &'static str {
        self.stable_name()
    }

    pub(crate) fn stable_name(self) -> &'static str {
        match self {
            GenerateStage::Resolve => "resolve",
            GenerateStage::Materialize => "materialize",
            GenerateStage::Integrate => "integrate",
            GenerateStage::Prune => "prune",
            GenerateStage::Reduce => "reduce",
            GenerateStage::Selftest => "selftest",
            GenerateStage::Metadata => "metadata",
            GenerateStage::Commit => "commit",
            GenerateStage::Publish => "publish",
        }
    }
}

impl std::fmt::Display for GenerateStage {
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
        stage: GenerateStage,
    }

    #[test]
    fn test_generate_stage_serialization_is_stable() {
        let cases = [
            (GenerateStage::Resolve, "resolve"),
            (GenerateStage::Materialize, "materialize"),
            (GenerateStage::Integrate, "integrate"),
            (GenerateStage::Prune, "prune"),
            (GenerateStage::Reduce, "reduce"),
            (GenerateStage::Selftest, "selftest"),
            (GenerateStage::Metadata, "metadata"),
            (GenerateStage::Commit, "commit"),
            (GenerateStage::Publish, "publish"),
        ];
        assert_eq!(
            GenerateStage::ALL
                .into_iter()
                .map(GenerateStage::stable_name)
                .collect::<Vec<_>>(),
            cases.iter().map(|(_, value)| *value).collect::<Vec<_>>()
        );
        assert_eq!(
            GenerateStage::ALL.len(),
            cases.len(),
            "every GenerateStage variant must have a pinned serialization value"
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

            let decoded_json: GenerateStage =
                serde_json::from_str(&format!("\"{}\"", value)).unwrap();
            assert_eq!(decoded_json, stage);
        }
    }

    #[test]
    fn test_generate_stage_rejects_legacy_aliases() {
        for legacy_stage in [
            "prepare",
            "source",
            "lockfile",
            "patch",
            "integration",
            "reducer",
            "verify",
            "report",
            "output-metadata",
            "output-publish",
            "output-branch",
            "output-git-config",
            "output-commit",
        ] {
            let decoded =
                toml::from_str::<StageFixture>(&format!("stage = \"{}\"\n", legacy_stage));
            assert!(
                decoded.is_err(),
                "legacy generate stage alias must not deserialize: {}",
                legacy_stage
            );
        }
    }
}
