use crate::reminder::Reminder;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

pub struct Storage {
    path: PathBuf,
}

impl Storage {
    pub fn new() -> Result<Self> {
        let data_dir = dirs::data_local_dir()
            .context("Failed to get local data directory")?
            .join("reminder-cli");
        
        fs::create_dir_all(&data_dir)?;
        
        Ok(Self {
            path: data_dir.join("reminders.json"),
        })
    }

    pub fn load(&self) -> Result<Vec<Reminder>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&self.path)
            .context("Failed to read reminders file")?;
        
        if content.trim().is_empty() {
            return Ok(Vec::new());
        }

        let reminders: Vec<Reminder> = serde_json::from_str(&content)
            .context("Failed to parse reminders JSON")?;
        
        Ok(reminders)
    }

    pub fn save(&self, reminders: &[Reminder]) -> Result<()> {
        let content = serde_json::to_string_pretty(reminders)
            .context("Failed to serialize reminders")?;
        
        fs::write(&self.path, content)
            .context("Failed to write reminders file")?;
        
        Ok(())
    }

    pub fn add(&self, reminder: Reminder) -> Result<()> {
        let mut reminders = self.load()?;
        reminders.push(reminder);
        self.save(&reminders)
    }

    pub fn delete(&self, id: Uuid) -> Result<bool> {
        let mut reminders = self.load()?;
        let initial_len = reminders.len();
        reminders.retain(|r| r.id != id);
        
        if reminders.len() == initial_len {
            return Ok(false);
        }
        
        self.save(&reminders)?;
        Ok(true)
    }

    pub fn update(&self, id: Uuid, updater: impl FnOnce(&mut Reminder)) -> Result<bool> {
        let mut reminders = self.load()?;
        
        if let Some(reminder) = reminders.iter_mut().find(|r| r.id == id) {
            updater(reminder);
            self.save(&reminders)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn get(&self, id: Uuid) -> Result<Option<Reminder>> {
        let reminders = self.load()?;
        Ok(reminders.into_iter().find(|r| r.id == id))
    }

    pub fn pid_file_path() -> Result<PathBuf> {
        let data_dir = dirs::data_local_dir()
            .context("Failed to get local data directory")?
            .join("reminder-cli");
        
        fs::create_dir_all(&data_dir)?;
        Ok(data_dir.join("daemon.pid"))
    }

    pub fn log_file_path() -> Result<PathBuf> {
        let data_dir = dirs::data_local_dir()
            .context("Failed to get local data directory")?
            .join("reminder-cli");
        
        fs::create_dir_all(&data_dir)?;
        Ok(data_dir.join("daemon.log"))
    }
}
