use chrono::{DateTime, Local};
use cron::Schedule;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reminder {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub schedule: ReminderSchedule,
    pub created_at: DateTime<Local>,
    pub next_trigger: Option<DateTime<Local>>,
    pub completed: bool,
    #[serde(default)]
    pub paused: bool,
    #[serde(default)]
    pub tags: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReminderSchedule {
    OneTime(DateTime<Local>),
    Cron(String),
}

impl Reminder {
    pub fn new_one_time(
        title: String,
        description: Option<String>,
        time: DateTime<Local>,
        tags: HashSet<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            title,
            description,
            schedule: ReminderSchedule::OneTime(time),
            created_at: Local::now(),
            next_trigger: Some(time),
            completed: false,
            paused: false,
            tags,
        }
    }

    pub fn new_cron(
        title: String,
        description: Option<String>,
        cron_expr: String,
        tags: HashSet<String>,
    ) -> anyhow::Result<Self> {
        let schedule = Schedule::from_str(&cron_expr)?;
        let next = schedule.upcoming(Local).next();

        Ok(Self {
            id: Uuid::new_v4(),
            title,
            description,
            schedule: ReminderSchedule::Cron(cron_expr),
            created_at: Local::now(),
            next_trigger: next,
            completed: false,
            paused: false,
            tags,
        })
    }

    pub fn calculate_next_trigger(&mut self) {
        match &self.schedule {
            ReminderSchedule::OneTime(_) => {
                self.completed = true;
                self.next_trigger = None;
            }
            ReminderSchedule::Cron(expr) => {
                self.next_trigger = next_cron_trigger(expr);
            }
        }
    }

    pub fn is_due(&self) -> bool {
        if self.completed || self.paused {
            return false;
        }
        if let Some(next) = self.next_trigger {
            return Local::now() >= next;
        }
        false
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn resume(&mut self) {
        self.paused = false;
        // Recalculate next trigger for cron jobs
        if let ReminderSchedule::Cron(expr) = &self.schedule {
            self.next_trigger = next_cron_trigger(expr);
        }
    }

    pub fn status(&self) -> &'static str {
        if self.completed {
            "Completed"
        } else if self.paused {
            "Paused"
        } else {
            "Active"
        }
    }
}

fn next_cron_trigger(expr: &str) -> Option<DateTime<Local>> {
    Schedule::from_str(expr)
        .ok()
        .and_then(|schedule| schedule.upcoming(Local).next())
}
