export type StepKind =
  | "input"
  | "output"
  | "image-input"
  | "image-output"
  | "image-extract"
  | "image-overlay"
  | "image-patch"
  | "image-to-binary"
  | "image-extract-string"
  | "assert-equal"
  | "image-insert-binary"
  | "nvr-generate"
  | "nvr-patch-registers"
  | "nvr-inject-image"
  | "nvr-append-archive"
  | "fill"
  | "delete"
  | "insert"
  | "merge"
  | "crc32"
  | "sha256";

export type BasicProcessingKind = "fill" | "delete" | "insert" | "merge" | "crc32" | "sha256";
export type ImageProcessingKind = "image-extract" | "image-overlay" | "image-patch" | "image-to-binary" | "image-extract-string" | "assert-equal" | "image-insert-binary";
export type NvrProcessingKind = "nvr-generate" | "nvr-patch-registers" | "nvr-inject-image" | "nvr-append-archive";
export type PostbuildTemplate = "mcu" | "dsp" | "nvr";

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
  if (step.kind === "input" || step.kind === "image-input") return String(step.name ?? "");
  if (step.kind === "output" || step.kind === "image-output") return null;
  return String(step.output ?? "");
}

export function availableArtifacts(steps: WorkflowStep[], beforeIndex: number): string[] {
  return steps
    .slice(0, beforeIndex)
    .map(outputArtifact)
    .filter((name): name is string => Boolean(name));
}

