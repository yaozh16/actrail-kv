<!-- 本文件定义 Windows 兼容变更集成到 v0.1.2 的范围和验收要求。 -->
# Windows 兼容集成

## 状态

已完成集成与 Linux 本地验收；Windows 目标全 workspace、全目标编译检查通过，原生 Windows 和 PowerShell 5.1 运行待对应环境验证。详情见[实现设计](../../designs/v0.1.2/implementation.md#验收结果)。

## 范围

集成提交 `34de0d0` 的 Windows 文件共享、语料读取中止、PowerShell 5.1 与部署说明变更。

## 兼容要求

- 保持 v0.1.2 包版本和 schema v0.1.1。
- 保留报告裸文件名修复、废弃参数兼容语义和现行 Session 字段。
- 保留部署路径与工具链环境解析、中性文档名称及 Linux 报告子模块布局。
- 冲突按模块职责整合，保持两侧有效修复；不得恢复固定个人路径。

## 验收

- workspace 测试、格式检查、严格 Clippy 与三程序端到端验收。
- Linux 部署脚本语法及集成后的样例报告验证。
- Windows 目标检查与 PowerShell 验证按可用环境执行，明确未验证范围。
- 同步布局索引，检查文件长度、职责注释及用户批注。
