use chrono::{DateTime, Utc};

/// Elapsed time in hours between two timestamps.
/// Always non-negative; returns 0.0 if `end` is before `start`.
pub fn elapsed_hours(start: DateTime<Utc>, end: DateTime<Utc>) -> f64 {
    let secs = (end - start).num_seconds().max(0);
    secs as f64 / 3600.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn elapsed_hours_two_hour_gap() {
        let start = Utc::now();
        let end = start + Duration::hours(2);
        let h = elapsed_hours(start, end);
        assert!((h - 2.0).abs() < 0.01);
    }

    #[test]
    fn elapsed_hours_negative_returns_zero() {
        let start = Utc::now();
        let end = start - Duration::hours(1);
        assert_eq!(elapsed_hours(start, end), 0.0);
    }
}
