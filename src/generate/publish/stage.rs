use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
pub(crate) enum PublishStage {
    #[default]
    #[serde(rename = "check_output_plan")]
    CheckOutputPlan,
    #[serde(rename = "check_output_safety")]
    CheckOutputSafety,
    #[serde(rename = "reverify_candidate")]
    ReverifyCandidate,
    #[serde(rename = "stage_output_candidate")]
    StageOutputCandidate,
    #[serde(rename = "reverify_staged_candidate")]
    ReverifyStagedCandidate,
    #[serde(rename = "build_published_metadata")]
    BuildPublishedMetadata,
    #[serde(rename = "capture_output_rollback")]
    CaptureOutputRollback,
    #[serde(rename = "create_output_branch")]
    CreateOutputBranch,
    #[serde(rename = "sync_output_tree")]
    SyncOutputTree,
    #[serde(rename = "sync_output_metadata")]
    SyncOutputMetadata,
    #[serde(rename = "write_published_metadata")]
    WritePublishedMetadata,
    #[serde(rename = "commit_output")]
    CommitOutput,
    #[serde(rename = "build_published_snapshot")]
    BuildPublishedSnapshot,
    #[serde(rename = "update_authoritative_lockfile")]
    UpdateAuthoritativeLockfile,
}

impl PublishStage {
    #[allow(dead_code)]
    pub(crate) const ALL: [Self; 14] = [
        Self::CheckOutputPlan,
        Self::CheckOutputSafety,
        Self::ReverifyCandidate,
        Self::StageOutputCandidate,
        Self::ReverifyStagedCandidate,
        Self::BuildPublishedMetadata,
        Self::CaptureOutputRollback,
        Self::CreateOutputBranch,
        Self::SyncOutputTree,
        Self::SyncOutputMetadata,
        Self::WritePublishedMetadata,
        Self::CommitOutput,
        Self::BuildPublishedSnapshot,
        Self::UpdateAuthoritativeLockfile,
    ];

    pub(crate) const fn as_str(self) -> &'static str {
        self.stable_name()
    }

    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::CheckOutputPlan => "check_output_plan",
            Self::CheckOutputSafety => "check_output_safety",
            Self::ReverifyCandidate => "reverify_candidate",
            Self::StageOutputCandidate => "stage_output_candidate",
            Self::ReverifyStagedCandidate => "reverify_staged_candidate",
            Self::BuildPublishedMetadata => "build_published_metadata",
            Self::CaptureOutputRollback => "capture_output_rollback",
            Self::CreateOutputBranch => "create_output_branch",
            Self::SyncOutputTree => "sync_output_tree",
            Self::SyncOutputMetadata => "sync_output_metadata",
            Self::WritePublishedMetadata => "write_published_metadata",
            Self::CommitOutput => "commit_output",
            Self::BuildPublishedSnapshot => "build_published_snapshot",
            Self::UpdateAuthoritativeLockfile => "update_authoritative_lockfile",
        }
    }

    pub(crate) const fn description(self) -> &'static str {
        match self {
            Self::CheckOutputPlan => "check output repo matches resolved plan",
            Self::CheckOutputSafety => "check output repo safety",
            Self::ReverifyCandidate => "reverify candidate proof",
            Self::StageOutputCandidate => "stage output candidate",
            Self::ReverifyStagedCandidate => "reverify staged candidate proof",
            Self::BuildPublishedMetadata => "build published metadata",
            Self::CaptureOutputRollback => "capture output rollback state",
            Self::CreateOutputBranch => "create output branch",
            Self::SyncOutputTree => "sync output tree",
            Self::SyncOutputMetadata => "sync output metadata",
            Self::WritePublishedMetadata => "write published metadata",
            Self::CommitOutput => "commit output repository",
            Self::BuildPublishedSnapshot => "build published snapshot proof",
            Self::UpdateAuthoritativeLockfile => "update authoritative lockfile last",
        }
    }
}

impl std::fmt::Display for PublishStage {
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
        stage: PublishStage,
    }

    #[test]
    fn test_publish_stage_serialization_is_stable() {
        let cases = [
            (PublishStage::CheckOutputPlan, "check_output_plan"),
            (PublishStage::CheckOutputSafety, "check_output_safety"),
            (PublishStage::ReverifyCandidate, "reverify_candidate"),
            (PublishStage::StageOutputCandidate, "stage_output_candidate"),
            (
                PublishStage::ReverifyStagedCandidate,
                "reverify_staged_candidate",
            ),
            (
                PublishStage::BuildPublishedMetadata,
                "build_published_metadata",
            ),
            (
                PublishStage::CaptureOutputRollback,
                "capture_output_rollback",
            ),
            (PublishStage::CreateOutputBranch, "create_output_branch"),
            (PublishStage::SyncOutputTree, "sync_output_tree"),
            (PublishStage::SyncOutputMetadata, "sync_output_metadata"),
            (
                PublishStage::WritePublishedMetadata,
                "write_published_metadata",
            ),
            (PublishStage::CommitOutput, "commit_output"),
            (
                PublishStage::BuildPublishedSnapshot,
                "build_published_snapshot",
            ),
            (
                PublishStage::UpdateAuthoritativeLockfile,
                "update_authoritative_lockfile",
            ),
        ];
        assert_eq!(
            PublishStage::ALL
                .into_iter()
                .map(PublishStage::stable_name)
                .collect::<Vec<_>>(),
            cases.iter().map(|(_, value)| *value).collect::<Vec<_>>()
        );
        assert_eq!(
            PublishStage::ALL.len(),
            cases.len(),
            "every PublishStage variant must have a pinned serialization value"
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

            let decoded_json: PublishStage =
                serde_json::from_str(&format!("\"{}\"", value)).unwrap();
            assert_eq!(decoded_json, stage);
        }
    }

    #[test]
    fn test_publish_stage_descriptions_match_pipeline_order() {
        assert_eq!(
            PublishStage::ALL
                .into_iter()
                .map(PublishStage::description)
                .collect::<Vec<_>>(),
            crate::generate::publish::FIXED_PUBLISH_PIPELINE.to_vec()
        );
    }

    #[test]
    fn test_publish_stage_rejects_legacy_or_display_labels() {
        for legacy_stage in [
            "check output repo matches resolved plan",
            "check output repo safety",
            "reverify candidate proof",
            "stage output candidate",
            "write published metadata",
            "commit output repository",
            "update authoritative lockfile last",
            "output",
            "publish",
            "candidate",
            "metadata",
            "branch",
            "commit",
            "lockfile",
            "tag",
        ] {
            let decoded =
                toml::from_str::<StageFixture>(&format!("stage = \"{}\"\n", legacy_stage));
            assert!(
                decoded.is_err(),
                "legacy publish stage alias must not deserialize: {}",
                legacy_stage
            );
        }
    }
}
