use clap::{Parser, Subcommand};
use fpw_core::{
    model::parse_named_values, preview_workflow, report::ReportStatus, run_workflow,
    validate_workflow, RunOptions, Workflow,
};
use std::{
    fs,
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
};

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
            serve_web(&host, port)?;
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

fn serve_web(host: &str, port: u16) -> fpw_core::Result<()> {
    let address = format!("{host}:{port}");
    let listener = TcpListener::bind(&address)?;
    println!("FPW WebUI listening at http://{address}");
    println!("Press Ctrl+C to stop.");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = respond(stream) {
                    eprintln!("web request failed: {error}");
                }
            }
            Err(error) => eprintln!("web connection failed: {error}"),
        }
    }
    Ok(())
}

fn respond(mut stream: TcpStream) -> fpw_core::Result<()> {
    let mut buffer = [0_u8; 1024];
    let size = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..size]);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");

    if path == "/api/health" {
        let body = r#"{"status":"ok","service":"fpw-web"}"#;
        write_http_response(&mut stream, "application/json; charset=utf-8", body)?;
        return Ok(());
    }

    if path == "/api/recent-projects" {
        let recent = fpw_core::recent::load_recent_projects(None)?;
        let body = serde_json::to_string_pretty(&recent)?;
        write_http_response(&mut stream, "application/json; charset=utf-8", &body)?;
        return Ok(());
    }

    if path.starts_with("/api/") {
        write_http_status(
            &mut stream,
            404,
            "Not Found",
            "application/json; charset=utf-8",
            r#"{"error":"not found"}"#,
        )?;
        return Ok(());
    }

    if let Some((content_type, bytes)) = static_asset(path)? {
        write_http_bytes(&mut stream, 200, "OK", content_type, &bytes)?;
        return Ok(());
    }

    let body = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>FPW</title>
    <style>
      body { margin: 0; font-family: system-ui, sans-serif; background: #f3f6f8; color: #17202a; }
      main { max-width: 880px; margin: 72px auto; padding: 0 24px; }
      h1 { font-size: 32px; margin: 0 0 8px; }
      p { color: #5f7080; line-height: 1.6; }
      code { background: #e7eef3; padding: 2px 6px; border-radius: 4px; }
    </style>
  </head>
  <body>
    <main>
      <h1>FPW</h1>
      <p>Firmware Packaging Workflow local WebUI is running.</p>
      <p>The React editor scaffold lives in <code>web/</code>. The next step is wiring this server to static assets and core APIs.</p>
    </main>
  </body>
</html>"#;
    write_http_response(&mut stream, "text/html; charset=utf-8", body)?;
    Ok(())
}

fn write_http_response(
    stream: &mut TcpStream,
    content_type: &str,
    body: &str,
) -> fpw_core::Result<()> {
    write_http_status(stream, 200, "OK", content_type, body)
}

fn write_http_status(
    stream: &mut TcpStream,
    status_code: u16,
    status_text: &str,
    content_type: &str,
    body: &str,
) -> fpw_core::Result<()> {
    let response = format!(
        "HTTP/1.1 {status_code} {status_text}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes())?;
    Ok(())
}

fn write_http_bytes(
    stream: &mut TcpStream,
    status_code: u16,
    status_text: &str,
    content_type: &str,
    body: &[u8],
) -> fpw_core::Result<()> {
    let header = format!(
        "HTTP/1.1 {status_code} {status_text}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    Ok(())
}

fn static_asset(request_path: &str) -> fpw_core::Result<Option<(&'static str, Vec<u8>)>> {
    let dist = Path::new("web").join("dist");
    if !dist.is_dir() {
        return Ok(None);
    }
    let relative = request_path
        .trim_start_matches('/')
        .split('?')
        .next()
        .unwrap_or("");
    if relative.contains("..") {
        return Ok(None);
    }
    let path = if relative.is_empty() {
        dist.join("index.html")
    } else {
        dist.join(relative)
    };
    let path = if path.is_file() {
        path
    } else {
        dist.join("index.html")
    };
    if !path.is_file() {
        return Ok(None);
    }
    let content_type = content_type_for_path(&path);
    Ok(Some((content_type, fs::read(path)?)))
}

fn content_type_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("wasm") => "application/wasm",
        _ => "application/octet-stream",
    }
}
