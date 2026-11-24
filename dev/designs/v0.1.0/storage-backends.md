<!-- 本文件索引采集存储的当前架构及取代的版本决策。 -->
# 采集存储设计

- [采集存储架构](../../../docs/architectures/storage/index.md)
- [采集存储配置](../../../docs/configuration.md#采集存储)

## 决策索引

- [配置文件化设计](config-file.md)中 Receiver `--output` 必填的决策已更新。显式配置存储后端时，Receiver 从 `--storage-config` 获取目标。
- [分层存储配置](../../../docs/configuration.md#采集存储)取代原有的扁平 JSON 结构。后端选择和后端专属配置分别位于 `storage.backend` 及 `storage.backends.<backend>`。
