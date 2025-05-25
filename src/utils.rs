// src/utils.rs

//! Utility functions and helpers for Qorzen Core
//!
//! This module provides common utility functions that are used throughout
//! the Qorzen Core system and can be useful for plugins and applications.

use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::time::{sleep, timeout};
use uuid::Uuid;

use crate::error::{Error, ErrorKind, Result};

/// Timing utilities
pub mod timing {
    use super::*;

    /// Simple stopwatch for measuring execution time
    #[derive(Debug, Clone)]
    pub struct Stopwatch {
        start_time: Instant,
        lap_times: Vec<Instant>,
    }

    impl Stopwatch {
        /// Create and start a new stopwatch
        pub fn start() -> Self {
            Self {
                start_time: Instant::now(),
                lap_times: Vec::new(),
            }
        }

        /// Record a lap time
        pub fn lap(&mut self) -> Duration {
            let now = Instant::now();
            self.lap_times.push(now);
            now.duration_since(self.start_time)
        }

        /// Get elapsed time since start
        pub fn elapsed(&self) -> Duration {
            Instant::now().duration_since(self.start_time)
        }

        /// Stop the stopwatch and return total elapsed time
        pub fn stop(self) -> Duration {
            Instant::now().duration_since(self.start_time)
        }

        /// Get all lap times
        pub fn lap_times(&self) -> Vec<Duration> {
            self.lap_times
                .iter()
                .map(|&time| time.duration_since(self.start_time))
                .collect()
        }

        /// Reset the stopwatch
        pub fn reset(&mut self) {
            self.start_time = Instant::now();
            self.lap_times.clear();
        }
    }

    /// Execute a function and measure its execution time
    pub async fn measure_async<F, T>(future: F) -> (T, Duration)
    where
        F: Future<Output = T>,
    {
        let start = Instant::now();
        let result = future.await;
        let duration = start.elapsed();
        (result, duration)
    }

    /// Execute a function and measure its execution time (sync version)
    pub fn measure_sync<F, T>(func: F) -> (T, Duration)
    where
        F: FnOnce() -> T,
    {
        let start = Instant::now();
        let result = func();
        let duration = start.elapsed();
        (result, duration)
    }

    /// Get current Unix timestamp in seconds
    pub fn unix_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Get current Unix timestamp in milliseconds
    pub fn unix_timestamp_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Convert duration to human-readable string
    pub fn duration_to_human(duration: Duration) -> String {
        let total_seconds = duration.as_secs();
        let days = total_seconds / 86400;
        let hours = (total_seconds % 86400) / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        let millis = duration.subsec_millis();

        if days > 0 {
            format!("{}d {}h {}m {}s", days, hours, minutes, seconds)
        } else if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else if seconds > 0 {
            format!("{}.{:03}s", seconds, millis)
        } else {
            format!("{}ms", millis)
        }
    }
}

/// Retry utilities for handling transient failures
pub mod retry {
    use super::*;

    /// Retry configuration
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct RetryConfig {
        /// Maximum number of attempts
        pub max_attempts: u32,
        /// Initial delay between attempts
        pub initial_delay: Duration,
        /// Maximum delay between attempts
        pub max_delay: Duration,
        /// Backoff multiplier
        pub backoff_multiplier: f64,
        /// Whether to add jitter to delays
        pub jitter: bool,
    }

    impl Default for RetryConfig {
        fn default() -> Self {
            Self {
                max_attempts: 3,
                initial_delay: Duration::from_millis(100),
                max_delay: Duration::from_secs(30),
                backoff_multiplier: 2.0,
                jitter: true,
            }
        }
    }

    /// Retry a function with exponential backoff
    pub async fn retry_async<F, Fut, T, E>(
        mut func: F,
        config: RetryConfig,
    ) -> std::result::Result<T, E>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = std::result::Result<T, E>>,
        E: std::fmt::Display,
    {
        let mut attempt = 0;
        let mut delay = config.initial_delay;

        loop {
            attempt += 1;

            match func().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    if attempt >= config.max_attempts {
                        return Err(error);
                    }

                    tracing::warn!(
                        "Attempt {} failed, retrying in {:?}: {}",
                        attempt,
                        delay,
                        error
                    );

                    sleep(delay).await;

                    // Calculate next delay with exponential backoff
                    delay = Duration::from_millis(
                        ((delay.as_millis() as f64) * config.backoff_multiplier) as u64
                    );
                    delay = delay.min(config.max_delay);

                    // Add jitter if enabled
                    if config.jitter {
                        let jitter_range = delay.as_millis() as f64 * 0.1; // 10% jitter
                        let jitter = (rand::random::<f64>() - 0.5) * 2.0 * jitter_range;
                        let jittered_ms = (delay.as_millis() as f64 + jitter).max(0.0) as u64;
                        delay = Duration::from_millis(jittered_ms);
                    }
                }
            }
        }
    }

    /// Retry a synchronous function - Fixed version
    pub async fn retry_sync<F, T, E>(
        func: F,
        config: RetryConfig,
    ) -> std::result::Result<T, E>
    where
        F: Fn() -> std::result::Result<T, E> + Send + Sync + 'static,
        T: Send + 'static,
        E: std::fmt::Display + Send + 'static,
    {
        let func = std::sync::Arc::new(func);
        retry_async(
            move || {
                let func_clone = std::sync::Arc::clone(&func);
                async move { func_clone() }
            },
            config,
        )
            .await
    }
}

