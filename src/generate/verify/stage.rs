use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
pub(crate) enum VerificationStage {
    #[default]
    #[serde(rename = "ensure_candidate_observable")]
    EnsureCandidateObservable,
    #[serde(rename = "reject_temporary_metadata_paths")]
    RejectTemporaryMetadataPaths,
    #[serde(rename = "reject_host_metadata_paths")]
    RejectHostMetadataPaths,
    #[serde(rename = "reject_raw_metadata_logs")]
    RejectRawMetadataLogs,
    #[serde(rename = "verify_reproducible_metadata_timestamps")]
    VerifyReproducibleMetadataTimestamps,
    #[serde(rename = "read_candidate_metadata")]
    ReadCandidateMetadata,
    #[serde(rename = "fingerprint_candidate_tree")]
    FingerprintCandidateTree,
    #[serde(rename = "verify_candidate_metadata")]
    VerifyCandidateMetadata,
    #[serde(rename = "verify_reducer_success")]
    VerifyReducerSuccess,
    #[serde(rename = "verify_candidate_reports")]
    VerifyCandidateReports,
    #[serde(rename = "verify_report_paths")]
    VerifyReportPaths,
    #[serde(rename = "verify_reasoned_edits")]
    VerifyReasonedEdits,
    #[serde(rename = "verify_no_speculative_fallout")]
    VerifyNoSpeculativeFallout,
    #[serde(rename = "verify_no_unknown_diagnostics")]
    VerifyNoUnknownDiagnostics,
    #[serde(rename = "verify_no_unsupported_syntax")]
    VerifyNoUnsupportedSyntax,
    #[serde(rename = "verify_selftest_policy")]
    VerifySelftestPolicy,
    #[serde(rename = "fingerprint_candidate_metadata")]
    FingerprintCandidateMetadata,
}

impl VerificationStage {
    #[allow(dead_code)]
    pub(crate) const ALL: [Self; 17] = [
        Self::EnsureCandidateObservable,
        Self::RejectTemporaryMetadataPaths,
        Self::RejectHostMetadataPaths,
        Self::RejectRawMetadataLogs,
        Self::VerifyReproducibleMetadataTimestamps,
        Self::ReadCandidateMetadata,
        Self::FingerprintCandidateTree,
        Self::VerifyCandidateMetadata,
        Self::VerifyReducerSuccess,
        Self::VerifyCandidateReports,
        Self::VerifyReportPaths,
        Self::VerifyReasonedEdits,
        Self::VerifyNoSpeculativeFallout,
        Self::VerifyNoUnknownDiagnostics,
        Self::VerifyNoUnsupportedSyntax,
        Self::VerifySelftestPolicy,
        Self::FingerprintCandidateMetadata,
    ];

    pub(crate) const fn as_str(self) -> &'static str {
        self.stable_name()
    }

    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::EnsureCandidateObservable => "ensure_candidate_observable",
            Self::RejectTemporaryMetadataPaths => "reject_temporary_metadata_paths",
            Self::RejectHostMetadataPaths => "reject_host_metadata_paths",
            Self::RejectRawMetadataLogs => "reject_raw_metadata_logs",
            Self::VerifyReproducibleMetadataTimestamps => "verify_reproducible_metadata_timestamps",
            Self::ReadCandidateMetadata => "read_candidate_metadata",
            Self::FingerprintCandidateTree => "fingerprint_candidate_tree",
            Self::VerifyCandidateMetadata => "verify_candidate_metadata",
            Self::VerifyReducerSuccess => "verify_reducer_success",
            Self::VerifyCandidateReports => "verify_candidate_reports",
            Self::VerifyReportPaths => "verify_report_paths",
            Self::VerifyReasonedEdits => "verify_reasoned_edits",
            Self::VerifyNoSpeculativeFallout => "verify_no_speculative_fallout",
            Self::VerifyNoUnknownDiagnostics => "verify_no_unknown_diagnostics",
            Self::VerifyNoUnsupportedSyntax => "verify_no_unsupported_syntax",
            Self::VerifySelftestPolicy => "verify_selftest_policy",
            Self::FingerprintCandidateMetadata => "fingerprint_candidate_metadata",
        }
    }

    pub(crate) const fn description(self) -> &'static str {
        match self {
            Self::EnsureCandidateObservable => "ensure candidate tree and metadata are observable",
            Self::RejectTemporaryMetadataPaths => {
                "reject temporary paths in committed candidate metadata"
            }
            Self::RejectHostMetadataPaths => {
                "reject host absolute paths in committed candidate metadata"
            }
            Self::RejectRawMetadataLogs => "reject raw logs in committed candidate metadata",
            Self::VerifyReproducibleMetadataTimestamps => {
                "verify committed candidate metadata timestamps are reproducible"
            }
            Self::ReadCandidateMetadata => "read candidate metadata summary",
            Self::FingerprintCandidateTree => "fingerprint candidate tree",
            Self::VerifyCandidateMetadata => {
                "verify candidate metadata matches plan and tree fingerprint"
            }
            Self::VerifyReducerSuccess => "verify reducer success policy",
            Self::VerifyCandidateReports => "verify candidate report presence",
            Self::VerifyReportPaths => "verify report paths are relative and normalized",
            Self::VerifyReasonedEdits => "verify reducer edit provenance",
            Self::VerifyNoSpeculativeFallout => "verify no broad speculative fallout edits",
            Self::VerifyNoUnknownDiagnostics => "verify unknown diagnostic policy",
            Self::VerifyNoUnsupportedSyntax => "verify unsupported syntax policy",
            Self::VerifySelftestPolicy => "verify selftest policy",
            Self::FingerprintCandidateMetadata => "fingerprint candidate metadata",
        }
    }
}

