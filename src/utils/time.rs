// src/utils/time.rs - Cross-platform time utilities

use chrono::{DateTime, Duration, Utc};

/// Cross-platform time utilities that work on both native and WASM
pub struct Time;

impl Time {
    /// Get current UTC time - works on both native and WASM
    pub fn now() -> DateTime<Utc> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Utc::now()
        }

        #[cfg(target_arch = "wasm32")]
        {
            let millis = js_sys::Date::now() as i64;
            match DateTime::from_timestamp_millis(millis) {
                Some(dt) => dt,
                None => {
                    web_sys::console::error_1(
                        &"Failed to create DateTime from JS timestamp".into(),
                    );
                    // Fallback to a fixed date
                    DateTime::from_timestamp(1640995200, 0).unwrap()
                }
            }
        }
    }

    /// Get current timestamp as milliseconds since epoch
    pub fn now_millis() -> u64 {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Utc::now().timestamp_millis() as u64
        }

        #[cfg(target_arch = "wasm32")]
        {
            js_sys::Date::now() as u64
        }
    }

    /// Create a DateTime from milliseconds since epoch
    pub fn from_millis(millis: i64) -> DateTime<Utc> {
        DateTime::from_timestamp_millis(millis).unwrap_or_else(|| {
            DateTime::from_timestamp(1640995200, 0).unwrap() // Fallback
        })
    }

    /// Create a duration from milliseconds
    pub fn duration_millis(millis: i64) -> Duration {
        Duration::milliseconds(millis)
    }

    /// Create a duration from seconds
    pub fn duration_secs(secs: i64) -> Duration {
        Duration::seconds(secs)
    }

    /// Create a duration from hours
    pub fn duration_hours(hours: i64) -> Duration {
        Duration::hours(hours)
    }

    /// Create a duration from days
    pub fn duration_days(days: i64) -> Duration {
        Duration::days(days)
    }
}