/// Collection utilities
pub mod collections {
    use std::collections::HashMap;
    use std::hash::Hash;

    /// Group items by a key function
    pub fn group_by<T, K, F>(items: Vec<T>, key_fn: F) -> HashMap<K, Vec<T>>
    where
        K: Hash + Eq,
        F: Fn(&T) -> K,
    {
        let mut groups = HashMap::new();
        for item in items {
            let key = key_fn(&item);
            groups.entry(key).or_insert_with(Vec::new).push(item);
        }
        groups
    }

    /// Partition items into two groups based on a predicate
    pub fn partition<T, F>(items: Vec<T>, predicate: F) -> (Vec<T>, Vec<T>)
    where
        F: Fn(&T) -> bool,
    {
        let mut true_items = Vec::new();
        let mut false_items = Vec::new();

        for item in items {
            if predicate(&item) {
                true_items.push(item);
            } else {
                false_items.push(item);
            }
        }

        (true_items, false_items)
    }

    /// Find duplicates in a collection
    pub fn find_duplicates<T>(items: &[T]) -> Vec<T>
    where
        T: Hash + Eq + Clone,
    {
        let mut seen = std::collections::HashSet::new();
        let mut duplicates = std::collections::HashSet::new();

        for item in items {
            if !seen.insert(item) {
                duplicates.insert(item.clone());
            }
        }

        duplicates.into_iter().collect()
    }
}

/// String utilities
pub mod strings {
    /// Truncate string to maximum length with ellipsis
    pub fn truncate(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len.saturating_sub(3)])
        }
    }

    /// Convert string to snake_case
    pub fn to_snake_case(s: &str) -> String {
        let mut result = String::new();
        let mut prev_char_was_uppercase = false;

        for (i, ch) in s.chars().enumerate() {
            if ch.is_uppercase() {
                if i > 0 && !prev_char_was_uppercase {
                    result.push('_');
                }
                result.push(ch.to_lowercase().next().unwrap_or(ch));
                prev_char_was_uppercase = true;
            } else {
                result.push(ch);
                prev_char_was_uppercase = false;
            }
        }

        result
    }

    /// Convert string to kebab-case
    pub fn to_kebab_case(s: &str) -> String {
        to_snake_case(s).replace('_', "-")
    }

    /// Convert string to PascalCase
    pub fn to_pascal_case(s: &str) -> String {
        s.split(&['_', '-', ' '][..])
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
                }
            })
            .collect()
    }

    /// Generate a random string of specified length
    pub fn random_string(length: usize) -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                               abcdefghijklmnopqrstuvwxyz\
                               0123456789";

        let mut rng = rand::thread_rng();
        (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
}

/// Async utilities
pub mod async_utils {
    use super::*;

    /// Execute a function with a timeout
    pub async fn with_timeout<F, T>(
        future: F,
        timeout_duration: Duration,
    ) -> Result<T>
    where
        F: Future<Output = T>,
    {
        timeout(timeout_duration, future)
            .await
            .map_err(|_| Error::timeout("Operation timed out"))
    }

    /// Execute multiple futures concurrently with a limit
    pub async fn execute_with_concurrency_limit<F, T>(
        futures: Vec<F>,
        limit: usize,
    ) -> Vec<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        use futures::stream::{FuturesUnordered, StreamExt};
        use tokio::sync::Semaphore;
        use std::sync::Arc;

        let semaphore = Arc::new(Semaphore::new(limit));
        let mut tasks = FuturesUnordered::new();

        for future in futures {
            let permit = Arc::clone(&semaphore);
            let task = async move {
                let _permit = permit.acquire().await.unwrap();
                future.await
            };
            tasks.push(tokio::spawn(task));
        }

        let mut results = Vec::new();
        while let Some(result) = tasks.next().await {
            if let Ok(value) = result {
                results.push(value);
            }
        }

        results
    }

    /// Race multiple futures and return the first to complete
    pub async fn race<T>(futures: Vec<Pin<Box<dyn Future<Output = T> + Send>>>) -> Option<T> {
        if futures.is_empty() {
            return None;
        }

        use futures::future::select_all;
        let (result, _index, _remaining) = select_all(futures).await;
        Some(result)
    }
}

/// Validation utilities
pub mod validation {
    use super::*;
    use std::net::IpAddr;
    use std::str::FromStr;

    /// Email validation (basic)
    pub fn is_valid_email(email: &str) -> bool {
        email.contains('@') && email.contains('.') && email.len() > 5
    }

