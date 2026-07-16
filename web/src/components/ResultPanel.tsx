import type { RunWorkflowResponse } from "../api";
import { useI18n } from "../i18n";

export type OperationResult =
  | { kind: "message"; tone: "success" | "error"; title: string; detail: string }
  | { kind: "preview"; lines: string[]; command?: string }
  | { kind: "run"; response: RunWorkflowResponse };

export function ResultPanel({ result }: { result: OperationResult | null }) {
  const { t } = useI18n();
  if (!result) return <section className="resultPanel emptyResult"><span>{t("Execution output")}</span><p>{t("Validate or preview the workflow, then run it.")}</p></section>;
  if (result.kind === "message") return <section className={`resultPanel ${result.tone}`}><span>{result.title}</span><p>{result.detail}</p></section>;
  if (result.kind === "preview") return <section className="resultPanel"><span>{t("Execution preview")}</span><ol className="previewLines">{result.lines.map((line, index) => <li key={`${line}-${index}`}><b>{String(index + 1).padStart(2, "0")}</b>{line}</li>)}</ol>{result.command ? <div className="commandPreview"><div><b>{t("CLI command")}</b><button onClick={() => navigator.clipboard.writeText(result.command ?? "")}>{t("Copy command")}</button></div><code>{result.command}</code><p>{t("Run this command in the same environment where FPW is installed.")}</p></div> : null}</section>;
  const { report, reportPaths, warnings } = result.response;
  return (
    <section className={`resultPanel runResult ${report.status}`}>
      <div className="resultSummary"><div><span>{t(report.status === "success" ? "Run succeeded" : "Run failed")}</span><strong>{report.durationMs} ms</strong></div><code title={report.workflowSha256}>{report.workflowSha256.slice(0, 16)}...</code></div>
      <div className="stepReports">{report.steps.map((step) => <div key={step.id} className={step.status}><b>{step.status}</b><strong>{step.id}</strong><span>{step.kind} · {step.durationMs} ms</span>{step.message ? <p>{step.message}</p> : null}</div>)}</div>
      <div className="fileReports">{report.files.map((file) => <p key={`${file.role}-${file.path}`}><b>{file.role}</b> {file.name} · {file.sizeBytes} bytes<br /><span>{file.path}</span></p>)}</div>
      <div className="reportPaths"><b>{t("Reports")}</b>{reportPaths.map((path) => <code key={path}>{path}</code>)}</div>
      {warnings.map((warning) => <p className="warningText" key={warning}>{warning}</p>)}
    </section>
  );
}
