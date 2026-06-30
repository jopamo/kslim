use super::{
    AbiPolicyConfig, ArchPolicyConfig, BaseSection, BuildMatrixConfig, FeatureConfig, GitConfig,
    IntegrationsConfig, KslimConfig, OutputConfig, PerformanceConfig, ProfileConfig,
    ProfileSection, ProjectConfig, PublishConfig, ReducerConfig, ReportConfig, RuntimeMatrixConfig,
    SecurityConfig, SelfTestConfig, UpstreamConfig,
};

pub fn default_kslim_config(name: &str, output_path: &str) -> KslimConfig {
    KslimConfig {
        project: ProjectConfig {
            name: name.to_string(),
        },
        upstream: UpstreamConfig {
            name: "linux".to_string(),
            url: "/path/to/linux/.git".to_string(),
            mode: None,
            cache: None,
        },
        output: OutputConfig::new(output_path),
        git: GitConfig::default(),
        publish: None,
    }
}

pub fn default_profile_config(ref_name: &str) -> ProfileConfig {
    ProfileConfig {
        profile: ProfileSection {
            name: "default".to_string(),
            inherits: None,
            description: "Unmodified upstream Linux emitted by kslim".to_string(),
        },
        base: BaseSection {
            r#ref: ref_name.to_string(),
        },
        slim: None,
        features: FeatureConfig::default(),
        abi: AbiPolicyConfig::default(),
        arch: ArchPolicyConfig::default(),
        build_matrix: BuildMatrixConfig::default(),
        runtime_matrix: RuntimeMatrixConfig::default(),
        reports: ReportConfig::default(),
        security: SecurityConfig::default(),
        performance: PerformanceConfig::default(),
        patches: None,
        integrations: IntegrationsConfig::default(),
        reducer: ReducerConfig::default(),
        selftests: SelfTestConfig::default(),
    }
}

pub fn default_publish_config(remote: &str) -> PublishConfig {
    PublishConfig {
        remote: remote.to_string(),
    }
}

