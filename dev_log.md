# FPW - Firmware Packaging Workflow 开发记录

## 2026-07-15

### 项目背景

- 新项目名称：FPW - Firmware Packaging Workflow
- 功能对标项目：FirmwareFlow
  - GitHub：https://github.com/crazy0104/FirmwareFlow
- 初始界面形态：CLI + WebUI

### 对标项目初步理解

FirmwareFlow 是一个面向固件后处理的可视化工作流工具，核心能力包括固件合并、填充、删除、CRC、SHA256、RSA 签名、AES-CMAC、HMAC、AES 加解密、工作流保存加载，以及导出 C/CMake 工程或 Windows 可执行程序。

FPW 需要明确是否只是复刻 FirmwareFlow，还是在交互形态、跨平台、流水线集成、可扩展性、可部署方式上做新的产品边界。

### /grilling 第一轮问题

1. FPW 的核心用户是谁：固件工程师、测试工程师、DevOps/CI 维护者，还是安全发布负责人？
2. CLI 和 WebUI 的关系是什么：CLI 是核心能力，WebUI 调 CLI；还是 WebUI/后端共享同一套领域逻辑？
3. WebUI 是本地单机 WebUI，还是可多人访问的服务端 WebUI？
4. 是否必须兼容 FirmwareFlow 的 `.ffc` 工作流文件格式？
5. 第一版必须支持哪些固件处理步骤，哪些可以延期？
6. 是否需要生成独立可执行工具，还是只需要执行/导出脚本或工作流配置？
7. 目标平台是什么：Windows only，还是 Windows/Linux/macOS？
8. FPW 对 CI/CD 的要求是什么：是否要能在无 GUI、无交互环境中跑完整工作流？
9. 加密/签名密钥如何管理：明文路径、环境变量、密钥库、HSM/KMS，还是先不处理？
10. WebUI 是否需要用户、权限、项目空间、审计日志这些服务端能力？

### 已确认信息

- 第一目标用户：
  - 固件工程师
  - 产线人员
- MVP 阶段暂不区分固件工程师和产线人员角色，不做差异化权限与界面。

### /grilling 追问：用户角色边界

固件工程师和产线人员很可能需要两套不同体验：

1. 固件工程师是否负责“设计/编辑工作流”，产线人员只负责“选择版本并执行工作流”？
2. 产线人员是否允许修改参数，例如输入文件、输出路径、序列号、密钥、烧录批次号？
3. 产线执行时是否必须记录操作日志、输入文件哈希、输出文件哈希、操作者、时间、工位/设备编号？
4. 产线人员使用的是 CLI、WebUI，还是一个极简执行页面？
5. 如果执行失败，产线人员应该看到完整技术日志，还是只看到可操作的错误提示？

### 设计影响

- 暂不引入用户角色、权限矩阵、只读执行模式、产线专用极简页面。
- MVP 可以先采用统一工作台：同一用户既能编辑工作流，也能执行工作流。
- 后续如需产线模式，可再加入工作流锁定、参数白名单、执行审计和只执行页面。

### 已确认架构方向

- WebUI 形态：本机 WebUI，通过本机浏览器访问。
- WebUI 启动方式：`fpw web` 启动本地服务，然后浏览器访问。
- CLI：必须可以独立运行完整工作流，不依赖 WebUI。
- 产品形态：FPW 是“一个命令行工具 + 一个 Web 管理界面”。
- 目标平台：Windows + Linux。
- CLI 最小命令集接受以下方向：
  - `fpw run workflow.fwp --input ... --output ...`
  - `fpw validate workflow.fwp`
  - `fpw preview workflow.fwp`
  - `fpw web`
- 工作流文件后缀：`.fwp`
- WebUI 编辑出的工作流必须能被 CLI 100% 执行。

### 设计影响

- CLI 应作为核心执行入口，支持无 GUI、无浏览器环境运行。
- WebUI 应作为工作流编辑、管理、预览、执行辅助界面。
- 工作流解析、校验、执行、步骤注册等核心逻辑应避免只存在于前端。
- 第一版不得绑定 Windows-only 能力，例如 PowerShell、`.bat`、Windows 注册表、Tauri-only 桌面壳。
- 路径、报告目录、最近项目目录和本机 Web 服务需要同时适配 Windows 与 Linux。
- 后续架构更适合拆成：
  - `fpw` CLI
  - `fpw-core` 工作流模型/校验/执行核心
  - `fpw-web` 本机 WebUI

