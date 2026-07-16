# FPW - 固件打包工作流

[English](README.md) | [User Manual](User-Manual.md) | [中文用户手册](User-Manual-CN.md) | [FWP Schema](docs/fwp-schema-v1.md)

FPW 是一个本地优先的固件打包工作流工具，用于可重复地处理原始二进制固件镜像。

当前版本：**v0.0.2**

- `fpw` 是主要的命令行执行器。
- `fpw web` 启动本地 WebUI，用于创建、管理、预览和执行工作流。
- `.fwp` 是可纳入版本管理的 JSON 工作流文件，不依赖 WebUI 也能执行。
- 目标平台为 Windows 和 Linux。

## 主要功能

- 按数组顺序执行 `input`、`output`、`fill`、`insert`、`merge`、`crc32` 和 `sha256` 步骤。
- 生成 JSON/TXT 执行报告，记录命令、耗时、步骤状态和文件哈希。
- WebUI 提供五阶段创建向导和工作流文件库。
- 支持创建、打开、保存、复制、归档和导入工作流。
- WebUI 支持英文和简体中文，首次使用默认英文。
- Run Preview 同时显示执行预览和可复制的 CLI 命令。
- 对 FirmwareFlow `.ffc` 文件提供尽力转换。

## Windows 快速开始

需要安装：

- Rust 工具链和 Cargo
- Node.js 和 npm

在仓库根目录构建 WebUI 和 Release 可执行文件：

```powershell
Set-Location web
npm install
npm run build
Set-Location ..
cargo build --release -p fpw-cli
```

校验、预览并执行示例：

```powershell
.\target\release\fpw.exe validate examples\fill-crc-sha.fwp
.\target\release\fpw.exe preview examples\fill-crc-sha.fwp
.\target\release\fpw.exe run examples\fill-crc-sha.fwp
```

启动 WebUI：

```powershell
.\target\release\fpw.exe web --host 127.0.0.1 --port 4769
```

浏览器访问 `http://127.0.0.1:4769/`。使用 WebUI 期间需要保持启动终端运行。

可在另一个终端停止或重启已登记的 WebUI 服务：

```powershell
.\target\release\fpw.exe web stop
.\target\release\fpw.exe web restart
.\target\release\fpw.exe web restart --host 127.0.0.1 --port 4769
```

`restart` 默认复用上一次记录的 host 和 port，也可以显式覆盖。

### 自动生成发布包

```powershell
.\scripts\package-release.ps1
```

脚本自动读取项目版本、构建 WebUI 和 Release CLI，并生成 `release\FPW-v0.0.2.zip`。使用 `-SkipBuild` 可以基于已有的 `target\release\fpw.exe` 和 `web\dist` 快速重新打包。

## CLI 命令

```text
fpw validate <workflow.fwp>
fpw preview <workflow.fwp>
fpw run <workflow.fwp> [--input name=path] [--output name=path] [--report-dir path]
fpw config [--output workflow.fwp]
fpw web [--host 127.0.0.1] [--port 4769]
fpw web stop
fpw web restart [--host 127.0.0.1] [--port 4769]
fpw import-ffc <workflow.ffc> --output <workflow.fwp>
fpw recent list
fpw recent add <workflow.fwp>
```

可以重复使用 `--input` 和 `--output` 覆盖工作流中同名的输入输出路径：

```powershell
.\target\release\fpw.exe run workflows\release.fwp `
  --input firmware=C:\firmware\app.bin `
  --output image=C:\firmware\release.bin `
  --report-dir C:\firmware\reports
```

工作流内部声明的相对路径以 `.fwp` 文件所在目录为基准。CLI 的输入输出覆盖路径以及报告目录，如果使用相对路径，则以当前进程目录为基准。

## WebUI 使用流程

![FPW 中文工作流库](docs/images/webui-library-zh.png)

WebUI 将操作划分为三个任务区域：

1. 在工作流文件库中管理 `.fwp` 文件。
2. 通过五阶段向导创建或编辑工作流。
3. 选择已保存的工作流，预览并执行。

Run Preview 会生成与当前工作流和路径覆盖一致的 `fpw run` 命令。可以复制到另一个已安装 FPW 的终端执行。如果 `fpw` 不在 `PATH` 中，将命令开头替换为 `.\target\release\fpw.exe`。

![FPW 中文运行预览和 CLI 命令](docs/images/webui-run-preview-zh.png)

WebUI 默认把受管工作流保存在 `workflows/`。归档操作会将文件移动到 `workflows/.trash/`，不会永久删除。

环境变量：

- `FPW_WORKFLOW_HOME`：覆盖 WebUI 工作流文件库目录。
- `FPW_CONFIG_HOME`：覆盖最近项目等本地状态的保存目录。

## 执行报告

每次成功执行 `fpw run` 都会生成 JSON 和 TXT 报告，默认写入当前进程目录下的 `fpw-reports/`。

报告包含：

- FPW 版本和工作流标识
- 工作流 SHA256
- 可复现的命令参数
- 开始时间、结束时间和执行耗时
- 每个步骤的状态和错误
- 输入输出文件大小和 SHA256

## 仓库结构

```text
crates/fpw-core/   工作流模型、校验、执行和报告
crates/fpw-cli/    CLI 和本地 Web 服务
web/               React/Vite WebUI
docs/              Schema 和架构文档
examples/          示例工作流和二进制输入
workflows/         WebUI 默认管理的工作流文件库
```