pub fn amdgpu_prune_profile_template(ref_name: &str) -> String {
    format!(
        r#"# Copy this file to `profiles/amdgpu-prune.toml` and adjust for your tree.
#
# Goal:
#   remove AMDGPU, regenerate the tree, and fail generation if compilation breaks.
#
# Suggested loop:
#   1. start with the narrow DRM target
#   2. make `kslim generate` pass
#   3. widen to `vmlinux` and `modules`
#   4. add more ARCH/config variants if needed

[profile]
name = "amdgpu-prune"
description = "Remove AMDGPU and prove compilation still works"
# Profile inheritance is parsed but currently fails validation until resolver
# support lands. Keep inherited intent explicit here for now.
# inherits = "base-profile"

[base]
ref = "{ref_name}"

[slim]
remove_paths = ["drivers/gpu/drm/amd/amdgpu"]
remove_configs = ["DRM_AMDGPU", "DRM_AMDGPU_SI"]
# Optional: force explicit Kconfig defaults in the generated tree.
# Values are raw Kconfig default expressions.
# set_defaults = {{ DRM_AMDGPU_WERROR = "n" }}
#
# Do not set this unless you intentionally want a remove_paths entry of "."
# to target the kernel tree root itself.
# unsafe_allow_root_path_removal = false

# Public header and UAPI surface removal are ABI-sensitive and disabled by default.
# Exact public-header or UAPI removal requires explicit policy:
# Public-header approval does not approve UAPI removal; UAPI uses its own flag.
#
# [abi]
# allow_public_header_removal = false
# allow_uapi_header_removal = false

# Architecture policy constrains which `arch/*` Kconfig trees are treated as
# live for dead-definition solver proofs. It does not replace explicit
# `slim.remove_paths` removals or `[[selftests.kernel_builds]].env.ARCH`
# coverage.
#
# [arch]
# primary_arch = "x86"
# secondary_arches = ["arm64", "riscv"]
# disabled_arches = []
# allow_arch_local_removal = false
# preserve_arch_shared = true

# Future build matrix policy is parsed, recorded as inert fingerprint truth,
# and currently fails validation when nondefault until build matrix planning
# lands. Use `[selftests]` / `[[selftests.kernel_builds]]` for effective build
# coverage today.
#
# [build_matrix]
# enabled = true
# presets = ["default"]
# arches = ["x86"]
# config_targets = ["defconfig"]
# targets = ["vmlinux", "modules"]
# jobs = 16
# fail_on_error = true

# Future runtime matrix policy is parsed but currently fails validation until
# runtime matrix planning lands. Use `[selftests].commands` for explicit
# external runtime smoke commands today.
#
# [runtime_matrix]
# enabled = true
# boot_arches = ["x86"]
# qemu_machines = ["q35"]
# kunit_suites = []
# kselftest_targets = []
# module_smoke = false
# require_clean_dmesg = true
# boot_timeout_seconds = 60
# fail_on_error = true

# Future report policy is parsed but currently fails validation until report planning lands.
# Committed report artifacts and redaction policy are fixed today.
#
# [reports]
# formats = ["text", "markdown", "json"]
# include_edit_records = true
# include_diagnostics = true
# include_source_map = false
# redact_host_paths = true
# include_raw_logs = false
# fail_on_error = true

# Future security policy is parsed but currently fails validation until security planning lands.
# Security trust-boundary checks are fixed and fail-closed today.
#
# [security]
# allow_network = false
# require_local_upstream = true
# reject_host_paths_in_committed_metadata = true
# reject_temp_paths_in_committed_metadata = true
# reject_raw_logs_in_committed_metadata = true
# require_reproducible_timestamps = true
# require_phase_typed_metadata = true
# compatibility_mode = "legacy"
# fail_on_policy_violation = true

# Future performance policy is parsed but currently fails validation until
# performance planning lands. Hot-path work shape is fixed today.
#
# [performance]
# enabled = false
# max_worker_threads = 16
# max_io_threads = 4
# cache_tree_index = false
# incremental_reindex = false
# collect_timing_metrics = false
# profile_hot_paths = false
# fail_on_regression = true

# Optional: bound deterministic build-fixup retries for known reducer fallout.
# Set to 0 to disable retry passes.
#
# [reducer]
# max_fixup_passes = 3
# report_unsupported_expressions = true
# fail_on_unknown_diagnostics = true
# reject_unproven_fixups = true
# reject_unreasoned_edits = true
# reject_speculative_fallout_edits = true
# fail_on_missing_prune_paths = false
# ignore_unsupported_special_removals = false

# Optional: pull the latest committed patch stack from a live git worktree
# before pruning. kslim derives patches from:
#   git merge-base HEAD <base_remote>/<base_ref> .. HEAD
#
# [patches]
# source = "worktree"
# path = "/home/me/projects/kforge/worktrees/linux-gen-btf"
# base_remote = "upstream"
# base_ref = "master"
# require_clean = true
#
# Or pull from multiple worktrees in order:
#
# [patches]
#
# [[patches.sources]]
# source = "worktree"
# path = "/home/me/projects/kforge/worktrees/linux-gen-btf"
# base_remote = "upstream"
# base_ref = "master"
# require_clean = true
#
# [[patches.sources]]
# source = "worktree"
# path = "/home/me/projects/kforge/worktrees/linux-zstd"
# base_remote = "upstream"
# base_ref = "master"
# require_clean = true

[selftests]
check_kconfig_sources = true
check_makefiles = true

[[selftests.kernel_builds]]
name = "x86-defconfig-drm"
config_target = "defconfig"
targets = ["drivers/gpu/drm/"]
jobs = 16
output_dir = ".kslim-selftest/x86-defconfig-drm"

[[selftests.kernel_builds]]
name = "x86-defconfig-full"
config_target = "defconfig"
targets = ["vmlinux", "modules"]
jobs = 16
output_dir = ".kslim-selftest/x86-defconfig-full"

# Example cross-build:
# [[selftests.kernel_builds]]
# name = "arm64-defconfig-drm"
# config_target = "defconfig"
# targets = ["drivers/gpu/drm/"]
# jobs = 16
# output_dir = ".kslim-selftest/arm64-defconfig-drm"
# env = {{ ARCH = "arm64", CROSS_COMPILE = "aarch64-linux-gnu-" }}
"#,
    )
}