### /grilling 追问：工作流格式与执行模型

1. `.fwp` 文件是否使用 JSON，方便 diff、版本管理和人工排错？
2. `.fwp` 是否需要导入 FirmwareFlow 的 `.ffc`，还是完全不兼容也可以？
3. 工作流执行时输入输出参数应该按“命名参数”传递，还是像 FirmwareFlow 一样按 CLI 位置顺序传递？
4. 工作流里的文件路径是否允许绝对路径，还是推荐相对工作流文件目录？
5. 失败策略是什么：任一步失败立即停止，还是允许某些步骤失败后继续？

### 已确认工作流格式与执行模型

- `.fwp` 使用 JSON 格式。
- 支持导入 FirmwareFlow `.ffc`。
- CLI 输入输出使用命名参数传递，例如 `--input firmware=app.bin --output signed=out.bin`。
- 工作流里的路径默认可以相对 `.fwp` 文件所在目录。
- 执行失败策略：任一步失败立即停止。

### 设计影响

- 需要定义 FPW 自有 JSON schema，同时实现 `.ffc -> .fwp` 的导入转换器。
- CLI 参数解析需要支持多值命名映射。
- 执行引擎需要有统一的工作目录解析规则。
- 每个步骤需要明确输出产物名，供后续步骤和 CLI 输出映射引用。

### /grilling 追问：MVP 步骤范围

1. MVP 是否必须支持 FirmwareFlow 当前全部步骤？
2. 如果不能全部做，第一版最小步骤集合是否可以是：`input`、`output`、`merge`、`fill`、`crc32`、`sha256`？
3. RSA 签名是否 MVP 必须有？
4. AES-CMAC、HMAC-SHA256、AES-CTR/CBC/ECB 是否 MVP 必须有？
5. 是否需要支持 Intel HEX / Motorola SREC，还是第一版只处理原始 `.bin`？

### 已确认 MVP 步骤范围

- 第一版做最小闭环，不要求覆盖 FirmwareFlow 全部能力。
- MVP 步骤集合：
  - `input`
  - `output`
  - `merge`
  - `fill`
  - `insert`
  - `crc32`
  - `sha256`
- 第一版不要求 RSA 签名。
- 第一版不要求 AES-CMAC、HMAC-SHA256、AES-CTR、AES-CBC、AES-ECB。
- 第一版只处理原始 `.bin` 文件，不支持 Intel HEX / Motorola SREC。

### 设计影响

- MVP 可先实现字节数组级别的文件处理引擎。
- 地址/offset 语义需要统一为 `.bin` 文件偏移，不处理地址记录格式。
- 后续扩展 HEX/SREC 时，需要在 IO 层引入镜像地址空间模型。
- 加密签名步骤可以作为插件化步骤或后续内置步骤扩展。

### /grilling 追问：MVP 输出与项目边界

1. MVP 是否需要像 FirmwareFlow 一样“生成 C/CMake 工程”？
2. MVP 是否需要“生成独立可执行程序”，还是 CLI 本身执行 `.fwp` 就够？
3. WebUI 第一版是否只需要编辑、保存、加载、预览、执行工作流？
4. 是否需要项目列表/最近项目，还是先只支持打开单个 `.fwp`？
5. 是否需要导出执行报告，例如 JSON/HTML/TXT 日志？

### 已确认 MVP 输出与项目边界

- 第一版不生成 C/CMake 工程。
- 第一版不生成每个工作流专属的独立可执行程序。
- `fpw` CLI 本身作为独立可执行程序使用：
  - `fpw config`：通过向导生成 `.fwp` 配置文件。
  - `fpw run xxx.fwp`：执行配置动作。
- WebUI 第一版支持：
  - 编辑工作流
  - 保存工作流
  - 加载工作流
  - 预览工作流
  - 执行工作流
- 第一版需要最近项目/项目列表。
- 第一版需要导出执行报告，例如 JSON/TXT 日志。

### 设计影响

