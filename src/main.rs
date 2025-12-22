use anyhow::{bail, Result};
use chrono::{Local, NaiveDateTime};
use clap::{Parser, Subcommand};
use cron::Schedule;
use reminder_cli::daemon::{daemon_status, run_daemon_loop, start_daemon, stop_daemon};
use reminder_cli::reminder::{Reminder, ReminderSchedule};
use reminder_cli::storage::Storage;
use std::path::PathBuf;
use std::str::FromStr;
use tabled::settings::object::{Columns, Object, Rows};
use tabled::settings::{Color, Modify, Style, Width};
use tabled::{Table, Tabled};
use uuid::Uuid;

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

        /// Time for one-time reminder (format: "YYYY-MM-DD HH:MM")
        #[arg(short = 'T', long, conflicts_with = "cron")]
        time: Option<String>,

        /// Cron expression for periodic reminder (e.g., "0 0 9 * * *" for daily at 9am)
        #[arg(short, long, conflicts_with = "time")]
        cron: Option<String>,
    },

    /// List all reminders
    List,

    /// Delete a reminder
    Delete {
        /// ID of the reminder to delete
        #[arg(short, long)]
        id: String,
    },

    /// Edit an existing reminder
    Edit {
        /// ID of the reminder to edit
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
    },

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
    Run,
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
        } => add_reminder(&storage, title, description, time, cron),

        Commands::List => list_reminders(&storage),

        Commands::Delete { id } => delete_reminder(&storage, &id),

        Commands::Edit {
            id,
            title,
            description,
            time,
            cron,
        } => edit_reminder(&storage, &id, title, description, time, cron),

        Commands::Daemon { action } => match action {
            DaemonAction::Start => start_daemon(),
            DaemonAction::Stop => stop_daemon(),
            DaemonAction::Status => daemon_status(),
            DaemonAction::Run => run_daemon_loop(),
        },

        Commands::Export { output } => export_reminders(&storage, &output),

        Commands::Import { input, overwrite } => import_reminders(&storage, &input, overwrite),
    }
}

fn add_reminder(
    storage: &Storage,
    title: String,
    description: Option<String>,
    time: Option<String>,
    cron: Option<String>,
) -> Result<()> {
    let reminder = if let Some(cron_expr) = cron {
        // Validate cron expression
        if Schedule::from_str(&cron_expr).is_err() {
            bail!(
                "Invalid cron expression: {}\n\
                Valid format: 'sec min hour day month weekday'\n\
                Example: '0 0 9 * * *' (daily at 9:00 AM)",
                cron_expr
            );
        }
        Reminder::new_cron(title, description, cron_expr)?
    } else if let Some(time_str) = time {
        let naive = NaiveDateTime::parse_from_str(&time_str, "%Y-%m-%d %H:%M")
            .map_err(|_| anyhow::anyhow!(
                "Invalid time format: {}\nExpected format: YYYY-MM-DD HH:MM",
                time_str
            ))?;
        let datetime = naive.and_local_timezone(Local).single()
            .ok_or_else(|| anyhow::anyhow!("Invalid local time"))?;
        Reminder::new_one_time(title, description, datetime)
    } else {
        bail!("Either --time or --cron must be specified");
    };

    println!("✓ Reminder added successfully!");
    println!("  ID: {}", reminder.id);
    println!("  Title: {}", reminder.title);
    if let Some(desc) = &reminder.description {
        println!("  Description: {}", desc);
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
}

fn list_reminders(storage: &Storage) -> Result<()> {
    let mut reminders = storage.load()?;

    if reminders.is_empty() {
        println!("No reminders scheduled.");
        return Ok(());
    }

    // Sort by next trigger time
    reminders.sort_by(|a, b| match (&a.next_trigger, &b.next_trigger) {
        (Some(ta), Some(tb)) => ta.cmp(tb),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });

    // Track which rows are completed (1-indexed, row 0 is header)
    let completed_rows: Vec<usize> = reminders
        .iter()
        .enumerate()
        .filter(|(_, r)| r.completed)
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
                id: r.id.to_string(),
                title: r.title.clone(),
                next_trigger: r
                    .next_trigger
                    .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "Completed".to_string()),
                schedule_type: type_str,
            }
        })
        .collect();

    // Track which rows are active one-time or periodic
    let onetime_rows: Vec<usize> = reminders
        .iter()
        .enumerate()
        .filter(|(_, r)| !r.completed && matches!(r.schedule, ReminderSchedule::OneTime(_)))
        .map(|(i, _)| i + 1)
        .collect();

    let periodic_rows: Vec<usize> = reminders
        .iter()
        .enumerate()
        .filter(|(_, r)| !r.completed && matches!(r.schedule, ReminderSchedule::Cron(_)))
        .map(|(i, _)| i + 1)
        .collect();

    let mut table = Table::new(rows);
    table.with(Style::rounded());
    table.with(Modify::new(Columns::single(3)).with(Width::increase(10)));

    // Apply gray color to completed rows
    for row_idx in completed_rows {
        table.modify(Rows::single(row_idx), Color::FG_BRIGHT_BLACK);
    }

    // Apply cyan to active one-time rows (Type column only)
    for row_idx in onetime_rows {
        table.modify(
            Rows::single(row_idx).intersect(Columns::single(3)),
            Color::FG_CYAN,
        );
    }

    // Apply green to active periodic rows (Type column only)
    for row_idx in periodic_rows {
        table.modify(
            Rows::single(row_idx).intersect(Columns::single(3)),
            Color::FG_GREEN,
        );
    }

    println!("{}", table);

    Ok(())
}

