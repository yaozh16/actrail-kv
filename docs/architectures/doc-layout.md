<!-- 本文件定义项目文档树的职责边界与当前落点规则。 -->
# Documentation Layout

## 原则

- `docs/` 描述项目的当前累计状态，不以功能版本为目录或文件名。
- `dev/requirements/<version>/` 保存特定版本的需求、验收范围和已失效决策索引。
- `dev/designs/<version>/` 保存该版本的设计索引及开发期实现设计；索引可以指向 `docs/` 中仍然有效的当前架构文档。
- 一个文档只说明一种边界：运行配置、代码布局、分析器流水线、分析器输出或跨二进制约束。避免用泛化文件承载所有内容。

## 当前文档树

```text
README.md                                      # 构建、三二进制启动方式与用户入口
docs/
├── capabilities.md                             # 面向客户的可识别问题、报告价值与使用条件
├── configuration.md                            # 实际 CLI 参数、固定阈值、资源预算与部署参数
├── deployment/                                # 上游接入与全链路部署操作
│   ├── fullchain-deployment-guide.md           # Linux/Windows 部署指南与输入转换边界
│   ├── fullchain-deploy-linux.sh               # Linux 管理入口
│   ├── fullchain-deploy-linux/
│   │   └── report.sh                           # 报告生成与现行 Session 统计
│   └── fullchain-deploy-windows.ps1            # Windows 管理入口
├── concepts/                                   # 面向新手的稳定业务概念
│   ├── template.md                              # 请求模板、稳定片段、槽位及其分析产物映射
│   ├── context_defect.md                        # 上下文结构缺陷、正例及非缺陷边界
│   └── session-prefix.md                        # Session 相邻请求的前缀延续、分类与指标边界
└── architectures/
    ├── code-layout.md                          # 源码模块、三个二进制与调用关系
    ├── deployment.md                           # 外部采集入口、三个二进制与报告产物流向
    ├── assets/                                 # 架构图源文件及渲染图片
    ├── doc-layout.md                           # 本文档：文档树及职责边界
    ├── index.md                                # 三个二进制的架构入口
    ├── constraints.md                          # 跨 receiver/analyze/report 的运行与数据处理约束
    ├── receiver/                               # actrail-kv-receiver 专属设计
    │   ├── input.md                            # HTTP 接收协议
    │   └── output.md                           # requests.ndjson 行级协议
    ├── analyze/                                # actrail-kv-analyze 专属设计
    │   ├── input.md                            # requests.ndjson 及 payload 可分析边界
    │   ├── pipeline.md                         # 输入投影、模板抽取与诊断流水线
    │   └── output.md                           # analysis.json 协议、字段、示例与排序
    └── report/                                 # actrail-kv-report 专属设计
        ├── input.md                            # analysis.json 校验契约
        └── output.md                           # report.html 展示契约
dev/
├── requirements/                              # 版本需求和验收案例
│   ├── v0.1.0/                                # v0.1.0 已实现需求记录
│   ├── v0.1.1/
│   │   ├── target.md                          # Session 前缀延续与多位点分析目标
│   │   ├── feature-integration.md             # 固定版本、JSON 等价、候选采样、LCS 与 fixtures 集成要求
│   │   └── customer-capabilities.md           # 面向客户的能力说明文档要求
│   └── v0.1.2/
│       ├── target.md                          # 报告路径、兼容配置与交付维护要求
│       └── windows-integration.md             # Windows 文件共享、读取中止与部署兼容集成
└── designs/                                   # 版本设计索引与开发期实现设计
    ├── v0.1.0/                                # v0.1.0 设计记录
    ├── v0.1.1/
    │   ├── implementation.md                  # v0.1.1 设计索引
    │   └── session-prefix-and-multi-site.md   # Session 与多位点实现设计
    └── v0.1.2/
        └── implementation.md                  # 兼容修复设计、失效索引与验收结果
```

`dev/requirements/v0.1.0/` 和 `dev/designs/v0.1.0/` 同时保留候选相似度位移鲁棒记录，以及已由 v0.1.1 取代的 Session prefix-switch 历史记录；失效状态在对应文件内明确标注。

## 新文档落点

| 内容 | 落点 |
|---|---|
| 面向客户的当前能力、典型问题和使用条件 | `docs/capabilities.md` |
| 用户可操作的参数、默认值、部署设置 | `docs/configuration.md` |
| 上游接入步骤、全链路部署与管理脚本 | `docs/deployment/` |
| 面向新手的稳定业务概念 | `docs/concepts/` |
| 当前模块依赖、目录或二进制调用关系 | `docs/architectures/` |
| 某二进制的内部架构 | `docs/architectures/<binary>/` |
| 跨二进制的不可违反运行/数据约束 | `docs/architectures/constraints.md` |
| 对外或落盘产物的字段契约 | 产出该产物的二进制目录中的 `output.md` |
| 一次版本的需求、验收变化或已失效选择 | `dev/requirements/<version>/` 或 `dev/designs/<version>/` |