- MVP 不需要代码生成器、CMake 模板、工具链管理、OpenSSL 打包。
- 核心复杂度集中在工作流 schema、CLI、执行引擎、WebUI 编辑器、报告生成。
- `fpw config` 需要设计交互式向导，也可能需要非交互参数模式方便脚本化。
- 报告系统需要从第一版就统一记录输入、输出、步骤、耗时、hash、错误信息。

### /grilling 追问：执行报告与可追溯

1. 执行报告默认保存在哪里：工作流同目录、用户指定目录，还是当前执行目录？
2. 报告格式第一版是否只做 JSON + TXT？
3. 报告是否必须包含输入/输出文件 SHA256？
4. 是否需要记录 FPW 版本、工作流文件 hash、执行命令、开始/结束时间？
5. 最近项目/项目列表是只保存在本机，还是跟 `.fwp` 项目目录绑定？

### 已确认执行报告与可追溯默认值

- 执行报告默认保存到当前执行目录下的 `fpw-reports/`。
- CLI 支持通过 `--report-dir <dir>` 覆盖报告目录。
- 报告格式第一版支持：
  - JSON：机器可读，适合 CI、追溯、后续导入。
  - TXT：人类可读摘要，适合现场快速查看。
- 报告默认包含输入文件和输出文件 SHA256。
- 报告默认记录：
  - FPW 版本
  - 工作流文件路径
  - 工作流文件 SHA256
  - 执行命令
  - 开始时间
  - 结束时间
  - 总耗时
  - 每个步骤的状态、输入、输出、耗时、错误信息
- 最近项目/项目列表只保存在本机用户目录，不写入 `.fwp` 项目目录。

### 设计影响

- CLI 的 `run` 命令需要默认生成报告，并提供关闭或覆盖选项。
- WebUI 执行工作流后应展示报告路径和摘要。
- 最近项目存储应使用平台用户数据目录，例如 `~/.config/fpw/recent-projects.json` 或等价平台目录。

### /grilling 追问：技术栈与架构落地

1. CLI/core 推荐使用 Rust，可以得到单文件可执行程序、强类型、跨平台和较好的二进制处理性能，是否接受？
2. WebUI 推荐使用 React + Vite，本地由 `fpw web` 内嵌或启动静态资源服务，是否接受？
3. 工作流编辑器是否继续采用节点画布形态，还是第一版先用表单/步骤列表降低复杂度？
4. `.fwp` schema 是否需要从第一版就版本化，例如 `{ "schemaVersion": 1, ... }`？
5. `.ffc` 导入是否只做 best-effort 转换到 MVP 支持步骤，不支持的 RSA/AES 步骤给出明确 warning？

### 已确认技术栈与架构默认值

- CLI/core 使用 Rust。
- WebUI 使用 React + Vite。
- `fpw web` 启动本机 HTTP 服务并提供 WebUI。
- MVP 第一版工作流编辑器优先采用步骤列表/表单，不做节点画布。
- `.fwp` schema 从第一版开始版本化，例如 `schemaVersion: 1`。
- `.ffc` 导入采用 best-effort 策略：
  - MVP 支持步骤尽量转换。
  - RSA/AES/HMAC/HEX/SREC 等不支持能力给出明确 warning。
  - 导入结果必须可保存为 `.fwp`。

### 设计影响

- 第一版不引入复杂画布交互，可更快完成 CLI/WebUI 闭环。
- Rust crate 可以拆成 core、cli、web server 三层，避免 WebUI 和 CLI 逻辑分叉。
- `.fwp` schema 必须稳定定义输入、输出、步骤、参数、路径解析和版本字段。
- `.ffc` 导入器需要输出 warnings，WebUI 和 CLI 都应能展示。

### /grilling 追问：步骤语义细节

1. `fill` 对 `.bin` 的 offset 范围使用闭区间 `start:end`，还是半开区间 `start:length`？
2. `crc32` 使用哪种 CRC32 变体：默认推荐 IEEE CRC-32，poly `0x04C11DB7`，init `0xFFFFFFFF`，xorout `0xFFFFFFFF`，refin/refout true，可以吗？
3. `crc32` 写入字节序默认 little-endian，可以吗？
4. `sha256` 是输出 digest 文件，还是把 digest 写入原 bin 的指定 offset？MVP 推荐两个步骤：`sha256` 输出 digest，后续用 `insert`/`write` 再写入。
5. `merge` 遇到重叠范围时默认报错，可以吗？

