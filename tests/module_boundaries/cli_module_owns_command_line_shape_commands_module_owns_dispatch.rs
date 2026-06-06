use super::common::*;

#[test]
fn cli_module_owns_command_line_shape_commands_module_owns_dispatch() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let main = production_source(&root.join("src/main.rs"));
    let cli_mod = production_source(&root.join("src/cli/mod.rs"));
    let cli_entrypoint = production_source(&root.join("src/cli/entrypoint.rs"));
    let cli_command = production_source(&root.join("src/cli/command.rs"));
    let cli_args = production_source(&root.join("src/cli/args.rs"));
    let cli = cli_sources(root);
    let commands = commands_source(root);

    assert!(
        main.contains("mod commands;") && main.contains("cli::main();"),
        "main.rs should stay thin while registering command dispatch"
    );
    let unexpected_main_lines = main
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| {
            !is_private_module_declaration(line)
                && !matches!(*line, "fn main() {" | "cli::main();" | "}")
        })
        .collect::<Vec<_>>();
    assert!(
        unexpected_main_lines.is_empty(),
        "main.rs should be CLI startup only: private module declarations plus fn main() delegating to cli::main(); unexpected lines: {unexpected_main_lines:#?}"
    );

    assert!(
        !root.join("src/cli.rs").exists(),
        "CLI ownership should live in src/cli/mod.rs and owned submodules, not legacy src/cli.rs"
    );
    assert!(
        !root.join("src/commands.rs").exists() && root.join("src/commands/mod.rs").exists(),
        "command dispatch ownership should live in src/commands/mod.rs, not legacy src/commands.rs"
    );
    for required in [
        "mod args;",
        "mod command;",
        "mod entrypoint;",
        "pub(crate) use args::{",
        "pub(crate) use command::{BaseCommands, Cli, Commands, UpstreamCommands};",
        "pub(crate) use entrypoint::main;",
    ] {
        assert!(
            cli_mod.contains(required),
            "src/cli/mod.rs should remain the CLI facade and register/re-export {required}"
        );
    }
    assert!(
        cli_entrypoint.contains("Cli::parse()")
            && cli_entrypoint.contains("crate::commands::run(cli)")
            && cli_entrypoint.contains("env_logger::Builder::from_env"),
        "src/cli/entrypoint.rs should own parser startup and delegate parsed CLI to command dispatch"
    );
    assert!(
        cli_command.contains("#[derive(Parser)]")
            && cli_command.contains("pub struct Cli")
            && cli_command.contains("#[derive(Subcommand)]")
            && cli_command.contains("pub enum Commands")
            && cli_command.contains("pub enum UpstreamCommands")
            && cli_command.contains("pub enum BaseCommands"),
        "src/cli/command.rs should own top-level parser and subcommand enum shape"
    );

    for required in [
        "pub struct InitArgs",
        "pub struct ValidateConfigArgs",
        "pub struct PlanArgs",
        "pub struct FeatureImpactArgs",
        "pub struct ReduceTreeArgs",
        "pub struct ResolveArgs",
        "pub struct GenerateArgs",
        "pub struct PublishArgs",
        "pub struct ReportArgs",
        "pub struct CompareArgs",
        "pub struct ExplainEditArgs",
        "pub struct ExplainSymbolArgs",
        "pub struct ExplainFeatureArgs",
        "pub struct ExplainAbiArgs",
        "pub struct MatrixArgs",
        "pub struct SelftestArgs",
        "pub struct FuzzFixturesArgs",
    ] {
        assert!(
            cli_args.contains(required),
            "src/cli/args.rs should own command-line argument shape item {required}"
        );
    }

    for forbidden_business_or_dispatch in [
        "crate::config",
        "crate::generate",
        "crate::publish",
        "crate::upstream",
        "crate::output_repo",
        "crate::manifest",
        "fn cmd_",
        "pub fn run(",
        "std::fs::write",
    ] {
        assert!(
            !cli.contains(forbidden_business_or_dispatch),
            "src/cli/* must not own command orchestration or business logic; found {forbidden_business_or_dispatch}"
        );
    }
    let crate_references = crate_references(&cli);
    assert_eq!(
        crate_references,
        vec!["crate::commands::run"],
        "src/cli/* should reference only command dispatch, not execution subsystems"
    );

    for required in [
        "pub(crate) fn run(cli: Cli)",
        "Commands::ValidateConfig(args) => cmd_validate_config(args)",
        "Commands::Plan(args) => cmd_plan(args)",
        "Commands::FeatureImpact(args) => cmd_feature_impact(args)",
        "Commands::ReduceTree(args) => cmd_reduce_tree(args)",
        "Commands::Generate(args) => cmd_generate(args)",
        "Commands::Publish(args) => cmd_publish(args)",
        "Commands::Report(args) => cmd_report(args)",
        "Commands::Status => cmd_status()",
        "Commands::Repair => cmd_repair()",
        "Commands::ExplainEdit(args) => cmd_explain_edit(args)",
        "Commands::ExplainSymbol(args) => cmd_explain_symbol(args)",
        "Commands::ExplainFeature(args) => cmd_explain_feature(args)",
        "Commands::ExplainAbi(args) => cmd_explain_abi(args)",
        "Commands::Matrix(args) => cmd_matrix(args)",
        "Commands::Selftest(args) => cmd_selftest(args)",
        "Commands::FuzzFixtures(args) => cmd_fuzz_fixtures(args)",
        "fn cmd_init(args: InitArgs)",
        "fn cmd_validate_config(args: ValidateConfigArgs)",
        "fn cmd_plan(args: PlanArgs)",
        "fn cmd_feature_impact(args: FeatureImpactArgs)",
        "fn cmd_reduce_tree(args: ReduceTreeArgs)",
        "fn cmd_generate(args: GenerateArgs)",
        "fn cmd_publish(args: PublishArgs)",
        "fn cmd_report(args: ReportArgs)",
        "fn cmd_status()",
        "fn cmd_repair()",
        "fn cmd_explain_edit(args: ExplainEditArgs)",
        "fn cmd_explain_symbol(args: ExplainSymbolArgs)",
        "fn cmd_explain_feature(args: ExplainFeatureArgs)",
        "fn cmd_explain_abi(args: ExplainAbiArgs)",
        "fn cmd_matrix(args: MatrixArgs)",
        "fn cmd_selftest(args: SelftestArgs)",
        "fn cmd_fuzz_fixtures(args: FuzzFixturesArgs)",
        "fn cmd_compare(args: CompareArgs)",
        "generate::generate_with_source_maps(&config, &profile, &opts, Some(source_maps))",
        "publish::publish(&request, &opts)",
    ] {
        assert!(
            commands.contains(required),
            "src/commands/* should own command dispatch/orchestration item {required}"
        );
    }

    for forbidden_clap_shape in [
        "use clap",
        "Cli::parse",
        "env_logger",
        "#[derive(Parser)]",
        "#[derive(Subcommand)]",
        "#[derive(clap::Args)]",
        "#[arg(",
    ] {
        assert!(
            !commands.contains(forbidden_clap_shape),
            "src/commands/* should consume parsed CLI models instead of defining clap shape; found {forbidden_clap_shape}"
        );
    }
}

fn crate_references(source: &str) -> Vec<String> {
    let mut references = Vec::new();
    let mut offset = 0usize;
    while let Some(relative) = source[offset..].find("crate::") {
        let start = offset + relative;
        let rest = &source[start..];
        let end = rest
            .find(|ch: char| !(ch == ':' || ch == '_' || ch.is_ascii_alphanumeric()))
            .unwrap_or(rest.len());
        references.push(rest[..end].to_string());
        offset = start + end;
    }
    references.sort();
    references.dedup();
    references
}

fn is_private_module_declaration(line: &str) -> bool {
    let Some(module) = line
        .strip_prefix("mod ")
        .and_then(|line| line.strip_suffix(';'))
    else {
        return false;
    };
    !module.is_empty()
        && module
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}
