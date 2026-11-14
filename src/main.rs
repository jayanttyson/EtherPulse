#![windows_subsystem = "windows"]

mod icmp;
mod server;
mod state;
mod window;

use std::sync::Arc;
use std::thread;
use std::time::Duration;
use icmp::{query_system_adapter, IcmpEngine};
use state::{AppState, SharedState};

fn find_available_port(start_port: u16) -> u16 {
    for port in start_port..start_port + 20 {
        if std::net::TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return port;
        }
    }
    start_port
}

fn main() {
    // 1. Single instance check
    if window::is_already_running() {
        // Already running, launch desktop window to activate existing instance
        window::launch_desktop_window("http://127.0.0.1:3000");
        return;
    }

    // 2. Determine port (default 3000)
    let port = find_available_port(3000);

    // 3. Initialize Shared State immediately
    let state: SharedState = Arc::new(std::sync::Mutex::new(AppState::new("lan_disconnects.txt", port)));

    // 4. Start HTTP Server immediately so UI loads without any delay
    let app_url = format!("http://127.0.0.1:{}", port);
    if let Err(e) = server::start_server(port, state.clone()) {
        eprintln!("Failed to start server: {}", e);
        return;
    }

    // 5. Launch Single Desktop Application Window
    window::launch_desktop_window(&app_url);

    // 6. Spawn background ICMP ping worker
    let ping_state = state.clone();
    thread::spawn(move || {
        let engine = match IcmpEngine::new() {
            Ok(e) => e,
            Err(err) => {
                eprintln!("Failed to init ICMP engine: {}", err);
                return;
            }
        };

        let mut loop_counter: u64 = 0;

        while server::is_running() {
            let (target_gw, target_net, interval_ms, is_paused) = {
                let guard = ping_state.lock().unwrap();
                (
                    guard.target_gateway.clone(),
                    guard.target_internet.clone(),
                    guard.ping_interval_ms,
                    guard.monitoring_paused,
                )
            };

            if !is_paused {
                // 1. Ping Gateway Target (Primary LAN Cable Integrity Check)
                let gw_result = engine.ping(&target_gw, 800);
                {
                    let mut guard = ping_state.lock().unwrap();
                    guard.record_gateway_ping(&gw_result);
                }

                // 2. Ping Internet Target every 2 cycles to isolate LAN vs WAN
                if loop_counter % 2 == 0 {
                    let net_result = engine.ping(&target_net, 1000);
                    let mut guard = ping_state.lock().unwrap();
                    guard.record_internet_ping(&net_result);
                }
            }

            loop_counter = loop_counter.wrapping_add(1);
            thread::sleep(Duration::from_millis(interval_ms));
        }
    });

    // 7. Spawn background adapter refresher thread (runs immediately once, then every 10 seconds)
    let adapter_state = state.clone();
    thread::spawn(move || {
        let info = query_system_adapter();
        {
            let mut guard = adapter_state.lock().unwrap();
            if guard.target_gateway == "192.168.1.1" && !info.gateway.is_empty() {
                guard.target_gateway = info.gateway.clone();
            }
            guard.adapter = info;
        }

        while server::is_running() {
            thread::sleep(Duration::from_secs(10));
            let info = query_system_adapter();
            let mut guard = adapter_state.lock().unwrap();
            guard.adapter = info;
        }
    });

    // 8. Main thread lifecycle loop
    while server::is_running() {
        thread::sleep(Duration::from_millis(200));
    }
}
