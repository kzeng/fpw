export type StepKind = "input" | "output" | "fill" | "insert" | "merge" | "crc32" | "sha256";

export type WorkflowStep = {
  id: string;
  kind: StepKind;
  [key: string]: unknown;
};

export type Workflow = {
  schemaVersion: 1;
  name: string;
  description?: string;
  steps: WorkflowStep[];
};

export type WorkflowSummary = {
  path: string;
  name: string;
  description?: string;
  stepCount: number;
  updatedAtUnixMs: number;
};

export type OpenWorkflow = {
  path: string;
  absolutePath: string;
  workflow: Workflow;
};

export function emptyWorkflow(): Workflow {
  return { schemaVersion: 1, name: "", description: "", steps: [] };
}

export function workflowFileName(name: string): string {
  const slug = name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return `${slug || "workflow"}.fwp`;
}

export function outputArtifact(step: WorkflowStep): string | null {
  if (step.kind === "input") return String(step.name ?? "");
  if (step.kind === "output") return null;
  return String(step.output ?? "");
}

export function availableArtifacts(steps: WorkflowStep[], beforeIndex: number): string[] {
  return steps
    .slice(0, beforeIndex)
    .map(outputArtifact)
    .filter((name): name is string => Boolean(name));
}

export function newProcessingStep(kind: Exclude<StepKind, "input" | "output">, index: number, input: string): WorkflowStep {
  const suffix = index + 1;
  switch (kind) {
    case "fill":
      return { id: `fill_${suffix}`, kind, input, output: `filled_${suffix}`, offset: "0x0", length: 16, value: "0xFF" };
    case "insert":
      return { id: `insert_${suffix}`, kind, base: input, insert: input, output: `inserted_${suffix}`, offset: "0x0" };
    case "merge":
      return { id: `merge_${suffix}`, kind, output: `merged_${suffix}`, parts: input ? [{ input, offset: "0x0" }] : [] };
    case "crc32":
      return { id: `crc_${suffix}`, kind, input, output: `with_crc_${suffix}`, range: { offset: "0x0", length: 16 }, writeOffset: "0x0", endian: "little" };
    case "sha256":
      return { id: `sha_${suffix}`, kind, input, output: `digest_${suffix}` };
  }
}

function quoteCommandArgument(value: string): string {
  if (/^[A-Za-z0-9_./:\\=-]+$/.test(value)) return value;
  return `"${value.replaceAll('"', '\\"')}"`;
}

export function buildRunCommand(
  workflowPath: string,
  inputs: Record<string, string>,
  outputs: Record<string, string>,
  reportDir: string,
): string {
  const argumentsList = ["fpw", "run", workflowPath];
  for (const [name, path] of Object.entries(inputs)) {
    if (path.trim()) argumentsList.push("--input", `${name}=${path}`);
  }
  for (const [name, path] of Object.entries(outputs)) {
    if (path.trim()) argumentsList.push("--output", `${name}=${path}`);
  }
  if (reportDir.trim()) argumentsList.push("--report-dir", reportDir);
  return argumentsList.map(quoteCommandArgument).join(" ");
}
