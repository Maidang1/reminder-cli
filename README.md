# Reminder CLI

一个基于 Rust 的命令行提醒工具，支持 cron 表达式设置周期性提醒，并通过系统通知提醒用户。

## 安装

```bash
cargo build --release
# 可执行文件位于 ./target/release/reminder
```

## 使用方法

### 添加提醒

```bash
# 一次性提醒
reminder add -t "开会" -d "项目周会" -T "2025-12-25 10:00"

# 周期性提醒（使用 cron 表达式）
reminder add -t "每日站会" -c "0 0 9 * * *"      # 每天 9:00
reminder add -t "周报" -c "0 0 17 * * FRI"       # 每周五 17:00
```

Cron 表达式格式：`秒 分 时 日 月 星期`

### 列出所有提醒

```bash
reminder list
```

### 删除提醒

```bash
reminder delete -i <REMINDER_ID>
```

### 编辑提醒

```bash
reminder edit -i <REMINDER_ID> -t "新标题"
reminder edit -i <REMINDER_ID> -D "新描述"
reminder edit -i <REMINDER_ID> -T "2025-12-31 23:59"
reminder edit -i <REMINDER_ID> -c "0 30 8 * * *"
```

### 后台守护进程

```bash
# 启动守护进程（在后台监控提醒）
reminder daemon start

# 查看守护进程状态
reminder daemon status

# 停止守护进程
reminder daemon stop
```

## 数据存储

提醒数据存储在：
- macOS: `~/Library/Application Support/reminder-cli/reminders.json`
- Linux: `~/.local/share/reminder-cli/reminders.json`

## License

MIT
