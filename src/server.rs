use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::Deserialize;
use crate::state::SharedState;

static RUNNING: AtomicBool = AtomicBool::new(true);
static CLIENT_CONNECTED: AtomicBool = AtomicBool::new(false);
static LAST_HEARTBEAT_MS: AtomicU64 = AtomicU64::new(0);
static PENDING_EXIT_MS: AtomicU64 = AtomicU64::new(0);

const HTML_INDEX: &str = include_str!("web/index.html");

pub fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn is_running() -> bool {
    RUNNING.load(Ordering::Relaxed)
}

pub fn record_heartbeat() {
    LAST_HEARTBEAT_MS.store(now_epoch_ms(), Ordering::Relaxed);
    CLIENT_CONNECTED.store(true, Ordering::Relaxed);
}

pub fn schedule_exit(delay_ms: u64) {
    PENDING_EXIT_MS.store(now_epoch_ms() + delay_ms, Ordering::Relaxed);
}

#[allow(dead_code)]
pub fn stop_server() {
    RUNNING.store(false, Ordering::Relaxed);
}

#[derive(Deserialize)]
struct ConfigPayload {
    target_gateway: Option<String>,
    target_internet: Option<String>,
    ping_interval_ms: Option<u64>,
    sound_enabled: Option<bool>,
    monitoring_paused: Option<bool>,
}

pub fn start_server(port: u16, state: SharedState) -> Result<(), String> {
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).map_err(|e| format!("Failed to bind server to {}: {}", addr, e))?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;

    LAST_HEARTBEAT_MS.store(now_epoch_ms(), Ordering::Relaxed);

    // Watchdog thread: monitors client heartbeats and unload beacons
    thread::spawn(move || {
        let startup_time = std::time::Instant::now();
        let startup_grace_period = Duration::from_secs(20);

        while RUNNING.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(150));
            let now = now_epoch_ms();

            let pending_exit = PENDING_EXIT_MS.load(Ordering::Relaxed);
            if pending_exit > 0 && now >= pending_exit {
                RUNNING.store(false, Ordering::Relaxed);
                break;
            }

            if CLIENT_CONNECTED.load(Ordering::Relaxed) && startup_time.elapsed() > startup_grace_period {
                let last = LAST_HEARTBEAT_MS.load(Ordering::Relaxed);
                if now.saturating_sub(last) > 8000 {
                    RUNNING.store(false, Ordering::Relaxed);
                    break;
                }
            }
        }
    });

    // Accept loop in a separate thread
    let server_state = state.clone();
    thread::spawn(move || {
        while RUNNING.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let st = server_state.clone();
                    thread::spawn(move || {
                        handle_client(stream, st);
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => {
                    thread::sleep(Duration::from_millis(50));
                }
            }
        }
    });

    Ok(())
}

fn handle_client(mut stream: TcpStream, state: SharedState) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(4)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(4)));

    let mut buf = [0u8; 4096];
    let bytes_read = match stream.read(&mut buf) {
        Ok(n) if n > 0 => n,
        _ => return,
    };

    let req_str = String::from_utf8_lossy(&buf[..bytes_read]);
    let mut lines = req_str.lines();
    let req_line = match lines.next() {
        Some(l) => l,
        None => return,
    };

    let parts: Vec<&str> = req_line.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }

    let method = parts[0];
    let path = parts[1];

    if path == "/" || path == "/index.html" {
        send_response(&mut stream, "200 OK", "text/html; charset=utf-8", HTML_INDEX.as_bytes());
        return;
    }

    if path.starts_with("/api/heartbeat") {
        record_heartbeat();
        send_json(&mut stream, "{\"status\":\"ok\"}");
        return;
    }

    if path.starts_with("/api/status") {
        record_heartbeat();
        let json = {
            let guard = state.lock().unwrap();
            serde_json::to_string(&*guard).unwrap_or_else(|_| "{}".to_string())
        };
        send_json(&mut stream, &json);
        return;
    }

    if path.starts_with("/api/events") {
        record_heartbeat();
        let json = {
            let guard = state.lock().unwrap();
            serde_json::to_string(&guard.events).unwrap_or_else(|_| "[]".to_string())
        };
        send_json(&mut stream, &json);
        return;
    }

    if path.starts_with("/api/config") && method == "POST" {
        record_heartbeat();
        if let Some(body) = req_str.split("\r\n\r\n").nth(1) {
            if let Ok(cfg) = serde_json::from_str::<ConfigPayload>(body) {
                let mut guard = state.lock().unwrap();
                if let Some(gw) = cfg.target_gateway {
                    guard.target_gateway = gw;
                }
                if let Some(net) = cfg.target_internet {
                    guard.target_internet = net;
                }
                if let Some(intv) = cfg.ping_interval_ms {
                    guard.ping_interval_ms = intv.clamp(200, 10000);
                }
                if let Some(snd) = cfg.sound_enabled {
                    guard.sound_enabled = snd;
                }
                if let Some(pause) = cfg.monitoring_paused {
                    guard.monitoring_paused = pause;
                }
            }
        }
        send_json(&mut stream, "{\"success\":true}");
        return;
    }

    if path.starts_with("/api/action/clear") && method == "POST" {
        record_heartbeat();
        {
            let mut guard = state.lock().unwrap();
            guard.clear_events();
        }
        send_json(&mut stream, "{\"success\":true}");
        return;
    }

    if path.starts_with("/api/action/open-log") && method == "POST" {
        record_heartbeat();
        let log_file = {
            let guard = state.lock().unwrap();
            guard.log_file_path.clone()
        };
        // Launch notepad via native Win32 ShellExecuteW (ZERO cmd window)
        crate::window::open_file_native(&log_file);
        send_json(&mut stream, "{\"success\":true}");
        return;
    }

    if path.starts_with("/api/export") {
        record_heartbeat();
        if path.contains("format=csv") {
            let csv = {
                let guard = state.lock().unwrap();
                guard.export_csv()
            };
            let headers = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/csv\r\nContent-Disposition: attachment; filename=\"lan_disconnects_export.csv\"\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                csv.len()
            );
            let _ = stream.write_all(headers.as_bytes());
            let _ = stream.write_all(csv.as_bytes());
            return;
        } else {
            let json = {
                let guard = state.lock().unwrap();
                serde_json::to_string_pretty(&guard.events).unwrap_or_else(|_| "[]".to_string())
            };
            let headers = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Disposition: attachment; filename=\"lan_disconnects_export.json\"\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                json.len()
            );
            let _ = stream.write_all(headers.as_bytes());
            let _ = stream.write_all(json.as_bytes());
            return;
        }
    }

    if path.starts_with("/api/exit") {
        schedule_exit(200);
        send_json(&mut stream, "{\"success\":true,\"message\":\"Exiting\"}");
        return;
    }

    send_response(&mut stream, "404 Not Found", "text/plain", b"Not Found");
}

fn send_response(stream: &mut TcpStream, status: &str, content_type: &str, body: &[u8]) {
    let header = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
        status,
        content_type,
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
}

fn send_json(stream: &mut TcpStream, json: &str) {
    send_response(stream, "200 OK", "application/json; charset=utf-8", json.as_bytes());
}
