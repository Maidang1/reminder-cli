use anyhow::{bail, Result};
use chrono::Local;
use clap::{Parser, Subcommand};
use cron::Schedule;
use reminder_cli::cron_parser::parse_cron;
use reminder_cli::daemon::{
    daemon_status, install_autostart, run_daemon_loop, start_daemon, stop_daemon,
};
use reminder_cli::logger::get_logger;
use reminder_cli::reminder::{Reminder, ReminderSchedule};
use reminder_cli::storage::Storage;
use reminder_cli::time_parser::parse_time;
use reminder_cli::{log_info, log_warn};
use std::collections::HashSet;
use std::path::PathBuf;
use std::str::FromStr;
use tabled::settings::object::{Columns, Object, Rows};
use tabled::settings::{Color, Modify, Style, Width};
use tabled::{Table, Tabled};

#[derive(Parser)]
#[command(name = "reminder")]
#[command(about = "A CLI reminder tool with cron support", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new reminder
    Add {
        /// Title of the reminder
        #[arg(short, long)]
        title: String,

        /// Description of the reminder (optional)
        #[arg(short, long)]
        description: Option<String>,

        /// Time for reminder (supports: "2025-12-25 10:00", "30m", "2h", "tomorrow 9am")
        #[arg(short = 'T', long, conflicts_with = "cron")]
        time: Option<String>,

        /// Cron expression or English (e.g., "0 0 9 * * *" or "every day at 9am")
        #[arg(short, long, conflicts_with = "time")]
        cron: Option<String>,

        /// Tags for categorization (comma-separated)
        #[arg(long, value_delimiter = ',')]
        tags: Option<Vec<String>>,
    },

    /// List all reminders
    List {
        /// Filter by tag
        #[arg(long)]
        tag: Option<String>,

        /// Show all including paused
        #[arg(short, long)]
        all: bool,
    },

    /// Show details of a specific reminder
    Show {
        /// ID of the reminder (can use short ID prefix)
        id: String,
    },

    /// Delete a reminder
    Delete {
        /// ID of the reminder to delete (can use short ID prefix)
        #[arg(short, long)]
        id: String,
    },

    /// Edit an existing reminder
    Edit {
        /// ID of the reminder to edit (can use short ID prefix)
        #[arg(short, long)]
        id: String,

        /// New title (optional)
        #[arg(short, long)]
        title: Option<String>,

        /// New description (optional)
        #[arg(short = 'D', long)]
        description: Option<String>,

        /// New time for one-time reminder (optional)
        #[arg(short = 'T', long)]
        time: Option<String>,

        /// New cron expression (optional)
        #[arg(short, long)]
        cron: Option<String>,

        /// Add tags (comma-separated)
        #[arg(long, value_delimiter = ',')]
        add_tags: Option<Vec<String>>,

        /// Remove tags (comma-separated)
        #[arg(long, value_delimiter = ',')]
        remove_tags: Option<Vec<String>>,
    },

    /// Pause a reminder
    Pause {
        /// ID of the reminder to pause
        id: String,
    },

    /// Resume a paused reminder
    Resume {
        /// ID of the reminder to resume
        id: String,
    },

    /// Clean up completed reminders
    Clean,

    /// List all tags
    Tags,

    /// Manage the background daemon
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },

    /// Export reminders to a JSON file
    Export {
        /// Output file path
        #[arg(short, long, default_value = "reminders_export.json")]
        output: PathBuf,
    },

    /// Import reminders from a JSON file
    Import {
        /// Input file path
        #[arg(short, long)]
        input: PathBuf,

        /// Overwrite existing reminders with same ID
        #[arg(short = 'f', long, default_value = "false")]
        overwrite: bool,
    },

    /// View and manage logs
    Logs {
        #[command(subcommand)]
        action: LogsAction,
    },
}

#[derive(Subcommand)]
enum DaemonAction {
    /// Start the daemon
    Start,
    /// Stop the daemon
    Stop,
    /// Check daemon status
    Status,
    /// Run the daemon (internal use)
    #[command(hide = true)]
    Run,
    /// Install auto-start configuration
    Install,
}

#[derive(Subcommand)]
enum LogsAction {
    /// Show recent log entries
    Show {
        /// Number of lines to show (default: 50)
        #[arg(short, long, default_value = "50")]
        lines: usize,
    },
    /// Show log file path and size
    Info,
    /// Clear all logs
    Clear,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let storage = Storage::new()?;

