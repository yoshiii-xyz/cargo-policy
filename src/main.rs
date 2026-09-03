use std::{path::PathBuf, process::ExitCode};

use cargo_policy::{
    CheckReport, CompiledPolicy, ExplainReport, check, compile_policy, explain, render_check_text,
    render_compiled_text, render_explain_text, render_json,
};
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "cargo-policy",
    version,
    about = "Check Cargo dependency-source policy"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Check {
        #[arg(long)]
        policy: PathBuf,
        #[arg(long)]
        manifest_path: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    Explain {
        package: String,
        #[arg(long, default_value = "policy.toml")]
        policy: PathBuf,
        #[arg(long)]
        manifest_path: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    Compile {
        #[arg(long, default_value = "policy.toml")]
        policy: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Check {
            policy,
            manifest_path,
            format,
        } => match check(&policy, manifest_path.as_deref()) {
            Ok(report) => finish_check(&report, format),
            Err(error) => finish_error(error),
        },
        Command::Explain {
            package,
            policy,
            manifest_path,
            format,
        } => match explain(&policy, &package, manifest_path.as_deref()) {
            Ok(report) => finish_explain(&report, format),
            Err(error) => finish_error(error),
        },
        Command::Compile { policy, format } => match compile_policy(&policy) {
            Ok(policy) => finish_compile(&policy, format),
            Err(error) => finish_error(error),
        },
    }
}

fn finish_check(report: &CheckReport, format: OutputFormat) -> ExitCode {
    match print_value(report, format, render_check_text) {
        Ok(()) => ExitCode::from(report.exit_code() as u8),
        Err(error) => finish_render_error(error),
    }
}

fn finish_explain(report: &ExplainReport, format: OutputFormat) -> ExitCode {
    match print_value(report, format, render_explain_text) {
        Ok(()) => ExitCode::from(report.exit_code() as u8),
        Err(error) => finish_render_error(error),
    }
}

fn finish_compile(policy: &CompiledPolicy, format: OutputFormat) -> ExitCode {
    match print_value(policy, format, render_compiled_text) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => finish_render_error(error),
    }
}

fn print_value<T: serde::Serialize>(
    value: &T,
    format: OutputFormat,
    render_text: impl FnOnce(&T) -> String,
) -> serde_json::Result<()> {
    match format {
        OutputFormat::Text => {
            print!("{}", render_text(value));
            Ok(())
        }
        OutputFormat::Json => {
            println!("{}", render_json(value)?);
            Ok(())
        }
    }
}

fn finish_error(error: cargo_policy::PolicyError) -> ExitCode {
    eprintln!("cargo-policy: {error}");
    ExitCode::from(3)
}

fn finish_render_error(error: serde_json::Error) -> ExitCode {
    eprintln!("cargo-policy: could not render report: {error}");
    ExitCode::from(3)
}