fn delete_reminder(storage: &Storage, id: &str) -> Result<()> {
    let uuid = Uuid::parse_str(id)
        .map_err(|_| anyhow::anyhow!("Invalid reminder ID format"))?;

    if storage.delete(uuid)? {
        println!("✓ Reminder deleted successfully");
    } else {
        println!("✗ Reminder not found with ID: {}", id);
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
) -> Result<()> {
    let uuid = Uuid::parse_str(id)
        .map_err(|_| anyhow::anyhow!("Invalid reminder ID format"))?;

    let updated = storage.update(uuid, |reminder| {
        if let Some(new_title) = title {
            reminder.title = new_title;
        }
        if let Some(new_desc) = description {
            reminder.description = Some(new_desc);
        }
        if let Some(time_str) = time {
            if let Ok(naive) = NaiveDateTime::parse_from_str(&time_str, "%Y-%m-%d %H:%M") {
                if let Some(datetime) = naive.and_local_timezone(Local).single() {
                    reminder.schedule = ReminderSchedule::OneTime(datetime);
                    reminder.next_trigger = Some(datetime);
                    reminder.completed = false;
                }
            }
        }
        if let Some(cron_expr) = cron {
            if let Ok(schedule) = Schedule::from_str(&cron_expr) {
                reminder.schedule = ReminderSchedule::Cron(cron_expr);
                reminder.next_trigger = schedule.upcoming(Local).next();
                reminder.completed = false;
            }
        }
    })?;

    if updated {
        println!("✓ Reminder updated successfully");
        if let Some(reminder) = storage.get(uuid)? {
            println!("  ID: {}", reminder.id);
            println!("  Title: {}", reminder.title);
            if let Some(desc) = &reminder.description {
                println!("  Description: {}", desc);
            }
            if let Some(next) = reminder.next_trigger {
                println!("  Next trigger: {}", next.format("%Y-%m-%d %H:%M:%S"));
            }
        }
    } else {
        println!("✗ Reminder not found with ID: {}", id);
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
        println!("  Skipped: {} reminder(s) (duplicate IDs, use -f to overwrite)", skipped);
    }
    
    Ok(())
}
