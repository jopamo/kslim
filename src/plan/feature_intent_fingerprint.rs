use crate::state::{FeatureIntentPlan, ResolvedCandidateState};
use super::{append_fingerprint_line, bool_string};

pub(super) fn append_feature_intent_fingerprint_lines(
    out: &mut String,
    resolved: &ResolvedCandidateState,
) {
    append_fingerprint_line(
        out,
        "resolved.feature_graph_fingerprint",
        resolved.feature_graph_fingerprint.as_str(),
    );
    append_fingerprint_line(
        out,
        "resolved.removal_manifest_fingerprint",
        resolved.removal_manifest_fingerprint.as_str(),
    );
    append_fingerprint_line(
        out,
        "resolved.abi_policy_fingerprint",
        resolved.abi_policy_fingerprint.as_str(),
    );
    append_fingerprint_line(
        out,
        "resolved.arch_policy_fingerprint",
        resolved.arch_policy_fingerprint.as_str(),
    );
    append_feature_intent_plan_fingerprint_lines(out, &resolved.feature_intent_plan);
}

fn append_feature_intent_plan_fingerprint_lines(out: &mut String, plan: &FeatureIntentPlan) {
    append_fingerprint_line(
        out,
        "resolved.feature_intent_plan.intent_count",
        &plan.intents.len().to_string(),
    );
    for (idx, intent) in plan.intents.iter().enumerate() {
        let prefix = format!("resolved.feature_intent_plan.intents.{idx}");
        append_fingerprint_line(out, &format!("{prefix}.stable_id"), &intent.stable_id);
        append_fingerprint_line(out, &format!("{prefix}.action"), &intent.action);
        append_fingerprint_line(out, &format!("{prefix}.name"), &intent.name);
        append_fingerprint_line(
            out,
            &format!("{prefix}.kind"),
            intent.kind.as_deref().unwrap_or("<none>"),
        );
        for path in &intent.roots {
            append_fingerprint_line(
                out,
                &format!("{prefix}.roots"),
                &path.as_path().to_string_lossy(),
            );
        }
        for path in &intent.remove_paths {
            append_fingerprint_line(
                out,
                &format!("{prefix}.remove_paths"),
                &path.as_path().to_string_lossy(),
            );
        }
        for config in &intent.configs {
            append_fingerprint_line(out, &format!("{prefix}.configs"), config.as_str());
        }
        for config in &intent.remove_configs {
            append_fingerprint_line(out, &format!("{prefix}.remove_configs"), config.as_str());
        }
        for symbol in &intent.exported_symbols {
            append_fingerprint_line(out, &format!("{prefix}.exported_symbols"), symbol.as_str());
        }
        for symbol in &intent.remove_exported_symbols {
            append_fingerprint_line(
                out,
                &format!("{prefix}.remove_exported_symbols"),
                symbol.as_str(),
            );
        }
        for module in &intent.module_names {
            append_fingerprint_line(out, &format!("{prefix}.module_names"), module.as_str());
        }
        for module in &intent.remove_module_names {
            append_fingerprint_line(
                out,
                &format!("{prefix}.remove_module_names"),
                module.as_str(),
            );
        }
        for alias in &intent.module_aliases {
            append_fingerprint_line(out, &format!("{prefix}.module_aliases"), alias.as_str());
        }
        for alias in &intent.remove_module_aliases {
            append_fingerprint_line(
                out,
                &format!("{prefix}.remove_module_aliases"),
                alias.as_str(),
            );
        }
        for compatible in &intent.device_compatibles {
            append_fingerprint_line(
                out,
                &format!("{prefix}.device_compatibles"),
                compatible.as_str(),
            );
        }
        for compatible in &intent.remove_device_compatibles {
            append_fingerprint_line(
                out,
                &format!("{prefix}.remove_device_compatibles"),
                compatible.as_str(),
            );
        }
        for id in &intent.acpi_ids {
            append_fingerprint_line(out, &format!("{prefix}.acpi_ids"), id.as_str());
        }
        for id in &intent.remove_acpi_ids {
            append_fingerprint_line(out, &format!("{prefix}.remove_acpi_ids"), id.as_str());
        }
        for id in &intent.pci_ids {
            append_fingerprint_line(out, &format!("{prefix}.pci_ids"), id.as_str());
        }
        for id in &intent.remove_pci_ids {
            append_fingerprint_line(out, &format!("{prefix}.remove_pci_ids"), id.as_str());
        }
        for id in &intent.usb_ids {
            append_fingerprint_line(out, &format!("{prefix}.usb_ids"), id.as_str());
        }
        for id in &intent.remove_usb_ids {
            append_fingerprint_line(out, &format!("{prefix}.remove_usb_ids"), id.as_str());
        }
        for path in &intent.firmware_paths {
            append_fingerprint_line(out, &format!("{prefix}.firmware_paths"), path.as_str());
        }
        for path in &intent.remove_firmware_paths {
            append_fingerprint_line(
                out,
                &format!("{prefix}.remove_firmware_paths"),
                path.as_str(),
            );
        }
        for initcall in &intent.initcalls {
            append_fingerprint_line(out, &format!("{prefix}.initcalls"), initcall.as_str());
        }
        for initcall in &intent.remove_initcalls {
            append_fingerprint_line(
                out,
                &format!("{prefix}.remove_initcalls"),
                initcall.as_str(),
            );
        }
        for surface in &intent.runtime_registrations {
            append_fingerprint_line(
                out,
                &format!("{prefix}.runtime_registrations"),
                surface.as_str(),
            );
        }
        for surface in &intent.remove_runtime_registrations {
            append_fingerprint_line(
                out,
                &format!("{prefix}.remove_runtime_registrations"),
                surface.as_str(),
            );
        }
        for path in &intent.docs {
            append_fingerprint_line(out, &format!("{prefix}.docs"), path.as_str());
        }
        for path in &intent.remove_docs {
            append_fingerprint_line(out, &format!("{prefix}.remove_docs"), path.as_str());
        }
        for path in &intent.tools {
            append_fingerprint_line(out, &format!("{prefix}.tools"), path.as_str());
        }
        for path in &intent.remove_tools {
            append_fingerprint_line(out, &format!("{prefix}.remove_tools"), path.as_str());
        }
        for path in &intent.samples {
            append_fingerprint_line(out, &format!("{prefix}.samples"), path.as_str());
        }
        for path in &intent.remove_samples {
            append_fingerprint_line(out, &format!("{prefix}.remove_samples"), path.as_str());
        }
        for suite in &intent.kunit_suites {
            append_fingerprint_line(out, &format!("{prefix}.kunit_suites"), suite.as_str());
        }
        for suite in &intent.remove_kunit_suites {
            append_fingerprint_line(
                out,
                &format!("{prefix}.remove_kunit_suites"),
                suite.as_str(),
            );
        }
        for target in &intent.kselftest_targets {
            append_fingerprint_line(out, &format!("{prefix}.kselftest_targets"), target.as_str());
        }
        for target in &intent.remove_kselftest_targets {
            append_fingerprint_line(
                out,
                &format!("{prefix}.remove_kselftest_targets"),
                target.as_str(),
            );
        }
        append_fingerprint_line(
            out,
            &format!("{prefix}.allow_public_header_removal"),
            bool_string(intent.allow_public_header_removal),
        );
        append_fingerprint_line(
            out,
            &format!("{prefix}.allow_uapi_header_removal"),
            bool_string(intent.allow_uapi_header_removal),
        );
        for arch in &intent.arch_scope {
            append_fingerprint_line(out, &format!("{prefix}.arch_scope"), arch.as_str());
        }
        append_fingerprint_line(
            out,
            &format!("{prefix}.safety"),
            intent
                .safety
                .map(|safety| safety.as_str())
                .unwrap_or("<none>"),
        );
        append_fingerprint_line(
            out,
            &format!("{prefix}.preserve_uapi"),
            bool_string(intent.preserve_uapi),
        );
        append_fingerprint_line(
            out,
            &format!("{prefix}.preserve_module_aliases"),
            bool_string(intent.preserve_module_aliases),
        );
        append_fingerprint_line(
            out,
            &format!("{prefix}.require_clean_boot"),
            bool_string(intent.require_clean_boot),
        );
        append_fingerprint_line(
            out,
            &format!("{prefix}.report_only"),
            bool_string(intent.report_only),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{self, FeatureIntentConfig};
    use crate::state::{
        CliOverrides, FeatureIntentEntryPlan, FeatureIntentPlan, ProfileName,
        RequestedGenerateState, ResolvedCandidateState,
    };
    use crate::generate::GenerateOptions;
    use crate::lockfile::ResolvedBase;
    use crate::paths::RequestedConfigPath;

    fn default_generate_options() -> GenerateOptions {
        GenerateOptions {
            dry_run: false,
            deep_dry_run: false,
            report_only: false,
            keep_temp: false,
            max_fixup_passes: None,
            matrix: None,
            offline: false,
            frozen_plan: None,
            force: false,
            base_ref: None,
            feature: None,
            remove_feature: None,
            preserve_feature: None,
            arch: None,
            primary_arch: None,
            secondary_arch: None,
            safety: None,
            strict: false,
            no_strict: false,
            run_selftests: true,
        }
    }

    fn requested_state() -> RequestedGenerateState {
        let opts = default_generate_options();
        RequestedGenerateState::new(
            RequestedConfigPath::new("/tmp/project/kslim.toml").unwrap(),
            ProfileName::new("default").unwrap(),
            CliOverrides::from_options(&opts),
        )
    }

    fn resolved_base() -> ResolvedBase {
        ResolvedBase {
            upstream: String::from("linux"),
            url: String::from("/tmp/linux.git"),
            r#ref: String::from("v1.0"),
            commit: String::from("deadbeef"),
            resolved_at: String::from("2026-01-01T00:00:00Z"),
        }
    }

    #[test]
    fn feature_intent_fingerprint_records_stable_list() {
        let entry = FeatureIntentEntryPlan::from_config(
            "remove",
            "bluetooth",
            &FeatureIntentConfig {
                kind: Some(String::from("subsystem")),
                roots: vec![String::from("net/bluetooth")],
                configs: vec![String::from("BT")],
                exported_symbols: vec![String::from("bt_sock_register")],
                module_names: vec![String::from("btusb")],
                module_aliases: vec![String::from("usb:v*p*d*dc*dsc*dp*ic*isc*ip*in*")],
                device_compatibles: vec![String::from("qcom,ipq8064")],
                acpi_ids: vec![String::from("PNP0C09")],
                pci_ids: vec![String::from("8086:1572")],
                usb_ids: vec![String::from("0BDA:8153")],
                firmware_paths: vec![String::from("amdgpu/polaris10_mc.bin")],
                initcalls: vec![String::from("bt_init")],
                runtime_registrations: vec![String::from("module_init:bt_init")],
                docs: vec![String::from("Documentation/networking/bluetooth.rst")],
                tools: vec![String::from("tools/perf")],
                samples: vec![String::from("samples/bpf")],
                kunit_suites: vec![String::from("bt_test")],
                kselftest_targets: vec![String::from("net")],
                arch_scope: vec![String::from("x86")],
                safety: Some(config::FeatureSafetyLevel::Surgical),
                require_clean_boot: true,
                report_only: true,
                ..FeatureIntentConfig::default()
            },
        )
        .unwrap();
        let plan = FeatureIntentPlan {
            intents: vec![entry],
        };
        let mut out = String::new();

        append_feature_intent_plan_fingerprint_lines(&mut out, &plan);

        assert!(out.contains("resolved.feature_intent_plan.intent_count=1"));
        assert!(out.contains("resolved.feature_intent_plan.intents.0.stable_id=feature-intent-"));
        assert!(out.contains("resolved.feature_intent_plan.intents.0.action=remove"));
        assert!(out.contains("resolved.feature_intent_plan.intents.0.name=bluetooth"));
        assert!(out.contains("resolved.feature_intent_plan.intents.0.roots=net/bluetooth"));
        assert!(out.contains("resolved.feature_intent_plan.intents.0.configs=BT"));
        assert!(out
            .contains("resolved.feature_intent_plan.intents.0.exported_symbols=bt_sock_register"));
        assert!(out.contains("resolved.feature_intent_plan.intents.0.module_names=btusb"));
        assert!(out.contains(
            "resolved.feature_intent_plan.intents.0.module_aliases=usb:v*p*d*dc*dsc*dp*ic*isc*ip*in*"
        ));
        assert!(
            out.contains("resolved.feature_intent_plan.intents.0.device_compatibles=qcom,ipq8064")
        );
        assert!(out.contains("resolved.feature_intent_plan.intents.0.acpi_ids=PNP0C09"));
        assert!(out.contains("resolved.feature_intent_plan.intents.0.pci_ids=8086:1572"));
        assert!(out.contains("resolved.feature_intent_plan.intents.0.usb_ids=0BDA:8153"));
        assert!(out.contains(
            "resolved.feature_intent_plan.intents.0.firmware_paths=amdgpu/polaris10_mc.bin"
        ));
        assert!(out.contains("resolved.feature_intent_plan.intents.0.initcalls=bt_init"));
        assert!(out.contains(
            "resolved.feature_intent_plan.intents.0.runtime_registrations=module_init:bt_init"
        ));
        assert!(out.contains(
            "resolved.feature_intent_plan.intents.0.docs=Documentation/networking/bluetooth.rst"
        ));
        assert!(out.contains("resolved.feature_intent_plan.intents.0.tools=tools/perf"));
        assert!(out.contains("resolved.feature_intent_plan.intents.0.samples=samples/bpf"));
        assert!(out.contains("resolved.feature_intent_plan.intents.0.kunit_suites=bt_test"));
        assert!(out.contains("resolved.feature_intent_plan.intents.0.kselftest_targets=net"));
        assert!(out.contains("resolved.feature_intent_plan.intents.0.arch_scope=x86"));
        assert!(out.contains("resolved.feature_intent_plan.intents.0.safety=surgical"));
    }

    #[test]
    fn generate_plan_fingerprint_includes_feature_intent_list() {
        let config = config::default_kslim_config("demo", "/tmp/output");
        let mut profile = config::default_profile_config("v1.0");
        profile.features.preserve.insert(
            String::from("netfilter"),
            FeatureIntentConfig {
                kind: Some(String::from("subsystem")),
                roots: vec![String::from("net/netfilter")],
                configs: vec![String::from("NETFILTER")],
                ..FeatureIntentConfig::default()
            },
        );
        let resolved = ResolvedCandidateState::from_resolved_inputs(
            &config,
            &profile,
            resolved_base(),
            None,
            "unmodified-upstream",
            "kslim/v1.0/default",
        )
        .unwrap();

        let plan = super::super::GeneratePlan::new(requested_state(), resolved).unwrap();
        let serialization = plan.fingerprint.stable_serialization();

        assert!(serialization.contains("resolved.feature_intent_plan.intent_count=1"));
        assert!(serialization.contains("resolved.feature_graph_fingerprint=feature-graph-"));
        assert!(serialization.contains("resolved.removal_manifest_fingerprint=removal-manifest-"));
        assert!(serialization.contains("resolved.abi_policy_fingerprint=abi-policy-"));
        assert!(serialization.contains("resolved.arch_policy_fingerprint=arch-policy-"));
        assert!(serialization.contains("resolved.feature_intent_plan.intents.0.name=netfilter"));
        assert!(serialization.contains("resolved.feature_intent_plan.intents.0.action=preserve"));
        assert!(
            serialization.contains("resolved.feature_intent_plan.intents.0.roots=net/netfilter")
        );
        assert!(serialization.contains("resolved.feature_intent_plan.intents.0.configs=NETFILTER"));
    }
}
