<!-- 本文件定义 Windows 兼容变更集成到 v0.1.2 的范围和验收要求。 -->
# Windows 兼容集成

## 状态

已实现；Linux 验收与 Windows 目标编译检查结果见[实现设计](../../designs/v0.1.2/implementation.md#验收结果)。

## 范围

Windows 文件共享、语料读取中止、PowerShell 5.1 兼容及部署配置。

## 功能要求

- Windows 采集文件支持单写者和并发读者。
- 底层 I/O 读取失败立即返回错误及行号；坏 JSON 行记为 skipped。
- PowerShell 5.1 脚本提供启动、环境输出、健康检查、报告和停止命令。
- 包版本为 v0.1.2，分析产物使用 schema v0.1.1。

## 验收

- workspace 测试、格式检查、严格 Clippy 与三程序端到端验收。
- Linux 部署脚本语法及集成后的样例报告验证。
- Windows 目标编译检查、文件共享测试与 PowerShell 5.1 脚本验证。
- 同步布局索引，检查文件长度、职责注释及用户批注。
