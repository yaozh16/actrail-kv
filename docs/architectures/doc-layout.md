```text
README.md # 项目定位、核心能力、快速试用与文档入口
docs/ # 项目当前状态的文档
├── architecture/
│   └── config-layout.md [x] # 迁移目标配置项、业务结构体、默认值和必填条件
├── configuration.md # 统一设置、CLI 覆盖、资源预算与部署参数
├── deployment/
│   └── fullchain-deployment-guide.md # actrail-kv 自身部署和统一接收协议的验证
├── concepts/ # 面向新手的稳定业务概念
│   ├── template.md # 请求模板、稳定片段、槽位及其分析产物映射
│   ├── context_defect.md # 上下文结构缺陷、正例及非缺陷边界
│   └── prefix_reuse.md # 结构复用率、可恢复上界及其口径边界
├── reference/ # 公共接口和产物契约
│   ├── index.md # 命令接口入口
│   ├── receiver/ # HTTP 上报协议和采集记录格式
│   ├── analyze/ # 分析语料、筛选和 analysis.json 格式
│   └── report/ # 分析结果输入和双格式报告
├── integration/ # 可独立选取的供给方接入材料
│   └── <provider>/ # 某一个供给方的图、说明和联调脚本
└── architectures/ # 内部设计和模块职责
    ├── code-layout.md # 源码模块、命令与调用关系
    ├── deployment.md # 外部采集入口与报告产物流向
    ├── doc-layout.md # 文档树及职责边界
    ├── index.md # 二进制及共有架构入口
    ├── constraints.md # 跨 receiver/analyze/report 的运行与数据处理约束
    ├── storage/ # Receiver、Analyzer 和 Export 的共享存储
    │   └── index.md # 接口、工厂、读取边界和保留期
    └── analyze/ # actrail-kv-analyze 专属设计
        └── pipeline.md # 输入投影、模板抽取与诊断流水线
dev/
├── requirements/<version>/ # 版本需求、验收案例和已失效决策索引
└── designs/<version>/ # 指向当前架构正文的版本设计索引
```
