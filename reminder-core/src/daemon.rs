use super::notification::send_notification;
use super::storage::Storage;
use super::{log_debug, log_error, log_info};
use anyhow::{Context, Result};
use chrono::Local;
use std::fs;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

const FILE_CHECK_INTERVAL_SECS: u64 = 5;
const HEARTBEAT_INTERVAL_SECS: u64 = 30;
const HEARTBEAT_TIMEOUT_SECS: u64 = 120;

pub fn start_daemon() -> Result<()> {
    let pid_file = Storage::pid_file_path()?;

    if is_daemon_running()? {
        println!("Daemon is already running");
        return Ok(());
    }

    let exe = std::env::current_exe()?;

    let child = Command::new(exe)
        .arg("daemon")
        .arg("run")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Failed to start daemon process")?;

    fs::write(&pid_file, child.id().to_string())?;
    println!("Daemon started with PID: {}", child.id());

    Ok(())
}

pub fn stop_daemon() -> Result<()> {
    let pid_file = Storage::pid_file_path()?;

    if !pid_file.exists() {
        println!("Daemon is not running");
        return Ok(());
    }

    let pid_str = fs::read_to_string(&pid_file)?;
    let pid: i32 = pid_str.trim().parse()?;

    #[cfg(unix)]
    {
        let _ = Command::new("kill").arg(pid.to_string()).status();
    }

    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .status();
    }

    fs::remove_file(&pid_file)?;
    println!("Daemon stopped");

    Ok(())
}

pub fn daemon_status() -> Result<()> {
    let running = is_daemon_running()?;
    let healthy = is_daemon_healthy()?;

    if running {
        let pid_file = Storage::pid_file_path()?;
        let pid = fs::read_to_string(&pid_file)?;
        println!("Daemon is running (PID: {})", pid.trim());

        if healthy {
            println!("Health: OK (heartbeat active)");
        } else {
            println!("Health: WARNING (heartbeat stale - daemon may be stuck)");
        }

        // Show last heartbeat time
        if let Ok(heartbeat_path) = Storage::heartbeat_file_path() {
            if heartbeat_path.exists() {
                if let Ok(content) = fs::read_to_string(&heartbeat_path) {
                    if let Ok(timestamp) = content.trim().parse::<i64>() {
                        let dt = chrono::DateTime::from_timestamp(timestamp, 0)
                            .map(|t| t.with_timezone(&Local));
                        if let Some(dt) = dt {
                            println!("Last heartbeat: {}", dt.format("%Y-%m-%d %H:%M:%S"));
                        }
                    }
                }
            }
        }
    } else {
        println!("Daemon is not running");
    }
    Ok(())
}

pub fn is_daemon_running() -> Result<bool> {
    let pid_file = Storage::pid_file_path()?;

    if !pid_file.exists() {
        return Ok(false);
    }

    let pid_str = fs::read_to_string(&pid_file)?;
    let pid: u32 = match pid_str.trim().parse() {
        Ok(p) => p,
        Err(_) => {
            fs::remove_file(&pid_file)?;
            return Ok(false);
        }
    };

    #[cfg(unix)]
    {
        let output = Command::new("kill").args(["-0", &pid.to_string()]).output();

        match output {
            Ok(o) => Ok(o.status.success()),
            Err(_) => {
                fs::remove_file(&pid_file)?;
                Ok(false)
            }
        }
    }

    #[cfg(windows)]
    {
        let output = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid)])
            .output();

        match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                Ok(stdout.contains(&pid.to_string()))
            }
            Err(_) => {
                fs::remove_file(&pid_file)?;
                Ok(false)
            }
        }
    }
}

fn write_heartbeat() {
    if let Ok(heartbeat_path) = Storage::heartbeat_file_path() {
        let timestamp = Local::now().timestamp().to_string();
        let _ = fs::write(heartbeat_path, timestamp);
    }
}

fn check_heartbeat() -> Result<bool> {
    let heartbeat_path = Storage::heartbeat_file_path()?;

    if !heartbeat_path.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(&heartbeat_path)?;
    let timestamp: i64 = content.trim().parse().unwrap_or(0);
    let now = Local::now().timestamp();

    Ok((now - timestamp) < HEARTBEAT_TIMEOUT_SECS as i64)
}

pub fn is_daemon_healthy() -> Result<bool> {
    if !is_daemon_running()? {
        return Ok(false);
    }
    check_heartbeat()
}