    /// URL validation (basic)
    pub fn is_valid_url(url: &str) -> bool {
        url.starts_with("http://") || url.starts_with("https://")
    }

    /// IP address validation
    pub fn is_valid_ip(ip: &str) -> bool {
        IpAddr::from_str(ip).is_ok()
    }

    /// UUID validation
    pub fn is_valid_uuid(uuid: &str) -> bool {
        Uuid::from_str(uuid).is_ok()
    }

    /// Port number validation
    pub fn is_valid_port(port: u16) -> bool {
        port > 0 && port <= 65535
    }

    /// File path validation (basic security check)
    pub fn is_safe_path(path: &str) -> bool {
        !path.contains("..") && !path.starts_with('/') && !path.contains('\0')
    }

    /// Password strength validation
    pub fn validate_password_strength(password: &str, min_length: usize) -> Vec<String> {
        let mut errors = Vec::new();

        if password.len() < min_length {
            errors.push(format!("Password must be at least {} characters", min_length));
        }

        if !password.chars().any(|c| c.is_uppercase()) {
            errors.push("Password must contain at least one uppercase letter".to_string());
        }

        if !password.chars().any(|c| c.is_lowercase()) {
            errors.push("Password must contain at least one lowercase letter".to_string());
        }

        if !password.chars().any(|c| c.is_ascii_digit()) {
            errors.push("Password must contain at least one digit".to_string());
        }

        if !password.chars().any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c)) {
            errors.push("Password must contain at least one special character".to_string());
        }

        errors
    }
}

/// Compression utilities
pub mod compression {
    use super::*;

    /// Compress data using gzip
    pub fn compress_gzip(data: &[u8]) -> Result<Vec<u8>> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).map_err(|e| {
            Error::new(
                ErrorKind::Io,
                format!("Failed to compress data: {}", e),
            )
        })?;

        encoder.finish().map_err(|e| {
            Error::new(
                ErrorKind::Io,
                format!("Failed to finish compression: {}", e),
            )
        })
    }

    /// Decompress gzip data
    pub fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>> {
        use flate2::read::GzDecoder;
        use std::io::Read;

        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).map_err(|e| {
            Error::new(
                ErrorKind::Io,
                format!("Failed to decompress data: {}", e),
            )
        })?;

        Ok(decompressed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stopwatch() {
        let mut stopwatch = timing::Stopwatch::start();
        std::thread::sleep(Duration::from_millis(10));
        let lap1 = stopwatch.lap();
        std::thread::sleep(Duration::from_millis(10));
        let total = stopwatch.stop();

        assert!(lap1.as_millis() >= 10);
        assert!(total.as_millis() >= 20);
    }

    #[test]
    fn test_duration_to_human() {
        assert_eq!(timing::duration_to_human(Duration::from_millis(500)), "500ms");
        assert_eq!(timing::duration_to_human(Duration::from_secs(1)), "1.000s");
        assert_eq!(timing::duration_to_human(Duration::from_secs(61)), "1m 1s");
        assert_eq!(timing::duration_to_human(Duration::from_secs(3661)), "1h 1m 1s");
    }

    #[test]
    fn test_string_utilities() {
        assert_eq!(strings::to_snake_case("HelloWorld"), "hello_world");
        assert_eq!(strings::to_kebab_case("HelloWorld"), "hello-world");
        assert_eq!(strings::to_pascal_case("hello_world"), "HelloWorld");
        assert_eq!(strings::truncate("Hello, World!", 10), "Hello, ...");
    }

    #[test]
    fn test_validation() {
        assert!(validation::is_valid_email("test@example.com"));
        assert!(!validation::is_valid_email("invalid-email"));
        assert!(validation::is_valid_url("https://example.com"));
        assert!(!validation::is_valid_url("not-a-url"));
        assert!(validation::is_valid_ip("192.168.1.1"));
        assert!(!validation::is_valid_ip("999.999.999.999"));
    }

    #[test]
    fn test_collections() {
        let items = vec!["apple", "banana", "apricot", "berry"];
        let groups = collections::group_by(items, |item| item.chars().next().unwrap());

        assert_eq!(groups.get(&'a').unwrap().len(), 2);
        assert_eq!(groups.get(&'b').unwrap().len(), 2);

        let numbers = vec![1, 2, 3, 4, 5, 6];
        let (evens, odds) = collections::partition(numbers, |&n| n % 2 == 0);
        assert_eq!(evens, vec![2, 4, 6]);
        assert_eq!(odds, vec![1, 3, 5]);
    }

    #[tokio::test]
    async fn test_retry() {
        let mut attempts = 0;
        let result = retry::retry_async(
            || {
                attempts += 1;
                async move {
                    if attempts < 3 {
                        Err("Failed")
                    } else {
                        Ok("Success")
                    }
                }
            },
            retry::RetryConfig {
                max_attempts: 5,
                initial_delay: Duration::from_millis(1),
                ..Default::default()
            },
        ).await;

        assert_eq!(result.unwrap(), "Success");
        assert_eq!(attempts, 3);
    }
}