export function newProcessingStep(kind: BasicProcessingKind, index: number, input: string): WorkflowStep {
  const suffix = index + 1;
  switch (kind) {
    case "fill":
      return { id: `fill_${suffix}`, kind, input, output: `filled_${suffix}`, offset: "0x0", length: 16, value: "0xFF" };
    case "delete":
      return { id: `delete_${suffix}`, kind, input, output: `deleted_${suffix}`, range: { offset: "0x0", length: 16 } };
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

export function newImageProcessingStep(kind: ImageProcessingKind, index: number, input: string): WorkflowStep {
  const suffix = index + 1;
  switch (kind) {
    case "image-extract":
      return { id: `image_extract_${suffix}`, kind, input, output: `image_region_${suffix}`, address: "0x08000000", length: "0x2000" };
    case "image-overlay":
      return { id: `image_overlay_${suffix}`, kind, base: input, overlays: [], output: `overlaid_image_${suffix}`, overlap: "error" };
    case "image-patch":
      return { id: `image_patch_${suffix}`, kind, input, output: `patched_image_${suffix}`, address: "0x08000000", data: "0000" };
    case "image-to-binary":
      return { id: `image_to_binary_${suffix}`, kind, input, output: `binary_image_${suffix}`, address: "0x08000000", length: "0x200000", fill: "0xFF" };
    case "image-extract-string":
      return { id: `extract_string_${suffix}`, kind, input, output: `text_${suffix}`, address: "0x08010250", length: 7, trim: "null-space" };
    case "assert-equal":
      return { id: `assert_equal_${suffix}`, kind, left: input, right: input, message: "Values differ" };
    case "image-insert-binary":
      return { id: `insert_binary_${suffix}`, kind, base: input, input: "", output: `image_with_binary_${suffix}`, maxLength: "0x93000", parts: [{ sourceOffset: "0x0", address: "0x08080000", length: "0x80000" }] };
  }
}

export function newNvrProcessingStep(kind: NvrProcessingKind, index: number, input: string): WorkflowStep {
  const suffix = index + 1;
  switch (kind) {
    case "nvr-generate":
      return { id: `nvr_generate_${suffix}`, kind, output: `nvr_block_${suffix}`, workbook: "default_nvr/config.xlsm", page: 254, bankStart: 0, bankEnd: 0, registerStart: 128, registerEnd: 255, baseAddress: "0x08002000", versionSheet: "Cover", versionCell: "E3", ignoreMaskRule: false, alternateBase: false, sheets: [{ name: "0_254", bank: 0, rowStart: 3, rowEnd: 146, dataColumn: 7 }] };
    case "nvr-patch-registers":
      return { id: `nvr_patch_${suffix}`, kind, input, output: `patched_nvr_${suffix}`, patches: [{ bank: 0, register: 128, data: "00" }] };
    case "nvr-inject-image":
      return { id: `nvr_inject_${suffix}`, kind, image: input, nvr: "", output: `image_with_nvr_${suffix}`, mirrorOffset: "0x100000" };
    case "nvr-append-archive":
      return { id: `nvr_archive_${suffix}`, kind, archive: input, nvr: "", output: `archive_with_nvr_${suffix}`, tool: "tools/imgAr.exe", encryption: "enc0" };
  }
}

export function postbuildTemplate(template: PostbuildTemplate): Workflow {
  if (template === "nvr") {
    return {
      schemaVersion: 1,
      name: "postbuild-nvr",
      description: "Generate NVR register data from XLSM and write it as a binary block.",
      steps: [
        { id: "generate_nvr", kind: "nvr-generate", output: "nvr_block", workbook: "default_nvr/config.xlsm", page: 254, bankStart: 0, bankEnd: 0, registerStart: 128, registerEnd: 255, baseAddress: "0x08002000", versionSheet: "Cover", versionCell: "E3", ignoreMaskRule: false, alternateBase: false, sheets: [{ name: "0_254", bank: 0, rowStart: 3, rowEnd: 146, dataColumn: 7 }] },
        { id: "write_nvr", kind: "output", input: "nvr_block", name: "nvr_bin", path: "out/nvr.bin" },
      ],
    };
  }
  if (template === "dsp") {
    return {
      schemaVersion: 1,
      name: "postbuild-dsp-inject",
      description: "Inject DSP P1/P2 data into an MCU Intel HEX image.",
      steps: [
        { id: "read_mcu_image", kind: "image-input", name: "mcu_hex" },
        { id: "read_dsp", kind: "input", name: "dsp" },
        { id: "inject_dsp", kind: "image-insert-binary", base: "mcu_hex", input: "dsp", output: "mcu_with_dsp", maxLength: "0x93000", parts: [{ sourceOffset: "0x0", address: "0x08080000", length: "0x80000" }, { sourceOffset: "0x80000", address: "0x08180000", length: "0x13000" }] },
        { id: "write_dsp_hex", kind: "image-output", input: "mcu_with_dsp", name: "jlink_dsp_hex", path: "out/postbuild-mcu-dsp.hex", recordSize: 16 },
        { id: "make_dsp_bin", kind: "image-to-binary", input: "mcu_with_dsp", output: "jlink_dsp_bin", address: "0x08000000", length: "0x200000", fill: "0xFF" },
        { id: "write_dsp_bin", kind: "output", input: "jlink_dsp_bin", name: "jlink_dsp_bin", path: "out/postbuild-mcu-dsp.bin" },
      ],
    };
  }
  return {
    schemaVersion: 1,
    name: "postbuild-mcu-merge",
    description: "Merge Gboot, Image A, and Image B into J-Link HEX and BIN outputs.",
    steps: [
      { id: "read_gboot", kind: "image-input", name: "gboot" },
      { id: "read_image_a", kind: "image-input", name: "image_a" },
      { id: "read_image_b", kind: "image-input", name: "image_b" },
      { id: "extract_gboot_a", kind: "image-extract", input: "gboot", output: "gboot_a", address: "0x08000000", length: "0x2000" },
      { id: "extract_gboot_b", kind: "image-extract", input: "gboot", output: "gboot_b", address: "0x08100000", length: "0x2000" },
      { id: "merge_gboot", kind: "image-overlay", base: "gboot_a", overlays: ["gboot_b"], output: "gboot_banks", overlap: "error" },
      { id: "merge_images", kind: "image-overlay", base: "gboot_banks", overlays: ["image_a", "image_b"], output: "full_image", overlap: "error" },
      { id: "select_image_a", kind: "image-patch", input: "full_image", output: "selected_image", address: "0x0810C000", data: "0000" },
      { id: "version_a", kind: "image-extract-string", input: "selected_image", output: "image_a_version", address: "0x08010250", length: 7, trim: "null-space" },
      { id: "version_b", kind: "image-extract-string", input: "selected_image", output: "image_b_version", address: "0x08110250", length: 7, trim: "null-space" },
      { id: "check_versions", kind: "assert-equal", left: "image_a_version", right: "image_b_version", message: "Image A and Image B firmware versions differ" },
      { id: "write_hex", kind: "image-output", input: "selected_image", name: "jlink_hex", path: "out/postbuild-mcu.hex", recordSize: 16 },
      { id: "make_bin", kind: "image-to-binary", input: "selected_image", output: "jlink_bin", address: "0x08000000", length: "0x200000", fill: "0xFF" },
      { id: "write_bin", kind: "output", input: "jlink_bin", name: "jlink_bin", path: "out/postbuild-mcu.bin" },
    ],
  };
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
