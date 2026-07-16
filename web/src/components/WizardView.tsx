import { useState } from "react";
import { createWorkflow, previewWorkflow, saveWorkflow, validateWorkflow } from "../api";
import {
  availableArtifacts,
  newProcessingStep,
  workflowFileName,
  type StepKind,
  type Workflow,
  type WorkflowStep,
} from "../workflow";
import { useI18n } from "../i18n";

const processingKinds: Array<Exclude<StepKind, "input" | "output">> = ["fill", "insert", "merge", "crc32", "sha256"];

type Props = {
  initialWorkflow: Workflow;
  initialPath: string;
  isNew: boolean;
  onCancel: () => void;
  onSaved: (path: string) => void;
};

export function WizardView({ initialWorkflow, initialPath, isNew, onCancel, onSaved }: Props) {
  const { t } = useI18n();
  const stages = [t("Workflow details"), t("Inputs"), t("Processing steps"), t("Outputs"), t("Review and save")];
  const [workflow, setWorkflow] = useState(initialWorkflow);
  const [path, setPath] = useState(initialPath);
  const [stage, setStage] = useState(0);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [preview, setPreview] = useState<string[]>([]);
  const [advanced, setAdvanced] = useState(false);
  const [advancedText, setAdvancedText] = useState(() => JSON.stringify(initialWorkflow, null, 2));

  function updateStep(index: number, patch: Partial<WorkflowStep>) {
    setWorkflow((current) => ({ ...current, steps: current.steps.map((step, stepIndex) => stepIndex === index ? { ...step, ...patch } : step) }));
  }

  function removeStep(index: number) {
    setWorkflow((current) => ({ ...current, steps: current.steps.filter((_, stepIndex) => stepIndex !== index) }));
  }

  function moveStep(index: number, direction: -1 | 1) {
    setWorkflow((current) => {
      const target = index + direction;
      if (target < 0 || target >= current.steps.length) return current;
      const category = (step: WorkflowStep) => step.kind === "input" ? "input" : step.kind === "output" ? "output" : "processing";
      if (category(current.steps[index]) !== category(current.steps[target])) return current;
      const steps = [...current.steps];
      [steps[index], steps[target]] = [steps[target], steps[index]];
      return { ...current, steps };
    });
  }

  function addInput() {
    setWorkflow((current) => {
      const count = current.steps.filter((step) => step.kind === "input").length + 1;
      const step: WorkflowStep = { id: `input_${count}`, kind: "input", name: `input_${count}`, path: "input.bin" };
      const index = current.steps.findIndex((item) => item.kind !== "input");
      const steps = [...current.steps];
      steps.splice(index < 0 ? steps.length : index, 0, step);
      return { ...current, steps };
    });
  }

  function addProcessing(kind: Exclude<StepKind, "input" | "output">) {
    setWorkflow((current) => {
      const outputIndex = current.steps.findIndex((step) => step.kind === "output");
      const insertIndex = outputIndex < 0 ? current.steps.length : outputIndex;
      const artifacts = availableArtifacts(current.steps, insertIndex);
      const step = newProcessingStep(kind, current.steps.length, artifacts.at(-1) ?? "");
      const steps = [...current.steps];
      steps.splice(insertIndex, 0, step);
      return { ...current, steps };
    });
  }

  function addOutput() {
    setWorkflow((current) => {
      const artifacts = availableArtifacts(current.steps, current.steps.length);
      const count = current.steps.filter((step) => step.kind === "output").length + 1;
      return {
        ...current,
        steps: [...current.steps, { id: `output_${count}`, kind: "output", input: artifacts.at(-1) ?? "", name: `output_${count}`, path: `out/output_${count}.bin` }],
      };
    });
  }

  async function review() {
    setBusy(true);
    setMessage("");
    try {
      await validateWorkflow(workflow);
      const result = await previewWorkflow(workflow);
      setPreview(result.lines);
      setMessage(t("Core validation passed. The workflow can be saved."));
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }

  async function save() {
    setBusy(true);
    setMessage("");
    try {
      await validateWorkflow(workflow);
      if (isNew) await createWorkflow(path, workflow);
      else await saveWorkflow(path, workflow);
      onSaved(path);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }

  function applyAdvancedJson() {
    try {
      const parsed = JSON.parse(advancedText) as Workflow;
      setWorkflow(parsed);
      setMessage(t("Advanced JSON applied. Core validation will still run before save."));
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  }

  const indexedSteps = workflow.steps.map((step, index) => ({ step, index }));
  const currentItems = stage === 1
    ? indexedSteps.filter(({ step }) => step.kind === "input")
    : stage === 2
      ? indexedSteps.filter(({ step }) => !["input", "output"].includes(step.kind))
      : indexedSteps.filter(({ step }) => step.kind === "output");

  return (
    <section className="wizardView">
      <header className="viewHeader compactHeader">
        <div><span className="eyebrow">Guided authoring</span><h2>{isNew ? t("Create workflow") : t("Edit {name}", { name: workflow.name })}</h2><p>{t("Build a reviewable, repeatable .fwp through guided forms.")}</p></div>
        <button onClick={onCancel}>{t("Back to library")}</button>
      </header>

      <nav className="wizardRail" aria-label={t("Create workflow")}>
        {stages.map((name, index) => <button className={index === stage ? "active" : index < stage ? "complete" : ""} key={name} onClick={() => setStage(index)}><b>{index + 1}</b><span>{name}</span></button>)}
      </nav>

      <div className="wizardBody">
        {stage === 0 ? (
          <section className="formStage narrowStage">
            <span className="stageNumber">01</span><h3>{t("What problem does this workflow solve?")}</h3><p>{t("The name appears in the workflow library and execution reports.")}</p>
            <label>{t("Workflow name")}<input value={workflow.name} onChange={(event) => {
              const name = event.target.value;
              setWorkflow((current) => ({ ...current, name }));
              if (isNew) setPath(workflowFileName(name));
            }} placeholder="production-image" /></label>
            <label>{t("Description")}<textarea value={workflow.description ?? ""} onChange={(event) => setWorkflow((current) => ({ ...current, description: event.target.value }))} placeholder={t("Merge boot and app, then write the release CRC.")} /></label>
            <label>{t("Library file name")}<input value={path} disabled={!isNew} onChange={(event) => setPath(event.target.value)} /><small>{t("Only relative .fwp paths inside the workflow library are allowed.")}</small></label>
          </section>
        ) : null}

        {stage >= 1 && stage <= 3 ? (
          <section className="formStage">
            <div className="stageIntro"><span className="stageNumber">0{stage + 1}</span><div><h3>{stages[stage]}</h3><p>{t(stage === 1 ? "Declare the firmware files required at execution time." : stage === 2 ? "Add binary processing operations in execution order." : "Select the artifacts that must be written to disk.")}</p></div></div>
            {stage === 2 ? <div className="stepPalette">{processingKinds.map((kind) => <button key={kind} onClick={() => addProcessing(kind)}>+ {kind}</button>)}</div> : <button className="addStepButton" onClick={stage === 1 ? addInput : addOutput}>+ {t(stage === 1 ? "Add input" : "Add output")}</button>}
            <div className="stepForms">
              {currentItems.length === 0 ? <div className="stageEmpty">{t("No {stage} yet. Add one above.", { stage: stages[stage].toLowerCase() })}</div> : currentItems.map(({ step, index }) => (
                <StepEditor key={`${step.id}-${index}`} step={step} index={index} allSteps={workflow.steps} update={(patch) => updateStep(index, patch)} remove={() => removeStep(index)} move={(direction) => moveStep(index, direction)} />
              ))}
            </div>
          </section>
        ) : null}

        {stage === 4 ? (
          <section className="formStage reviewStage">
            <div className="stageIntro"><span className="stageNumber">05</span><div><h3>{t("Review and save")}</h3><p>{t("Use the same fpw-core validation as the CLI before writing this workflow.")}</p></div></div>
            <div className="reviewActions"><button onClick={review} disabled={busy}>{t("Validate and preview")}</button><button onClick={() => { setAdvanced(!advanced); setAdvancedText(JSON.stringify(workflow, null, 2)); }}>{t("Advanced JSON")}</button><button className="primaryButton" onClick={save} disabled={busy}>{t(busy ? "Creating..." : isNew ? "Create .fwp" : "Save changes")}</button></div>
            {message ? <div className="wizardMessage">{message}</div> : null}
            {preview.length ? <ol className="previewLines">{preview.map((line, index) => <li key={line}><b>{String(index + 1).padStart(2, "0")}</b>{line}</li>)}</ol> : null}
            {advanced ? <div className="advancedEditor"><textarea value={advancedText} onChange={(event) => setAdvancedText(event.target.value)} spellCheck={false} /><button onClick={applyAdvancedJson}>{t("Apply JSON to wizard")}</button></div> : null}
          </section>
        ) : null}
      </div>

      <footer className="wizardFooter"><button disabled={stage === 0} onClick={() => setStage((value) => Math.max(0, value - 1))}>{t("Previous")}</button><span>{t("Step {current} of {total}", { current: stage + 1, total: stages.length })}</span><button className="primaryButton" disabled={stage === stages.length - 1} onClick={() => setStage((value) => Math.min(stages.length - 1, value + 1))}>{t("Next")}</button></footer>
    </section>
  );
}

function StepEditor({ step, index, allSteps, update, remove, move }: {
  step: WorkflowStep;
  index: number;
  allSteps: WorkflowStep[];
  update: (patch: Partial<WorkflowStep>) => void;
  remove: () => void;
  move: (direction: -1 | 1) => void;
}) {
  const { t } = useI18n();
  const artifacts = availableArtifacts(allSteps, index);
  const select = (label: string, field: string) => <label>{t(label)}<select value={String(step[field] ?? "")} onChange={(event) => update({ [field]: event.target.value })}><option value="">{t("Select artifact")}</option>{artifacts.map((name) => <option key={name} value={name}>{name}</option>)}</select></label>;
  const field = (label: string, name: string, placeholder = "") => <label>{t(label)}<input value={String(step[name] ?? "")} placeholder={placeholder} onChange={(event) => update({ [name]: event.target.value })} /></label>;
  return (
    <article className="stepForm">
      <header><span>{String(index + 1).padStart(2, "0")}</span><b>{step.kind}</b><div><button onClick={() => move(-1)} title={t("Move up")}>↑</button><button onClick={() => move(1)} title={t("Move down")}>↓</button><button className="dangerButton" onClick={remove}>{t("Remove")}</button></div></header>
      <div className="stepFields">
        {field("Step ID", "id")}
        {step.kind === "input" ? <>{field("Input name", "name")}{field("Default file path", "path", "firmware.bin")}</> : null}
        {step.kind === "output" ? <>{select("Source artifact", "input")}{field("Output name", "name")}{field("Default output path", "path", "out/image.bin")}</> : null}
        {step.kind === "fill" ? <>{select("Input artifact", "input")}{field("Output artifact", "output")}{field("Offset", "offset", "0x100")}{field("Length", "length", "16")}{field("Fill value", "value", "0xFF")}</> : null}
        {step.kind === "insert" ? <>{select("Base artifact", "base")}{select("Inserted artifact", "insert")}{field("Output artifact", "output")}{field("Write offset", "offset", "0x200")}</> : null}
        {step.kind === "crc32" ? <>{select("Input artifact", "input")}{field("Output artifact", "output")}<RangeFields step={step} update={update} />{field("CRC write offset", "writeOffset", "0xFFC")}<label>{t("Endian")}<select value={String(step.endian ?? "little")} onChange={(event) => update({ endian: event.target.value })}><option value="little">little</option><option value="big">big</option></select></label></> : null}
        {step.kind === "sha256" ? <>{select("Input artifact", "input")}{field("Digest artifact", "output")}<label className="checkLabel"><input type="checkbox" checked={Boolean(step.range)} onChange={(event) => update({ range: event.target.checked ? { offset: "0x0", length: 16 } : undefined })} />{t("Hash only a byte range")}</label>{step.range ? <RangeFields step={step} update={update} /> : null}</> : null}
        {step.kind === "merge" ? <MergeFields step={step} artifacts={artifacts} update={update} /> : null}
      </div>
    </article>
  );
}

function RangeFields({ step, update }: { step: WorkflowStep; update: (patch: Partial<WorkflowStep>) => void }) {
  const { t } = useI18n();
  const range = (step.range ?? { offset: "0x0", length: 16 }) as { offset: unknown; length: unknown };
  return <><label>{t("Range offset")}<input value={String(range.offset)} onChange={(event) => update({ range: { ...range, offset: event.target.value } })} /></label><label>{t("Range length")}<input value={String(range.length)} onChange={(event) => update({ range: { ...range, length: event.target.value } })} /></label></>;
}

function MergeFields({ step, artifacts, update }: { step: WorkflowStep; artifacts: string[]; update: (patch: Partial<WorkflowStep>) => void }) {
  const { t } = useI18n();
  const parts = (step.parts ?? []) as Array<{ input: string; offset: unknown }>;
  return <><label>{t("Output artifact")}<input value={String(step.output ?? "")} onChange={(event) => update({ output: event.target.value })} /></label><div className="mergeParts"><b>{t("Merge parts")}</b>{parts.map((part, index) => <div key={index}><select value={part.input} onChange={(event) => update({ parts: parts.map((item, itemIndex) => itemIndex === index ? { ...item, input: event.target.value } : item) })}><option value="">{t("Select artifact")}</option>{artifacts.map((name) => <option key={name}>{name}</option>)}</select><input value={String(part.offset)} onChange={(event) => update({ parts: parts.map((item, itemIndex) => itemIndex === index ? { ...item, offset: event.target.value } : item) })} /><button className="dangerButton" onClick={() => update({ parts: parts.filter((_, itemIndex) => itemIndex !== index) })}>×</button></div>)}<button onClick={() => update({ parts: [...parts, { input: artifacts.at(-1) ?? "", offset: "0x0" }] })}>+ {t("Add part")}</button></div></>;
}