pub fn kernel_build_iteration_guide() -> &'static str {
    r#"# Kernel build iteration after code removal

Use kernel build selftests to make `kslim generate` fail if your slimmed tree no longer compiles.

Core loop:

1. remove code and related config symbols together
2. optionally pull the latest committed patch stack from a worktree
3. regenerate the tree
4. run real kernel build targets in selftests
5. allow bounded deterministic fixup retries for known reducer fallout
6. adjust pruning and repeat

Run `kslim validate-config` after editing `kslim.toml` or profile files. It validates the project config and profiles without resolving upstream refs or mutating output.

Run `kslim plan` to resolve the immutable generate plan, base commit, output branch, and reducer inputs without materializing or publishing a candidate tree. Use `kslim plan --frozen-plan PATH` to write that resolved plan to a frozen document. `kslim generate --frozen-plan PATH` and `kslim reduce-tree --frozen-plan PATH` consume the frozen document, verify its schema/tool/base truth, reject new plan overrides, and do not reread mutable config/profile intent or refresh upstream refs.
Profile-aware commands accept `--profile NAME`, default to `default`, and load `profiles/NAME.toml`; path-like profile names are rejected.
Use `--feature NAME` with `plan`, `generate`, `reduce-tree`, or `feature-impact` to restrict named feature intent to one declared `features.remove.NAME` or `features.preserve.NAME` entry. Use `--remove-feature NAME` to require a declared `features.remove.NAME` entry, or `--preserve-feature NAME` to require a declared `features.preserve.NAME` entry. Use `--arch ARCH`, `--primary-arch ARCH`, or `--secondary-arch ARCH` to keep only named feature intent with an empty `arch_scope` or a matching `arch_scope` entry. Use `--safety conservative|normal|aggressive|surgical|unsafe` to override the safety level recorded for active named removal feature intent. Use `--strict` to force strict reducer publication gates on for the resolved profile. Use `--no-strict` to explicitly force those same gates off. Use `generate --dry-run` to resolve and print the same plan without materializing output, writing attempt metadata, or updating `kslim.lock`. Use `generate --deep-dry-run` to materialize and verify the candidate in an isolated temporary tree without publishing output, writing attempt metadata, or updating `kslim.lock`. Use `generate --report-only` to resolve the same plan and write only non-authoritative attempt report metadata. Use `generate --keep-temp` with a materializing generate mode to preserve the private candidate temp tree for debugging without making it authoritative. Use `--max-fixup-passes N` with `plan`, `generate`, or `reduce-tree` to override the profile reducer retry bound for deterministic fixup passes. Use `--matrix default|extended|hardening|runtime` with `plan`, `generate`, `reduce-tree`, `matrix`, or `selftest` to select the effective selftest matrix preset. Use global `--no-network` to reject network-backed upstream or publish endpoints; existing local upstream paths, local publish paths, and local publish file URLs remain allowed. Use global `--offline` to imply `--no-network` and resolve base commits from `kslim.lock` instead of refreshing upstream refs.

Run `kslim feature-impact` to inspect named remove/preserve feature intent and effective path/config impact before generating.

Run `kslim reduce-tree --tree PATH` to apply the reducer directly to an existing kernel checkout. This mutates only that tree; it does not generate, publish, update `.kslim`, or update `kslim.lock`.

Practical order:

1. start narrow: affected subsystem only
2. then add a full build: `vmlinux`, `modules`
3. then add more architectures or config targets

For AMDGPU, begin with:

- `remove_paths = ["drivers/gpu/drm/amd/amdgpu"]`
- `remove_configs = ["DRM_AMDGPU", "DRM_AMDGPU_SI"]`

