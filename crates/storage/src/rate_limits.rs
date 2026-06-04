use chrono::Utc;
use common::CiteError;
use rusqlite::{params, OptionalExtension};

use crate::util::{format_dt, i64_to_u32, storage_err};
use crate::Database;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateLimitDecision {
    Allowed,
    Blocked { retry_after_seconds: u32 },
}

impl Database {
    pub fn check_and_increment_rate_limit(
        &self,
        route: &str,
        key: &str,
        max_requests: u32,
        window_seconds: u32,
    ) -> Result<RateLimitDecision, CiteError> {
        let now_epoch = Utc::now().timestamp();
        self.check_and_increment_rate_limit_at(route, key, max_requests, window_seconds, now_epoch)
    }

    pub fn check_and_increment_rate_limit_at(
        &self,
        route: &str,
        key: &str,
        max_requests: u32,
        window_seconds: u32,
        now_epoch: i64,
    ) -> Result<RateLimitDecision, CiteError> {
        if max_requests == 0 || window_seconds == 0 {
            return Err(CiteError::InvalidParameter {
                message: "rate limit config must have max_requests>0 and window_seconds>0"
                    .to_string(),
            });
        }

        let window = window_seconds as i64;
        let window_start_epoch = (now_epoch / window) * window;
        let now = format_dt(&Utc::now());

        let tx = self.conn.unchecked_transaction().map_err(storage_err)?;

        let current_count: Option<i64> = tx
            .query_row(
                "SELECT request_count FROM rate_limit_counters
                 WHERE key = ?1 AND route = ?2 AND window_start_epoch = ?3",
                params![key, route, window_start_epoch],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_err)?;

        if current_count.unwrap_or(0) >= max_requests as i64 {
            tx.commit().map_err(storage_err)?;
            let retry_after = i64_to_u32(
                "retry_after",
                ((window_start_epoch + window) - now_epoch).max(1),
            )?;
            return Ok(RateLimitDecision::Blocked {
                retry_after_seconds: retry_after,
            });
        }

        tx.execute(
            "INSERT INTO rate_limit_counters (key, route, window_start_epoch, request_count, updated_at)
             VALUES (?1, ?2, ?3, 1, ?4)
             ON CONFLICT(key, route, window_start_epoch)
             DO UPDATE SET request_count = request_count + 1, updated_at = excluded.updated_at",
            params![key, route, window_start_epoch, now],
        )
        .map_err(storage_err)?;

        tx.commit().map_err(storage_err)?;
        Ok(RateLimitDecision::Allowed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_allows_up_to_max_then_blocks() {
        let db = Database::open_memory().unwrap();
        let key = "local:test-provider";
        let route = "search";
        let window = 60;
        let now = 1_700_000_000;

        assert_eq!(
            db.check_and_increment_rate_limit_at(route, key, 2, window, now)
                .unwrap(),
            RateLimitDecision::Allowed
        );
        assert_eq!(
            db.check_and_increment_rate_limit_at(route, key, 2, window, now + 1)
                .unwrap(),
            RateLimitDecision::Allowed
        );

        let decision = db
            .check_and_increment_rate_limit_at(route, key, 2, window, now + 2)
            .unwrap();
        assert!(matches!(decision, RateLimitDecision::Blocked { .. }));
    }

    #[test]
    fn test_rate_limit_window_rollover_allows_again() {
        let db = Database::open_memory().unwrap();
        let key = "local:test-provider";
        let route = "context";
        let window = 60;
        let now = 1_700_000_000;

        assert_eq!(
            db.check_and_increment_rate_limit_at(route, key, 1, window, now)
                .unwrap(),
            RateLimitDecision::Allowed
        );

        let blocked = db
            .check_and_increment_rate_limit_at(route, key, 1, window, now + 1)
            .unwrap();
        assert!(matches!(blocked, RateLimitDecision::Blocked { .. }));

        assert_eq!(
            db.check_and_increment_rate_limit_at(route, key, 1, window, now + 61)
                .unwrap(),
            RateLimitDecision::Allowed
        );
    }

    #[test]
    fn test_rate_limit_persists_across_db_reopen() {
        let temp_dir = std::env::temp_dir().join(format!(
            "aicite_rate_limit_{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let key = "local:test-provider";
        let route = "retrieve";
        let window = 60;
        let now = 1_700_000_000;

        {
            let db = Database::open(&temp_dir).unwrap();
            assert_eq!(
                db.check_and_increment_rate_limit_at(route, key, 1, window, now)
                    .unwrap(),
                RateLimitDecision::Allowed
            );
        }

        {
            let db = Database::open(&temp_dir).unwrap();
            let blocked = db
                .check_and_increment_rate_limit_at(route, key, 1, window, now + 1)
                .unwrap();
            assert!(matches!(blocked, RateLimitDecision::Blocked { .. }));
        }

        let _ = std::fs::remove_file(temp_dir.join("cite.db"));
        let _ = std::fs::remove_file(temp_dir.join("cite.db-wal"));
        let _ = std::fs::remove_file(temp_dir.join("cite.db-shm"));
        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
