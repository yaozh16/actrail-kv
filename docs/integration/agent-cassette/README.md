<!-- 本文件描述 agent-cassette 场景的独立联调入口。 -->
# agent-cassette 接入

agent-cassette 可在代理转发时，把采集记录发送到 [Receiver HTTP 接口](../../reference/receiver/input.md)。采集、转发和上报配置由 agent-cassette 管理。

[Linux 联调脚本](fullchain-deploy-linux.sh)和 [Windows 联调脚本](fullchain-deploy-windows.ps1)可启动或复用两个进程并生成报告。运行脚本的 `--help` 可查看支持的操作。`CASSETTE_REPO` 和 `CASSETTE_CONF` 分别指定 agent-cassette 的代码和配置路径。Linux 部署需带上本目录的 `linux/` 辅助脚本。

actrail-kv 也可由其[通用部署命令](../../deployment/fullchain-deployment-guide.md)独立启动。
