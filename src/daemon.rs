use crate::notification::send_notification;
use crate::storage::Storage;
use anyhow::{Context, Result};
use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

const POLL_INTERVAL_SECS: u64 = 10;

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
    if is_daemon_running()? {
        let pid_file = Storage::pid_file_path()?;
        let pid = fs::read_to_string(&pid_file)?;
        println!("Daemon is running (PID: {})", pid.trim());
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
        let output = Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output();

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

fn log_daemon(message: &str) {
    if let Ok(log_path) = Storage::log_file_path() {
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
            let _ = writeln!(file, "[{}] {}", timestamp, message);
        }
    }
}

pub fn run_daemon_loop() -> Result<()> {
    let storage = Storage::new()?;
    log_daemon("Daemon started");

    loop {
        match storage.load() {
            Ok(mut reminders) => {
                let mut updated = false;

                for reminder in reminders.iter_mut() {
                    if reminder.is_due() {
                        log_daemon(&format!("Triggering reminder: {}", reminder.title));

                        if let Err(e) = send_notification(reminder) {
                            log_daemon(&format!("Failed to send notification: {}", e));
                        }
                        reminder.calculate_next_trigger();
                        updated = true;
                    }
                }

                if updated {
                    if let Err(e) = storage.save(&reminders) {
                        log_daemon(&format!("Failed to save reminders: {}", e));
                    }
                }
            }
            Err(e) => {
                log_daemon(&format!("Failed to load reminders: {}", e));
            }
        }

        thread::sleep(Duration::from_secs(POLL_INTERVAL_SECS));
    }
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
        println!("Windows auto-start: Add a shortcut to 'reminder daemon start' in your Startup folder");
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