`remove_configs` entries and `set_defaults` keys must be single Kconfig identifiers, not expressions.
`remove_paths = ["."]` is rejected unless `[slim]` sets `unsafe_allow_root_path_removal = true`.
Derived exported-symbol proof entries are single C identifiers from `EXPORT_SYMBOL*()` providers.
Module-name report entries are basename-only kernel module names without `.ko`; dashes are canonicalized to underscores.
Device-compatible report entries are `vendor,device` tokens with nonempty vendor and device parts.
ACPI ID report entries are uppercase ASCII ACPI hardware IDs.
PCI ID report entries are uppercase hexadecimal `VVVV:DDDD` vendor/device IDs.
USB ID report entries are uppercase hexadecimal `VVVV:PPPP` vendor/product IDs.
Firmware-path report entries are firmware-loader relative paths; absolute paths and `..` are rejected.
Initcall report entries are C identifier initcall entry points.
Runtime-registration report entries use `registration_macro:entry_point` form.
Documentation report entries are relative paths under `Documentation`.
Tool report entries are relative paths under `tools`.
Sample report entries are relative paths under `samples`.
KUnit suite report entries are literal suite names.
kselftest target report entries are literal target names.
Derived header report entries are normalized relative paths ending in `.h`.
UAPI paths are ABI-sensitive relative paths under `include/uapi`, `include/generated/uapi`, or `arch/*/include[/generated]/uapi`.
Derived kbuild object report entries are normalized relative object paths ending in `.o` or directory refs ending in `/`.

ABI-sensitive removals fail closed. Enable only the exact surface you intend:

```toml
[abi]
allow_public_header_removal = false
allow_uapi_header_removal = false
```

Public-header approval does not approve UAPI removal; UAPI uses its own flag.

Architecture policy constrains which `arch/*` Kconfig trees are treated as
live for dead-definition solver proofs. It does not replace explicit
`slim.remove_paths` removals or `[[selftests.kernel_builds]].env.ARCH`
coverage.

```toml
[arch]
primary_arch = "x86"
secondary_arches = ["arm64", "riscv"]
disabled_arches = []
allow_arch_local_removal = false
preserve_arch_shared = true
```

Future build matrix policy is parsed, recorded as inert fingerprint truth, and
currently fails validation when nondefault until build matrix planning lands.
Use `[selftests]` / `[[selftests.kernel_builds]]` for effective build coverage
today.

```toml
[build_matrix]
enabled = true
presets = ["default"]
arches = ["x86"]
config_targets = ["defconfig"]
targets = ["vmlinux", "modules"]
jobs = 16
fail_on_error = true
```

Future runtime matrix policy is parsed but currently fails validation until
runtime matrix planning lands. Use `[selftests].commands` for explicit external
runtime smoke commands today.

```toml
[runtime_matrix]
enabled = true
boot_arches = ["x86"]
qemu_machines = ["q35"]
kunit_suites = []
kselftest_targets = []
module_smoke = false
require_clean_dmesg = true
boot_timeout_seconds = 60
fail_on_error = true
```

Future report policy is parsed but currently fails validation until report planning lands.
Committed report artifacts and redaction policy are fixed today.

```toml
[reports]
formats = ["text", "markdown", "json"]
include_edit_records = true
include_diagnostics = true
include_source_map = false
redact_host_paths = true
include_raw_logs = false
fail_on_error = true
```

The loader preserves source-map provenance for explicit values, built-in
defaults, and CLI base-ref overrides. CLI base-ref overrides are trimmed before
resolution and empty overrides fail. Report-only debug output includes that
source map as non-authoritative attempt metadata. Frozen generate plans store
source maps with temporary roots, workspace roots, and host absolute paths
replaced by stable tokens. Plan fingerprints include the selected profile,
normalized CLI overrides, resolved base commit, resolved feature graph
fingerprint, resolved ABI policy, resolved build matrix policy, kslim tool
version, and a host-path-normalized source-map serialization. The serialization
does not use raw temporary paths as digest input. The serialization is a line-oriented
`key=escaped-value` format with pinned `format` and `version` fields, explicit
escaping for backslash, newline, carriage return, and tab. Map-like fingerprint
inputs are serialized in sorted key order. Set-like array inputs are normalized
before fingerprint serialization; order-sensitive build, argument, and command
sequences remain ordered. Enum-like fingerprint values use explicit stable
tokens, not Rust variant or debug names.

