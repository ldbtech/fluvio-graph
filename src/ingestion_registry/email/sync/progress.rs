//! Shared Gmail sync progress for HTTP polling (e.g. `GET /sync/gmail/progress`).

use serde::Serialize;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
pub struct GmailSyncProgressSnapshot {
    pub running: bool,
    pub mode: String,
    pub phase: String,
    pub threads_done: u32,
    pub threads_total: u32,
    /// Rough 0–100 while fetching threads; `null` when indeterminate (incremental / query without totals).
    pub percent: Option<f32>,
    pub chunks: usize,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<GmailSyncResultSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GmailSyncResultSummary {
    pub chunks: usize,
    pub nodes_added: usize,
    pub structured_edges: usize,
    pub graph_nodes: usize,
    pub graph_edges: usize,
}

#[derive(Debug)]
struct Inner {
    snap: GmailSyncProgressSnapshot,
}

/// Thread-safe progress; clone the `Arc` into `GmailConnector` and the HTTP handler.
#[derive(Debug)]
pub struct GmailSyncProgress {
    inner: Mutex<Inner>,
}

impl GmailSyncProgress {
    pub fn new_idle() -> Self {
        Self {
            inner: Mutex::new(Inner {
                snap: GmailSyncProgressSnapshot {
                    running: false,
                    mode: String::new(),
                    phase: "idle".into(),
                    threads_done: 0,
                    threads_total: 0,
                    percent: None,
                    chunks: 0,
                    error: None,
                    result: None,
                },
            }),
        }
    }

    pub fn snapshot(&self) -> GmailSyncProgressSnapshot {
        self.inner.lock().unwrap().snap.clone()
    }

    /// Returns `Err` if a sync is already running.
    pub fn try_begin(&self, mode: &str) -> Result<(), ()> {
        let mut g = self.inner.lock().unwrap();
        if g.snap.running {
            return Err(());
        }
        g.snap.running = true;
        g.snap.mode = mode.to_string();
        g.snap.phase = "starting".into();
        g.snap.threads_done = 0;
        g.snap.threads_total = 0;
        g.snap.percent = Some(0.0);
        g.snap.chunks = 0;
        g.snap.error = None;
        g.snap.result = None;
        Ok(())
    }

    pub fn set_phase(&self, phase: &str) {
        let mut g = self.inner.lock().unwrap();
        g.snap.phase = phase.into();
    }

    pub fn set_listing_labels(&self) {
        let mut g = self.inner.lock().unwrap();
        g.snap.phase = "listing_labels".into();
        g.snap.percent = Some(2.0);
    }

    pub fn set_listing_threads(&self) {
        let mut g = self.inner.lock().unwrap();
        g.snap.phase = "listing_threads".into();
        g.snap.percent = Some(4.0);
    }

    pub fn set_thread_totals(&self, total: usize) {
        let mut g = self.inner.lock().unwrap();
        g.snap.threads_total = total as u32;
        g.snap.threads_done = 0;
        g.snap.phase = "fetching_threads".into();
    }

    /// `done` is 1-based index of last completed thread (or 0-based count done).
    pub fn set_thread_progress(&self, done: usize, total: usize, chunks: usize) {
        let mut g = self.inner.lock().unwrap();
        g.snap.threads_done = done as u32;
        g.snap.threads_total = total as u32;
        g.snap.chunks = chunks;
        g.snap.phase = "fetching_threads".into();
        if total > 0 {
            // Reserve headroom for graph write on the server (85% max here).
            let p = (done as f32 / total as f32) * 85.0;
            g.snap.percent = Some(p.min(85.0));
        } else {
            g.snap.percent = None;
        }
    }

    pub fn set_indeterminate_phase(&self, phase: &str) {
        let mut g = self.inner.lock().unwrap();
        g.snap.phase = phase.into();
        g.snap.percent = None;
    }

    pub fn set_message_list_progress(&self, done: usize, total: usize, chunks: usize) {
        let mut g = self.inner.lock().unwrap();
        g.snap.threads_done = done as u32;
        g.snap.threads_total = total as u32;
        g.snap.chunks = chunks;
        g.snap.phase = "fetching_messages".into();
        if total > 0 {
            g.snap.percent = Some((done as f32 / total as f32 * 80.0).min(80.0));
        } else {
            g.snap.percent = None;
        }
    }

    pub fn set_building_graph(&self) {
        let mut g = self.inner.lock().unwrap();
        g.snap.phase = "building_graph".into();
        g.snap.percent = Some(92.0);
    }

    pub fn finish_ok(&self, summary: GmailSyncResultSummary) {
        let mut g = self.inner.lock().unwrap();
        g.snap.running = false;
        g.snap.phase = "done".into();
        g.snap.percent = Some(100.0);
        g.snap.result = Some(summary);
        g.snap.error = None;
    }

    pub fn finish_err(&self, message: String) {
        let mut g = self.inner.lock().unwrap();
        g.snap.running = false;
        g.snap.phase = "error".into();
        g.snap.percent = None;
        g.snap.error = Some(message);
        g.snap.result = None;
    }
}
