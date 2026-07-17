import React, { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { Activity, FilePlus2, LibraryBig, Play } from "lucide-react";
import {
  archiveWorkflow,
  duplicateWorkflow,
  getHealth,
  importWorkflow,
  listWorkflows,
  openWorkflow,
} from "./api";
import { LibraryView } from "./components/LibraryView";
import { RunView } from "./components/RunView";
import { WizardView } from "./components/WizardView";
import { emptyWorkflow, type OpenWorkflow, type Workflow, type WorkflowSummary } from "./workflow";
import { I18nProvider, useI18n } from "./i18n";
import "./styles.css";

type View = "library" | "wizard" | "run";

function App() {
  const { language, setLanguage, t } = useI18n();
  const [view, setView] = useState<View>("library");
  const [serviceStatus, setServiceStatus] = useState("checking");
  const [root, setRoot] = useState("workflows");
  const [workflows, setWorkflows] = useState<WorkflowSummary[]>([]);
  const [selected, setSelected] = useState<OpenWorkflow | null>(null);
  const [draft, setDraft] = useState<Workflow>(emptyWorkflow());
  const [draftPath, setDraftPath] = useState("workflow.fwp");
  const [isNew, setIsNew] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  async function refreshLibrary() {
    setBusy(true);
    try {
      const response = await listWorkflows();
      setRoot(response.root);
      setWorkflows(response.workflows);
      setError("");
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    getHealth().then((health) => setServiceStatus(health.status)).catch(() => setServiceStatus("offline"));
    refreshLibrary();
  }, []);

  async function load(path: string, target: "wizard" | "run") {
    setBusy(true);
    try {
      const opened = await openWorkflow(path);
      setSelected(opened);
      if (target === "wizard") {
        setDraft(opened.workflow);
        setDraftPath(opened.path);
        setIsNew(false);
      }
      setView(target);
      setError("");
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setBusy(false);
    }
  }

  function beginNew() {
    setDraft(emptyWorkflow());
    setDraftPath("workflow.fwp");
    setIsNew(true);
    setView("wizard");
  }

  async function libraryAction(action: () => Promise<unknown>) {
    setBusy(true);
    try {
      await action();
      await refreshLibrary();
      setError("");
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
      setBusy(false);
    }
  }

  return (
    <main className="appShell">
      <header className="topBar">
        <button className="brandButton" onClick={() => setView("library")}><span className="brandMark">FPW</span><span><b>Firmware workbench</b><small>{t("Author · manage · execute")}</small></span></button>
        <nav className="primaryNav"><button className={view === "library" ? "active" : ""} onClick={() => setView("library")}><LibraryBig size={16} aria-hidden="true" />{t("Workflow library")}</button><button className={view === "wizard" ? "active" : ""} onClick={beginNew}><FilePlus2 size={16} aria-hidden="true" />{t("Create workflow")}</button>{selected ? <button className={view === "run" ? "active" : ""} onClick={() => setView("run")}><Play size={16} aria-hidden="true" />{t("Run")}</button> : null}</nav>
        <div className="topUtilities"><div className="languageSwitch" aria-label="Language"><button className={language === "en" ? "active" : ""} onClick={() => setLanguage("en")}>EN</button><button className={language === "zh" ? "active" : ""} onClick={() => setLanguage("zh")}>中文</button></div><div className={`servicePill ${serviceStatus === "ok" ? "online" : ""}`}><Activity size={14} aria-hidden="true" /><span /> Core {serviceStatus}</div></div>
      </header>

      <div className="appContent">
        {view === "library" ? <LibraryView root={root} workflows={workflows} busy={busy} error={error} onNew={beginNew} onEdit={(path) => load(path, "wizard")} onRun={(path) => load(path, "run")} onRefresh={refreshLibrary} onDuplicate={(source, target) => libraryAction(() => duplicateWorkflow(source, target))} onArchive={(path) => libraryAction(() => archiveWorkflow(path))} onImport={(kind, source, target) => libraryAction(() => importWorkflow(kind, source, target))} /> : null}
        {view === "wizard" ? <WizardView key={`${isNew}-${draftPath}`} initialWorkflow={draft} initialPath={draftPath} isNew={isNew} onCancel={() => setView("library")} onSaved={async (path) => { await refreshLibrary(); await load(path, "run"); }} /> : null}
        {view === "run" && selected ? <RunView key={selected.path} selected={selected} onBack={() => setView("library")} onEdit={() => load(selected.path, "wizard")} /> : null}
      </div>
    </main>
  );
}

createRoot(document.getElementById("root")!).render(<React.StrictMode><I18nProvider><App /></I18nProvider></React.StrictMode>);
