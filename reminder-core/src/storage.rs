use super::reminder::Reminder;
use anyhow::{Context, Result};
use fs2::FileExt;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct Storage {
    path: PathBuf,
}

impl Storage {
    pub fn new() -> Result<Self> {
        let data_dir = Self::data_dir()?;
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
        let result = Self::read_reminders(&file);
        file.unlock().context("Failed to release lock")?;
        result
    }

    pub fn save(&self, reminders: &[Reminder]) -> Result<()> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.path)
            .context("Failed to open reminders file for writing")?;

        file.lock_exclusive()
            .context("Failed to acquire write lock")?;
        let result = self.write_atomically(&mut file, reminders);
        file.unlock().context("Failed to release lock")?;
        result
    }

    pub fn add(&self, reminder: Reminder) -> Result<()> {
        self.with_exclusive_reminders(|reminders| {
            reminders.push(reminder);
            Ok(())
        })
    }

    pub fn delete(&self, id: Uuid) -> Result<bool> {
        let mut deleted = false;
        self.with_exclusive_reminders(|reminders| {
            let initial_len = reminders.len();
            reminders.retain(|r| r.id != id);
            deleted = reminders.len() != initial_len;
            Ok(())
        })?;
        Ok(deleted)
    }

    pub fn update(
        &self,
        id: Uuid,
        updater: impl FnOnce(&mut Reminder) -> Result<()>,
    ) -> Result<bool> {
        let mut updated = false;
        self.with_exclusive_reminders(|reminders| {
            if let Some(reminder) = reminders.iter_mut().find(|r| r.id == id) {
                updater(reminder)?;
                updated = true;
            }
            Ok(())
        })?;
        Ok(updated)
    }

    pub fn get(&self, id: Uuid) -> Result<Option<Reminder>> {
        let reminders = self.load()?;
        Ok(reminders.into_iter().find(|r| r.id == id))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Find reminder by short ID (prefix match)
    pub fn find_by_short_id(&self, short_id: &str) -> Result<Option<Reminder>> {
        let reminders = self.load()?;
        let mut found = None;
        let mut match_count = 0usize;

        for reminder in reminders {
            if Self::matches_short_id(reminder.id, short_id) {
                match_count += 1;
                if match_count == 1 {
                    found = Some(reminder);
                } else {
                    anyhow::bail!(
                        "Ambiguous ID '{}': matches {} reminders. Please use more characters.",
                        short_id,
                        match_count
                    );
                }
            }
        }

        Ok(found)
    }

    /// Delete reminder by short ID
    pub fn delete_by_short_id(&self, short_id: &str) -> Result<Option<Uuid>> {
        let mut deleted = None;
        self.with_exclusive_reminders(|reminders| {
            let matched = Self::find_short_id_match(reminders.iter(), short_id)?;
            if let Some(id) = matched {
                reminders.retain(|r| r.id != id);
                deleted = Some(id);
            }
            Ok(())
        })?;
        Ok(deleted)
    }

    /// Clean completed reminders
    pub fn clean_completed(&self) -> Result<usize> {
        let mut removed = 0usize;
        self.with_exclusive_reminders(|reminders| {
            let initial_len = reminders.len();
            reminders.retain(|r| !r.completed);
            removed = initial_len - reminders.len();
            Ok(())
        })?;
        Ok(removed)
    }

    pub fn pid_file_path() -> Result<PathBuf> {
        let data_dir = Self::data_dir()?;
        Ok(data_dir.join("daemon.pid"))
    }

    pub fn log_file_path() -> Result<PathBuf> {
        let data_dir = Self::data_dir()?;
        Ok(data_dir.join("daemon.log"))
    }

    pub fn heartbeat_file_path() -> Result<PathBuf> {
        let data_dir = Self::data_dir()?;
        Ok(data_dir.join("daemon.heartbeat"))
    }

    /// Filter reminders by tag
    pub fn filter_by_tag(&self, tag: &str) -> Result<Vec<Reminder>> {
        let reminders = self.load()?;
        Ok(reminders
            .into_iter()
            .filter(|r| r.tags.contains(tag))
            .collect())
    }

    /// Get all unique tags
    pub fn get_all_tags(&self) -> Result<Vec<String>> {
        let reminders = self.load()?;
        let mut tags: Vec<String> = reminders
            .iter()
            .flat_map(|r| r.tags.iter().cloned())
            .collect();
        tags.sort();
        tags.dedup();
        Ok(tags)
    }

    /// Pause reminder by short ID
    pub fn pause_by_short_id(&self, short_id: &str) -> Result<Option<Uuid>> {
        let reminder = self.find_by_short_id(short_id)?;
        if let Some(r) = reminder {
            let id = r.id;
            self.update(id, |rem| {
                rem.pause();
                Ok(())
            })?;
            Ok(Some(id))
        } else {
            Ok(None)
        }
    }

    /// Resume reminder by short ID
    pub fn resume_by_short_id(&self, short_id: &str) -> Result<Option<Uuid>> {
        let reminder = self.find_by_short_id(short_id)?;
        if let Some(r) = reminder {
            let id = r.id;
            self.update(id, |rem| {
                rem.resume();
                Ok(())
            })?;
            Ok(Some(id))
        } else {
            Ok(None)
        }
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

        let mut imported_count = 0;
        let mut skipped_count = 0;
        self.with_exclusive_reminders(|existing| {
            let mut index_by_id = std::collections::HashMap::with_capacity(existing.len());
            for (idx, reminder) in existing.iter().enumerate() {
                index_by_id.insert(reminder.id, idx);
            }

            for reminder in imported {
                if let Some(&idx) = index_by_id.get(&reminder.id) {
                    if overwrite {
                        existing[idx] = reminder;
                        imported_count += 1;
                    } else {
                        skipped_count += 1;
                    }
                } else {
                    index_by_id.insert(reminder.id, existing.len());
                    existing.push(reminder);
                    imported_count += 1;
                }
            }
            Ok(())
        })?;
        Ok((imported_count, skipped_count))
    }

    fn data_dir() -> Result<PathBuf> {
        let data_dir = dirs::data_local_dir()
            .context("Failed to get local data directory")?
            .join("reminder-cli");
        fs::create_dir_all(&data_dir)?;
        Ok(data_dir)
    }

    fn read_reminders(file: &File) -> Result<Vec<Reminder>> {
        let mut content = String::new();
        let mut reader = file;
        reader
            .read_to_string(&mut content)
            .context("Failed to read reminders file")?;

        if content.trim().is_empty() {
            return Ok(Vec::new());
        }

        serde_json::from_str(&content).context("Failed to parse reminders JSON")
    }

    fn write_atomically(&self, file: &mut File, reminders: &[Reminder]) -> Result<()> {
        let content =
            serde_json::to_vec_pretty(reminders).context("Failed to serialize reminders")?;
        let tmp_path = self.path.with_extension("json.tmp");

        {
            let mut tmp_file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&tmp_path)
                .context("Failed to open temporary reminders file")?;
            tmp_file
                .write_all(&content)
                .context("Failed to write temporary reminders file")?;
            tmp_file
                .sync_all()
                .context("Failed to flush temporary reminders file")?;
        }

        fs::rename(&tmp_path, &self.path).context("Failed to replace reminders file")?;
        file.seek(SeekFrom::Start(0))
            .context("Failed to rewind reminders file handle")?;
        Ok(())
    }

    fn with_exclusive_reminders<T>(
        &self,
        mutator: impl FnOnce(&mut Vec<Reminder>) -> Result<T>,
    ) -> Result<T> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.path)
            .context("Failed to open reminders file")?;
        file.lock_exclusive()
            .context("Failed to acquire write lock")?;

        let mut reminders = Self::read_reminders(&file)?;
        let result = mutator(&mut reminders);
        match result {
            Ok(value) => {
                self.write_atomically(&mut file, &reminders)?;
                file.unlock().context("Failed to release lock")?;
                Ok(value)
            }
            Err(err) => {
                file.unlock().context("Failed to release lock")?;
                Err(err)
            }
        }
    }

    fn find_short_id_match<'a>(
        reminders: impl Iterator<Item = &'a Reminder>,
        short_id: &str,
    ) -> Result<Option<Uuid>> {
        let mut found = None;
        let mut match_count = 0usize;

        for reminder in reminders {
            if Self::matches_short_id(reminder.id, short_id) {
                match_count += 1;
                if match_count == 1 {
                    found = Some(reminder.id);
                } else {
                    anyhow::bail!(
                        "Ambiguous ID '{}': matches {} reminders. Please use more characters.",
                        short_id,
                        match_count
                    );
                }
            }
        }

        Ok(found)
    }

    fn matches_short_id(id: Uuid, short_id: &str) -> bool {
        let mut buffer = Uuid::encode_buffer();
        id.hyphenated().encode_lower(&mut buffer).starts_with(short_id)
    }
}