### 已确认步骤语义默认值

这些语义作为第一版默认值记录，后续可以通过 schema 升级或步骤参数扩展修改。

- `fill`
  - 只处理 `.bin` 字节偏移。
  - 范围使用 `offset + length` 半开区间语义。
  - 示例：`offset=0x100`、`length=16` 表示写入 `[0x100, 0x110)`。
  - 默认填充值建议为 `0xFF`。
- `insert`
  - 将一个二进制文件插入或覆盖到目标 `.bin` 的指定 offset。
  - MVP 默认语义：覆盖写入，不移动后续字节。
  - 如果写入范围超过目标文件末尾，默认扩展文件并用 `0xFF` 填充空洞。
- `merge`
  - 合并多个 `.bin` 片段或两个输入文件。
  - MVP 默认遇到重叠范围时报错，不静默覆盖。
  - 后续可增加 `overwrite`、`priority` 等策略。
- `crc32`
  - 默认 CRC32 变体：IEEE CRC-32。
  - 参数：
    - poly：`0x04C11DB7`
    - init：`0xFFFFFFFF`
    - xorout：`0xFFFFFFFF`
    - refin：`true`
    - refout：`true`
  - 写入字节序默认 little-endian。
  - 默认语义：计算指定范围的 CRC32，并写入目标 offset。
- `sha256`
  - MVP 默认只输出 digest 文件。
  - 不直接写回 `.bin`。
  - 如需写入 digest，后续通过 `insert` 或新增 `write` 步骤实现。
- `input`
  - 定义命名输入，例如 `firmware`、`boot`、`app`。
  - CLI 使用 `--input name=path` 传递。
- `output`
  - 定义命名输出，例如 `image`、`digest`。
  - CLI 使用 `--output name=path` 传递。

### 第一版暂不实现功能清单

为了方便后续版本迭代，第一版明确不做以下功能：

- 节点画布式工作流编辑器。
- C/CMake 工程生成。
- 按工作流生成专属独立 `.exe`。
- RSA PKCS#1 v1.5 签名。
- RSA-PSS 签名。
- AES-CMAC。
- HMAC-SHA256。
- AES-CTR。
- AES-CBC。
- AES-ECB。
- Intel HEX 支持。
- Motorola SREC 支持。
- 多用户/权限/角色区分。
- 产线专用只执行页面。
- 工具链管理。
- OpenSSL 打包。
- HSM/KMS/密钥库集成。
- 复杂失败策略，例如失败后继续执行。
- 分支、条件、循环类工作流。

### /grilling 追问：下一步落地

1. 是否现在开始创建 Rust + React 项目骨架？
2. 是否先实现 CLI/core，再实现 WebUI？
3. 是否先写 `.fwp` schema 文档和示例文件，再写执行器？

### 已落地项目骨架

- 已创建 README 和 `.gitignore`。
- 已创建 `.fwp` schema 文档：
  - `docs/fwp-schema-v1.md`
- 已创建示例工作流：
  - `examples/fill-crc-sha.fwp`
  - `examples/merge-insert.fwp`
- 已创建示例 fixture：
  - `examples/fixtures/app.bin`
  - `examples/fixtures/boot.bin`
  - `examples/fixtures/metadata.bin`
- 已创建 Rust workspace：
  - `crates/fpw-core`
  - `crates/fpw-cli`
- `fpw-core` 已包含：
  - workflow model
  - JSON 加载
  - schema version 校验
  - MVP 步骤校验
  - MVP 步骤执行：`input`、`output`、`fill`、`insert`、`merge`、`crc32`、`sha256`
  - JSON/TXT 报告结构与写出
  - `.ffc` 导入占位模块
- `fpw-cli` 已包含命令入口：
  - `fpw validate`
  - `fpw preview`
  - `fpw run`
  - `fpw config`
  - `fpw web`
- `fpw web` 已实现最小跨平台本地 HTTP 服务占位页，后续接入 React 静态资源与 API。
- 已创建 React + Vite WebUI scaffold：
  - `web/`
  - MVP 先提供 JSON 编辑与步骤列表预览。