Future security policy is parsed but currently fails validation until security planning lands.
Security trust-boundary checks are fixed and fail-closed today.

```toml
[security]
allow_network = false
require_local_upstream = true
reject_host_paths_in_committed_metadata = true
reject_temp_paths_in_committed_metadata = true
reject_raw_logs_in_committed_metadata = true
require_reproducible_timestamps = true
require_phase_typed_metadata = true
compatibility_mode = "legacy"
fail_on_policy_violation = true
```

Future performance policy is parsed but currently fails validation until
performance planning lands. Hot-path work shape is fixed today.

```toml
[performance]
enabled = false
max_worker_threads = 16
max_io_threads = 4
cache_tree_index = false
incremental_reindex = false
collect_timing_metrics = false
profile_hot_paths = false
fail_on_regression = true
```

Named feature removals and preservations can declare roots, Kconfig symbols,
exported symbols, module names, module aliases, devicetree compatibles, ACPI
IDs, PCI IDs, USB IDs, firmware paths, initcalls, runtime registrations, docs, tools, samples, KUnit suites, and kselftest targets.
Named feature removals can also declare exact `remove_paths`, `remove_configs`,
`remove_exported_symbols`, `remove_module_names`, `remove_module_aliases`,
`remove_device_compatibles`, `remove_acpi_ids`, `remove_pci_ids`, and
`remove_usb_ids`, `remove_firmware_paths`, `remove_initcalls`, and
`remove_runtime_registrations`, `remove_docs`, `remove_tools`, `remove_samples`, `remove_kunit_suites`, and `remove_kselftest_targets` when a feature root, symbol,
module, alias, compatible, ACPI ID, PCI ID, USB ID, firmware path, initcall,
runtime registration, documentation set, tool set, sample set, KUnit suite, or kselftest target would be too broad. Preserved roots are kept out of broad
candidate pruning, and preserved Kconfig symbols cannot also be declared for
removal. Per-feature safety levels are normalized
(`conservative`, `normal`, `aggressive`, `surgical`, or `unsafe`), default to
`normal`, and are included in the resolved plan fingerprint; reducer mutation
gates still come from `[reducer]` until detailed safety-level behavior lands.
Per-feature arch scopes are validated, default to unscoped, and are included
in the resolved plan fingerprint; reducer mutation gates still do not filter
by architecture until arch-policy planning lands. Per-feature clean-boot
test requirements are validated, default to off, and are included in the
resolved plan fingerprint; execution still comes from the selected `[selftests]`
until per-feature test execution planning lands. Per-feature report-only modes
are validated, default to off, and are included in the resolved plan
fingerprint; generation still follows whole-run CLI `--report-only` until
per-feature report execution planning lands. Per-feature UAPI/module
preservation policy still fails validation. Feature `remove_paths` and
`remove_configs` use the same fail-closed validation as direct `[slim]`
input. Feature `exported_symbols` and `remove_exported_symbols` are typed C
identifiers and resolve to exported-symbol facts; consumer proof remains in
the exported-symbol graph/proof pass. Feature `module_names` and
`remove_module_names` are basename-only kernel module names without `.ko`; they
resolve to module-name facts, with dashes canonicalized to underscores, before
module-alias metadata or module proof. Feature `module_aliases` and
`remove_module_aliases` are literal kernel module alias patterns without
whitespace; they resolve to module-alias facts before module-alias
extraction/proof. Feature `device_compatibles` and
`remove_device_compatibles` are literal devicetree compatible strings in
`vendor,device` form; they resolve to devicetree-compatible facts before
device-binding proof. Feature `acpi_ids` and `remove_acpi_ids` are literal
ACPI hardware IDs using 4 to 16 uppercase ASCII letters and digits; they
resolve to ACPI ID facts before ACPI device-table proof. Feature `pci_ids` and
`remove_pci_ids` are literal PCI vendor/device IDs in uppercase hexadecimal
`VVVV:DDDD` form; they resolve to PCI ID facts before PCI device-table proof. Feature `usb_ids` and
`remove_usb_ids` are literal USB vendor/product IDs in uppercase hexadecimal
`VVVV:PPPP` form; they resolve to USB ID facts before USB device-table proof. Feature `firmware_paths` and
`remove_firmware_paths` are firmware-loader relative paths; they resolve to
firmware-path facts before firmware-loader proof. Feature `initcalls` and
`remove_initcalls` are C identifier initcall entry points; they resolve to
initcall facts before initcall macro proof. Feature `runtime_registrations`
and `remove_runtime_registrations` use `registration_macro:entry_point` form
and resolve to runtime-registration facts before no-live-entry-point proof.
Feature `docs` and `remove_docs` are relative paths under `Documentation`;
they resolve to documentation facts before doc index or link-check proof. Feature
`tools` and `remove_tools` are relative paths under `tools`; they resolve to
tool facts before tool build or test proof. Feature `samples` and
`remove_samples` are relative paths under `samples`; they resolve to sample
facts before sample build or runtime proof. Feature `kunit_suites` and
`remove_kunit_suites` are literal KUnit suite names; they resolve to KUnit
suite facts before KUnit execution proof. Feature `kselftest_targets` and
`remove_kselftest_targets` are literal kselftest target names; they resolve to
kselftest target facts before kselftest execution proof. Exact public-header
or UAPI removal still needs explicit approval, either in `[abi]` or on the
named feature removal.

