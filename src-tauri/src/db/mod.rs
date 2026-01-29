//! Database module for persisting power readings and statistics
//!
//! Uses SQLite for efficient local storage of:
//! - Power readings (sampled data)
//! - Daily aggregated statistics
//! - Session tracking

use crate::core::{Error, PowerReading, Result, Session};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Database manager
pub struct Database {
    conn: Connection,
}

/// Daily statistics record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyStats {
    pub date: String,
    pub total_wh: f64,
    pub total_cost: Option<f64>,
    pub avg_watts: f64,
    pub max_watts: f64,
    pub pricing_mode: Option<String>,
}

/// Power reading database record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerReadingRecord {
    pub id: i64,
    pub timestamp: i64,
    pub power_watts: f64,
    pub source: String,
    pub components: Option<String>,
}

impl Database {
    /// Create a new database connection
    pub fn new() -> Result<Self> {
        let db_path = Self::db_path()?;
        let conn = Connection::open(&db_path)?;

        let db = Self { conn };
        db.init_schema()?;

        Ok(db)
    }

    /// Get the database file path
    fn db_path() -> Result<PathBuf> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| Error::Database(rusqlite::Error::InvalidPath(PathBuf::new())))?;

        let app_dir = data_dir.join("powercost-tracker");
        std::fs::create_dir_all(&app_dir)?;

        Ok(app_dir.join("data.db"))
    }

    /// Initialize database schema
    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            -- Power readings history
            CREATE TABLE IF NOT EXISTS power_readings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp INTEGER NOT NULL,
                power_watts REAL NOT NULL,
                source TEXT NOT NULL,
                components TEXT
            );

            -- Daily aggregates
            CREATE TABLE IF NOT EXISTS daily_stats (
                date TEXT PRIMARY KEY,
                total_wh REAL NOT NULL,
                total_cost REAL,
                avg_watts REAL,
                max_watts REAL,
                pricing_mode TEXT
            );

            -- Sessions for surplus tracking
            CREATE TABLE IF NOT EXISTS sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                start_time INTEGER NOT NULL,
                end_time INTEGER,
                baseline_watts REAL,
                total_wh REAL,
                surplus_wh REAL,
                surplus_cost REAL,
                label TEXT
            );

            -- Indexes
            CREATE INDEX IF NOT EXISTS idx_readings_timestamp ON power_readings(timestamp);
            "#,
        )?;

        Ok(())
    }

    /// Insert a power reading
    pub fn insert_reading(&self, reading: &PowerReading) -> Result<()> {
        let components_json = reading
            .components
            .as_ref()
            .map(|c| serde_json::to_string(c).unwrap_or_default());

        self.conn.execute(
            "INSERT INTO power_readings (timestamp, power_watts, source, components) VALUES (?1, ?2, ?3, ?4)",
            params![reading.timestamp, reading.power_watts, reading.source, components_json],
        )?;

        Ok(())
    }

    /// Get power readings for a time range
    pub fn get_readings(&self, start: i64, end: i64) -> Result<Vec<PowerReadingRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, power_watts, source, components
             FROM power_readings
             WHERE timestamp >= ?1 AND timestamp <= ?2
             ORDER BY timestamp ASC",
        )?;

        let readings = stmt
            .query_map(params![start, end], |row| {
                Ok(PowerReadingRecord {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    power_watts: row.get(2)?,
                    source: row.get(3)?,
                    components: row.get(4)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(readings)
    }

    /// Update or insert daily statistics
    pub fn upsert_daily_stats(&self, stats: &DailyStats) -> Result<()> {
        self.conn.execute(
            r#"INSERT INTO daily_stats (date, total_wh, total_cost, avg_watts, max_watts, pricing_mode)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6)
               ON CONFLICT(date) DO UPDATE SET
                   total_wh = ?2,
                   total_cost = ?3,
                   avg_watts = ?4,
                   max_watts = ?5,
                   pricing_mode = ?6"#,
            params![
                stats.date,
                stats.total_wh,
                stats.total_cost,
                stats.avg_watts,
                stats.max_watts,
                stats.pricing_mode
            ],
        )?;

        Ok(())
    }

    /// Get daily statistics for a date range
    pub fn get_daily_stats(&self, start: &str, end: &str) -> Result<Vec<DailyStats>> {
        let mut stmt = self.conn.prepare(
            "SELECT date, total_wh, total_cost, avg_watts, max_watts, pricing_mode
             FROM daily_stats
             WHERE date >= ?1 AND date <= ?2
             ORDER BY date ASC",
        )?;

        let stats = stmt
            .query_map(params![start, end], |row| {
                Ok(DailyStats {
                    date: row.get(0)?,
                    total_wh: row.get(1)?,
                    total_cost: row.get(2)?,
                    avg_watts: row.get(3)?,
                    max_watts: row.get(4)?,
                    pricing_mode: row.get(5)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(stats)
    }

    /// Clean up old readings (keep only last N days of detailed data)
    pub fn cleanup_old_readings(&self, days_to_keep: u32) -> Result<u64> {
        let cutoff = chrono::Utc::now().timestamp() - (days_to_keep as i64 * 24 * 60 * 60);

        let deleted = self.conn.execute(
            "DELETE FROM power_readings WHERE timestamp < ?1",
            params![cutoff],
        )?;

        Ok(deleted as u64)
    }

    /// Get total readings count
    pub fn get_readings_count(&self) -> Result<i64> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM power_readings", [], |row| row.get(0))?;
        Ok(count)
    }

    /// Compute and update daily stats from power readings for a specific date
    /// This aggregates all readings for the given date and updates the daily_stats table
    /// If `rate_per_kwh` is provided, cost will be calculated as total_kwh * rate
    pub fn update_daily_stats_for_date(&self, date: &str, pricing_mode: Option<&str>, rate_per_kwh: Option<f64>) -> Result<Option<DailyStats>> {
        // Get start and end timestamps for the date
        let start_of_day = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|e| Error::Database(rusqlite::Error::InvalidParameterName(e.to_string())))?
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        let end_of_day = start_of_day + 86400; // 24 hours in seconds

        // Aggregate readings for this date
        let result: std::result::Result<(f64, f64, f64, i64), rusqlite::Error> = self.conn.query_row(
            "SELECT
                COALESCE(AVG(power_watts), 0.0) as avg_watts,
                COALESCE(MAX(power_watts), 0.0) as max_watts,
                COALESCE(SUM(power_watts), 0.0) as sum_watts,
                COUNT(*) as count
             FROM power_readings
             WHERE timestamp >= ?1 AND timestamp < ?2",
            params![start_of_day, end_of_day],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        );

        match result {
            Ok((avg_watts, max_watts, sum_watts, count)) => {
                if count == 0 {
                    return Ok(None);
                }

                // Estimate total Wh based on average power and assumed runtime
                // Since readings are taken every ~10 seconds (every 10 monitoring cycles at 1s each),
                // we can estimate energy from the sum of power readings
                // Each reading represents approximately 10 seconds of monitoring
                let hours_per_reading = 10.0 / 3600.0; // 10 seconds in hours
                let total_wh = sum_watts * hours_per_reading;

                let total_cost = rate_per_kwh.map(|rate| (total_wh / 1000.0) * rate);

                let stats = DailyStats {
                    date: date.to_string(),
                    total_wh,
                    total_cost,
                    avg_watts,
                    max_watts,
                    pricing_mode: pricing_mode.map(String::from),
                };

                self.upsert_daily_stats(&stats)?;
                Ok(Some(stats))
            }
            Err(e) => Err(Error::Database(e)),
        }
    }

    /// Update daily stats for today based on current readings
    pub fn update_today_stats(&self, pricing_mode: Option<&str>, rate_per_kwh: Option<f64>) -> Result<Option<DailyStats>> {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        self.update_daily_stats_for_date(&today, pricing_mode, rate_per_kwh)
    }

    /// Rebuild daily stats for all dates that have readings
    pub fn rebuild_all_daily_stats(&self, pricing_mode: Option<&str>, rate_per_kwh: Option<f64>) -> Result<u32> {
        // Get all distinct dates from power_readings
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT date(timestamp, 'unixepoch') as reading_date
             FROM power_readings
             ORDER BY reading_date ASC"
        )?;

        let dates: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        let mut count = 0;
        for date in dates {
            if self.update_daily_stats_for_date(&date, pricing_mode, rate_per_kwh)?.is_some() {
                count += 1;
            }
        }

        Ok(count)
    }

    // ===== Session Management =====

    /// Start a new tracking session
    pub fn start_session(&self, baseline_watts: f64, label: Option<&str>) -> Result<i64> {
        let now = chrono::Utc::now().timestamp();

        self.conn.execute(
            "INSERT INTO sessions (start_time, baseline_watts, total_wh, surplus_wh, surplus_cost, label)
             VALUES (?1, ?2, 0.0, 0.0, 0.0, ?3)",
            params![now, baseline_watts, label],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// End a tracking session
    pub fn end_session(&self, session_id: i64, total_wh: f64, surplus_wh: f64, surplus_cost: f64) -> Result<Option<Session>> {
        let now = chrono::Utc::now().timestamp();

        self.conn.execute(
            "UPDATE sessions SET end_time = ?1, total_wh = ?2, surplus_wh = ?3, surplus_cost = ?4
             WHERE id = ?5",
            params![now, total_wh, surplus_wh, surplus_cost, session_id],
        )?;

        self.get_session(session_id)
    }

    /// Get a specific session by ID
    pub fn get_session(&self, session_id: i64) -> Result<Option<Session>> {
        let result = self.conn.query_row(
            "SELECT id, start_time, end_time, baseline_watts, total_wh, surplus_wh, surplus_cost, label
             FROM sessions WHERE id = ?1",
            params![session_id],
            |row| {
                Ok(Session {
                    id: Some(row.get(0)?),
                    start_time: row.get(1)?,
                    end_time: row.get(2)?,
                    baseline_watts: row.get(3)?,
                    total_wh: row.get(4)?,
                    surplus_wh: row.get(5)?,
                    surplus_cost: row.get(6)?,
                    label: row.get(7)?,
                })
            },
        );

        match result {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Error::Database(e)),
        }
    }

    /// Get all sessions, optionally limited
    pub fn get_sessions(&self, limit: Option<u32>) -> Result<Vec<Session>> {
        let query = match limit {
            Some(n) => format!(
                "SELECT id, start_time, end_time, baseline_watts, total_wh, surplus_wh, surplus_cost, label
                 FROM sessions ORDER BY start_time DESC LIMIT {}", n
            ),
            None => "SELECT id, start_time, end_time, baseline_watts, total_wh, surplus_wh, surplus_cost, label
                     FROM sessions ORDER BY start_time DESC".to_string(),
        };

        let mut stmt = self.conn.prepare(&query)?;

        let sessions = stmt
            .query_map([], |row| {
                Ok(Session {
                    id: Some(row.get(0)?),
                    start_time: row.get(1)?,
                    end_time: row.get(2)?,
                    baseline_watts: row.get(3)?,
                    total_wh: row.get(4)?,
                    surplus_wh: row.get(5)?,
                    surplus_cost: row.get(6)?,
                    label: row.get(7)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(sessions)
    }

    /// Update session statistics (called during active session)
    pub fn update_session_stats(&self, session_id: i64, total_wh: f64, surplus_wh: f64, surplus_cost: f64) -> Result<()> {
        self.conn.execute(
            "UPDATE sessions SET total_wh = ?1, surplus_wh = ?2, surplus_cost = ?3 WHERE id = ?4",
            params![total_wh, surplus_wh, surplus_cost, session_id],
        )?;
        Ok(())
    }

    /// Get the most recent active (unended) session
    pub fn get_active_session(&self) -> Result<Option<Session>> {
        let result = self.conn.query_row(
            "SELECT id, start_time, end_time, baseline_watts, total_wh, surplus_wh, surplus_cost, label
             FROM sessions WHERE end_time IS NULL ORDER BY start_time DESC LIMIT 1",
            [],
            |row| {
                Ok(Session {
                    id: Some(row.get(0)?),
                    start_time: row.get(1)?,
                    end_time: row.get(2)?,
                    baseline_watts: row.get(3)?,
                    total_wh: row.get(4)?,
                    surplus_wh: row.get(5)?,
                    surplus_cost: row.get(6)?,
                    label: row.get(7)?,
                })
            },
        );

        match result {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Error::Database(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn create_test_db() -> Database {
        let conn = Connection::open_in_memory().unwrap();
        let db = Database { conn };
        db.init_schema().unwrap();
        db
    }

    #[test]
    fn test_insert_and_get_reading() {
        let db = create_test_db();

        let reading = PowerReading::new(100.0, "test", false);
        db.insert_reading(&reading).unwrap();

        let readings = db.get_readings(0, i64::MAX).unwrap();
        assert_eq!(readings.len(), 1);
        assert!((readings[0].power_watts - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_daily_stats() {
        let db = create_test_db();

        let stats = DailyStats {
            date: "2024-01-15".into(),
            total_wh: 1500.0,
            total_cost: Some(0.35),
            avg_watts: 62.5,
            max_watts: 150.0,
            pricing_mode: Some("simple".into()),
        };

        db.upsert_daily_stats(&stats).unwrap();

        let retrieved = db.get_daily_stats("2024-01-01", "2024-01-31").unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].date, "2024-01-15");
    }

    #[test]
    fn test_update_daily_stats_from_readings() {
        let db = create_test_db();

        // Insert readings with a specific timestamp (2024-01-15 12:00:00 UTC)
        let base_timestamp = 1705320000i64; // 2024-01-15 12:00:00 UTC

        // Insert multiple readings
        for i in 0..6 {
            db.conn.execute(
                "INSERT INTO power_readings (timestamp, power_watts, source, components) VALUES (?1, ?2, ?3, NULL)",
                params![base_timestamp + i * 10, 100.0 + (i as f64 * 10.0), "test"],
            ).unwrap();
        }

        // Update daily stats for that date
        let result = db.update_daily_stats_for_date("2024-01-15", Some("simple"), Some(0.20)).unwrap();
        assert!(result.is_some());

        let stats = result.unwrap();
        assert_eq!(stats.date, "2024-01-15");
        assert!(stats.avg_watts > 0.0);
        assert!(stats.max_watts >= stats.avg_watts);
        assert!(stats.total_wh > 0.0);
        assert_eq!(stats.pricing_mode, Some("simple".to_string()));

        // Verify it was saved to the database
        let retrieved = db.get_daily_stats("2024-01-15", "2024-01-15").unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].date, "2024-01-15");
    }

    #[test]
    fn test_update_daily_stats_no_readings() {
        let db = create_test_db();

        // Try to update stats for a date with no readings
        let result = db.update_daily_stats_for_date("2024-01-15", Some("simple"), None).unwrap();
        assert!(result.is_none());
    }
}
