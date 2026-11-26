<!-- 本文件说明原生动态库和 Rust SDK 的构建、调用及兼容升级方式。 -->
# 原生动态库与 Rust SDK

Rust SDK 通过 C ABI 加载原生库。receiver 持续写入采集存储，analyzer 按需筛选读取存储并返回内存分析结果，reporter 直接接受该结果并返回内存报告。文件导出由调用方显式触发。

## 构建与交付

```bash
cargo build --release \
  -p actrail-kv-sdk-receiver \
  -p actrail-kv-sdk-analyzer \
  -p actrail-kv-sdk-reporter
```

默认产物位于 `target/release/`：

| 文件 | 能力 |
| --- | --- |
| `libactrail_receiver.so` | 持久化采集 |
| `libactrail_analyzer.so` | 按需分析及分析结果导出 |
| `libactrail_reporter.so` | 内存报告渲染及报告导出 |

宿主依赖 `crates/sdk/rust` 中的 `actrail-kv-sdk`，其依赖图不包含采集、分析或报告核心实现。C 类型和函数声明见 [actrail.h](../../../crates/sdk/native/contracts/include/actrail.h)。动态库按目标 Linux 架构和系统运行库环境构建，只加载受信任且遵守 ABI 的库。

## Rust 调用顺序

1. 通过 `ReceiverFactory::create(config)` 创建持久采集实例，调用 `submit(&input)` 等待后端写入完成。
2. 通过 `AnalyzerFactory::create(config)` 创建分析实例，调用 `analyze(&selection)` 获取 `AnalysisOutput`。
3. 通过 `ReporterFactory::create(config)` 创建报告实例，调用 `render(&analysis, &options)` 获取 `ReportOutput`。
4. 根据需要读取 `analysis.json()` 或 `report.reports()`，或者调用输出对象的 `export(path)`。
5. 显式调用实例的 `close()` 取得关闭结果。输出对象可继续存活，析构时交还原库释放。

完整可运行示例见 [sdk_pipeline.rs](../../../crates/sdk/rust/examples/sdk_pipeline.rs)：

```bash
cargo run --release -p actrail-kv-sdk --example sdk_pipeline -- \
  target/release \
  /tmp/actrail-sdk-example
```

示例向存储提交四条记录，完成内存分析与渲染后显式导出 HTML。receiver 的写入不依赖导出操作。

## 配置与筛选

`SdkConfig.config_toml` 为 `None` 时读取默认 `~/.actrail/actrail-kv/settings.toml`，缺失配置会补齐写回。文件内的相对路径按文件目录解析，与 CLI 一致。`Some(text)` 使用传入的 TOML，相对路径按 `base_dir` 解析，`Some(String::new())` 使用默认值。显式导出路径始终以 `base_dir` 为基准。

`overrides` 接收 SDK 提供的强类型覆盖结构体。显式字段优先于基础配置，未设置字段继续继承，覆盖值不写回文件。例如只覆盖报告语言：

```rust
SdkOverrides {
    report: Some(ReportSettingsOverrides {
        locale: Some("zh-CN".into()),
        ..Default::default()
    }),
    ..Default::default()
}
```

嵌套配置按字段覆盖，数组整体替换。类型由配置定义生成，字段名与配置文件一致。

- receiver 使用 `[storage]`，默认 SQLite 数据库为配置基准目录下的 `captures.sqlite`。
- analyzer 使用 `[analyzer]` 和 `[storage]`。显式配置 `analyzer.input` 时读取该 NDJSON 文件。`analyzer.output` 不触发 SDK 自动写文件。
- reporter 使用 `[report]` 的格式与语言设置。直接渲染不使用 `report.input` 读取文件，也不使用 `report.output` 自动写文件。

`Selection` 只覆盖显式提供的筛选项。`session_keys` 和 `time_ranges` 的 `None` 继承配置，空数组清除对应限制。`RecordLimit::Inherit` 继承 `last_n`，`All` 清除限制，`Last(n)` 指定数量。时间边界使用 RFC3339，沿用存储和分析的半开区间语义。

`RenderOptions` 可覆盖格式和语言。格式缺省时沿用配置，配置也缺省则同时返回 HTML 和 Markdown。单格式输出使用 `export(path)`，多格式输出使用 `export_format(path, format)` 指定要写出的正文。

## 所有权与错误

所有方法均为阻塞调用，同一实例要求独占访问。异步宿主可把调用放入自己的阻塞任务池。运行时与后台维护任务由动态库管理，库不注册宿主进程的退出信号。

`AnalysisOutput` 保留完整原始 JSON。reporter 借用同一缓冲区调用原生入口，宿主不重新序列化分析对象。库内仍有线程传递所需的复制和 JSON 解析。原动态库保持加载，直到最后一个相关实例及输出释放。

导出结果不包含 SDK 响应信封。分析和报告输出保留来源保护信息，导出拒绝覆盖其采集存储或文件输入。不同来源的文件仍由调用方选择和管理。

失败通过 `SdkError` 返回。业务失败包含稳定状态码与 `ErrorDetail`。输入错误可修正后重试，提交不会自动重试，避免重复采集。内部 panic 后实例拒绝继续执行，调用方应关闭实例。

## 独立升级与验证

停止宿主后替换兼容 `.so`，再重启。无需重新编译或链接宿主。ABI 主版本需匹配，分析产物 schema 也必须与 reporter 兼容。CLI 的文件生成与读取流程保持不变。

```bash
cargo build --release --workspace
cargo test --release -p actrail-kv-sdk --test native
bash tests/end_to_end/sdk_upgrade.sh
```

原生集成测试使用当前测试构建目录中的动态库，也可通过 `ACTRAIL_SDK_LIB_DIR` 指定库目录。升级测试使用两份按公开 C 头文件编译的测试库，验证同一宿主可执行文件哈希不变、兼容替换生效、不兼容版本及缺失符号被拒绝。
