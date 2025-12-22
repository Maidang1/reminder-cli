use crate::notification::send_notification;
use crate::storage::Storage;
use anyhow::{Context, Result};
use std::fs;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

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

pub fn run_daemon_loop() -> Result<()> {
    let storage = Storage::new()?;
    
    loop {
        let mut reminders = storage.load()?;
        let mut updated = false;

        for reminder in reminders.iter_mut() {
            if reminder.is_due() {
                if let Err(e) = send_notification(reminder) {
                    eprintln!("Failed to send notification: {}", e);
                }
                reminder.calculate_next_trigger();
                updated = true;
            }
        }

        if updated {
            storage.save(&reminders)?;
        }

        thread::sleep(Duration::from_secs(30));
    }
}
