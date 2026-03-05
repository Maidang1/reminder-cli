use anyhow::Result;
use chrono::Local;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

const MAX_LOG_SIZE: u64 = 1024 * 1024; // 1MB
const LOG_FILE_NAME: &str = "reminder.log";
const OLD_LOG_FILE_NAME: &str = "reminder.log.old";

pub struct Logger {
    path: PathBuf,
    old_path: PathBuf,
}

impl Logger {
    pub fn new() -> Result<Self> {
        let data_dir = dirs::data_local_dir()
            .ok_or_else(|| anyhow::anyhow!("Failed to get local data directory"))?
            .join("reminder-cli");

        fs::create_dir_all(&data_dir)?;

        Ok(Self {
            path: data_dir.join(LOG_FILE_NAME),
            old_path: data_dir.join(OLD_LOG_FILE_NAME),
        })
    }

    fn rotate_if_needed(&self) -> Result<()> {
        if !self.path.exists() {
            return Ok(());
        }

        let metadata = fs::metadata(&self.path)?;
        if metadata.len() >= MAX_LOG_SIZE {
            // Remove old log if exists
            if self.old_path.exists() {
                fs::remove_file(&self.old_path)?;
            }
            // Rename current log to old
            fs::rename(&self.path, &self.old_path)?;
        }

        Ok(())
    }

    pub fn log(&self, level: LogLevel, message: &str) {
        if let Err(e) = self.log_internal(level, message) {
            eprintln!("Failed to write log: {}", e);
        }
    }

    fn log_internal(&self, level: LogLevel, message: &str) -> Result<()> {
        self.rotate_if_needed()?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let level_str = match level {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        };

        writeln!(file, "[{}] [{}] {}", timestamp, level_str, message)?;

        Ok(())
    }

    pub fn info(&self, message: &str) {
        self.log(LogLevel::Info, message);
    }

    pub fn warn(&self, message: &str) {
        self.log(LogLevel::Warn, message);
    }

    pub fn error(&self, message: &str) {
        self.log(LogLevel::Error, message);
    }

    pub fn debug(&self, message: &str) {
        self.log(LogLevel::Debug, message);
    }

    /// Get the last N lines from the log file
    pub fn tail(&self, lines: usize) -> Result<Vec<String>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let all_lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();

        let start = if all_lines.len() > lines {
            all_lines.len() - lines
        } else {
            0
        };

        Ok(all_lines[start..].to_vec())
    }

    /// Get log file path
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Get log file size in bytes
    pub fn size(&self) -> Result<u64> {
        if !self.path.exists() {
            return Ok(0);
        }
        Ok(fs::metadata(&self.path)?.len())
    }

    /// Clear all logs
    pub fn clear(&self) -> Result<()> {
        if self.path.exists() {
            fs::remove_file(&self.path)?;
        }
        if self.old_path.exists() {
            fs::remove_file(&self.old_path)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

// Global logger instance
use std::sync::OnceLock;

static LOGGER: OnceLock<Logger> = OnceLock::new();

pub fn get_logger() -> &'static Logger {
    LOGGER.get_or_init(|| Logger::new().expect("Failed to initialize logger"))
}

// Convenience macros
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::logger::get_logger().info(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::logger::get_logger().warn(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::logger::get_logger().error(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::logger::get_logger().debug(&format!($($arg)*))
    };
}
