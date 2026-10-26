<!-- 本文件是 v0.1.0 设计索引；实现契约及用户配置分别维护在其链接的独立文档中。 -->
# v0.1.0 实现设计索引

## 需求与验收

- [MVP 需求及验收案例](../../requirements/v0.1.0/llm-kv-structure-diagnostics-mvp.md)
- [Analyzer 统一缺陷模型重写要求](../../requirements/v0.1.0/analyzer-rewrite.md)
- [会话上下文刷新检测需求](../../requirements/v0.1.0/session-context-refresh-detection.md)
- [HTML 与 Markdown 双报告需求](../../requirements/v0.1.0/report-markdown-output.md)
- [Report Locale 需求](../../requirements/v0.1.0/report-locale.md)
- [HTML 编译期模板需求](../../requirements/v0.1.0/report-html-template.md)

## 实现设计

- [Analyzer 统一缺陷模型重写设计](analyzer-rewrite.md)
- [会话上下文刷新检测设计](session-context-refresh-detection.md)
- [HTML 与 Markdown 双报告设计](report-markdown-output.md)
- [Report Locale 设计](report-locale.md)
- [HTML 编译期模板设计](report-html-template.md)

## 架构与实现契约

- [代码布局](../../../docs/architectures/code-layout.md)：模块边界、三个二进制及调用顺序。
- [架构入口](../../../docs/architectures/index.md)：当前累计的输入投影、模板抽取、诊断、聚合、排序和运行约束。

## 运行配置

- [配置参考](../../../docs/configuration.md)：命令行参数、默认算法阈值、资源预算及部署前提。

## 失效设计索引

- [`report-markdown-output.md`](report-markdown-output.md) 中“HTML renderer 现有逻辑保持不变”的
  迁移约束已由[编译期模板设计](report-html-template.md)取代；HTML 输出契约仍有效。