pub fn run_daemon_loop() -> Result<()> {
    let storage = Storage::new()?;
    log_info!("Daemon started");
    write_heartbeat();

    let mut reminders = storage.load()?;
    let mut last_known_mtime = reminders_file_mtime(&storage)?;
    let mut last_heartbeat_at = std::time::Instant::now();

    loop {
        match reminders_file_mtime(&storage) {
            Ok(mtime) if mtime != last_known_mtime => match storage.load() {
                Ok(loaded) => {
                    reminders = loaded;
                    last_known_mtime = mtime;
                    log_debug!("Reloaded reminders after file change");
                }
                Err(e) => log_error!("Failed to reload reminders after file change: {}", e),
            },
            Ok(_) => {}
            Err(e) => log_error!("Failed to check reminders file metadata: {}", e),
        }

        let mut updated = false;
        for reminder in &mut reminders {
            if reminder.is_due() {
                log_info!("Triggering reminder: {}", reminder.title);

                if let Err(e) = send_notification(reminder) {
                    log_error!("Failed to send notification: {}", e);
                }
                reminder.calculate_next_trigger();
                updated = true;
            }
        }

        if updated {
            if let Err(e) = storage.save(&reminders) {
                log_error!("Failed to save reminders: {}", e);
            } else {
                last_known_mtime = reminders_file_mtime(&storage).ok().flatten();
            }
        }

        if last_heartbeat_at.elapsed() >= Duration::from_secs(HEARTBEAT_INTERVAL_SECS) {
            write_heartbeat();
            log_debug!("Heartbeat written");
            last_heartbeat_at = std::time::Instant::now();
        }

        thread::sleep(next_sleep_duration(&reminders, &last_heartbeat_at));
    }
}

fn reminders_file_mtime(storage: &Storage) -> Result<Option<std::time::SystemTime>> {
    match fs::metadata(storage.path()) {
        Ok(metadata) => Ok(metadata.modified().ok()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err.into()),
    }
}

fn next_sleep_duration(
    reminders: &[super::reminder::Reminder],
    last_heartbeat_at: &std::time::Instant,
) -> Duration {
    let heartbeat_due_in = Duration::from_secs(HEARTBEAT_INTERVAL_SECS)
        .saturating_sub(last_heartbeat_at.elapsed());

    let next_trigger_in = reminders
        .iter()
        .filter(|reminder| !reminder.completed && !reminder.paused)
        .filter_map(|reminder| reminder.next_trigger)
        .filter_map(|next| (next - Local::now()).to_std().ok())
        .min()
        .unwrap_or(Duration::from_secs(FILE_CHECK_INTERVAL_SECS));

    next_trigger_in
        .min(heartbeat_due_in)
        .min(Duration::from_secs(FILE_CHECK_INTERVAL_SECS))
        .max(Duration::from_secs(1))
}

/// Generate launchd plist for macOS auto-start
#[cfg(target_os = "macos")]
pub fn generate_launchd_plist() -> Result<String> {
    let exe = std::env::current_exe()?;
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.reminder-cli.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>daemon</string>
        <string>run</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>"#,
        exe.display()
    );
    Ok(plist)
}

/// Generate systemd service for Linux auto-start
#[cfg(target_os = "linux")]
pub fn generate_systemd_service() -> Result<String> {
    let exe = std::env::current_exe()?;
    let service = format!(
        r#"[Unit]
Description=Reminder CLI Daemon
After=network.target

[Service]
Type=simple
ExecStart={} daemon run
Restart=always
RestartSec=10

[Install]
WantedBy=default.target"#,
        exe.display()
    );
    Ok(service)
}

pub fn install_autostart() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let plist = generate_launchd_plist()?;
        let plist_path = dirs::home_dir()
            .context("Failed to get home directory")?
            .join("Library/LaunchAgents/com.reminder-cli.daemon.plist");

        fs::write(&plist_path, plist)?;
        println!("Created launchd plist at: {}", plist_path.display());
        println!("To enable: launchctl load {}", plist_path.display());
    }

    #[cfg(target_os = "linux")]
    {
        let service = generate_systemd_service()?;
        let service_path = dirs::home_dir()
            .context("Failed to get home directory")?
            .join(".config/systemd/user/reminder-cli.service");

        if let Some(parent) = service_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&service_path, service)?;
        println!("Created systemd service at: {}", service_path.display());
        println!("To enable: systemctl --user enable --now reminder-cli");
    }

    #[cfg(target_os = "windows")]
    {
        println!(
            "Windows auto-start: Add a shortcut to 'reminder daemon start' in your Startup folder"
        );
        println!(
            "Startup folder: {}",
            dirs::data_local_dir()
                .map(|p| p
                    .parent()
                    .unwrap_or(&p)
                    .join("Roaming/Microsoft/Windows/Start Menu/Programs/Startup")
                    .display()
                    .to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        );
    }

    Ok(())
}
