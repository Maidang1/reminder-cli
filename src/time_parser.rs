use anyhow::{bail, Result};
use chrono::{DateTime, Datelike, Duration, Local, NaiveDateTime, NaiveTime, Weekday};
use regex::Regex;

/// Parse time string supporting multiple formats:
/// - Absolute: "2025-12-25 10:00"
/// - Relative: "30m", "2h", "1d", "1w"
/// - Natural: "tomorrow 9am", "next monday 14:00", "today 18:30"
pub fn parse_time(input: &str) -> Result<DateTime<Local>> {
    let input = input.trim().to_lowercase();

    // Try absolute format first
    if let Ok(dt) = parse_absolute(&input) {
        return Ok(dt);
    }

    // Try relative format
    if let Ok(dt) = parse_relative(&input) {
        return Ok(dt);
    }

    // Try natural language
    if let Ok(dt) = parse_natural(&input) {
        return Ok(dt);
    }

    bail!(
        "Invalid time format: {}\n\
        Supported formats:\n\
        - Absolute: \"2025-12-25 10:00\"\n\
        - Relative: \"30m\", \"2h\", \"1d\", \"1w\"\n\
        - Natural: \"tomorrow 9am\", \"next monday 14:00\"",
        input
    )
}

fn parse_absolute(input: &str) -> Result<DateTime<Local>> {
    let naive = NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M")?;
    naive
        .and_local_timezone(Local)
        .single()
        .ok_or_else(|| anyhow::anyhow!("Invalid local time"))
}

fn parse_relative(input: &str) -> Result<DateTime<Local>> {
    let re = Regex::new(r"^(\d+)\s*(m|min|mins|minute|minutes|h|hr|hrs|hour|hours|d|day|days|w|week|weeks)$")?;

    if let Some(caps) = re.captures(input) {
        let amount: i64 = caps[1].parse()?;
        let unit = &caps[2];

        let duration = match unit {
            "m" | "min" | "mins" | "minute" | "minutes" => Duration::minutes(amount),
            "h" | "hr" | "hrs" | "hour" | "hours" => Duration::hours(amount),
            "d" | "day" | "days" => Duration::days(amount),
            "w" | "week" | "weeks" => Duration::weeks(amount),
            _ => bail!("Unknown time unit: {}", unit),
        };

        return Ok(Local::now() + duration);
    }

    bail!("Not a relative time format")
}

fn parse_natural(input: &str) -> Result<DateTime<Local>> {
    let now = Local::now();
    let today = now.date_naive();

    // Parse time part (e.g., "9am", "14:00", "9:30pm")
    let time_re = Regex::new(r"(\d{1,2})(?::(\d{2}))?\s*(am|pm)?$")?;

    let (date_part, time_part) = if let Some(pos) = input.rfind(char::is_whitespace) {
        let (d, t) = input.split_at(pos);
        (d.trim(), t.trim())
    } else {
        // Could be just a day reference without time
        (input, "")
    };

    // Parse the date part
    let target_date = match date_part {
        "today" => today,
        "tomorrow" => today + Duration::days(1),
        "yesterday" => today - Duration::days(1),
        s if s.starts_with("next ") => {
            let day_name = &s[5..];
            let target_weekday = parse_weekday(day_name)?;
            next_weekday(today, target_weekday)
        }
        s if s.starts_with("this ") => {
            let day_name = &s[5..];
            let target_weekday = parse_weekday(day_name)?;
            this_weekday(today, target_weekday)
        }
        _ => {
            // Try parsing as weekday directly
            if let Ok(weekday) = parse_weekday(date_part) {
                next_weekday(today, weekday)
            } else {
                bail!("Unknown date reference: {}", date_part)
            }
        }
    };

    // Parse the time part
    let target_time = if time_part.is_empty() {
        NaiveTime::from_hms_opt(9, 0, 0).unwrap() // Default to 9:00 AM
    } else if let Some(caps) = time_re.captures(time_part) {
        let mut hour: u32 = caps[1].parse()?;
        let minute: u32 = caps.get(2).map(|m| m.as_str().parse().unwrap()).unwrap_or(0);
        let ampm = caps.get(3).map(|m| m.as_str());

        match ampm {
            Some("am") => {
                if hour == 12 {
                    hour = 0;
                }
            }
            Some("pm") => {
                if hour != 12 {
                    hour += 12;
                }
            }
            _ => {
                // 24-hour format or unknown, no change needed
            }
        }

        NaiveTime::from_hms_opt(hour, minute, 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid time: {}:{}", hour, minute))?
    } else {
        bail!("Invalid time format: {}", time_part)
    };

    let naive_dt = target_date.and_time(target_time);
    naive_dt
        .and_local_timezone(Local)
        .single()
        .ok_or_else(|| anyhow::anyhow!("Invalid local time"))
}

fn parse_weekday(s: &str) -> Result<Weekday> {
    match s {
        "monday" | "mon" => Ok(Weekday::Mon),
        "tuesday" | "tue" | "tues" => Ok(Weekday::Tue),
        "wednesday" | "wed" => Ok(Weekday::Wed),
        "thursday" | "thu" | "thur" | "thurs" => Ok(Weekday::Thu),
        "friday" | "fri" => Ok(Weekday::Fri),
        "saturday" | "sat" => Ok(Weekday::Sat),
        "sunday" | "sun" => Ok(Weekday::Sun),
        _ => bail!("Unknown weekday: {}", s),
    }
}

fn next_weekday(from: chrono::NaiveDate, target: Weekday) -> chrono::NaiveDate {
    let current = from.weekday();
    let days_ahead = (target.num_days_from_monday() as i64 - current.num_days_from_monday() as i64 + 7) % 7;
    let days_ahead = if days_ahead == 0 { 7 } else { days_ahead };
    from + Duration::days(days_ahead)
}

fn this_weekday(from: chrono::NaiveDate, target: Weekday) -> chrono::NaiveDate {
    let current = from.weekday();
    let days_ahead = (target.num_days_from_monday() as i64 - current.num_days_from_monday() as i64 + 7) % 7;
    from + Duration::days(days_ahead)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relative_time() {
        let now = Local::now();
        
        let result = parse_time("30m").unwrap();
        assert!((result - now).num_minutes() >= 29 && (result - now).num_minutes() <= 31);

        let result = parse_time("2h").unwrap();
        assert!((result - now).num_hours() >= 1 && (result - now).num_hours() <= 3);

        let result = parse_time("1d").unwrap();
        assert!((result - now).num_days() == 1);
    }

    #[test]
    fn test_natural_time() {
        let result = parse_time("tomorrow 9am");
        assert!(result.is_ok());

        let result = parse_time("next monday 14:00");
        assert!(result.is_ok());
    }
}