### 当前验证状态

- 已使用 Node 校验以下 JSON 文件可解析：
  - `web/package.json`
  - `web/tsconfig.json`
  - `examples/fill-crc-sha.fwp`
  - `examples/merge-insert.fwp`
- 当前环境缺少 Rust 工具链，尚未运行 `cargo build`、`cargo test`、`cargo fmt`。
- 当前环境未安装 Web 依赖，尚未运行 `npm install`、`npm run build`。

### 继续落地记录

- 已补充架构文档：
  - `docs/architecture.md`
- 已强化 `fpw run` 行为：
  - 执行失败时仍生成报告。
  - 报告状态为 `failed` 时 CLI 返回错误，避免失败被误判为成功。
- 已补充 core 单元测试源码，覆盖：
  - `fill` 半开区间写入。
  - `insert` 覆盖写入。
  - `crc32` little-endian 写入。
  - `sha256` 输出 32 字节 digest。
  - `merge` 重叠范围报错。
- 已扩展 `fpw web` 最小本地 HTTP 服务：
  - `GET /`
  - `GET /api/health`
- 已收紧 schema 文档，移除当前 Rust model 尚未实现的顶层 `outputs` 示例字段。

### 下一步建议

1. 安装 Rust 工具链后运行：
   - `cargo fmt`
   - `cargo test`
   - `cargo build -p fpw-cli`
2. 根据编译结果修正 Rust 细节。
3. 接入 `fpw web` 静态资源服务，让 CLI 可以直接服务 `web/dist`。
4. 增加 WebUI 调用本地 API 的 validate/preview/run 能力。

### 继续下一步落地记录

- 已新增最近项目 core 模块：
  - `crates/fpw-core/src/recent.rs`
  - Windows 默认目录：`%APPDATA%/fpw`
  - Linux 默认目录：`$XDG_CONFIG_HOME/fpw` 或 `~/.config/fpw`
- 已扩展 CLI：
  - `fpw import-ffc <source.ffc> --output <target.fwp>`
  - `fpw recent list`
  - `fpw recent add <workflow.fwp>`
- `fpw run` 执行后会写入最近项目列表。
- 已扩展 `fpw web` API：
  - `GET /api/health`
  - `GET /api/recent-projects`
  - 未实现的 `/api/*` 返回 JSON 404。
- 已新增 WebUI API client：
  - `web/src/api.ts`
- WebUI 侧栏已接入：
  - 服务状态
  - 最近项目列表
- 已更新文档：
  - `README.md`
  - `docs/architecture.md`
  - `docs/fwp-schema-v1.md`

### 后续待实现

- WebUI 仍需接入 validate/preview/run API。
- 最近项目 API 目前只支持读取，后续需要支持添加/删除。

### 继续实现记录：FFC 导入与静态资源服务

- `.ffc` 导入已从占位推进为 best-effort 转换：
  - `input` -> `input`
  - `output` -> `output`
  - `fill` -> `fill`
  - `crc` -> `crc32`
  - `sha256-bin` -> `sha256`
  - `sha256` -> `sha256` + `insert`，并给 warning
  - `insert` -> `insert`
  - `merge` -> `merge`，并给 range slicing warning
  - RSA/AES/HMAC 等不支持步骤跳过并 warning
- 已补充 `.ffc` 导入单元测试源码。
- `fpw web` 已支持服务 `web/dist` 静态资源。
- 如果 `web/dist` 不存在，`fpw web` 回退到内置占位页。

### 当前仍待验证

- 因当前环境缺少 Rust 工具链，上述 Rust 改动仍需 `cargo fmt/test/build` 验证。

### 环境安装与验证完成记录

- 已通过 apt 安装 Rust 工具链：
  - `rustc 1.93.1`
  - `cargo 1.93.1`
  - `rustfmt 1.8.0`
- 已安装 WebUI npm 依赖：
  - `npm install`
- Rust 验证已完成：
  - `cargo fmt --check`
  - `cargo test`
  - `cargo build -p fpw-cli`
- Web 验证已完成：
  - `npm run build`
