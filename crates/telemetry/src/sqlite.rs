use crate::history::{HistoryStore, Sample};
use rusqlite::{params, Connection};
use shookshmam_sensor_model::{SensorStatus, Snapshot};
use std::path::Path;

/// Durable SQLite history store (PHASE-1-SPEC §7).
///
/// - UTC millisecond timestamps.
/// - Batched inserts inside a single transaction.
/// - Indexes on `(sensor_id, timestamp)` and `timestamp`.
/// - 30-day retention is applied by the caller via `prune(retention_cutoff_ms(..))`.
/// - No data ever leaves the machine.
pub struct SqliteHistory {
    conn: Connection,
}

impl SqliteHistory {
    /// Open (creating if needed) a history database at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        Self::init(conn)
    }

    /// Open an in-memory database (handy for tests).
    pub fn open_in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        Self::init(conn)
    }

    fn init(conn: Connection) -> Result<Self, String> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS samples (
                 sensor_id   TEXT    NOT NULL,
                 device_id   TEXT    NOT NULL,
                 value       REAL,
                 unit        TEXT    NOT NULL,
                 status      TEXT    NOT NULL,
                 source      TEXT    NOT NULL,
                 ts_ms       INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_samples_sensor_ts
                 ON samples (sensor_id, ts_ms);
             CREATE INDEX IF NOT EXISTS idx_samples_ts
                 ON samples (ts_ms);
             CREATE TABLE IF NOT EXISTS meta (
                 key   TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );",
        )
        .map_err(|e| e.to_string())?;
        Ok(SqliteHistory { conn })
    }
}

impl HistoryStore for SqliteHistory {
    fn insert_snapshot(&mut self, snapshot: &Snapshot) -> Result<usize, String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        let mut inserted = 0usize;
        {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT INTO samples
                     (sensor_id, device_id, value, unit, status, source, ts_ms)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                )
                .map_err(|e| e.to_string())?;
            for r in &snapshot.readings {
                stmt.execute(params![
                    r.id,
                    r.device_id,
                    r.value,
                    r.unit,
                    status_to_str(r.status),
                    r.source,
                    r.timestamp_unix_ms,
                ])
                .map_err(|e| e.to_string())?;
                inserted += 1;
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(inserted)
    }

    fn query(&self, sensor_id: &str, from_ms: i64, to_ms: i64) -> Result<Vec<Sample>, String> {
        let mut stmt = self
            .conn
            .prepare_cached(
                "SELECT sensor_id, device_id, value, unit, status, source, ts_ms
                 FROM samples
                 WHERE sensor_id = ?1 AND ts_ms >= ?2 AND ts_ms <= ?3
                 ORDER BY ts_ms ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![sensor_id, from_ms, to_ms], |row| {
                Ok(Sample {
                    sensor_id: row.get(0)?,
                    device_id: row.get(1)?,
                    value: row.get(2)?,
                    unit: row.get(3)?,
                    status: status_from_str(&row.get::<_, String>(4)?),
                    source: row.get(5)?,
                    timestamp_unix_ms: row.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    fn prune(&mut self, cutoff_ms: i64) -> Result<usize, String> {
        // Delete in bounded batches so a long-neglected DB cannot block the app
        // on one huge DELETE (PHASE-1-SPEC §7).
        const BATCH: usize = 5_000;
        let mut total = 0usize;
        loop {
            let n = self
                .conn
                .execute(
                    "DELETE FROM samples WHERE rowid IN
                     (SELECT rowid FROM samples WHERE ts_ms < ?1 LIMIT ?2)",
                    params![cutoff_ms, BATCH as i64],
                )
                .map_err(|e| e.to_string())?;
            total += n;
            if n < BATCH {
                break;
            }
        }
        self.conn
            .execute(
                "INSERT INTO meta (key, value) VALUES ('last_prune_ms', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![cutoff_ms.to_string()],
            )
            .map_err(|e| e.to_string())?;
        Ok(total)
    }

    fn last_prune_ms(&self) -> Option<i64> {
        self.conn
            .query_row("SELECT value FROM meta WHERE key = 'last_prune_ms'", [], |row| {
                row.get::<_, String>(0)
            })
            .ok()
            .and_then(|s| s.parse::<i64>().ok())
    }

    fn count(&self) -> Result<usize, String> {
        self.conn
            .query_row("SELECT COUNT(*) FROM samples", [], |row| row.get::<_, i64>(0))
            .map(|n| n as usize)
            .map_err(|e| e.to_string())
    }
}

fn status_to_str(s: SensorStatus) -> &'static str {
    match s {
        SensorStatus::Available => "available",
        SensorStatus::Unsupported => "unsupported",
        SensorStatus::Unavailable => "unavailable",
        SensorStatus::PermissionDenied => "permission_denied",
        SensorStatus::Stale => "stale",
        SensorStatus::Error => "error",
    }
}

fn status_from_str(s: &str) -> SensorStatus {
    match s {
        "available" => SensorStatus::Available,
        "unsupported" => SensorStatus::Unsupported,
        "unavailable" => SensorStatus::Unavailable,
        "permission_denied" => SensorStatus::PermissionDenied,
        "stale" => SensorStatus::Stale,
        _ => SensorStatus::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::retention_cutoff_ms;
    use shookshmam_sensor_model::{SensorCategory, SensorReading};

    fn snap(ts: i64, val: f64) -> Snapshot {
        Snapshot::new(
            ts,
            vec![SensorReading::available(
                "cpu.temp",
                "cpu0",
                "CPU",
                SensorCategory::Temperature,
                val,
                "sim",
                ts,
            )],
        )
    }

    #[test]
    fn insert_query_prune_roundtrip() {
        let mut h = SqliteHistory::open_in_memory().unwrap();
        let day = 24 * 60 * 60 * 1000i64;
        h.insert_snapshot(&snap(day, 40.0)).unwrap();
        h.insert_snapshot(&snap(40 * day, 41.0)).unwrap();
        assert_eq!(h.count().unwrap(), 2);

        let got = h.query("cpu.temp", 0, 100 * day).unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].value, Some(40.0));

        // Retain 30 days relative to day 45 -> the day-1 sample expires.
        let removed = h.prune(retention_cutoff_ms(45 * day, 30)).unwrap();
        assert_eq!(removed, 1);
        assert_eq!(h.count().unwrap(), 1);
        assert!(h.last_prune_ms().is_some());
    }

    #[test]
    fn status_roundtrips() {
        for s in [
            SensorStatus::Available,
            SensorStatus::Stale,
            SensorStatus::Unavailable,
            SensorStatus::Unsupported,
            SensorStatus::PermissionDenied,
            SensorStatus::Error,
        ] {
            assert_eq!(status_from_str(status_to_str(s)), s);
        }
    }
}
