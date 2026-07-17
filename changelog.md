# Changelog

FPW 的重要版本变更记录在此文件中。

## v0.0.2 - 2026-07-17

### Added

- 新增 `fpw web stop`，用于停止已登记的本地 WebUI 服务。
- 新增 `fpw web restart`，默认复用上一次记录的 host 和 port，并支持显式覆盖。
- 新增 Web 服务 PID、host、port 和版本登记文件。
- 新增 `scripts/package-release.ps1` Windows 自动构建和打包脚本。
- 新增中英文 README、用户手册和 WebUI 截图。
- 新增 `changelog.md` 版本迭代记录。
- WebUI 引入 `lucide-react` 图标库，为导航、工作流管理、向导操作、执行控制和状态反馈增加语义化图标。

### Fixed

- Windows 下的 `web stop/restart` 改用原生进程 API，不再依赖本地化的 `tasklist` 输出和外部 `taskkill` 命令。
- 修复 WebUI Run Preview 缺少可复制 CLI 命令的问题。

### Changed

- Release 包统一命名为 `FPW-v0.0.2.zip`。
- `fpw-web-output/` 和 `release/` 加入 Git 忽略规则。
- WebUI 归档操作改为无文字的垃圾桶图标按钮，导入界面仅提供 FPW `.fwp` 格式。

## v0.0.1 - 2026-07-16

### Added

- 首个可用版本。
- 支持 `.fwp` 工作流校验、预览和执行。
- 支持 `input`、`output`、`fill`、`insert`、`merge`、`crc32` 和 `sha256` 步骤。
- 提供本地 WebUI、五阶段工作流创建向导和工作流文件库。
- 支持英文和简体中文界面。
- 支持 JSON/TXT 执行报告和 FirmwareFlow `.ffc` 尽力导入。
