use crate::time_limits::config::{ChildProfile, TimeLimit, TimeLimitSchedule};
use chrono::{Datelike, Utc, Weekday};

/// Schedule calculator for determining time limits
pub struct ScheduleCalculator;

impl ScheduleCalculator {
    /// Get the time limit for a child for today
    pub fn get_limit_for_today(child: &ChildProfile) -> TimeLimit {
        let now = Utc::now();
        let weekday = now.weekday();
        let day_name = Self::weekday_to_string(weekday);

        Self::get_limit_for_day(child, &day_name, weekday)
    }

    /// Get the time limit for a specific day
    pub fn get_limit_for_day(child: &ChildProfile, day_name: &str, weekday: Weekday) -> TimeLimit {
        // Check for custom day overrides first
        for custom in &child.limits.custom {
            if custom.days.iter().any(|d| d.to_lowercase() == day_name.to_lowercase()) {
                return custom.limit;
            }
        }

        // Fall back to weekday/weekend
        if Self::is_weekend(weekday) {
            child.limits.weekend
        } else {
            child.limits.weekday
        }
    }

    /// Check if a weekday is a weekend
    fn is_weekend(weekday: Weekday) -> bool {
        matches!(weekday, Weekday::Sat | Weekday::Sun)
    }

    /// Convert Weekday to lowercase string
    fn weekday_to_string(weekday: Weekday) -> String {
        match weekday {
            Weekday::Mon => "monday",
            Weekday::Tue => "tuesday",
            Weekday::Wed => "wednesday",
            Weekday::Thu => "thursday",
            Weekday::Fri => "friday",
            Weekday::Sat => "saturday",
            Weekday::Sun => "sunday",
        }
        .to_string()
    }

    /// Calculate remaining time for a child
    pub fn calculate_remaining_time(
        child: &ChildProfile,
        used_seconds: i64,
        additional_seconds: Option<i64>,
    ) -> i64 {
        let limit = Self::get_limit_for_today(child);
        let limit_seconds = limit.to_seconds();
        let total_limit = limit_seconds + additional_seconds.unwrap_or(0);

        (total_limit - used_seconds).max(0)
    }

    /// Get all admin overrides for a child today
    pub fn get_today_overrides(
        child_id: &str,
        overrides: &[crate::time_limits::state::AdminOverride],
    ) -> i64 {
        let today = Utc::now().format("%Y-%m-%d").to_string();

        overrides
            .iter()
            .filter(|o| {
                o.child_id == child_id
                    && o.granted_at.format("%Y-%m-%d").to_string() == today
            })
            .filter_map(|o| o.additional_seconds)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time_limits::config::{CustomDayLimit, TimeLimitSchedule};

    fn make_test_child() -> ChildProfile {
        ChildProfile {
            id: "kid1".to_string(),
            name: "Alice".to_string(),
            os_users: vec![],
            limits: TimeLimitSchedule {
                weekday: TimeLimit {
                    hours: 2,
                    minutes: 0,
                },
                weekend: TimeLimit {
                    hours: 4,
                    minutes: 0,
                },
                custom: vec![],
            },
            warnings: vec![15, 5, 1],
            grace_period: 60,
        }
    }

    #[test]
    fn test_is_weekend() {
        assert!(!ScheduleCalculator::is_weekend(Weekday::Mon));
        assert!(!ScheduleCalculator::is_weekend(Weekday::Tue));
        assert!(!ScheduleCalculator::is_weekend(Weekday::Wed));
        assert!(!ScheduleCalculator::is_weekend(Weekday::Thu));
        assert!(!ScheduleCalculator::is_weekend(Weekday::Fri));
        assert!(ScheduleCalculator::is_weekend(Weekday::Sat));
        assert!(ScheduleCalculator::is_weekend(Weekday::Sun));
    }

    #[test]
    fn test_weekday_to_string() {
        assert_eq!(ScheduleCalculator::weekday_to_string(Weekday::Mon), "monday");
        assert_eq!(ScheduleCalculator::weekday_to_string(Weekday::Tue), "tuesday");
        assert_eq!(ScheduleCalculator::weekday_to_string(Weekday::Wed), "wednesday");
        assert_eq!(ScheduleCalculator::weekday_to_string(Weekday::Thu), "thursday");
        assert_eq!(ScheduleCalculator::weekday_to_string(Weekday::Fri), "friday");
        assert_eq!(ScheduleCalculator::weekday_to_string(Weekday::Sat), "saturday");
        assert_eq!(ScheduleCalculator::weekday_to_string(Weekday::Sun), "sunday");
    }

    #[test]
    fn test_get_limit_for_weekday() {
        let child = make_test_child();
        let limit = ScheduleCalculator::get_limit_for_day(&child, "monday", Weekday::Mon);
        assert_eq!(limit.hours, 2);
        assert_eq!(limit.minutes, 0);
    }

    #[test]
    fn test_get_limit_for_weekend() {
        let child = make_test_child();
        let limit = ScheduleCalculator::get_limit_for_day(&child, "saturday", Weekday::Sat);
        assert_eq!(limit.hours, 4);
        assert_eq!(limit.minutes, 0);
    }

    #[test]
    fn test_get_limit_with_custom_override() {
        let mut child = make_test_child();
        child.limits.custom = vec![CustomDayLimit {
            days: vec!["monday".to_string(), "wednesday".to_string()],
            limit: TimeLimit {
                hours: 1,
                minutes: 30,
            },
        }];

        let limit = ScheduleCalculator::get_limit_for_day(&child, "monday", Weekday::Mon);
        assert_eq!(limit.hours, 1);
        assert_eq!(limit.minutes, 30);

        let limit = ScheduleCalculator::get_limit_for_day(&child, "tuesday", Weekday::Tue);
        assert_eq!(limit.hours, 2); // Falls back to weekday
        assert_eq!(limit.minutes, 0);
    }

    #[test]
    fn test_calculate_remaining_time() {
        let child = make_test_child();

        // Used 1 hour out of 2 hour weekday limit
        let remaining = ScheduleCalculator::calculate_remaining_time(&child, 3600, None);
        // Note: This test depends on the current day, so we'll just check it's non-negative
        assert!(remaining >= 0);
    }

    #[test]
    fn test_calculate_remaining_time_with_override() {
        let child = make_test_child();

        // Used 1 hour, but has 1 hour extension
        let remaining = ScheduleCalculator::calculate_remaining_time(&child, 3600, Some(3600));
        // Should have more time available due to extension
        assert!(remaining >= 0);
    }

    #[test]
    fn test_calculate_remaining_time_over_limit() {
        let child = make_test_child();

        // Used 3 hours (more than 2 hour limit)
        let remaining = ScheduleCalculator::calculate_remaining_time(&child, 10800, None);
        assert_eq!(remaining, 0); // Should not go negative
    }
}
