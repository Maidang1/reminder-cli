use crate::reminder::Reminder;
use anyhow::{Context, Result};
use fs2::FileExt;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
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

        let file = File::open(&self.path).context("Failed to open reminders file")?;
        file.lock_shared().context("Failed to acquire read lock")?;

        let mut content = String::new();
        let mut reader = &file;
        reader
            .read_to_string(&mut content)
            .context("Failed to read reminders file")?;

        file.unlock().context("Failed to release lock")?;

        if content.trim().is_empty() {
            return Ok(Vec::new());
        }

        let reminders: Vec<Reminder> =
            serde_json::from_str(&content).context("Failed to parse reminders JSON")?;

        Ok(reminders)
    }

    pub fn save(&self, reminders: &[Reminder]) -> Result<()> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.path)
            .context("Failed to open reminders file for writing")?;

        file.lock_exclusive()
            .context("Failed to acquire write lock")?;

        let content =
            serde_json::to_string_pretty(reminders).context("Failed to serialize reminders")?;

        let mut writer = &file;
        writer
            .write_all(content.as_bytes())
            .context("Failed to write reminders file")?;

        file.unlock().context("Failed to release lock")?;

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

    /// Find reminder by short ID (prefix match)
    pub fn find_by_short_id(&self, short_id: &str) -> Result<Option<Reminder>> {
        let reminders = self.load()?;
        let matches: Vec<_> = reminders
            .into_iter()
            .filter(|r| r.id.to_string().starts_with(short_id))
            .collect();

        match matches.len() {
            0 => Ok(None),
            1 => Ok(Some(matches.into_iter().next().unwrap())),
            _ => anyhow::bail!(
                "Ambiguous ID '{}': matches {} reminders. Please use more characters.",
                short_id,
                matches.len()
            ),
        }
    }

    /// Delete reminder by short ID
    pub fn delete_by_short_id(&self, short_id: &str) -> Result<Option<Uuid>> {
        let mut reminders = self.load()?;
        let matches: Vec<_> = reminders
            .iter()
            .filter(|r| r.id.to_string().starts_with(short_id))
            .map(|r| r.id)
            .collect();

        match matches.len() {
            0 => Ok(None),
            1 => {
                let id = matches[0];
                reminders.retain(|r| r.id != id);
                self.save(&reminders)?;
                Ok(Some(id))
            }
            _ => anyhow::bail!(
                "Ambiguous ID '{}': matches {} reminders. Please use more characters.",
                short_id,
                matches.len()
            ),
        }
    }

    /// Clean completed reminders
    pub fn clean_completed(&self) -> Result<usize> {
        let mut reminders = self.load()?;
        let initial_len = reminders.len();
        reminders.retain(|r| !r.completed);
        let removed = initial_len - reminders.len();

        if removed > 0 {
            self.save(&reminders)?;
        }

        Ok(removed)
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

    /// Export all reminders to a JSON file
    pub fn export_to_file(&self, path: &Path) -> Result<usize> {
        let reminders = self.load()?;
        let count = reminders.len();

        let content = serde_json::to_string_pretty(&reminders)
            .context("Failed to serialize reminders for export")?;

        fs::write(path, content).context("Failed to write export file")?;

        Ok(count)
    }

    /// Import reminders from a JSON file
    /// Returns (imported_count, skipped_count)
    pub fn import_from_file(&self, path: &Path, overwrite: bool) -> Result<(usize, usize)> {
        let content = fs::read_to_string(path).context("Failed to read import file")?;

        let imported: Vec<Reminder> =
            serde_json::from_str(&content).context("Failed to parse import JSON")?;

        let mut existing = self.load()?;
        let existing_ids: std::collections::HashSet<Uuid> = existing.iter().map(|r| r.id).collect();

        let mut imported_count = 0;
        let mut skipped_count = 0;

        for reminder in imported {
            if existing_ids.contains(&reminder.id) {
                if overwrite {
                    existing.retain(|r| r.id != reminder.id);
                    existing.push(reminder);
                    imported_count += 1;
                } else {
                    skipped_count += 1;
                }
            } else {
                existing.push(reminder);
                imported_count += 1;
            }
        }

        self.save(&existing)?;
        Ok((imported_count, skipped_count))
    }
}
