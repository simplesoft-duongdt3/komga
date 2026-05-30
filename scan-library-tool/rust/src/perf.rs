use std::time::Instant;
use serde::Serialize;
use chrono::Utc;

#[derive(Debug, Clone, Serialize)]
pub struct PhaseEntry {
    pub phase: String,
    pub elapsed_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanPerf {
    pub request_id: String,
    pub exported_at: String,
    pub total_ms: u64,
    pub phases: Vec<PhaseEntry>,
}

pub struct ScanTimer {
    request_id: String,
    start: Instant,
    phases: Vec<PhaseEntry>,
    current: Option<(String, Instant, Option<serde_json::Value>)>,
}

impl ScanTimer {
    pub fn new(request_id: String) -> Self {
        Self {
            request_id,
            start: Instant::now(),
            phases: Vec::new(),
            current: None,
        }
    }

    pub fn begin(&mut self, name: &str) {
        self.end_current();
        self.current = Some((name.to_string(), Instant::now(), None));
    }

    pub fn end(&mut self) {
        self.end_current();
    }

    fn end_current(&mut self) {
        if let Some((name, start, meta)) = self.current.take() {
            let elapsed_ms = start.elapsed().as_millis() as u64;
            self.phases.push(PhaseEntry {
                phase: name,
                elapsed_ms,
                meta,
            });
        }
    }

    pub fn record_phase(&mut self, name: &str, elapsed_ms: u64) {
        self.phases.push(PhaseEntry {
            phase: name.to_string(),
            elapsed_ms,
            meta: None,
        });
    }

    pub fn finish(&mut self) -> ScanPerf {
        self.end_current();
        let total_ms = self.start.elapsed().as_millis() as u64;
        ScanPerf {
            request_id: self.request_id.clone(),
            exported_at: Utc::now().to_rfc3339(),
            total_ms,
            phases: self.phases.clone(),
        }
    }
}