```toml
[features.remove.bluetooth]
kind = "subsystem"
roots = ["net/bluetooth", "drivers/bluetooth"]
remove_paths = ["drivers/bluetooth/btusb.c"]
configs = ["BT"]
remove_configs = ["BT_HCIBTUSB"]
exported_symbols = ["bt_sock_register"]
remove_exported_symbols = ["bt_sock_unregister"]
module_names = ["btusb"]
remove_module_names = ["bt_debug"]
module_aliases = ["usb:v*p*d*dc*dsc*dp*ic*isc*ip*in*"]
remove_module_aliases = ["pci:v00008086d00001572sv*sd*bc*sc*i*"]
device_compatibles = ["qcom,ipq8064"]
remove_device_compatibles = ["vendor,removed-device"]
acpi_ids = ["PNP0C09"]
remove_acpi_ids = ["ACPI0003"]
pci_ids = ["8086:1572"]
remove_pci_ids = ["10EC:8168"]
usb_ids = ["0BDA:8153"]
remove_usb_ids = ["046D:C52B"]
firmware_paths = ["amdgpu/polaris10_mc.bin"]
remove_firmware_paths = ["iwlwifi-7260-17.ucode"]
initcalls = ["bt_init"]
remove_initcalls = ["btusb_driver_init"]
runtime_registrations = ["module_init:bt_init"]
remove_runtime_registrations = ["module_platform_driver:btusb_driver"]
docs = ["Documentation/networking/bluetooth.rst"]
remove_docs = ["Documentation/driver-api/btusb.rst"]
tools = ["tools/perf"]
remove_tools = ["tools/objtool"]
samples = ["samples/bpf"]
remove_samples = ["samples/hidraw"]
kunit_suites = ["bt_test"]
remove_kunit_suites = ["btusb-test"]
kselftest_targets = ["net"]
remove_kselftest_targets = ["bpf"]
# safety = "surgical" # conservative|normal|aggressive|surgical|unsafe
# arch_scope = ["x86"]
# require_clean_boot = true
# report_only = true
# If this feature explicitly removes ABI-facing headers:
# allow_public_header_removal = true
# allow_uapi_header_removal = true

[features.preserve.netfilter]
kind = "subsystem"
roots = ["net/netfilter"]
configs = ["NETFILTER"]
exported_symbols = ["nf_register_net_hook"]
module_names = ["nf_conntrack"]
module_aliases = ["of:N*T*Cqcom,ipq8064"]
device_compatibles = ["brcm,bcm2835-aux-uart"]
acpi_ids = ["PRP0001"]
pci_ids = ["1AF4:1000"]
usb_ids = ["1D6B:0002"]
firmware_paths = ["qcom/venus-5.2/venus.mbn"]
initcalls = ["nf_conntrack_standalone_init"]
runtime_registrations = ["module_init:nf_conntrack_standalone_init"]
docs = ["Documentation/networking/nf_conntrack-sysctl.rst"]
tools = ["tools/testing/selftests/netfilter"]
samples = ["samples/kobject"]
kunit_suites = ["nf_conntrack_test"]
kselftest_targets = ["drivers/net"]

# Not supported yet: per-feature UAPI/module preservation policy
```

