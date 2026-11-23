//! Telemetry scheduling, freshness tracking, bounded in-memory history and the
//! durable SQLite history store (PHASE-1-SPEC §5, §7; work items P1-06, P1-07).

mod freshness;
mod history;
mod scheduler;
mod sqlite;

pub use freshness::{RingBuffer, TelemetryTracker};
pub use history::{retention_cutoff_ms, HistoryStore, MemoryHistory, Sample};
pub use scheduler::Scheduler;
pub use sqlite::SqliteHistory;
