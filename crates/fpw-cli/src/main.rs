use clap::{Parser, Subcommand};
use fpw_core::{
    image::SparseImage, model::parse_named_values, preview_workflow, report::ReportStatus,
    run_workflow, validate_workflow, RunOptions, Workflow,
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
#[command(version)]
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
        #[command(subcommand)]
        command: Option<WebCommand>,
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
    Image {
        #[command(subcommand)]
        command: ImageCommand,
    },
}

#[derive(Debug, Subcommand)]
enum RecentCommand {
    List,
    Add { workflow: PathBuf },
}

#[derive(Debug, Subcommand)]
enum ImageCommand {
    Inspect {
        input: PathBuf,
    },
    ToBin {
        input: PathBuf,
        output: PathBuf,
        #[arg(long, value_parser = parse_u32)]
        address: u32,
        #[arg(long, value_parser = parse_usize)]
        length: usize,
        #[arg(long, default_value = "0xFF", value_parser = parse_u8)]
        fill: u8,
    },
}

#[derive(Debug, Subcommand)]
enum WebCommand {
    Stop,
    Restart {
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        port: Option<u16>,
    },
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
        Command::Web {
            command,
            host,
            port,
        } => match command {
            None => web::serve_web(&host, port)?,
            Some(WebCommand::Stop) => {
                web::stop_web()?;
            }
            Some(WebCommand::Restart {
                host: restart_host,
                port: restart_port,
            }) => {
                let previous = web::stop_web()?;
                let host = restart_host
                    .or_else(|| previous.as_ref().map(|server| server.host.clone()))
                    .unwrap_or(host);
                let port = restart_port
                    .or_else(|| previous.as_ref().map(|server| server.port))
                    .unwrap_or(port);
                web::serve_web(&host, port)?;
            }
        },
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
        Command::Image { command } => match command {
            ImageCommand::Inspect { input } => {
                let image = read_intel_hex(&input)?;
                println!("format: intel-hex");
                println!("data-bytes: {}", image.data_len());
                println!(
                    "address-range: {}",
                    match (image.min_address(), image.max_address()) {
                        (Some(start), Some(end)) => {
                            format!("0x{start:08X}..0x{:08X}", end.saturating_add(1))
                        }
                        _ => "empty".to_string(),
                    }
                );
                println!(
                    "start-address: {}",
                    image
                        .start_address()
                        .map(|address| format!("0x{address:08X}"))
                        .unwrap_or_else(|| "none".to_string())
                );
                for (index, segment) in image.segments().iter().enumerate() {
                    println!(
                        "segment[{index}]: 0x{:08X}..0x{:08X} ({} bytes)",
                        segment.address,
                        segment.address + segment.data.len() as u32,
                        segment.data.len()
                    );
                }
            }
            ImageCommand::ToBin {
                input,
                output,
                address,
                length,
                fill,
            } => {
                let image = read_intel_hex(&input)?;
                let bytes = image.to_binary(address, length, fill)?;
                if let Some(parent) = output
                    .parent()
                    .filter(|parent| !parent.as_os_str().is_empty())
                {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&output, bytes)?;
                println!(
                    "created: {} (address 0x{address:08X}, length 0x{length:X}, fill 0x{fill:02X})",
                    output.display()
                );
            }
        },
    }
    Ok(())
}

fn read_intel_hex(path: &PathBuf) -> fpw_core::Result<SparseImage> {
    let source = fs::read_to_string(path)?;
    SparseImage::from_intel_hex(&source)
}

fn parse_u32(value: &str) -> Result<u32, String> {
    parse_unsigned(value)
        .and_then(|number| u32::try_from(number).map_err(|_| format!("value exceeds u32: {value}")))
}

fn parse_usize(value: &str) -> Result<usize, String> {
    parse_unsigned(value).and_then(|number| {
        usize::try_from(number).map_err(|_| format!("value exceeds usize: {value}"))
    })
}

fn parse_u8(value: &str) -> Result<u8, String> {
    parse_unsigned(value).and_then(|number| {
        u8::try_from(number).map_err(|_| format!("value is not a byte: {value}"))
    })
}

fn parse_unsigned(value: &str) -> Result<u64, String> {
    let trimmed = value.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16).map_err(|_| format!("invalid number: {value}"))
    } else {
        trimmed
            .parse::<u64>()
            .map_err(|_| format!("invalid number: {value}"))
    }
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
