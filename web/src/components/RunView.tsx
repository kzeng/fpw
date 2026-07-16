import { useMemo, useState } from "react";
import { previewWorkflow, runWorkflow, validateWorkflow } from "../api";
import { buildRunCommand, type OpenWorkflow, type WorkflowStep } from "../workflow";
import { ResultPanel, type OperationResult } from "./ResultPanel";
import { useI18n } from "../i18n";

type Props = { selected: OpenWorkflow; onBack: () => void; onEdit: () => void };

export function RunView({ selected, onBack, onEdit }: Props) {
  const { t } = useI18n();
  const inputs = useMemo(() => selected.workflow.steps.filter((step) => step.kind === "input"), [selected]);
  const outputs = useMemo(() => selected.workflow.steps.filter((step) => step.kind === "output"), [selected]);
  const [inputPaths, setInputPaths] = useState<Record<string, string>>(() => Object.fromEntries(inputs.map((step) => [String(step.name), ""])));
  const [outputPaths, setOutputPaths] = useState<Record<string, string>>(() => Object.fromEntries(outputs.map((step) => [String(step.name), ""])));
  const [reportDir, setReportDir] = useState("fpw-reports");
  const [busy, setBusy] = useState<"validate" | "preview" | "run" | null>(null);
  const [result, setResult] = useState<OperationResult | null>(null);

  async function execute(operation: "validate" | "preview" | "run") {
    setBusy(operation);
    try {
      if (operation === "validate") {
        await validateWorkflow(selected.workflow);
        setResult({ kind: "message", tone: "success", title: t("Workflow valid"), detail: t("{count} steps passed core validation.", { count: selected.workflow.steps.length }) });
      } else if (operation === "preview") {
        const response = await previewWorkflow(selected.workflow);
        setResult({ kind: "preview", lines: response.lines, command: buildRunCommand(selected.absolutePath, inputPaths, outputPaths, reportDir) });
      } else {
        const response = await runWorkflow({
          workflow: selected.workflow,
          workflowPath: selected.absolutePath,
          inputs: Object.fromEntries(Object.entries(inputPaths).filter(([, value]) => value.trim())),
          outputs: Object.fromEntries(Object.entries(outputPaths).filter(([, value]) => value.trim())),
          reportDir,
        });
        setResult({ kind: "run", response });
      }
    } catch (error) {
      setResult({ kind: "message", tone: "error", title: t("Operation failed"), detail: error instanceof Error ? error.message : String(error) });
    } finally {
      setBusy(null);
    }
  }

  return (
    <section className="runView">
      <header className="viewHeader compactHeader"><div><span className="eyebrow">Controlled execution</span><h2>{t("Run {name}", { name: selected.workflow.name })}</h2><p>{selected.workflow.description || selected.path}</p></div><div className="headerActions"><button onClick={onBack}>{t("Back to library")}</button><button onClick={onEdit}>{t("Edit configuration")}</button></div></header>
      <div className="runLayout">
        <aside className="runIdentity"><span className="sectionLabel">{t("Selected workflow")}</span><h3>{selected.workflow.name}</h3><code>{selected.path}</code><dl><div><dt>{t("Steps")}</dt><dd>{selected.workflow.steps.length}</dd></div><div><dt>{t("Inputs")}</dt><dd>{inputs.length}</dd></div><div><dt>{t("Outputs")}</dt><dd>{outputs.length}</dd></div></dl><ol>{selected.workflow.steps.map((step: WorkflowStep, index) => <li key={`${step.id}-${index}`}><b>{String(index + 1).padStart(2, "0")}</b><span>{step.id}</span><code>{step.kind}</code></li>)}</ol></aside>
        <section className="runConfiguration">
          <div className="runSection"><span className="sectionLabel">01 · {t("Inputs")}</span><h3>{t("Choose inputs for this run")}</h3>{inputs.map((step) => <label key={step.id}>{String(step.name)}<input value={inputPaths[String(step.name)] ?? ""} onChange={(event) => setInputPaths((current) => ({ ...current, [String(step.name)]: event.target.value }))} placeholder={t("Default: {path}", { path: String(step.path ?? t("Not configured")) })} /><small>{t("Leave blank to use the default path from the .fwp file.")}</small></label>)}</div>
          <div className="runSection"><span className="sectionLabel">02 · {t("Outputs")}</span><h3>{t("Confirm output locations")}</h3>{outputs.map((step) => <label key={step.id}>{String(step.name)}<input value={outputPaths[String(step.name)] ?? ""} onChange={(event) => setOutputPaths((current) => ({ ...current, [String(step.name)]: event.target.value }))} placeholder={t("Default: {path}", { path: String(step.path ?? t("Not configured")) })} /><small>{t("Leave blank to resolve relative to the .fwp directory.")}</small></label>)}</div>
          <div className="runSection"><span className="sectionLabel">03 · {t("Reports")}</span><h3>{t("Execution records")}</h3><label>{t("Report directory")}<input value={reportDir} onChange={(event) => setReportDir(event.target.value)} /></label></div>
          <div className="runCommandBar"><button disabled={Boolean(busy)} onClick={() => execute("validate")}>{t(busy === "validate" ? "Validating..." : "Validate")}</button><button disabled={Boolean(busy)} onClick={() => execute("preview")}>{t(busy === "preview" ? "Previewing..." : "Preview")}</button><button className="primaryButton" disabled={Boolean(busy)} onClick={() => execute("run")}>{t(busy === "run" ? "Running..." : "Run workflow")}</button></div>
        </section>
      </div>
      <ResultPanel result={result} />
    </section>
  );
}
