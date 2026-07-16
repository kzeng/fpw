import type { OpenWorkflow, Workflow, WorkflowSummary } from "./workflow";

export type HealthResponse = { status: string; service: string };
export type RecentProject = { path: string; name: string; updatedAtUnixMs: number };
export type RecentProjectsResponse = { projects: RecentProject[] };
export type StepReport = { id: string; kind: string; status: "success" | "failed"; durationMs: number; message?: string };
export type FileReport = { role: string; name: string; path: string; sizeBytes: number; sha256: string };
export type ExecutionReport = {
  status: "success" | "failed";
  durationMs: number;
  workflowSha256: string;
  steps: StepReport[];
  files: FileReport[];
};
export type RunWorkflowResponse = { report: ExecutionReport; reportPaths: string[]; warnings: string[] };
export type WorkflowListResponse = { root: string; workflows: WorkflowSummary[] };

export class ApiError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.name = "ApiError";
    this.status = status;
  }
}

async function requestJson<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    ...init,
    headers: { Accept: "application/json", ...(init?.body ? { "Content-Type": "application/json" } : {}), ...init?.headers },
  });
  const payload = (await response.json().catch(() => ({}))) as { error?: string };
  if (!response.ok) throw new ApiError(response.status, payload.error ?? `${response.status} ${response.statusText}`);
  return payload as T;
}

function sendJson<T>(path: string, method: "POST" | "PUT", payload: unknown): Promise<T> {
  return requestJson<T>(path, { method, body: JSON.stringify(payload) });
}

export const getHealth = () => requestJson<HealthResponse>("/api/health");
export const getRecentProjects = () => requestJson<RecentProjectsResponse>("/api/recent-projects");
export const listWorkflows = () => requestJson<WorkflowListResponse>("/api/workflows");
export const openWorkflow = (path: string) => sendJson<OpenWorkflow>("/api/workflows/open", "POST", { path });
export const createWorkflow = (path: string, workflow: Workflow) =>
  sendJson<{ summary: WorkflowSummary; absolutePath: string }>("/api/workflows/create", "POST", { path, workflow });
export const saveWorkflow = (path: string, workflow: Workflow) =>
  sendJson<{ summary: WorkflowSummary }>("/api/workflows/save", "PUT", { path, workflow });
export const duplicateWorkflow = (sourcePath: string, targetPath: string) =>
  sendJson<{ summary: WorkflowSummary }>("/api/workflows/duplicate", "POST", { sourcePath, targetPath });
export const archiveWorkflow = (path: string) =>
  sendJson<{ archivedPath: string }>("/api/workflows/archive", "POST", { path });
export const importWorkflow = (kind: "fwp" | "ffc", sourcePath: string, targetPath: string) =>
  sendJson<{ summary: WorkflowSummary; warnings: string[] }>(`/api/workflows/import/${kind}`, "POST", { sourcePath, targetPath });
export const validateWorkflow = (workflow: Workflow) => sendJson<{ valid: true }>("/api/workflows/validate", "POST", { workflow });
export const previewWorkflow = (workflow: Workflow) => sendJson<{ lines: string[] }>("/api/workflows/preview", "POST", { workflow });
export const runWorkflow = (payload: {
  workflow: Workflow;
  workflowPath: string;
  inputs: Record<string, string>;
  outputs: Record<string, string>;
  reportDir?: string;
}) => sendJson<RunWorkflowResponse>("/api/workflows/run", "POST", payload);
