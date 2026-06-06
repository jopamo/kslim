//! Command-line parser shape and CLI process entrypoint.
//!
//! This module owns clap-facing structs, subcommands, argument attributes, and
//! the binary CLI startup path. Command execution and business logic stay in
//! the command dispatch module.

mod args;
mod command;
mod entrypoint;

pub(crate) use args::{
    CompareArgs, ExplainAbiArgs, ExplainEditArgs, ExplainFeatureArgs, ExplainSymbolArgs,
    FeatureImpactArgs, FuzzFixturesArgs, GenerateArgs, InitArgs, MatrixArgs, PlanArgs,
    PublishArgs, ReduceTreeArgs, ReportArgs, ResolveArgs, SelftestArgs, ValidateConfigArgs,
};
pub(crate) use command::{BaseCommands, Cli, Commands, UpstreamCommands};
pub(crate) use entrypoint::main;
