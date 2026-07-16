import { createContext, useContext, useEffect, useState, type ReactNode } from "react";

export type Language = "en" | "zh";

const zh: Record<string, string> = {
  "Workflow library": "工作流库",
  "Create, maintain, and select repeatable firmware packaging workflows.": "创建、维护并选择可重复执行的固件打包流程。",
  Refresh: "刷新",
  Import: "导入",
  "New workflow": "新建工作流",
  "Managed directory": "管理目录",
  "Source format": "来源格式",
  "Local source path": "本地源文件路径",
  "Library target name": "库内目标名称",
  "Import to library": "导入到工作流库",
  "Your workflow library is empty": "工作流库还是空的",
  "Create your first workflow with the guided authoring flow, or import an existing .fwp / .ffc file.": "从分步向导创建第一个工作流，或者导入已有的 .fwp / .ffc。",
  "Start authoring": "开始创建",
  "No description": "没有说明",
  Run: "运行",
  Edit: "编辑",
  Duplicate: "复制",
  Archive: "归档",
  "Duplicate as a library file": "复制为库内文件名",
  "Move {name} to .trash?": "将 {name} 移动到 .trash？",
  "Create workflow": "创建工作流",
  "Edit {name}": "编辑 {name}",
  "Build a reviewable, repeatable .fwp through guided forms.": "通过表单构建可审查、可重复执行的 .fwp。",
  "Back to library": "返回工作流库",
  "Workflow details": "基本信息",
  Inputs: "输入",
  "Processing steps": "处理步骤",
  Outputs: "输出",
  "Review and save": "检查并保存",
  "What problem does this workflow solve?": "这个工作流要解决什么问题？",
  "The name appears in the workflow library and execution reports.": "名称会出现在工作流库和执行报告中。",
  "Workflow name": "工作流名称",
  Description: "说明",
  "Library file name": "库内文件名",
  "Only relative .fwp paths inside the workflow library are allowed.": "只允许工作流库内的相对 .fwp 路径。",
  "Declare the firmware files required at execution time.": "声明执行时需要的固件文件。",
  "Add binary processing operations in execution order.": "按顺序添加二进制处理动作。",
  "Select the artifacts that must be written to disk.": "选择最终需要写出的 artifact。",
  "Add input": "添加输入",
  "Add output": "添加输出",
  "No {stage} yet. Add one above.": "还没有{stage}，从上方添加。",
  "Use the same fpw-core validation as the CLI before writing this workflow.": "保存前使用与 CLI 相同的 fpw-core 校验执行顺序和 artifact 引用。",
  "Validate and preview": "校验并预览",
  "Advanced JSON": "高级 JSON",
  "Creating...": "处理中...",
  "Create .fwp": "创建 .fwp",
  "Save changes": "保存修改",
  "Apply JSON to wizard": "应用 JSON 到向导",
  Previous: "上一步",
  Next: "下一步",
  "Step {current} of {total}": "步骤 {current} / {total}",
  "Core validation passed. The workflow can be saved.": "核心校验通过，可以保存。",
  "Advanced JSON applied. Core validation will still run before save.": "高级 JSON 已应用，保存前仍会经过核心校验。",
  "Merge boot and app, then write the release CRC.": "合并 boot 和 app，并写入发布 CRC。",
  "Select artifact": "选择 artifact",
  "Move up": "上移",
  "Move down": "下移",
  Remove: "移除",
  "Step ID": "步骤 ID",
  "Input name": "输入名称",
  "Default file path": "默认文件路径",
  "Source artifact": "来源 artifact",
  "Output name": "输出名称",
  "Default output path": "默认输出路径",
  "Input artifact": "输入 artifact",
  "Output artifact": "输出 artifact",
  Offset: "偏移",
  Length: "长度",
  "Fill value": "填充值",
  "Base artifact": "基础 artifact",
  "Inserted artifact": "插入 artifact",
  "Write offset": "写入偏移",
  "CRC write offset": "CRC 写入偏移",
  Endian: "字节序",
  "Digest artifact": "Digest artifact",
  "Hash only a byte range": "只计算指定范围",
  "Range offset": "范围偏移",
  "Range length": "范围长度",
  "Merge parts": "合并片段",
  "Add part": "添加片段",
  "Run {name}": "运行 {name}",
  "Edit configuration": "编辑配置",
  "Selected workflow": "已选工作流",
  Steps: "步骤",
  "Choose inputs for this run": "选择本次执行的输入",
  "Default: {path}": "默认：{path}",
  "Not configured": "未配置",
  "Leave blank to use the default path from the .fwp file.": "留空时使用 .fwp 中的默认路径。",
  "Confirm output locations": "确认输出位置",
  "Leave blank to resolve relative to the .fwp directory.": "留空时相对 .fwp 所在目录解析。",
  "Execution records": "执行记录",
  "Report directory": "报告目录",
  Validate: "校验",
  Preview: "预览",
  "Run workflow": "运行工作流",
  "Validating...": "校验中...",
  "Previewing...": "预览中...",
  "Running...": "执行中...",
  "Workflow valid": "工作流有效",
  "{count} steps passed core validation.": "{count} 个步骤通过核心校验。",
  "Operation failed": "操作失败",
  "Execution output": "执行输出",
  "Validate or preview the workflow, then run it.": "先校验或预览，然后运行工作流。",
  "Execution preview": "执行预览",
  "CLI command": "CLI 命令",
  "Copy command": "复制命令",
  "Run this command in the same environment where FPW is installed.": "可在已安装 FPW 的同一 CLI 环境中执行此命令。",
  "Run succeeded": "运行成功",
  "Run failed": "运行失败",
  Reports: "报告",
  "Author · manage · execute": "创建 · 管理 · 执行",
};

type I18nValue = {
  language: Language;
  setLanguage: (language: Language) => void;
  t: (key: string, values?: Record<string, string | number>) => string;
};

const I18nContext = createContext<I18nValue | null>(null);

export function I18nProvider({ children }: { children: ReactNode }) {
  const [language, setLanguageState] = useState<Language>(() => {
    const stored = window.localStorage.getItem("fpw-language");
    return stored === "zh" ? "zh" : "en";
  });
  useEffect(() => {
    document.documentElement.lang = language === "zh" ? "zh-CN" : "en";
    window.localStorage.setItem("fpw-language", language);
  }, [language]);
  const t = (key: string, values: Record<string, string | number> = {}) => {
    let text = language === "zh" ? (zh[key] ?? key) : key;
    for (const [name, value] of Object.entries(values)) text = text.replaceAll(`{${name}}`, String(value));
    return text;
  };
  return <I18nContext.Provider value={{ language, setLanguage: setLanguageState, t }}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nValue {
  const value = useContext(I18nContext);
  if (!value) throw new Error("useI18n must be used inside I18nProvider");
  return value;
}
