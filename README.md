# Reminder CLI

一个基于 Rust 的命令行提醒工具，支持 cron 表达式和自然语言设置周期性提醒，并通过系统通知提醒用户。

## 安装

### Homebrew (macOS/Linux)

```bash
brew tap Maidang1/tap
brew install reminder-cli
```

### Cargo

```bash
cargo install reminder-cli
```

### 从源码构建

```bash
git clone https://github.com/Maidang1/reminder-cli
cd reminder-cli
cargo build --release
```

## 使用方法

安装后提供两个命令：`reminder` 和短命令 `rem`，功能完全相同。

### 添加提醒

```bash
# 一次性提醒 - 绝对时间
rem add -t "开会" -d "项目周会" -T "2025-12-25 10:00"

# 一次性提醒 - 相对时间
rem add -t "休息" -T "30m"          # 30分钟后
rem add -t "午餐" -T "2h"           # 2小时后
rem add -t "明天" -T "1d"           # 1天后

# 一次性提醒 - 自然语言
rem add -t "开会" -T "tomorrow 9am"
rem add -t "汇报" -T "next monday 14:00"
rem add -t "下班" -T "today 18:00"

# 周期性提醒 - 标准 cron 格式
rem add -t "每日站会" -c "0 0 9 * * *"

# 周期性提醒 - 英文描述（自动转换为 cron）
rem add -t "喝水" -c "every hour"
rem add -t "站会" -c "every day at 9am"
rem add -t "周会" -c "every monday at 10am"
rem add -t "工作提醒" -c "every weekday at 8:30"
rem add -t "休息" -c "every 30 minutes"

# 带标签的提醒
rem add -t "开会" -T "tomorrow 9am" --tags work,important
```

### 列出提醒

```bash
rem list                  # 列出所有活跃提醒
rem list --all            # 包括已完成的
rem list --tag work       # 按标签筛选
```
<img width="950" height="280" alt="image" src="https://github.com/user-attachments/assets/0eaa8569-1ee7-41ab-9dc0-d74cefda65b4" />


### 查看提醒详情

```bash
rem show 1946    # 使用短 ID 即可
```

### 暂停/恢复提醒

```bash
rem pause 1946   # 暂停提醒
rem resume 1946  # 恢复提醒
```

### 编辑提醒

```bash
rem edit -i 1946 -t "新标题"
rem edit -i 1946 -D "新描述"
rem edit -i 1946 -T "2025-12-31 23:59"
rem edit -i 1946 -c "every day at 10am"
rem edit -i 1946 --add-tags urgent
rem edit -i 1946 --remove-tags work
```

### 删除提醒

```bash
rem delete -i 1946
```

### 标签管理

```bash
rem tags    # 列出所有标签及数量
```

### 清理已完成的提醒

```bash
rem clean
```

### 后台守护进程

```bash
rem daemon start     # 启动守护进程
rem daemon status    # 查看状态（包括健康检查）
rem daemon stop      # 停止守护进程
rem daemon install   # 安装开机自启（macOS/Linux）
```

### 导入/导出

```bash
rem export -o backup.json    # 导出
rem import -i backup.json    # 导入
rem import -i backup.json -f # 导入并覆盖重复
```

## 时间格式

### 一次性提醒 (-T)

| 格式 | 示例 |
|------|------|
| 绝对时间 | `2025-12-25 10:00` |
| 相对时间 | `30m`, `2h`, `1d`, `1w` |
| 自然语言 | `tomorrow 9am`, `next monday 14:00`, `today 18:00` |

### 周期性提醒 (-c)

| 格式 | 示例 |
|------|------|
| 标准 cron | `0 0 9 * * *` (秒 分 时 日 月 星期) |
| 英文描述 | `every hour`, `every day at 9am`, `every monday at 10am` |

## 数据存储

- macOS: `~/Library/Application Support/reminder-cli/`
- Linux: `~/.local/share/reminder-cli/`
- Windows: `%LOCALAPPDATA%\reminder-cli\`

## License

MIT
