use anyhow::{bail, Result};
use cron::Schedule;
use std::str::FromStr;

/// Parse cron expression from either standard cron format or English description
/// 
/// Examples:
/// - Standard: "0 0 9 * * *" (every day at 9am)
/// - English: "every day at 9am", "every monday at 14:00", "every hour"
pub fn parse_cron(input: &str) -> Result<String> {
    let input = input.trim();

    // First try as standard cron expression
    if Schedule::from_str(input).is_ok() {
        return Ok(input.to_string());
    }

    // Try to parse as English description
    match english_to_cron::str_cron_syntax(input) {
        Ok(cron_expr) => {
            // Validate the generated cron expression
            if Schedule::from_str(&cron_expr).is_ok() {
                Ok(cron_expr)
            } else {
                bail!(
                    "Generated invalid cron expression from '{}': {}",
                    input,
                    cron_expr
                )
            }
        }
        Err(_) => {
            bail!(
                "Invalid cron expression: {}\n\
                \n\
                Supported formats:\n\
                - Standard cron: \"0 0 9 * * *\" (sec min hour day month weekday)\n\
                - English: \"every day at 9am\", \"every monday at 14:00\", \"every hour\"\n\
                \n\
                English examples:\n\
                - \"every minute\"\n\
                - \"every hour\"\n\
                - \"every day at 9am\"\n\
                - \"every monday at 10:00\"\n\
                - \"every weekday at 8:30\"\n\
                - \"every 30 minutes\"",
                input
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_cron() {
        let result = parse_cron("0 0 9 * * *");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "0 0 9 * * *");
    }

    #[test]
    fn test_english_cron() {
        // These tests depend on the english-to-cron library behavior
        let result = parse_cron("every day at 9am");
        assert!(result.is_ok());
    }
}
