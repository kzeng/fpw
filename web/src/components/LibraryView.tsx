import { useState } from "react";
import { Binary, Copy, Cpu, Database, FilePlus2, FolderOpen, Pencil, Play, RefreshCw, Trash2, Upload } from "lucide-react";
import type { PostbuildTemplate, WorkflowSummary } from "../workflow";
import { useI18n } from "../i18n";

type Props = {
  root: string;
  workflows: WorkflowSummary[];
  busy: boolean;
  error: string;
  onNew: () => void;
  onTemplate: (template: PostbuildTemplate) => void;
  onEdit: (path: string) => void;
  onRun: (path: string) => void;
  onRefresh: () => void;
  onDuplicate: (source: string, target: string) => void;
  onArchive: (path: string) => void;
  onImport: (kind: "fwp" | "ffc", source: string, target: string) => void;
};

export function LibraryView(props: Props) {
  const { language, t } = useI18n();
  const [importOpen, setImportOpen] = useState(false);
  const [sourcePath, setSourcePath] = useState("");
  const [targetPath, setTargetPath] = useState("imported.fwp");

  return (
    <section className="libraryView">
      <header className="viewHeader">
        <div>
          <span className="eyebrow">Workflow library</span>
          <h2>{t("Workflow library")}</h2>
          <p>{t("Create, maintain, and select repeatable firmware packaging workflows.")}</p>
        </div>
        <div className="headerActions">
          <button onClick={props.onRefresh} disabled={props.busy}><RefreshCw className={props.busy ? "isSpinning" : ""} size={16} aria-hidden="true" />{t("Refresh")}</button>
          <button onClick={() => setImportOpen((value) => !value)}><Upload size={16} aria-hidden="true" />{t("Import")}</button>
          <button className="primaryButton" onClick={props.onNew}><FilePlus2 size={16} aria-hidden="true" />{t("New workflow")}</button>
        </div>
      </header>

      <div className="libraryRoot"><FolderOpen size={16} aria-hidden="true" /><span>{t("Managed directory")}</span><code>{props.root || "workflows"}</code></div>
      {props.error ? <div className="inlineError">{props.error}</div> : null}

      <section className="templateShelf">
        <div className="templateIntro"><span className="eyebrow">{t("Postbuild templates")}</span></div>
        <button className="templateCard" onClick={() => props.onTemplate("mcu")}><Cpu size={22} aria-hidden="true" /><span><b>{t("MCU image package")}</b><small>{t("Merge Gboot, Image A, and Image B; validate versions; export HEX and BIN.")}</small></span></button>
        <button className="templateCard" onClick={() => props.onTemplate("dsp")}><Binary size={22} aria-hidden="true" /><span><b>{t("DSP injection")}</b><small>{t("Insert DSP P1/P2 into an MCU image with the legacy address layout.")}</small></span></button>
        <button className="templateCard nvrTemplateCard" onClick={() => props.onTemplate("nvr")}><Database size={22} aria-hidden="true" /><span><b>{t("NVR package")}</b><small>{t("Generate NVR register blocks from XLSM, inject images, or append NVR-REG archives.")}</small></span></button>
      </section>

      {importOpen ? (
        <form className="importPanel" onSubmit={(event) => {
          event.preventDefault();
          props.onImport("fwp", sourcePath, targetPath);
        }}>
          <label>{t("Source format")}<select value="fwp" disabled><option value="fwp">FPW .fwp</option></select></label>
          <label>{t("Local source path")}<input value={sourcePath} onChange={(event) => setSourcePath(event.target.value)} placeholder="C:/firmware/workflow.fwp" required /></label>
          <label>{t("Library target name")}<input value={targetPath} onChange={(event) => setTargetPath(event.target.value)} required /></label>
          <button className="primaryButton" disabled={props.busy}><Upload size={16} aria-hidden="true" />{t("Import to library")}</button>
        </form>
      ) : null}

      {props.workflows.length === 0 ? (
        <div className="libraryEmpty">
          <div className="emptyGlyph">.fwp</div>
          <h3>{t("Your workflow library is empty")}</h3>
          <p>{t("Create your first workflow with the guided authoring flow, or import an existing .fwp file.")}</p>
          <button className="primaryButton" onClick={props.onNew}><FilePlus2 size={16} aria-hidden="true" />{t("Start authoring")}</button>
        </div>
      ) : (
        <div className="workflowGrid">
          {props.workflows.map((workflow) => (
            <article className="workflowCard" key={workflow.path}>
              <div className="cardTopline"><span>{workflow.stepCount} steps</span><code>{workflow.path}</code></div>
              <h3>{workflow.name}</h3>
              <p>{workflow.description || t("No description")}</p>
              <time>{new Date(workflow.updatedAtUnixMs).toLocaleString(language === "zh" ? "zh-CN" : "en-US")}</time>
              <div className="cardActions">
                <button className="primaryButton" onClick={() => props.onRun(workflow.path)}><Play size={15} aria-hidden="true" />{t("Run")}</button>
                <button onClick={() => props.onEdit(workflow.path)}><Pencil size={15} aria-hidden="true" />{t("Edit")}</button>
                <button onClick={() => {
                  const target = window.prompt(t("Duplicate as a library file"), workflow.path.replace(/\.fwp$/i, "-copy.fwp"));
                  if (target) props.onDuplicate(workflow.path, target);
                }}><Copy size={15} aria-hidden="true" />{t("Duplicate")}</button>
                <button className="dangerButton iconButton" title={t("Archive")} aria-label={t("Archive")} onClick={() => {
                  if (window.confirm(t("Move {name} to .trash?", { name: workflow.name }))) props.onArchive(workflow.path);
                }}><Trash2 size={15} aria-hidden="true" /></button>
              </div>
            </article>
          ))}
        </div>
      )}
    </section>
  );
}
