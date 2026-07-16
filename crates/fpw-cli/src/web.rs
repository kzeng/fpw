use crate::workflow_store::WorkflowStore;
use fpw_core::{preview_workflow, run_workflow_source, validate_workflow, RunOptions, Workflow};
use serde::Deserialize;
use serde_json::json;
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    time::Duration,
};

const MAX_REQUEST_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    body: Vec<u8>,
}

#[derive(Debug)]
struct HttpResponse {
    status_code: u16,
    status_text: &'static str,
    content_type: &'static str,
    body: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkflowRequest {
    workflow: Workflow,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunWorkflowRequest {
    workflow: Workflow,
    workflow_path: String,
    #[serde(default)]
    inputs: BTreeMap<String, String>,
    #[serde(default)]
    outputs: BTreeMap<String, String>,
    #[serde(default)]
    report_dir: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WorkflowPathRequest {
    path: String,
}

#[derive(Debug, Deserialize)]
struct ManagedWorkflowRequest {
    path: String,
    workflow: Workflow,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DuplicateWorkflowRequest {
    source_path: String,
    target_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportWorkflowRequest {
    source_path: String,
    target_path: String,
}

pub fn serve_web(host: &str, port: u16) -> fpw_core::Result<()> {
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
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let request = match read_request(&mut stream) {
        Ok(request) => request,
        Err(error) => {
            write_response(
                &mut stream,
                &json_error(400, "Bad Request", &error.to_string()),
            )?;
            return Ok(());
        }
    };

    if let Some(response) = api_response(&request.method, &request.path, &request.body) {
        write_response(&mut stream, &response)?;
        return Ok(());
    }

    if request.method != "GET" {
        write_response(
            &mut stream,
            &json_error(405, "Method Not Allowed", "method not allowed"),
        )?;
        return Ok(());
    }

    if let Some((content_type, bytes)) = static_asset(&request.path)? {
        write_response(
            &mut stream,
            &HttpResponse {
                status_code: 200,
                status_text: "OK",
                content_type,
                body: bytes,
            },
        )?;
        return Ok(());
    }

    write_response(
        &mut stream,
        &HttpResponse {
            status_code: 200,
            status_text: "OK",
            content_type: "text/html; charset=utf-8",
            body: fallback_page().as_bytes().to_vec(),
        },
    )?;
    Ok(())
}

fn api_response(method: &str, path: &str, body: &[u8]) -> Option<HttpResponse> {
    match (method, path) {
        ("GET", "/api/health") => Some(json_response(json!({
            "status": "ok",
            "service": "fpw-web"
        }))),
        ("GET", "/api/recent-projects") => {
            Some(match fpw_core::recent::load_recent_projects(None) {
                Ok(recent) => json_response(json!(recent)),
                Err(error) => json_error(500, "Internal Server Error", &error.to_string()),
            })
        }
        ("GET", "/api/workflows") => Some(list_workflows_api()),
        ("POST", "/api/workflows/open") => Some(open_workflow_api(body)),
        ("POST", "/api/workflows/create") => Some(create_workflow_api(body)),
        ("PUT", "/api/workflows/save") => Some(save_workflow_api(body)),
        ("POST", "/api/workflows/duplicate") => Some(duplicate_workflow_api(body)),
        ("POST", "/api/workflows/archive") => Some(archive_workflow_api(body)),
        ("POST", "/api/workflows/import/fwp") => Some(import_fwp_api(body)),
        ("POST", "/api/workflows/import/ffc") => Some(import_ffc_api(body)),
        ("POST", "/api/workflows/validate") => Some(validate_api(body)),
        ("POST", "/api/workflows/preview") => Some(preview_api(body)),
        ("POST", "/api/workflows/run") => Some(run_api(body)),
        (_, path) if path.starts_with("/api/") => {
            Some(if matches!(method, "GET" | "POST" | "PUT") {
                json_error(404, "Not Found", "API route not found")
            } else {
                json_error(405, "Method Not Allowed", "method not allowed")
            })
        }
        _ => None,
    }
}

fn list_workflows_api() -> HttpResponse {
    let store = WorkflowStore::default();
    match store.list() {
        Ok(workflows) => json_response(json!({
            "root": store.root().to_string_lossy(),
            "workflows": workflows
        })),
        Err(error) => json_error(500, "Internal Server Error", &error.to_string()),
    }
}

fn open_workflow_api(body: &[u8]) -> HttpResponse {
    let request: WorkflowPathRequest = match parse_json(body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let store = WorkflowStore::default();
    match store.open(&request.path) {
        Ok(workflow) => json_response(json!({
            "path": request.path,
            "absolutePath": store.root().join(&request.path).to_string_lossy(),
            "workflow": workflow
        })),
        Err(error) => json_error(404, "Not Found", &error.to_string()),
    }
}

fn create_workflow_api(body: &[u8]) -> HttpResponse {
    let request: ManagedWorkflowRequest = match parse_json(body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let store = WorkflowStore::default();
    match store.create(&request.path, &request.workflow) {
        Ok(summary) => json_response(json!({
            "summary": summary,
            "absolutePath": store.root().join(&request.path).to_string_lossy()
        })),
        Err(error) => json_error(422, "Unprocessable Entity", &error.to_string()),
    }
}

fn save_workflow_api(body: &[u8]) -> HttpResponse {
    let request: ManagedWorkflowRequest = match parse_json(body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    match WorkflowStore::default().save(&request.path, &request.workflow) {
        Ok(summary) => json_response(json!({ "summary": summary })),
        Err(error) => json_error(422, "Unprocessable Entity", &error.to_string()),
    }
}

fn duplicate_workflow_api(body: &[u8]) -> HttpResponse {
    let request: DuplicateWorkflowRequest = match parse_json(body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    match WorkflowStore::default().duplicate(&request.source_path, &request.target_path) {
        Ok(summary) => json_response(json!({ "summary": summary })),
        Err(error) => json_error(422, "Unprocessable Entity", &error.to_string()),
    }
}

fn archive_workflow_api(body: &[u8]) -> HttpResponse {
    let request: WorkflowPathRequest = match parse_json(body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    match WorkflowStore::default().archive(&request.path, fpw_core::report::unix_ms_now()) {
        Ok(archived_path) => json_response(json!({ "archivedPath": archived_path })),
        Err(error) => json_error(422, "Unprocessable Entity", &error.to_string()),
    }
}

fn import_fwp_api(body: &[u8]) -> HttpResponse {
    let request: ImportWorkflowRequest = match parse_json(body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    match WorkflowStore::default().import_fwp(Path::new(&request.source_path), &request.target_path)
    {
        Ok(summary) => json_response(json!({ "summary": summary, "warnings": [] })),
        Err(error) => json_error(422, "Unprocessable Entity", &error.to_string()),
    }
}

fn import_ffc_api(body: &[u8]) -> HttpResponse {
    let request: ImportWorkflowRequest = match parse_json(body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    match WorkflowStore::default().import_ffc(Path::new(&request.source_path), &request.target_path)
    {
        Ok((summary, warnings)) => {
            json_response(json!({ "summary": summary, "warnings": warnings }))
        }
        Err(error) => json_error(422, "Unprocessable Entity", &error.to_string()),
    }
}

fn validate_api(body: &[u8]) -> HttpResponse {
    let request: WorkflowRequest = match parse_json(body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    match validate_workflow(&request.workflow) {
        Ok(()) => json_response(json!({ "valid": true })),
        Err(error) => json_error(422, "Unprocessable Entity", &error.to_string()),
    }
}

fn preview_api(body: &[u8]) -> HttpResponse {
    let request: WorkflowRequest = match parse_json(body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    match preview_workflow(&request.workflow) {
        Ok(lines) => json_response(json!({ "lines": lines })),
        Err(error) => json_error(422, "Unprocessable Entity", &error.to_string()),
    }
}

fn run_api(body: &[u8]) -> HttpResponse {
    let request: RunWorkflowRequest = match parse_json(body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    if request.workflow_path.trim().is_empty() {
        return json_error(
            422,
            "Unprocessable Entity",
            "workflowPath is required to resolve relative paths",
        );
    }

    let workflow_path = PathBuf::from(&request.workflow_path);
    let workflow_source = match serde_json::to_vec_pretty(&request.workflow) {
        Ok(source) => source,
        Err(error) => return json_error(400, "Bad Request", &error.to_string()),
    };
    let report_dir = request
        .report_dir
        .as_deref()
        .filter(|path| !path.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("fpw-reports"));
    let mut command = vec![
        "fpw".to_string(),
        "run".to_string(),
        workflow_path.to_string_lossy().to_string(),
    ];
    for (name, path) in &request.inputs {
        command.push("--input".to_string());
        command.push(format!("{name}={path}"));
    }
    for (name, path) in &request.outputs {
        command.push("--output".to_string());
        command.push(format!("{name}={path}"));
    }
    command.push("--report-dir".to_string());
    command.push(report_dir.to_string_lossy().to_string());
    let options = RunOptions {
        inputs: request.inputs,
        outputs: request.outputs,
        report_dir: Some(report_dir.clone()),
        command,
    };
    let report = match run_workflow_source(
        &workflow_path,
        &workflow_source,
        &request.workflow,
        &options,
    ) {
        Ok(report) => report,
        Err(error) => return json_error(422, "Unprocessable Entity", &error.to_string()),
    };

    let stem = format!(
        "{}-{}",
        safe_report_stem(&request.workflow.name),
        report.started_at_unix_ms
    );
    let report_paths = match report.write_all(&report_dir, &stem) {
        Ok(paths) => paths,
        Err(error) => return json_error(500, "Internal Server Error", &error.to_string()),
    };
    let mut warnings = Vec::new();
    if let Err(error) = fpw_core::recent::touch_recent_project(
        None,
        &workflow_path,
        &request.workflow.name,
        report.started_at_unix_ms,
    ) {
        warnings.push(format!("failed to update recent projects: {error}"));
    }

    json_response(json!({
        "report": report,
        "reportPaths": report_paths,
        "warnings": warnings
    }))
}

fn parse_json<T: for<'de> Deserialize<'de>>(body: &[u8]) -> Result<T, HttpResponse> {
    serde_json::from_slice(body)
        .map_err(|error| json_error(400, "Bad Request", &format!("invalid JSON: {error}")))
}

fn safe_report_stem(name: &str) -> String {
    let stem = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if stem.trim_matches('_').is_empty() {
        "workflow".to_string()
    } else {
        stem
    }
}

fn read_request(stream: &mut TcpStream) -> fpw_core::Result<HttpRequest> {
    let mut data = Vec::new();
    let mut buffer = [0_u8; 8192];
    let header_end = loop {
        let size = stream.read(&mut buffer)?;
        if size == 0 {
            return Err(fpw_core::FpwError::Message(
                "connection closed before request headers completed".to_string(),
            ));
        }
        data.extend_from_slice(&buffer[..size]);
        if data.len() > MAX_REQUEST_BYTES {
            return Err(fpw_core::FpwError::Message(
                "request exceeds 2 MiB limit".to_string(),
            ));
        }
        if let Some(position) = find_header_end(&data) {
            break position;
        }
    };

    let headers = String::from_utf8_lossy(&data[..header_end]);
    let mut lines = headers.lines();
    let request_line = lines
        .next()
        .ok_or_else(|| fpw_core::FpwError::Message("HTTP request line is missing".to_string()))?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("/").to_string();
    if method.is_empty() {
        return Err(fpw_core::FpwError::Message(
            "HTTP method is missing".to_string(),
        ));
    }
    let content_length = lines
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .map(|(_, value)| value.trim().parse::<usize>())
        .transpose()
        .map_err(|_| fpw_core::FpwError::Message("invalid Content-Length".to_string()))?
        .unwrap_or(0);
    if content_length > MAX_REQUEST_BYTES {
        return Err(fpw_core::FpwError::Message(
            "request body exceeds 2 MiB limit".to_string(),
        ));
    }

    let body_start = header_end + 4;
    let expected_length = body_start
        .checked_add(content_length)
        .ok_or_else(|| fpw_core::FpwError::Message("request length overflow".to_string()))?;
    while data.len() < expected_length {
        let size = stream.read(&mut buffer)?;
        if size == 0 {
            return Err(fpw_core::FpwError::Message(
                "connection closed before request body completed".to_string(),
            ));
        }
        data.extend_from_slice(&buffer[..size]);
        if data.len() > MAX_REQUEST_BYTES + body_start {
            return Err(fpw_core::FpwError::Message(
                "request exceeds 2 MiB limit".to_string(),
            ));
        }
    }

    Ok(HttpRequest {
        method,
        path: path.split('?').next().unwrap_or("/").to_string(),
        body: data[body_start..expected_length].to_vec(),
    })
}

fn find_header_end(data: &[u8]) -> Option<usize> {
    data.windows(4).position(|window| window == b"\r\n\r\n")
}

fn json_response(value: serde_json::Value) -> HttpResponse {
    HttpResponse {
        status_code: 200,
        status_text: "OK",
        content_type: "application/json; charset=utf-8",
        body: serde_json::to_vec_pretty(&value).unwrap_or_else(|_| b"{}".to_vec()),
    }
}

fn json_error(status_code: u16, status_text: &'static str, message: &str) -> HttpResponse {
    HttpResponse {
        status_code,
        status_text,
        content_type: "application/json; charset=utf-8",
        body: serde_json::to_vec(&json!({ "error": message }))
            .unwrap_or_else(|_| b"{\"error\":\"unknown error\"}".to_vec()),
    }
}

fn write_response(stream: &mut TcpStream, response: &HttpResponse) -> fpw_core::Result<()> {
    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\n\r\n",
        response.status_code,
        response.status_text,
        response.content_type,
        response.body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(&response.body)?;
    Ok(())
}

fn static_asset(request_path: &str) -> fpw_core::Result<Option<(&'static str, Vec<u8>)>> {
    let dist = Path::new("web").join("dist");
    if !dist.is_dir() {
        return Ok(None);
    }
    let relative = request_path.trim_start_matches('/');
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

fn fallback_page() -> &'static str {
    r#"<!doctype html>
<html lang="en">
  <head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>FPW</title></head>
  <body><main><h1>FPW WebUI</h1><p>Build the React application with <code>cd web && npm run build</code>, then restart this server.</p></main></body>
</html>"#
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workflow_json(input_path: &Path, output_path: &Path) -> serde_json::Value {
        json!({
            "schemaVersion": 1,
            "name": "web-api-test",
            "steps": [
                {
                    "id": "firmware",
                    "kind": "input",
                    "name": "firmware",
                    "path": input_path
                },
                {
                    "id": "digest",
                    "kind": "sha256",
                    "input": "firmware",
                    "output": "digest"
                },
                {
                    "id": "write_digest",
                    "kind": "output",
                    "input": "digest",
                    "name": "digest",
                    "path": output_path
                }
            ]
        })
    }

    #[test]
    fn validate_and_preview_endpoints_use_core_logic() {
        let workflow = workflow_json(Path::new("input.bin"), Path::new("digest.bin"));
        let body = serde_json::to_vec(&json!({ "workflow": workflow })).unwrap();

        let validate = api_response("POST", "/api/workflows/validate", &body).unwrap();
        assert_eq!(validate.status_code, 200);
        let preview = api_response("POST", "/api/workflows/preview", &body).unwrap();
        assert_eq!(preview.status_code, 200);
        assert!(String::from_utf8(preview.body)
            .unwrap()
            .contains("sha256 firmware -> digest"));
    }

    #[test]
    fn validate_endpoint_returns_structured_error() {
        let body = serde_json::to_vec(&json!({
            "workflow": { "schemaVersion": 1, "name": "", "steps": [] }
        }))
        .unwrap();

        let response = api_response("POST", "/api/workflows/validate", &body).unwrap();

        assert_eq!(response.status_code, 422);
        let value: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert!(value["error"].as_str().unwrap().contains("name"));
    }

    #[test]
    fn run_endpoint_executes_draft_and_writes_reports() {
        let root = std::env::temp_dir().join(format!("fpw-web-api-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let input_path = root.join("input.bin");
        let output_path = root.join("digest.bin");
        let report_dir = root.join("reports");
        fs::write(&input_path, b"firmware").unwrap();
        let body = serde_json::to_vec(&json!({
            "workflow": workflow_json(&input_path, &output_path),
            "workflowPath": root.join("draft.fwp"),
            "reportDir": report_dir
        }))
        .unwrap();

        let response = api_response("POST", "/api/workflows/run", &body).unwrap();

        assert_eq!(response.status_code, 200);
        assert_eq!(fs::read(&output_path).unwrap().len(), 32);
        let value: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(value["report"]["status"], "success");
        assert_eq!(value["reportPaths"].as_array().unwrap().len(), 2);
        let command = value["report"]["command"].as_array().unwrap();
        assert_eq!(command[0], "fpw");
        assert_eq!(command[1], "run");
        assert!(command.iter().any(|value| value == "--report-dir"));
        fs::remove_dir_all(root).unwrap();
    }
}
