use crate::reminder::Reminder;
use crate::storage::Storage;
use anyhow::Result;
use std::fs::OpenOptions;
use std::io::Write;

pub fn send_notification(reminder: &Reminder) -> Result<()> {
    let result = notify_rust::Notification::new()
        .summary(&reminder.title)
        .body(reminder.description.as_deref().unwrap_or(""))
        .appname("Reminder CLI")
        .timeout(notify_rust::Timeout::Milliseconds(10000))
        .show();

    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Failed to show notification: {}, falling back to log", e);
            log_reminder(reminder)
        }
    }
}

fn log_reminder(reminder: &Reminder) -> Result<()> {
    let log_path = Storage::log_file_path()?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let description = reminder.description.as_deref().unwrap_or("");
    
    writeln!(
        file,
        "[{}] REMINDER: {} - {}",
        timestamp, reminder.title, description
    )?;

    Ok(())
}
