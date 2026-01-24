//! Database module for persisting power readings and statistics
//!
//! Uses SQLite for efficient local storage of:
//! - Power readings (sampled data)
//! - Daily aggregated statistics
//! - Session tracking

use crate::core::{Error, PowerReading, Result};
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
}
