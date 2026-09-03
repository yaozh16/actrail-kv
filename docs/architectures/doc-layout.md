<!-- 本文件定义项目文档树的职责边界与当前落点规则。 -->
# Documentation Layout

## 原则

- `docs/` 描述项目的当前累计状态，不以功能版本为目录或文件名。
- `dev/requirements/<version>/` 保存特定版本的需求、验收范围和已失效决策索引。
- `dev/designs/<version>/` 只保存该版本的设计索引；索引可以指向 `docs/` 中仍然有效的当前架构文档。
- 一个文档只说明一种边界：运行配置、代码布局、分析器流水线、分析器输出或跨二进制约束。避免用泛化文件承载所有内容。

## 当前文档树

```text
README.md                                      # 构建、三二进制启动方式与用户入口
docs/
├── configuration.md                            # 实际 CLI 参数、固定阈值、资源预算与部署参数
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
│   └── v0.1.1/
│       └── target.md                          # Session 前缀延续与多位点分析目标
└── designs/                                   # 版本设计索引与开发期实现设计
    ├── v0.1.0/                                # v0.1.0 设计记录
    └── v0.1.1/
        ├── implementation.md                  # v0.1.1 设计索引
        └── session-prefix-and-multi-site.md   # Session 与多位点实现设计
```

## 新文档落点

| 内容 | 落点 |
|---|---|
| 用户可操作的参数、默认值、部署设置 | `docs/configuration.md` |
| 面向新手的稳定业务概念 | `docs/concepts/` |
| 当前模块依赖、目录或二进制调用关系 | `docs/architectures/` |
| 某二进制的内部架构 | `docs/architectures/<binary>/` |
| 跨二进制的不可违反运行/数据约束 | `docs/architectures/constraints.md` |
| 对外或落盘产物的字段契约 | 产出该产物的二进制目录中的 `output.md` |
| 一次版本的需求、验收变化或已失效选择 | `dev/requirements/<version>/` 或 `dev/designs/<version>/` |
