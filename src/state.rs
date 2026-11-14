use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use crate::icmp::{chrono_date_time_str, chrono_now_str, AdapterInfo, PingResult};

pub type SharedState = Arc<Mutex<AppState>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencySample {
    pub time: String,
    pub rtt_ms: u32,
    pub success: bool,
    pub is_drop: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    pub id: u64,
    pub timestamp: String,
    pub event_type: String, // "DISCONNECT", "RESTORED", "PACKET_LOSS"
    pub target: String,
    pub duration_secs: Option<f32>,
    pub details: String,
    pub badge_type: String, // "danger", "success", "warning"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub is_gateway_online: bool,
    pub is_internet_online: bool,
    pub current_rtt_ms: u32,
    pub internet_rtt_ms: u32,
    pub target_gateway: String,
    pub target_internet: String,
    pub ping_interval_ms: u64,
    pub total_pings: u64,
    pub successful_pings: u64,
    pub failed_pings: u64,
    pub total_disconnects: u64,
    pub total_downtime_ms: u64,
    pub current_streak_secs: u64,
    pub monitoring_started_at: String,
    pub last_disconnect_start_ms: Option<u64>,
    pub last_disconnect_time_str: String,
    pub last_restored_time_str: String,
    pub rolling_latency: Vec<LatencySample>,
    pub events: Vec<LogEvent>,
    pub adapter: AdapterInfo,
    pub sound_enabled: bool,
    pub monitoring_paused: bool,
    pub log_file_path: String,
    pub port: u16,
}

impl AppState {
    pub fn new(log_path: &str, port: u16) -> Self {
        let now_str = chrono_date_time_str();
        
        // Ensure log file has initial start header if fresh
        let file_exists = PathBuf::from(log_path).exists();
        if !file_exists {
            if let Ok(mut file) = OpenOptions::new().create(true).write(true).open(log_path) {
                let _ = writeln!(file, "Monitoring started at {}", now_str);
            }
        }

        Self {
            is_gateway_online: true,
            is_internet_online: true,
            current_rtt_ms: 0,
            internet_rtt_ms: 0,
            target_gateway: "192.168.1.1".to_string(),
            target_internet: "1.1.1.1".to_string(),
            ping_interval_ms: 1000,
            total_pings: 0,
            successful_pings: 0,
            failed_pings: 0,
            total_disconnects: 0,
            total_downtime_ms: 0,
            current_streak_secs: 0,
            monitoring_started_at: now_str,
            last_disconnect_start_ms: None,
            last_disconnect_time_str: "None".to_string(),
            last_restored_time_str: "None".to_string(),
            rolling_latency: Vec::with_capacity(120),
            events: Vec::with_capacity(100),
            adapter: AdapterInfo::default(),
            sound_enabled: true,
            monitoring_paused: false,
            log_file_path: log_path.to_string(),
            port,
        }
    }

    pub fn record_gateway_ping(&mut self, res: &PingResult) {
        if self.monitoring_paused {
            return;
        }

        self.total_pings += 1;
        let time_now = chrono_now_str();
        let date_time_now = chrono_date_time_str();
        let now_epoch_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        if res.success {
            self.successful_pings += 1;
            self.current_rtt_ms = res.rtt_ms;
            self.current_streak_secs += (self.ping_interval_ms / 1000).max(1);

            // Was it previously disconnected?
            if !self.is_gateway_online {
                self.is_gateway_online = true;
                self.last_restored_time_str = date_time_now.clone();

                let outage_duration_secs = if let Some(start_ms) = self.last_disconnect_start_ms {
                    let diff_ms = now_epoch_ms.saturating_sub(start_ms);
                    self.total_downtime_ms += diff_ms;
                    (diff_ms as f32) / 1000.0
                } else {
                    1.0
                };
                self.last_disconnect_start_ms = None;

                // Log restoration event
                let event = LogEvent {
                    id: self.events.len() as u64 + 1,
                    timestamp: date_time_now.clone(),
                    event_type: "RESTORED".to_string(),
                    target: self.target_gateway.clone(),
                    duration_secs: Some(outage_duration_secs),
                    details: format!("Connection restored after {:.1}s outage (RTT: {}ms)", outage_duration_secs, res.rtt_ms),
                    badge_type: "success".to_string(),
                };
                self.events.insert(0, event);
                if self.events.len() > 100 {
                    self.events.pop();
                }

                // Also append to lan_disconnects.txt for full clarity
                self.append_log(&format!("RESTORED at {} (Outage: {:.1}s)", date_time_now, outage_duration_secs));
            }
        } else {
            self.failed_pings += 1;
            self.current_rtt_ms = 0;
            self.current_streak_secs = 0;

            // Is this the beginning of a disconnect?
            if self.is_gateway_online {
                self.is_gateway_online = false;
                self.total_disconnects += 1;
                self.last_disconnect_start_ms = Some(now_epoch_ms);
                self.last_disconnect_time_str = date_time_now.clone();

                // Log disconnect event
                let event = LogEvent {
                    id: self.events.len() as u64 + 1,
                    timestamp: date_time_now.clone(),
                    event_type: "DISCONNECT".to_string(),
                    target: self.target_gateway.clone(),
                    duration_secs: None,
                    details: format!("LAN Disconnect detected: {}", res.status_msg),
                    badge_type: "danger".to_string(),
                };
                self.events.insert(0, event);
                if self.events.len() > 100 {
                    self.events.pop();
                }

                // Write to lan_disconnects.txt matching user's original format!
                self.append_log(&format!("DISCONNECT at {}", date_time_now));
            }
        }

        // Add to rolling latency samples
        self.rolling_latency.push(LatencySample {
            time: time_now,
            rtt_ms: if res.success { res.rtt_ms } else { 0 },
            success: res.success,
            is_drop: !res.success,
        });

        if self.rolling_latency.len() > 90 {
            self.rolling_latency.remove(0);
        }
    }

    pub fn record_internet_ping(&mut self, res: &PingResult) {
        if self.monitoring_paused {
            return;
        }
        self.is_internet_online = res.success;
        self.internet_rtt_ms = if res.success { res.rtt_ms } else { 0 };
    }

    fn append_log(&self, line: &str) {
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&self.log_file_path) {
            let _ = writeln!(file, "{}", line);
        }
    }

    pub fn clear_events(&mut self) {
        self.events.clear();
        self.total_disconnects = 0;
        self.total_downtime_ms = 0;
        self.failed_pings = 0;
        self.successful_pings = 0;
        self.total_pings = 0;
    }

    pub fn export_csv(&self) -> String {
        let mut csv = String::from("ID,Timestamp,EventType,Target,DurationSeconds,Details\n");
        for ev in &self.events {
            let dur_str = ev.duration_secs.map(|d| format!("{:.1}", d)).unwrap_or_default();
            csv.push_str(&format!(
                "{},\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
                ev.id, ev.timestamp, ev.event_type, ev.target, dur_str, ev.details
            ));
        }
        csv
    }
}