impl std::fmt::Display for VerificationStage {
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
        stage: VerificationStage,
    }

    #[test]
    fn test_verification_stage_serialization_is_stable() {
        let cases = [
            (
                VerificationStage::EnsureCandidateObservable,
                "ensure_candidate_observable",
            ),
            (
                VerificationStage::RejectTemporaryMetadataPaths,
                "reject_temporary_metadata_paths",
            ),
            (
                VerificationStage::RejectHostMetadataPaths,
                "reject_host_metadata_paths",
            ),
            (
                VerificationStage::RejectRawMetadataLogs,
                "reject_raw_metadata_logs",
            ),
            (
                VerificationStage::VerifyReproducibleMetadataTimestamps,
                "verify_reproducible_metadata_timestamps",
            ),
            (
                VerificationStage::ReadCandidateMetadata,
                "read_candidate_metadata",
            ),
            (
                VerificationStage::FingerprintCandidateTree,
                "fingerprint_candidate_tree",
            ),
            (
                VerificationStage::VerifyCandidateMetadata,
                "verify_candidate_metadata",
            ),
            (
                VerificationStage::VerifyReducerSuccess,
                "verify_reducer_success",
            ),
            (
                VerificationStage::VerifyCandidateReports,
                "verify_candidate_reports",
            ),
            (VerificationStage::VerifyReportPaths, "verify_report_paths"),
            (
                VerificationStage::VerifyReasonedEdits,
                "verify_reasoned_edits",
            ),
            (
                VerificationStage::VerifyNoSpeculativeFallout,
                "verify_no_speculative_fallout",
            ),
            (
                VerificationStage::VerifyNoUnknownDiagnostics,
                "verify_no_unknown_diagnostics",
            ),
            (
                VerificationStage::VerifyNoUnsupportedSyntax,
                "verify_no_unsupported_syntax",
            ),
            (
                VerificationStage::VerifySelftestPolicy,
                "verify_selftest_policy",
            ),
            (
                VerificationStage::FingerprintCandidateMetadata,
                "fingerprint_candidate_metadata",
            ),
        ];
        assert_eq!(
            VerificationStage::ALL
                .into_iter()
                .map(VerificationStage::stable_name)
                .collect::<Vec<_>>(),
            cases.iter().map(|(_, value)| *value).collect::<Vec<_>>()
        );
        assert_eq!(
            VerificationStage::ALL.len(),
            cases.len(),
            "every VerificationStage variant must have a pinned serialization value"
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

            let decoded_json: VerificationStage =
                serde_json::from_str(&format!("\"{}\"", value)).unwrap();
            assert_eq!(decoded_json, stage);
        }
    }

    #[test]
    fn test_verification_stage_descriptions_match_pipeline_order() {
        assert_eq!(
            VerificationStage::ALL
                .into_iter()
                .map(VerificationStage::description)
                .collect::<Vec<_>>(),
            crate::generate::verify::FIXED_VERIFICATION_PIPELINE.to_vec()
        );
    }

    #[test]
    fn test_verification_stage_rejects_legacy_or_display_labels() {
        for legacy_stage in [
            "ensure candidate tree and metadata are observable",
            "reject temporary paths in committed candidate metadata",
            "read candidate metadata summary",
            "verify reducer success policy",
            "verify candidate report presence",
            "verify selftest policy",
            "candidate",
            "metadata",
            "tree",
            "reducer",
            "report",
            "diagnostics",
            "syntax",
            "selftest",
        ] {
            let decoded =
                toml::from_str::<StageFixture>(&format!("stage = \"{}\"\n", legacy_stage));
            assert!(
                decoded.is_err(),
                "legacy verification stage alias must not deserialize: {}",
                legacy_stage
            );
        }
    }
}