    match cli.command {
        Commands::Add {
            title,
            description,
            time,
            cron,
            tags,
        } => add_reminder(&storage, title, description, time, cron, tags),

        Commands::List { tag, all } => list_reminders(&storage, tag, all),

        Commands::Show { id } => show_reminder(&storage, &id),

        Commands::Delete { id } => delete_reminder(&storage, &id),

        Commands::Edit {
            id,
            title,
            description,
            time,
            cron,
            add_tags,
            remove_tags,
        } => edit_reminder(
            &storage,
            &id,
            title,
            description,
            time,
            cron,
            add_tags,
            remove_tags,
        ),

        Commands::Pause { id } => pause_reminder(&storage, &id),

        Commands::Resume { id } => resume_reminder(&storage, &id),

        Commands::Clean => clean_reminders(&storage),

        Commands::Tags => list_tags(&storage),

        Commands::Daemon { action } => match action {
            DaemonAction::Start => start_daemon(),
            DaemonAction::Stop => stop_daemon(),
            DaemonAction::Status => daemon_status(),
            DaemonAction::Run => run_daemon_loop(),
            DaemonAction::Install => install_autostart(),
        },

        Commands::Export { output } => export_reminders(&storage, &output),

        Commands::Import { input, overwrite } => import_reminders(&storage, &input, overwrite),

        Commands::Logs { action } => match action {
            LogsAction::Show { lines } => show_logs(lines),
            LogsAction::Info => logs_info(),
            LogsAction::Clear => clear_logs(),
        },
    }
}

fn add_reminder(
    storage: &Storage,
    title: String,
    description: Option<String>,
    time: Option<String>,
    cron: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<()> {
    let tags_set: HashSet<String> = tags.unwrap_or_default().into_iter().collect();

    let reminder = if let Some(cron_input) = cron {
        let cron_expr = parse_cron(&cron_input)?;
        Reminder::new_cron(title, description, cron_expr, tags_set)?
    } else if let Some(time_str) = time {
        let datetime = parse_time(&time_str)?;
        Reminder::new_one_time(title, description, datetime, tags_set)
    } else {
        bail!("Either --time or --cron must be specified");
    };

    let short_id = &reminder.id.to_string()[..8];
    log_info!("Added reminder: {} ({})", reminder.title, short_id);

    println!("✓ Reminder added successfully!");
    println!("  ID: {} (short: {})", reminder.id, short_id);
    println!("  Title: {}", reminder.title);
    if let Some(desc) = &reminder.description {
        println!("  Description: {}", desc);
    }
    if !reminder.tags.is_empty() {
        println!("  Tags: {}", reminder.tags.iter().cloned().collect::<Vec<_>>().join(", "));
    }
    if let Some(next) = reminder.next_trigger {
        println!("  Next trigger: {}", next.format("%Y-%m-%d %H:%M:%S"));
    }

    storage.add(reminder)?;
    Ok(())
}

#[derive(Tabled)]
struct ReminderRow {
    #[tabled(rename = "ID")]
    id: String,
    #[tabled(rename = "Title")]
    title: String,
    #[tabled(rename = "Next Trigger")]
    next_trigger: String,
    #[tabled(rename = "Type")]
    schedule_type: String,
    #[tabled(rename = "Status")]
    status: String,
}

fn list_reminders(storage: &Storage, tag_filter: Option<String>, show_all: bool) -> Result<()> {
    let mut reminders = if let Some(tag) = tag_filter {
        storage.filter_by_tag(&tag)?
    } else {
        storage.load()?
    };

    if !show_all {
        reminders.retain(|r| !r.completed);
    }

    if reminders.is_empty() {
        println!("No reminders found.");
        return Ok(());
    }

    reminders.sort_by(|a, b| match (&a.next_trigger, &b.next_trigger) {
        (Some(ta), Some(tb)) => ta.cmp(tb),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });

    let completed_rows: Vec<usize> = reminders
        .iter()
        .enumerate()
        .filter(|(_, r)| r.completed)
        .map(|(i, _)| i + 1)
        .collect();

    let paused_rows: Vec<usize> = reminders
        .iter()
        .enumerate()
        .filter(|(_, r)| r.paused && !r.completed)
        .map(|(i, _)| i + 1)
        .collect();

    let rows: Vec<ReminderRow> = reminders
        .iter()
        .map(|r| {
            let type_str = match &r.schedule {
                ReminderSchedule::OneTime(_) => "One-time".to_string(),
                ReminderSchedule::Cron(_) => "Periodic".to_string(),
            };

            ReminderRow {
                id: r.id.to_string()[..8].to_string(),
                title: truncate(&r.title, 25),
                next_trigger: r
                    .next_trigger
                    .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "-".to_string()),
                schedule_type: type_str,
                status: r.status().to_string(),
            }
        })
        .collect();

    let onetime_rows: Vec<usize> = reminders
        .iter()
        .enumerate()
        .filter(|(_, r)| {
            !r.completed && !r.paused && matches!(r.schedule, ReminderSchedule::OneTime(_))
        })
        .map(|(i, _)| i + 1)
        .collect();

    let periodic_rows: Vec<usize> = reminders
        .iter()
        .enumerate()
        .filter(|(_, r)| {
            !r.completed && !r.paused && matches!(r.schedule, ReminderSchedule::Cron(_))
        })
        .map(|(i, _)| i + 1)
        .collect();

    let mut table = Table::new(rows);
    table.with(Style::rounded());
    table.with(Modify::new(Columns::single(3)).with(Width::increase(10)));

    // Gray for completed
    for row_idx in completed_rows {
        table.modify(Rows::single(row_idx), Color::FG_BRIGHT_BLACK);
    }

    // Yellow for paused
    for row_idx in paused_rows {
        table.modify(Rows::single(row_idx), Color::FG_YELLOW);
    }

    // Cyan for active one-time
    for row_idx in onetime_rows {
        table.modify(
            Rows::single(row_idx).intersect(Columns::single(3)),
            Color::FG_CYAN,
        );
    }

    // Green for active periodic
    for row_idx in periodic_rows {
        table.modify(
            Rows::single(row_idx).intersect(Columns::single(3)),
            Color::FG_GREEN,
        );
    }

    println!("{}", table);

    Ok(())
}

