import { useState } from "react";
import { ArrowDown, ArrowLeft, ArrowUp, Braces, Check, ChevronLeft, ChevronRight, FileJson, LoaderCircle, Plus, Save, ShieldCheck, Trash2 } from "lucide-react";
import { createWorkflow, previewWorkflow, saveWorkflow, validateWorkflow } from "../api";
import {
  availableArtifacts,
  type BasicProcessingKind,
  type ImageProcessingKind,
  type NvrProcessingKind,
  newImageProcessingStep,
  newNvrProcessingStep,
  newProcessingStep,
  workflowFileName,
  type Workflow,
  type WorkflowStep,
} from "../workflow";
import { useI18n } from "../i18n";

const processingKinds: BasicProcessingKind[] = ["fill", "delete", "insert", "merge", "crc32", "sha256"];
const imageProcessingKinds: ImageProcessingKind[] = ["image-extract", "image-overlay", "image-patch", "image-to-binary", "image-extract-string", "assert-equal", "image-insert-binary"];
const nvrProcessingKinds: NvrProcessingKind[] = ["nvr-generate", "nvr-patch-registers", "nvr-inject-image", "nvr-append-archive"];
const operationHelp: Record<BasicProcessingKind | ImageProcessingKind | NvrProcessingKind, string> = {
  "fill": "Copy a BIN artifact and overwrite a byte range with one repeated value. Configure input, output, offset, length, and fill value. Use 0xFF to erase/reserve a firmware region without changing image length.",
  "delete": "Logically delete a byte range by replacing it with 0xFF while preserving the BIN length and every later offset. Configure input, output, range offset, and range length.",
  "insert": "Copy an entire BIN artifact into a base BIN at a byte offset. Existing bytes in that range are replaced and the base grows with 0xFF padding when needed. Use it for metadata or sub-image insertion.",
  "merge": "Create one BIN from multiple input artifacts placed at independent output offsets. Add each input/offset pair; gaps become 0xFF and overlapping ranges are rejected.",
  "crc32": "Calculate IEEE CRC-32 over a selected range of the current BIN, then write the four-byte result back at writeOffset. Choose little or big endian for the stored CRC bytes.",
  "sha256": "Calculate a SHA-256 digest for the complete BIN or an optional byte range. The output is a separate 32-byte binary artifact, normally written as a checksum file.",
  "image-extract": "Extract an absolute address range from a sparse Intel HEX image and create a new image artifact. Set source image, start address, and length. Use it to retain Gboot or a selected flash region.",
  "image-overlay": "Combine multiple sparse Intel HEX images by absolute address. Choose a base image, overlay images, and error/replace overlap policy. Use it to assemble Gboot, Image A, and Image B.",
  "image-patch": "Write explicit hexadecimal bytes at an absolute image address and create a patched image artifact. Use it for boot flags, version fields, or small configuration changes.",
  "image-to-binary": "Convert a selected absolute image address range to a contiguous BIN artifact. Sparse holes are filled with the configured fill byte, normally 0xFF.",
  "image-extract-string": "Read an ASCII string from an absolute image address into a text artifact. Configure address, length, and trim mode. Commonly used to read firmware versions.",
  "assert-equal": "Compare two text artifacts and stop the workflow when they differ. Use it after extracting Image A and Image B versions to prevent mismatched releases.",
  "image-insert-binary": "Split a BIN artifact into source ranges and insert each range at an absolute address in a sparse image. Use it for DSP P1/P2 injection and enforce maxLength when required.",
  "nvr-generate": "Read NVR register values from an XLSX/XLSM workbook. Configure page, bank/register range, Flash base address, version cell, and Sheet mappings. Produces an NVR artifact with imgAr metadata.",
  "nvr-patch-registers": "Create a derived NVR artifact by writing hex bytes to selected bank/register addresses. Use it for PCB revision, IHS/RHS, cooled, DSP, or AA product variants.",
  "nvr-inject-image": "Insert an NVR artifact at its calculated Flash address in an Intel HEX image. Set mirrorOffset, such as 0x100000, to write the same NVR into Image B.",
  "nvr-append-archive": "Append an NVR artifact to an existing firmware archive by invoking imgAr with file type NVR-REG. Configure the imgAr executable and enc0/enc1 mode.",
};

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
      const category = (step: WorkflowStep) => ["input", "image-input"].includes(step.kind) ? "input" : ["output", "image-output"].includes(step.kind) ? "output" : "processing";
      if (category(current.steps[index]) !== category(current.steps[target])) return current;
      const steps = [...current.steps];
      [steps[index], steps[target]] = [steps[target], steps[index]];
      return { ...current, steps };
    });
  }

  function addInput(kind: "input" | "image-input" = "input") {
    setWorkflow((current) => {
      const count = current.steps.filter((step) => step.kind === kind).length + 1;
      const image = kind === "image-input";
      const step: WorkflowStep = { id: `${image ? "hex" : "input"}_${count}`, kind, name: `${image ? "image" : "input"}_${count}`, path: image ? "firmware.hex" : "input.bin" };
      const index = current.steps.findIndex((item) => !["input", "image-input"].includes(item.kind));
      const steps = [...current.steps];
      steps.splice(index < 0 ? steps.length : index, 0, step);
      return { ...current, steps };
    });
  }

  function addProcessing(kind: BasicProcessingKind) {
    setWorkflow((current) => {
      const outputIndex = current.steps.findIndex((step) => ["output", "image-output"].includes(step.kind));
      const insertIndex = outputIndex < 0 ? current.steps.length : outputIndex;
      const artifacts = availableArtifacts(current.steps, insertIndex);
      const step = newProcessingStep(kind, current.steps.length, artifacts.at(-1) ?? "");
      const steps = [...current.steps];
      steps.splice(insertIndex, 0, step);
      return { ...current, steps };
    });
  }

  function addImageProcessing(kind: ImageProcessingKind) {
    setWorkflow((current) => {
      const outputIndex = current.steps.findIndex((step) => ["output", "image-output"].includes(step.kind));
      const insertIndex = outputIndex < 0 ? current.steps.length : outputIndex;
      const artifacts = availableArtifacts(current.steps, insertIndex);
      const step = newImageProcessingStep(kind, current.steps.length, artifacts.at(-1) ?? "");
      const steps = [...current.steps];
      steps.splice(insertIndex, 0, step);
      return { ...current, steps };
    });
  }

  function addNvrProcessing(kind: NvrProcessingKind) {
    setWorkflow((current) => {
      const outputIndex = current.steps.findIndex((step) => ["output", "image-output"].includes(step.kind));
      const insertIndex = outputIndex < 0 ? current.steps.length : outputIndex;
      const artifacts = availableArtifacts(current.steps, insertIndex);
      const step = newNvrProcessingStep(kind, current.steps.length, artifacts.at(-1) ?? "");
      const steps = [...current.steps];
      steps.splice(insertIndex, 0, step);
      return { ...current, steps };
    });
  }

  function addOutput(kind: "output" | "image-output" = "output") {
    setWorkflow((current) => {
      const artifacts = availableArtifacts(current.steps, current.steps.length);
      const count = current.steps.filter((step) => step.kind === kind).length + 1;
      const image = kind === "image-output";
      return {
        ...current,
        steps: [...current.steps, { id: `${image ? "hex_output" : "output"}_${count}`, kind, input: artifacts.at(-1) ?? "", name: `${image ? "hex" : "output"}_${count}`, path: `out/output_${count}.${image ? "hex" : "bin"}`, ...(image ? { recordSize: 16 } : {}) }],
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
    ? indexedSteps.filter(({ step }) => ["input", "image-input"].includes(step.kind))
    : stage === 2
      ? indexedSteps.filter(({ step }) => !["input", "image-input", "output", "image-output"].includes(step.kind))
      : indexedSteps.filter(({ step }) => ["output", "image-output"].includes(step.kind));

  return (
    <section className="wizardView">
      <header className="viewHeader compactHeader">
        <div><span className="eyebrow">Guided authoring</span><h2>{isNew ? t("Create workflow") : t("Edit {name}", { name: workflow.name })}</h2><p>{t("Build a reviewable, repeatable .fwp through guided forms.")}</p></div>
        <button onClick={onCancel}><ArrowLeft size={16} aria-hidden="true" />{t("Back to library")}</button>
      </header>

      <nav className="wizardRail" aria-label={t("Create workflow")}>
        {stages.map((name, index) => <button className={index === stage ? "active" : index < stage ? "complete" : ""} key={name} onClick={() => setStage(index)}><b>{index < stage ? <Check size={14} aria-hidden="true" /> : index + 1}</b><span>{name}</span></button>)}
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
            {stage === 2 ? <><div className="paletteLabel">{t("Binary operations")}</div><div className="stepPalette">{processingKinds.map((kind) => <button key={kind} className="operationHelpButton" data-tooltip={t(operationHelp[kind])} aria-label={`${kind}: ${t(operationHelp[kind])}`} onClick={() => addProcessing(kind)}><Plus size={15} aria-hidden="true" />{kind}</button>)}</div><div className="paletteLabel imagePaletteLabel">{t("Image / Postbuild operations")}</div><div className="stepPalette imagePalette">{imageProcessingKinds.map((kind) => <button key={kind} className="operationHelpButton" data-tooltip={t(operationHelp[kind])} aria-label={`${kind}: ${t(operationHelp[kind])}`} onClick={() => addImageProcessing(kind)}><Plus size={15} aria-hidden="true" />{kind}</button>)}</div><div className="paletteLabel imagePaletteLabel">{t("NVR operations")}</div><div className="stepPalette nvrPalette">{nvrProcessingKinds.map((kind) => <button key={kind} className="operationHelpButton" data-tooltip={t(operationHelp[kind])} aria-label={`${kind}: ${t(operationHelp[kind])}`} onClick={() => addNvrProcessing(kind)}><Plus size={15} aria-hidden="true" />{kind}</button>)}</div></> : <div className="addStepChoices"><button className="addStepButton" onClick={() => stage === 1 ? addInput("input") : addOutput("output")}><Plus size={16} aria-hidden="true" />{t(stage === 1 ? "Add BIN input" : "Add BIN output")}</button><button className="addStepButton imageAction" onClick={() => stage === 1 ? addInput("image-input") : addOutput("image-output")}><Plus size={16} aria-hidden="true" />{t(stage === 1 ? "Add Intel HEX input" : "Add Intel HEX output")}</button></div>}
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
            <div className="reviewActions"><button onClick={review} disabled={busy}>{busy ? <LoaderCircle className="isSpinning" size={16} aria-hidden="true" /> : <ShieldCheck size={16} aria-hidden="true" />}{t("Validate and preview")}</button><button onClick={() => { setAdvanced(!advanced); setAdvancedText(JSON.stringify(workflow, null, 2)); }}><Braces size={16} aria-hidden="true" />{t("Advanced JSON")}</button><button className="primaryButton" onClick={save} disabled={busy}><Save size={16} aria-hidden="true" />{t(busy ? "Creating..." : isNew ? "Create .fwp" : "Save changes")}</button></div>
            {message ? <div className="wizardMessage">{message}</div> : null}
            {preview.length ? <ol className="previewLines">{preview.map((line, index) => <li key={line}><b>{String(index + 1).padStart(2, "0")}</b>{line}</li>)}</ol> : null}
            {advanced ? <div className="advancedEditor"><textarea value={advancedText} onChange={(event) => setAdvancedText(event.target.value)} spellCheck={false} /><button onClick={applyAdvancedJson}><FileJson size={16} aria-hidden="true" />{t("Apply JSON to wizard")}</button></div> : null}
          </section>
        ) : null}
      </div>

      <footer className="wizardFooter"><button disabled={stage === 0} onClick={() => setStage((value) => Math.max(0, value - 1))}><ChevronLeft size={16} aria-hidden="true" />{t("Previous")}</button><span>{t("Step {current} of {total}", { current: stage + 1, total: stages.length })}</span><button className="primaryButton" disabled={stage === stages.length - 1} onClick={() => setStage((value) => Math.min(stages.length - 1, value + 1))}>{t("Next")}<ChevronRight size={16} aria-hidden="true" /></button></footer>
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
      <header><span>{String(index + 1).padStart(2, "0")}</span><b>{step.kind}</b><div><button className="iconButton" onClick={() => move(-1)} title={t("Move up")} aria-label={t("Move up")}><ArrowUp size={15} aria-hidden="true" /></button><button className="iconButton" onClick={() => move(1)} title={t("Move down")} aria-label={t("Move down")}><ArrowDown size={15} aria-hidden="true" /></button><button className="dangerButton" onClick={remove}><Trash2 size={15} aria-hidden="true" />{t("Remove")}</button></div></header>
      <div className="stepFields">
        {field("Step ID", "id")}
        {step.kind === "input" ? <>{field("Input name", "name")}{field("Default file path", "path", "firmware.bin")}</> : null}
        {step.kind === "image-input" ? <>{field("Input name", "name")}{field("Default Intel HEX path", "path", "firmware.hex")}</> : null}
        {step.kind === "output" ? <>{select("Source artifact", "input")}{field("Output name", "name")}{field("Default output path", "path", "out/image.bin")}</> : null}
        {step.kind === "image-output" ? <>{select("Source image artifact", "input")}{field("Output name", "name")}{field("Default output path", "path", "out/image.hex")}{field("HEX record size", "recordSize", "16")}</> : null}
        {step.kind === "fill" ? <>{select("Input artifact", "input")}{field("Output artifact", "output")}{field("Offset", "offset", "0x100")}{field("Length", "length", "16")}{field("Fill value", "value", "0xFF")}</> : null}
        {step.kind === "delete" ? <>{select("Input artifact", "input")}{field("Output artifact", "output")}<RangeFields step={step} update={update} /><p className="stepHint">{t("Deleted bytes become 0xFF; image length and later offsets do not change.")}</p></> : null}
        {step.kind === "insert" ? <>{select("Base artifact", "base")}{select("Inserted artifact", "insert")}{field("Output artifact", "output")}{field("Write offset", "offset", "0x200")}</> : null}
        {step.kind === "crc32" ? <>{select("Input artifact", "input")}{field("Output artifact", "output")}<RangeFields step={step} update={update} />{field("CRC write offset", "writeOffset", "0xFFC")}<label>{t("Endian")}<select value={String(step.endian ?? "little")} onChange={(event) => update({ endian: event.target.value })}><option value="little">little</option><option value="big">big</option></select></label></> : null}
        {step.kind === "sha256" ? <>{select("Input artifact", "input")}{field("Digest artifact", "output")}<label className="checkLabel"><input type="checkbox" checked={Boolean(step.range)} onChange={(event) => update({ range: event.target.checked ? { offset: "0x0", length: 16 } : undefined })} />{t("Hash only a byte range")}</label>{step.range ? <RangeFields step={step} update={update} /> : null}</> : null}
        {step.kind === "merge" ? <MergeFields step={step} artifacts={artifacts} update={update} /> : null}
        {step.kind === "image-extract" ? <>{select("Source image artifact", "input")}{field("Output artifact", "output")}{field("Absolute address", "address", "0x08000000")}{field("Length", "length", "0x2000")}</> : null}
        {step.kind === "image-overlay" ? <ImageOverlayFields step={step} artifacts={artifacts} update={update} /> : null}
        {step.kind === "image-patch" ? <>{select("Source image artifact", "input")}{field("Output artifact", "output")}{field("Absolute address", "address", "0x0810C000")}{field("Hex bytes", "data", "0000")}</> : null}
        {step.kind === "image-to-binary" ? <>{select("Source image artifact", "input")}{field("Output artifact", "output")}{field("Start address", "address", "0x08000000")}{field("Length", "length", "0x200000")}{field("Fill value", "fill", "0xFF")}</> : null}
        {step.kind === "image-extract-string" ? <>{select("Source image artifact", "input")}{field("Text artifact", "output")}{field("Absolute address", "address", "0x08010250")}{field("Length", "length", "7")}<label>{t("Trim mode")}<select value={String(step.trim ?? "null-space")} onChange={(event) => update({ trim: event.target.value })}><option value="null-space">null-space</option><option value="none">none</option></select></label></> : null}
        {step.kind === "assert-equal" ? <>{select("Left text artifact", "left")}{select("Right text artifact", "right")}{field("Failure message", "message", "Values differ")}</> : null}
        {step.kind === "image-insert-binary" ? <ImageInsertBinaryFields step={step} artifacts={artifacts} update={update} /> : null}
        {step.kind === "nvr-generate" ? <NvrGenerateFields step={step} update={update} /> : null}
        {step.kind === "nvr-patch-registers" ? <NvrPatchFields step={step} artifacts={artifacts} update={update} /> : null}
        {step.kind === "nvr-inject-image" ? <>{select("Source image artifact", "image")}{select("NVR artifact", "nvr")}{field("Output artifact", "output")}{field("Mirror bank offset", "mirrorOffset", "0x100000")}<p className="stepHint">{t("Leave mirror offset empty to inject only one image bank.")}</p></> : null}
        {step.kind === "nvr-append-archive" ? <>{select("Archive artifact", "archive")}{select("NVR artifact", "nvr")}{field("Output artifact", "output")}{field("imgAr executable", "tool", "tools/imgAr.exe")}<label>{t("Encryption mode")}<select value={String(step.encryption ?? "enc0")} onChange={(event) => update({ encryption: event.target.value })}><option value="enc0">enc0</option><option value="enc1">enc1</option></select></label></> : null}
      </div>
    </article>
  );
}

function NvrPatchFields({ step, artifacts, update }: { step: WorkflowStep; artifacts: string[]; update: (patch: Partial<WorkflowStep>) => void }) {
  const { t } = useI18n();
  const patches = (step.patches ?? []) as Array<{ bank: number; register: number; data: string }>;
  return <><label>{t("NVR artifact")}<select value={String(step.input ?? "")} onChange={(event) => update({ input: event.target.value })}>{artifacts.map((artifact) => <option key={artifact}>{artifact}</option>)}</select></label><label>{t("Output artifact")}<input value={String(step.output ?? "")} onChange={(event) => update({ output: event.target.value })} /></label><div className="mergeParts"><b>{t("Register patches")}</b>{patches.map((patch, index) => <div key={index}><input type="number" value={patch.bank} title={t("Bank")} onChange={(event) => update({ patches: patches.map((item, itemIndex) => itemIndex === index ? { ...item, bank: Number(event.target.value) } : item) })} /><input type="number" value={patch.register} title={t("Register")} onChange={(event) => update({ patches: patches.map((item, itemIndex) => itemIndex === index ? { ...item, register: Number(event.target.value) } : item) })} /><input value={patch.data} title={t("Hex bytes")} onChange={(event) => update({ patches: patches.map((item, itemIndex) => itemIndex === index ? { ...item, data: event.target.value } : item) })} /><button className="dangerButton iconButton" onClick={() => update({ patches: patches.filter((_, itemIndex) => itemIndex !== index) })}><Trash2 size={15} /></button></div>)}<button onClick={() => update({ patches: [...patches, { bank: 0, register: 128, data: "00" }] })}><Plus size={15} />{t("Add register patch")}</button></div></>;
}

function NvrGenerateFields({ step, update }: { step: WorkflowStep; update: (patch: Partial<WorkflowStep>) => void }) {
  const { t } = useI18n();
  const sheets = (step.sheets ?? []) as Array<{ name: string; bank: number; rowStart: number; rowEnd: number; dataColumn: number }>;
  const text = (label: string, name: string, placeholder = "") => <label>{t(label)}<input value={String(step[name] ?? "")} placeholder={placeholder} onChange={(event) => update({ [name]: event.target.value })} /></label>;
  const number = (label: string, name: string) => <label>{t(label)}<input type="number" min="0" value={Number(step[name] ?? 0)} onChange={(event) => update({ [name]: Number(event.target.value) })} /></label>;
  return <>{text("Output artifact", "output")}{text("NVR workbook", "workbook", "default_nvr/config.xlsm")}{number("NVR page", "page")}{number("Start bank", "bankStart")}{number("End bank", "bankEnd")}{number("Start register", "registerStart")}{number("End register", "registerEnd")}{text("Page base address", "baseAddress", "0x08002000")}{text("Version sheet", "versionSheet", "Cover")}{text("Version cell", "versionCell", "E3")}<label className="checkLabel"><input type="checkbox" checked={Boolean(step.ignoreMaskRule)} onChange={(event) => update({ ignoreMaskRule: event.target.checked })} />{t("Ignore NVR mask rule")}</label><label className="checkLabel"><input type="checkbox" checked={Boolean(step.alternateBase)} onChange={(event) => update({ alternateBase: event.target.checked })} />{t("Use alternate image bank as base")}</label><div className="mergeParts nvrSheets"><b>{t("Workbook sheet mappings")}</b>{sheets.map((sheet, index) => <div key={index}><input value={sheet.name} title={t("Sheet name")} onChange={(event) => update({ sheets: sheets.map((item, itemIndex) => itemIndex === index ? { ...item, name: event.target.value } : item) })} /><input type="number" value={sheet.bank} title={t("Bank")} onChange={(event) => update({ sheets: sheets.map((item, itemIndex) => itemIndex === index ? { ...item, bank: Number(event.target.value) } : item) })} /><input type="number" value={sheet.rowStart} title={t("Start row")} onChange={(event) => update({ sheets: sheets.map((item, itemIndex) => itemIndex === index ? { ...item, rowStart: Number(event.target.value) } : item) })} /><input type="number" value={sheet.rowEnd} title={t("End row")} onChange={(event) => update({ sheets: sheets.map((item, itemIndex) => itemIndex === index ? { ...item, rowEnd: Number(event.target.value) } : item) })} /><input type="number" value={sheet.dataColumn} title={t("Data column (zero-based)")} onChange={(event) => update({ sheets: sheets.map((item, itemIndex) => itemIndex === index ? { ...item, dataColumn: Number(event.target.value) } : item) })} /><button className="dangerButton iconButton" onClick={() => update({ sheets: sheets.filter((_, itemIndex) => itemIndex !== index) })}><Trash2 size={15} /></button></div>)}<button onClick={() => update({ sheets: [...sheets, { name: "0_254", bank: 0, rowStart: 3, rowEnd: 146, dataColumn: 7 }] })}><Plus size={15} />{t("Add sheet mapping")}</button><p className="stepHint">{t("Data column is zero-based: 7 means Excel column H. Rows without a register address in column A are skipped.")}</p></div></>;
}

function ImageOverlayFields({ step, artifacts, update }: { step: WorkflowStep; artifacts: string[]; update: (patch: Partial<WorkflowStep>) => void }) {
  const { t } = useI18n();
  const overlays = (step.overlays ?? []) as string[];
  return <><label>{t("Base image artifact")}<select value={String(step.base ?? "")} onChange={(event) => update({ base: event.target.value })}>{artifacts.map((name) => <option key={name}>{name}</option>)}</select></label><label>{t("Output artifact")}<input value={String(step.output ?? "")} onChange={(event) => update({ output: event.target.value })} /></label><label>{t("Overlap policy")}<select value={String(step.overlap ?? "error")} onChange={(event) => update({ overlap: event.target.value })}><option value="error">error</option><option value="replace">replace</option></select></label><div className="mergeParts"><b>{t("Overlay images")}</b>{overlays.map((name, index) => <div key={index}><select value={name} onChange={(event) => update({ overlays: overlays.map((item, itemIndex) => itemIndex === index ? event.target.value : item) })}>{artifacts.map((artifact) => <option key={artifact}>{artifact}</option>)}</select><button className="dangerButton iconButton" onClick={() => update({ overlays: overlays.filter((_, itemIndex) => itemIndex !== index) })}><Trash2 size={15} /></button></div>)}<button onClick={() => update({ overlays: [...overlays, artifacts.at(-1) ?? ""] })}><Plus size={15} />{t("Add overlay")}</button></div></>;
}

function ImageInsertBinaryFields({ step, artifacts, update }: { step: WorkflowStep; artifacts: string[]; update: (patch: Partial<WorkflowStep>) => void }) {
  const { t } = useI18n();
  const parts = (step.parts ?? []) as Array<{ sourceOffset: unknown; address: unknown; length?: unknown }>;
  const select = (label: string, name: string) => <label>{t(label)}<select value={String(step[name] ?? "")} onChange={(event) => update({ [name]: event.target.value })}>{artifacts.map((artifact) => <option key={artifact}>{artifact}</option>)}</select></label>;
  return <>{select("Base image artifact", "base")}{select("Binary input artifact", "input")}<label>{t("Output artifact")}<input value={String(step.output ?? "")} onChange={(event) => update({ output: event.target.value })} /></label><label>{t("Maximum input length")}<input value={String(step.maxLength ?? "")} onChange={(event) => update({ maxLength: event.target.value })} placeholder="0x93000" /></label><div className="mergeParts binaryImageParts"><b>{t("Binary image parts")}</b>{parts.map((part, index) => <div key={index}><input value={String(part.sourceOffset)} title={t("Source offset")} onChange={(event) => update({ parts: parts.map((item, itemIndex) => itemIndex === index ? { ...item, sourceOffset: event.target.value } : item) })} /><input value={String(part.address)} title={t("Target address")} onChange={(event) => update({ parts: parts.map((item, itemIndex) => itemIndex === index ? { ...item, address: event.target.value } : item) })} /><input value={String(part.length ?? "")} title={t("Length")} onChange={(event) => update({ parts: parts.map((item, itemIndex) => itemIndex === index ? { ...item, length: event.target.value } : item) })} /><button className="dangerButton iconButton" onClick={() => update({ parts: parts.filter((_, itemIndex) => itemIndex !== index) })}><Trash2 size={15} /></button></div>)}<button onClick={() => update({ parts: [...parts, { sourceOffset: "0x0", address: "0x08000000", length: "0x1000" }] })}><Plus size={15} />{t("Add part")}</button></div></>;
}

function RangeFields({ step, update }: { step: WorkflowStep; update: (patch: Partial<WorkflowStep>) => void }) {
  const { t } = useI18n();
  const range = (step.range ?? { offset: "0x0", length: 16 }) as { offset: unknown; length: unknown };
  return <><label>{t("Range offset")}<input value={String(range.offset)} onChange={(event) => update({ range: { ...range, offset: event.target.value } })} /></label><label>{t("Range length")}<input value={String(range.length)} onChange={(event) => update({ range: { ...range, length: event.target.value } })} /></label></>;
}

function MergeFields({ step, artifacts, update }: { step: WorkflowStep; artifacts: string[]; update: (patch: Partial<WorkflowStep>) => void }) {
  const { t } = useI18n();
  const parts = (step.parts ?? []) as Array<{ input: string; offset: unknown }>;
  return <><label>{t("Output artifact")}<input value={String(step.output ?? "")} onChange={(event) => update({ output: event.target.value })} /></label><div className="mergeParts"><b>{t("Merge parts")}</b>{parts.map((part, index) => <div key={index}><select value={part.input} onChange={(event) => update({ parts: parts.map((item, itemIndex) => itemIndex === index ? { ...item, input: event.target.value } : item) })}><option value="">{t("Select artifact")}</option>{artifacts.map((name) => <option key={name}>{name}</option>)}</select><input value={String(part.offset)} onChange={(event) => update({ parts: parts.map((item, itemIndex) => itemIndex === index ? { ...item, offset: event.target.value } : item) })} /><button className="dangerButton iconButton" aria-label={t("Remove")} title={t("Remove")} onClick={() => update({ parts: parts.filter((_, itemIndex) => itemIndex !== index) })}><Trash2 size={15} aria-hidden="true" /></button></div>)}<button onClick={() => update({ parts: [...parts, { input: artifacts.at(-1) ?? "", offset: "0x0" }] })}><Plus size={15} aria-hidden="true" />{t("Add part")}</button></div></>;
}
