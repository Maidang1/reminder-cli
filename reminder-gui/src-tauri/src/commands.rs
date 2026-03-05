use reminder_core::reminder::{Reminder, ReminderSchedule};
use reminder_core::storage::Storage;
use reminder_core::time_parser::parse_time;
use reminder_core::cron_parser::parse_cron;
use reminder_core::notification::send_notification;
use std::collections::HashSet;

#[derive(serde::Serialize)]
pub struct ReminderDto {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub schedule: serde_json::Value,
    pub created_at: String,
    pub next_trigger: Option<String>,
    pub completed: bool,
    pub paused: bool,
    pub tags: Vec<String>,
}

impl From<Reminder> for ReminderDto {
    fn from(r: Reminder) -> Self {
        let schedule = match &r.schedule {
            ReminderSchedule::OneTime(dt) => {
                serde_json::json!({ "OneTime": dt.to_rfc3339() })
            }
            ReminderSchedule::Cron(expr) => serde_json::json!({ "Cron": expr }),
        };

        ReminderDto {
            id: r.id.to_string(),
            title: r.title,
            description: r.description,
            schedule,
            created_at: r.created_at.to_rfc3339(),
            next_trigger: r.next_trigger.map(|dt| dt.to_rfc3339()),
            completed: r.completed,
            paused: r.paused,
            tags: r.tags.into_iter().collect(),
        }
    }
}

#[tauri::command]
pub fn get_reminders() -> Result<Vec<ReminderDto>, String> {
    let storage = Storage::new().map_err(|e| e.to_string())?;
    let reminders = storage.load().map_err(|e| e.to_string())?;
    Ok(reminders.into_iter().map(ReminderDto::from).collect())
}

#[tauri::command]
pub fn add_reminder(
    title: String,
    description: Option<String>,
    time: Option<String>,
    cron: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<ReminderDto, String> {
    let tags_set: HashSet<String> = tags.unwrap_or_default().into_iter().collect();

    let reminder = if let Some(cron_input) = cron {
        let cron_expr = parse_cron(&cron_input).map_err(|e| e.to_string())?;
        Reminder::new_cron(title, description, cron_expr, tags_set).map_err(|e| e.to_string())?
    } else if let Some(time_str) = time {
        let datetime = parse_time(&time_str).map_err(|e| e.to_string())?;
        Reminder::new_one_time(title, description, datetime, tags_set)
    } else {
        return Err("Either time or cron must be specified".to_string());
    };

    let dto = ReminderDto::from(reminder.clone());

    let storage = Storage::new().map_err(|e| e.to_string())?;
    storage.add(reminder).map_err(|e| e.to_string())?;

    Ok(dto)
}

#[tauri::command]
pub fn delete_reminder(id: String) -> Result<bool, String> {
    let storage = Storage::new().map_err(|e| e.to_string())?;
    storage.delete_by_short_id(&id).map_err(|e| e.to_string()).map(|o| o.is_some())
}

#[tauri::command]
pub fn pause_reminder(id: String) -> Result<bool, String> {
    let storage = Storage::new().map_err(|e| e.to_string())?;
    storage.pause_by_short_id(&id).map_err(|e| e.to_string()).map(|o| o.is_some())
}

#[tauri::command]
pub fn resume_reminder(id: String) -> Result<bool, String> {
    let storage = Storage::new().map_err(|e| e.to_string())?;
    storage.resume_by_short_id(&id).map_err(|e| e.to_string()).map(|o| o.is_some())
}

#[tauri::command]
pub fn get_tags() -> Result<Vec<String>, String> {
    let storage = Storage::new().map_err(|e| e.to_string())?;
    storage.get_all_tags().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn test_trigger(id: String) -> Result<bool, String> {
    let storage = Storage::new().map_err(|e| e.to_string())?;

    let reminder = storage
        .find_by_short_id(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Reminder not found".to_string())?;

    send_notification(&reminder).map_err(|e| e.to_string())?;

    Ok(true)
}