fn show_reminder(storage: &Storage, id: &str) -> Result<()> {
    let reminder = storage
        .find_by_short_id(id)?
        .ok_or_else(|| anyhow::anyhow!("Reminder not found with ID: {}", id))?;

    println!("ID:          {}", reminder.id);
    println!("Title:       {}", reminder.title);
    if let Some(desc) = &reminder.description {
        println!("Description: {}", desc);
    }
    if !reminder.tags.is_empty() {
        println!(
            "Tags:        {}",
            reminder.tags.iter().cloned().collect::<Vec<_>>().join(", ")
        );
    }
    println!(
        "Type:        {}",
        match &reminder.schedule {
            ReminderSchedule::OneTime(_) => "One-time",
            ReminderSchedule::Cron(expr) => {
                println!("Cron:        {}", expr);
                "Periodic"
            }
        }
    );
    println!(
        "Created:     {}",
        reminder.created_at.format("%Y-%m-%d %H:%M:%S")
    );
    if let Some(next) = reminder.next_trigger {
        println!("Next:        {}", next.format("%Y-%m-%d %H:%M:%S"));
    }
    println!("Status:      {}", reminder.status());

    Ok(())
}

fn delete_reminder(storage: &Storage, id: &str) -> Result<()> {
    match storage.delete_by_short_id(id)? {
        Some(uuid) => {
            log_info!("Deleted reminder: {}", uuid);
            println!("✓ Reminder deleted successfully (ID: {})", uuid);
        }
        None => {
            log_warn!("Delete failed: reminder not found with ID: {}", id);
            println!("✗ Reminder not found with ID: {}", id);
        }
    }

    Ok(())
}