If you need to keep code present but force different Kconfig defaults, add:

- `set_defaults = { DRM_AMDGPU_WERROR = "n" }`

Worktree-backed patches:

```toml
[patches]
source = "worktree"
path = "/home/me/projects/kforge/worktrees/linux-gen-btf"
base_remote = "upstream"
base_ref = "master"
require_clean = true
```

Or, for multiple worktrees applied in order:

```toml
[patches]

[[patches.sources]]
source = "worktree"
path = "/home/me/projects/kforge/worktrees/linux-gen-btf"
base_remote = "upstream"
base_ref = "master"
require_clean = true

[[patches.sources]]
source = "worktree"
path = "/home/me/projects/kforge/worktrees/linux-zstd"
base_remote = "upstream"
base_ref = "master"
require_clean = true
```

Each source tells `kslim` to derive the current patch stack from:

- `git merge-base HEAD upstream/master`
- all commits from that merge-base to `HEAD`

When multiple sources are listed, `kslim` applies them sequentially in the order written.

Kernel build selftest shape:

```toml
[[selftests.kernel_builds]]
name = "x86-defconfig-drm"
config_target = "defconfig"
targets = ["drivers/gpu/drm/"]
jobs = 16
output_dir = ".kslim-selftest/x86-defconfig-drm"
```

Run:

```sh
kslim generate
```

If the selected build fails, generation fails after any bounded deterministic
fixup retries are exhausted.

If `[publish] remote = "..."` is configured, run `kslim publish` after a successful generate to push the committed published branch and tag. Publish uses `kslim.lock` plus committed output metadata; it does not re-resolve upstream refs or reread profile intent.

Run `kslim report` to print the latest published `report.txt`. If no published report exists yet, it prints the non-authoritative attempt report and labels it as such.

Run `kslim status` to inspect project config, output repo state, authoritative published snapshot truth, and the latest non-authoritative attempt metadata.

Run `kslim repair` to clear stale non-authoritative `.kslim/attempt` metadata. It leaves `kslim.lock` and the output repo unchanged.

Run `kslim explain-edit PATH:LINE` to show the edit record, pass, reason, proof source, and related reports for a changed line.

Run `kslim explain-symbol CONFIG_FOO` to show the resolved Kconfig symbol decision, owner/proof source, matching edits, and related reports.

Use `kslim --explain PATH_OR_SYMBOL` as a shorthand. `PATH:LINE` routes to edit explanation, path-like values show all edits for that path, and Kconfig symbols route to symbol explanation.

Run `kslim explain-feature feature_name` to show the named feature decision, profile owner, proof source, explicit intent, and resolved impact.

Run `kslim explain-abi` to show profile ABI/UAPI policy, approval sources, fail-closed behavior, and ABI-sensitive removal decisions.

Run `kslim matrix` to show the selected selftest/build matrix and the parsed future build/runtime matrix policy for the profile.

Run `kslim selftest --tree PATH` to execute the selected profile selftests against an existing kernel checkout without generating or publishing output.

Run `kslim fuzz-fixtures` to write a deterministic parser/scanner seed corpus under `fuzz-fixtures/` for local reducer fuzzing and regression work.

Template:

- `profiles/amdgpu-prune.toml.example`
"#
}
