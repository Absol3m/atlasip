//! Exponential-backoff retry helper (P1-PERF-006).
//!
//! # Spec (backlog_performance_v1.txt §5)
//! - Maximum 2 retries → 3 total attempts.
//! - Base delay: 200 ms.
//! - Each subsequent delay doubles (200 ms → 400 ms).
//! - Every failed attempt is logged at `warn!` level.

use std::time::Duration;
use tracing::warn;

/// Retry `f` with exponential back-off.
///
/// * `label`       — caller-supplied string used in log messages (e.g. `"RDAP"`).
/// * `max_retries` — how many additional attempts after the first (spec: 2).
/// * `base_delay`  — initial sleep before the second attempt (spec: 200 ms).
/// * `f`           — closure that returns a `Future<Output = Result<T, E>>`.
///                   A fresh `Future` is created for every attempt by calling
///                   `f()` again, so `f` must be `FnMut`.
///
/// Returns the first `Ok(value)`, or the last `Err` if every attempt fails.
pub async fn retry_async<F, Fut, T, E>(
    label: &str,
    max_retries: u32,
    base_delay: Duration,
    mut f: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut delay = base_delay;

    for attempt in 0..=max_retries {
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) if attempt < max_retries => {
                warn!(
                    target: "atlasip::retry",
                    label = %label,
                    attempt = attempt + 1,
                    total = max_retries + 1,
                    delay_ms = delay.as_millis(),
                    error = %e,
                    "attempt failed, retrying"
                );
                tokio::time::sleep(delay).await;
                delay *= 2; // Exponential back-off.
            }
            Err(e) => return Err(e),
        }
    }

    // Unreachable: the loop above always returns inside the final iteration.
    unreachable!()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };

    // ── Succeeds on first attempt ─────────────────────────────────────────────

    #[tokio::test]
    async fn test_succeeds_first_attempt() {
        let calls = Arc::new(Mutex::new(0u32));
        let c = Arc::clone(&calls);

        let result: Result<&str, String> = retry_async(
            "TEST",
            2,
            Duration::from_millis(1),
            || {
                let c = Arc::clone(&c);
                async move {
                    *c.lock().unwrap() += 1;
                    Ok("ok")
                }
            },
        )
        .await;

        assert_eq!(result.unwrap(), "ok");
        assert_eq!(*calls.lock().unwrap(), 1, "should only be called once");
    }

    // ── Succeeds on second attempt ────────────────────────────────────────────

    #[tokio::test]
    async fn test_succeeds_on_retry() {
        let calls = Arc::new(Mutex::new(0u32));
        let c = Arc::clone(&calls);

        let result: Result<&str, String> = retry_async(
            "TEST",
            2,
            Duration::from_millis(1),
            || {
                let c = Arc::clone(&c);
                async move {
                    let mut n = c.lock().unwrap();
                    *n += 1;
                    if *n < 2 { Err("fail".to_string()) } else { Ok("ok") }
                }
            },
        )
        .await;

        assert_eq!(result.unwrap(), "ok");
        assert_eq!(*calls.lock().unwrap(), 2);
    }

    // ── Exhausts all retries and returns last error ───────────────────────────

    #[tokio::test]
    async fn test_all_attempts_fail() {
        let calls = Arc::new(Mutex::new(0u32));
        let c = Arc::clone(&calls);

        let result: Result<&str, String> = retry_async(
            "TEST",
            2,
            Duration::from_millis(1),
            || {
                let c = Arc::clone(&c);
                async move {
                    *c.lock().unwrap() += 1;
                    Err::<&str, _>("always fails".to_string())
                }
            },
        )
        .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "always fails");
        // 1 initial + 2 retries = 3 total calls.
        assert_eq!(*calls.lock().unwrap(), 3);
    }

    // ── Zero retries behaves like a single call ───────────────────────────────

    #[tokio::test]
    async fn test_zero_retries_no_retry() {
        let calls = Arc::new(Mutex::new(0u32));
        let c = Arc::clone(&calls);

        let result: Result<&str, String> = retry_async(
            "TEST",
            0,
            Duration::from_millis(1),
            || {
                let c = Arc::clone(&c);
                async move {
                    *c.lock().unwrap() += 1;
                    Err::<&str, _>("fail".to_string())
                }
            },
        )
        .await;

        assert!(result.is_err());
        assert_eq!(*calls.lock().unwrap(), 1, "zero retries = exactly one call");
    }

    // ── Delay doubles between attempts ───────────────────────────────────────
    // We only verify the call count here; actual timing is flaky in CI.

    #[tokio::test]
    async fn test_exponential_backoff_attempt_count() {
        let calls = Arc::new(Mutex::new(0u32));
        let c = Arc::clone(&calls);

        let _: Result<(), String> = retry_async(
            "EXP",
            2,
            Duration::from_millis(1),
            || {
                let c = Arc::clone(&c);
                async move {
                    *c.lock().unwrap() += 1;
                    Err("x".to_string())
                }
            },
        )
        .await;

        assert_eq!(*calls.lock().unwrap(), 3, "1 initial + 2 retries");
    }
}