fn edit_reminder(
    storage: &Storage,
    id: &str,
    title: Option<String>,
    description: Option<String>,
    time: Option<String>,
    cron: Option<String>,
    add_tags: Option<Vec<String>>,
    remove_tags: Option<Vec<String>>,
) -> Result<()> {
    let reminder = storage
        .find_by_short_id(id)?
        .ok_or_else(|| anyhow::anyhow!("Reminder not found with ID: {}", id))?;

    let uuid = reminder.id;

    let updated = storage.update(uuid, |reminder| {
        if let Some(new_title) = title {
            reminder.title = new_title;
        }
        if let Some(new_desc) = description {
            reminder.description = Some(new_desc);
        }
        if let Some(time_str) = time {
            if let Ok(datetime) = parse_time(&time_str) {
                reminder.schedule = ReminderSchedule::OneTime(datetime);
                reminder.next_trigger = Some(datetime);
                reminder.completed = false;
            }
        }
        if let Some(cron_input) = cron {
            if let Ok(cron_expr) = parse_cron(&cron_input) {
                if let Ok(schedule) = Schedule::from_str(&cron_expr) {
                    reminder.schedule = ReminderSchedule::Cron(cron_expr);
                    reminder.next_trigger = schedule.upcoming(Local).next();
                    reminder.completed = false;
                }
            }
        }
        if let Some(tags) = add_tags {
            for tag in tags {
                reminder.tags.insert(tag);
            }
        }
        if let Some(tags) = remove_tags {
            for tag in tags {
                reminder.tags.remove(&tag);
            }
        }
    })?;

    if updated {
        log_info!("Updated reminder: {}", uuid);
        println!("✓ Reminder updated successfully");
        if let Some(reminder) = storage.get(uuid)? {
            println!("  ID: {}", reminder.id);
            println!("  Title: {}", reminder.title);
            if let Some(desc) = &reminder.description {
                println!("  Description: {}", desc);
            }
            if !reminder.tags.is_empty() {
                println!(
                    "  Tags: {}",
                    reminder.tags.iter().cloned().collect::<Vec<_>>().join(", ")
                );
            }
            if let Some(next) = reminder.next_trigger {
                println!("  Next trigger: {}", next.format("%Y-%m-%d %H:%M:%S"));
            }
        }
    }

    Ok(())
}

fn pause_reminder(storage: &Storage, id: &str) -> Result<()> {
    match storage.pause_by_short_id(id)? {
        Some(uuid) => {
            log_info!("Paused reminder: {}", &uuid.to_string()[..8]);
            println!("✓ Reminder paused (ID: {})", &uuid.to_string()[..8]);
        }
        None => {
            println!("✗ Reminder not found with ID: {}", id);
        }
    }
    Ok(())
}

fn resume_reminder(storage: &Storage, id: &str) -> Result<()> {
    match storage.resume_by_short_id(id)? {
        Some(uuid) => {
            log_info!("Resumed reminder: {}", &uuid.to_string()[..8]);
            println!("✓ Reminder resumed (ID: {})", &uuid.to_string()[..8]);
        }
        None => {
            println!("✗ Reminder not found with ID: {}", id);
        }
    }
    Ok(())
}

fn list_tags(storage: &Storage) -> Result<()> {
    let tags = storage.get_all_tags()?;

    if tags.is_empty() {
        println!("No tags found.");
        return Ok(());
    }

    println!("Tags:");
    for tag in tags {
        let count = storage.filter_by_tag(&tag)?.len();
        println!("  {} ({})", tag, count);
    }

    Ok(())
}

fn export_reminders(storage: &Storage, output: &PathBuf) -> Result<()> {
    let count = storage.export_to_file(output)?;
    println!("✓ Exported {} reminder(s) to {}", count, output.display());
    Ok(())
}

fn import_reminders(storage: &Storage, input: &PathBuf, overwrite: bool) -> Result<()> {
    if !input.exists() {
        bail!("Import file not found: {}", input.display());
    }

    let (imported, skipped) = storage.import_from_file(input, overwrite)?;

    println!("✓ Import completed:");
    println!("  Imported: {} reminder(s)", imported);
    if skipped > 0 {
        println!(
            "  Skipped: {} reminder(s) (duplicate IDs, use -f to overwrite)",
            skipped
        );
    }

    Ok(())
}

fn clean_reminders(storage: &Storage) -> Result<()> {
    let removed = storage.clean_completed()?;

    if removed > 0 {
        log_info!("Cleaned {} completed reminder(s)", removed);
        println!("✓ Cleaned {} completed reminder(s)", removed);
    } else {
        println!("No completed reminders to clean");
    }

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(max_len - 3).collect::<String>())
    }
}

fn show_logs(lines: usize) -> Result<()> {
    let logger = get_logger();
    let log_lines = logger.tail(lines)?;

    if log_lines.is_empty() {
        println!("No logs found.");
        return Ok(());
    }

    for line in log_lines {
        println!("{}", line);
    }

    Ok(())
}

fn logs_info() -> Result<()> {
    let logger = get_logger();
    let size = logger.size()?;

    println!("Log file: {}", logger.path().display());
    println!(
        "Size: {:.2} KB / 1024 KB",
        size as f64 / 1024.0
    );

    Ok(())
}

fn clear_logs() -> Result<()> {
    let logger = get_logger();
    logger.clear()?;
    println!("✓ Logs cleared");
    Ok(())
}