- CLI 实测已完成：
  - `fpw validate examples/fill-crc-sha.fwp`
  - `fpw preview examples/fill-crc-sha.fwp`
  - `fpw config --output /tmp/fpw-config-test.fwp`
  - `fpw run examples/fill-crc-sha.fwp`
  - `fpw run examples/merge-insert.fwp`
  - `fpw import-ffc examples/firmwareflow-basic.ffc --output /tmp/firmwareflow-basic.fwp`
  - `fpw validate /tmp/firmwareflow-basic.fwp`
  - `fpw run /tmp/firmwareflow-basic.fwp`
- Web 服务实测已完成：
  - `fpw web --host 127.0.0.1 --port 4770`
  - `GET /`
  - `GET /api/health`
  - `GET /api/recent-projects`

### 验证中发现并修复的问题

- `fpw run` 在沙箱环境中写 `~/.config/fpw` 失败：
  - 新增 `FPW_CONFIG_HOME` 覆盖本机配置目录。
  - 最近项目写入失败改为 warning，不阻断工作流执行。
- 示例 `fill-crc-sha.fwp` 的 CRC range 超过小 fixture 文件长度：
  - 已调整示例 offset/range，使其开箱可执行。
- CLI override 路径原本按 `.fwp` 文件目录解析：
  - 已改为 workflow 内路径相对 `.fwp`，CLI `--input/--output` override 相对当前工作目录。
- `fpw web` 原本只有占位页：
  - 已支持服务 `web/dist`。

## 2026-07-16

### WebUI 与核心执行引擎连接

- 新增本地工作流 API：
  - `POST /api/workflows/validate`
  - `POST /api/workflows/preview`
  - `POST /api/workflows/run`
- API 直接接收并执行浏览器当前编辑的 workflow JSON，避免运行磁盘上的旧版本。
- `run` API 支持：
  - 工作流基准路径，用于解析 workflow 内的相对路径。
  - 命名 input/output 路径覆盖。
  - 报告目录覆盖。
  - 返回完整执行报告、JSON/TXT 报告路径和 warning。
- 本地 HTTP 请求读取改为按 `Content-Length` 接收完整 body，并限制为 2 MiB。
- WebUI 已加入：
  - Validate、Preview、Run workflow 操作。
  - 工作流基准路径配置。
  - `name=path` 输入输出覆盖配置。
  - 步骤执行状态、文件信息、错误和报告路径展示。
- 新增 3 个 API 测试，覆盖校验、结构化错误、预览和真实工作流执行。

### WebUI 工作流创建与管理闭环

- WebUI 已重构为三个独立任务区：
  - 工作流库。
  - 五步创建/编辑向导。
  - 独立运行与报告页面。
- 默认工作流管理目录为 `workflows/`，可通过 `FPW_WORKFLOW_HOME` 覆盖。
- 新增工作流管理 API：
  - 列表、打开、创建、保存。
  - 复制和可恢复归档。
  - 导入 `.fwp` 和 FirmwareFlow `.ffc`。
- 所有库内目标路径必须是无 `..` 的相对 `.fwp` 路径。
- 归档操作把文件移动到工作流库的 `.trash/`，不做永久删除。
- 创建向导阶段：
  1. 基本信息。
  2. 输入定义。
  3. `fill`、`insert`、`merge`、`crc32`、`sha256` 处理步骤。
  4. 输出定义。
  5. 核心校验、预览和保存。
- JSON 编辑器保留在最终检查阶段的高级模式中。
- 独立运行页根据工作流声明自动生成输入和输出表单，留空时保留 `.fwp` 的相对路径语义。
- 新增工作流存储单元测试，并完成管理 API 全流程端到端验证。

### WebUI 中英双语

- WebUI 支持 English 和简体中文即时切换。
- 新用户首次打开默认使用 English。
- 用户选择保存在浏览器 `localStorage` 的 `fpw-language` 中。
- 翻译范围覆盖工作流库、创建向导、步骤表单、运行页、结果页以及确认提示。
- 工作流名称、路径、步骤 ID、artifact、报告和后端错误保持原始数据，不做翻译。

### Run Preview CLI command

- Run Preview displays a copyable `fpw run` command alongside the execution preview.
- The command contains the selected managed workflow path, non-empty input/output overrides, and report directory.
- Web execution reports now store the same canonical CLI argument structure.
