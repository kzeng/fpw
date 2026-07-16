use clap::{Parser, Subcommand};
use fpw_core::{
    model::parse_named_values, preview_workflow, report::ReportStatus, run_workflow,
    validate_workflow, RunOptions, Workflow,
};
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

mod web;
mod workflow_store;

#[derive(Debug, Parser)]
#[command(name = "fpw")]
#[command(about = "Firmware Packaging Workflow")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Validate {
        workflow: PathBuf,
    },
    Preview {
        workflow: PathBuf,
    },
    Run {
        workflow: PathBuf,
        #[arg(long = "input")]
        inputs: Vec<String>,
        #[arg(long = "output")]
        outputs: Vec<String>,
        #[arg(long)]
        report_dir: Option<PathBuf>,
    },
    Config {
        #[arg(long)]
        output: Option<PathBuf>,
    },
    Web {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 4769)]
        port: u16,
    },
    ImportFfc {
        source: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Recent {
        #[command(subcommand)]
        command: RecentCommand,
    },
}

#[derive(Debug, Subcommand)]
enum RecentCommand {
    List,
    Add { workflow: PathBuf },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> fpw_core::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Validate { workflow } => {
            let workflow_model = Workflow::from_path(&workflow)?;
            validate_workflow(&workflow_model)?;
            println!("valid: {}", workflow.display());
        }
        Command::Preview { workflow } => {
            let workflow_model = Workflow::from_path(&workflow)?;
            for line in preview_workflow(&workflow_model)? {
                println!("{line}");
            }
        }
        Command::Run {
            workflow,
            inputs,
            outputs,
            report_dir,
        } => {
            let workflow_model = Workflow::from_path(&workflow)?;
            let options = RunOptions {
                inputs: parse_named_values(&inputs)?,
                outputs: parse_named_values(&outputs)?,
                report_dir,
                command: std::env::args().collect(),
            };
            let report = run_workflow(&workflow, &workflow_model, &options)?;
            if let Err(error) = fpw_core::recent::touch_recent_project(
                None,
                &workflow,
                &workflow_model.name,
                report.started_at_unix_ms,
            ) {
                eprintln!("warning: failed to update recent projects: {error}");
            }
            let dir = options
                .report_dir
                .clone()
                .unwrap_or_else(|| PathBuf::from("fpw-reports"));
            let stem = format!("{}-{}", workflow_model.name, report.started_at_unix_ms);
            let paths = report.write_all(&dir, &stem)?;
            println!("status: {}", report.status.as_str());
            for path in paths {
                println!("report: {}", path.display());
            }
            if report.status == ReportStatus::Failed {
                return Err(fpw_core::FpwError::Message(
                    "workflow execution failed; see report for details".to_string(),
                ));
            }
        }
        Command::Config { output } => {
            write_config(output)?;
        }
        Command::Web { host, port } => {
            web::serve_web(&host, port)?;
        }
        Command::ImportFfc { source, output } => {
            let result = fpw_core::ffc::import_ffc(&source)?;
            if let Some(parent) = output
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
            {
                fs::create_dir_all(parent)?;
            }
            fs::write(&output, serde_json::to_string_pretty(&result.workflow)?)?;
            println!("created: {}", output.display());
            for warning in result.warnings {
                println!("warning: {}", warning.message);
            }
        }
        Command::Recent { command } => match command {
            RecentCommand::List => {
                let recent = fpw_core::recent::load_recent_projects(None)?;
                println!("{}", serde_json::to_string_pretty(&recent)?);
            }
            RecentCommand::Add { workflow } => {
                let workflow_model = Workflow::from_path(&workflow)?;
                let recent = fpw_core::recent::touch_recent_project(
                    None,
                    &workflow,
                    &workflow_model.name,
                    fpw_core::report::unix_ms_now(),
                )?;
                println!("{}", serde_json::to_string_pretty(&recent)?);
            }
        },
    }
    Ok(())
}

fn write_config(output: Option<PathBuf>) -> fpw_core::Result<()> {
    let output = match output {
        Some(path) => path,
        None => {
            print!("Output .fwp path [workflow.fwp]: ");
            io::stdout().flush()?;
            let mut text = String::new();
            io::stdin().read_line(&mut text)?;
            let trimmed = text.trim();
            if trimmed.is_empty() {
                PathBuf::from("workflow.fwp")
            } else {
                PathBuf::from(trimmed)
            }
        }
    };

    let sample = serde_json::json!({
        "schemaVersion": 1,
        "name": "workflow",
        "description": "Generated by fpw config",
        "steps": [
            {
                "id": "firmware",
                "kind": "input",
                "name": "firmware",
                "path": "input.bin"
            },
            {
                "id": "write_image",
                "kind": "output",
                "input": "firmware",
                "name": "image",
                "path": "out/image.bin"
            }
        ]
    });

    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, serde_json::to_string_pretty(&sample)?)?;
    println!("created: {}", output.display());
    Ok(())
}
