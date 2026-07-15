import React, { useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import { getHealth, getRecentProjects, type RecentProject } from "./api";
import "./styles.css";

type StepKind = "input" | "output" | "fill" | "insert" | "merge" | "crc32" | "sha256";

type WorkflowStep = {
  id: string;
  kind: StepKind;
  [key: string]: unknown;
};

type Workflow = {
  schemaVersion: 1;
  name: string;
  description?: string;
  steps: WorkflowStep[];
};

const initialWorkflow: Workflow = {
  schemaVersion: 1,
  name: "workflow",
  description: "Local FPW workflow draft",
  steps: [
    { id: "firmware", kind: "input", name: "firmware", path: "input.bin" },
    { id: "write_image", kind: "output", input: "firmware", name: "image", path: "out/image.bin" },
  ],
};

function App() {
  const [workflowText, setWorkflowText] = useState(() => JSON.stringify(initialWorkflow, null, 2));
  const [serviceStatus, setServiceStatus] = useState("checking");
  const [recentProjects, setRecentProjects] = useState<RecentProject[]>([]);
  const parsed = useMemo(() => {
    try {
      return { workflow: JSON.parse(workflowText) as Workflow, error: null };
    } catch (error) {
      return { workflow: null, error: error instanceof Error ? error.message : String(error) };
    }
  }, [workflowText]);

  const stepCount = parsed.workflow?.steps?.length ?? 0;

  useEffect(() => {
    let active = true;

    Promise.all([getHealth(), getRecentProjects()])
      .then(([health, recent]) => {
        if (!active) return;
        setServiceStatus(health.status);
        setRecentProjects(recent.projects);
      })
      .catch((error) => {
        if (!active) return;
        setServiceStatus(error instanceof Error ? error.message : "offline");
      });

    return () => {
      active = false;
    };
  }, []);

  return (
    <main className="appShell">
      <header className="topBar">
        <div>
          <h1>FPW</h1>
          <p>Firmware Packaging Workflow</p>
        </div>
        <div className="statusPill">{parsed.error ? "Invalid JSON" : `${stepCount} steps`}</div>
      </header>

      <section className="workspace">
        <aside className="sidePanel">
          <h2>Workflow</h2>
          <dl>
            <div>
              <dt>Name</dt>
              <dd>{parsed.workflow?.name ?? "-"}</dd>
            </div>
            <div>
              <dt>Schema</dt>
              <dd>{parsed.workflow?.schemaVersion ?? "-"}</dd>
            </div>
          </dl>
          <h2>Service</h2>
          <dl>
            <div>
              <dt>Status</dt>
              <dd>{serviceStatus}</dd>
            </div>
          </dl>
          <h2>Recent</h2>
          <ol className="recentList">
            {recentProjects.length === 0 ? (
              <li className="emptyListItem">No recent projects</li>
            ) : (
              recentProjects.map((project) => (
                <li key={project.path}>
                  <strong>{project.name}</strong>
                  <span>{project.path}</span>
                </li>
              ))
            )}
          </ol>
          <h2>Steps</h2>
          <ol className="stepList">
            {parsed.workflow?.steps?.map((step) => (
              <li key={step.id}>
                <strong>{step.id}</strong>
                <span>{step.kind}</span>
              </li>
            )) ?? null}
          </ol>
        </aside>

        <section className="editorPanel">
          <div className="panelHeader">
            <h2>.fwp JSON</h2>
            <span>{parsed.error ?? "Ready for CLI execution"}</span>
          </div>
          <textarea
            aria-label="Workflow JSON"
            spellCheck={false}
            value={workflowText}
            onChange={(event) => setWorkflowText(event.target.value)}
          />
        </section>
      </section>
    </main>
  );
}

createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
