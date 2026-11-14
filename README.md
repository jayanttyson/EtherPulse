# EtherPulse • LAN Telemetry & Watchdog

> **High-frequency LAN cable integrity monitoring, gateway ping telemetry, and instant disconnect logging for Windows — 100% pure Rust, single-window UI, and absolute zero command prompt windows.**

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%20%7C%2011-blue.svg)](https://microsoft.com/windows)
[![Subsystem](https://img.shields.io/badge/subsystem-Windows%20GUI%20(Zero%20CMD)-brightgreen.svg)]()
[![License](https://img.shields.io/badge/license-MIT-purple.svg)](LICENSE)

---

## ⚡ Overview

**EtherPulse** is a lightweight, ultra-reliable Windows desktop watchdog designed to pinpoint physical Ethernet cable slips, loose RJ45 connectors, local network switch drops, and gateway unreachability.

Unlike typical network monitor utilities that spawn invisible `cmd.exe` or `ping.exe` child processes, EtherPulse is engineered in **100% pure Rust** utilizing direct Win32 API calls (`iphlpapi.dll` and `shell32.dll`). It runs entirely inside the native Windows GUI subsystem (`#![windows_subsystem = "windows"]`) ensuring **absolute zero console / CMD flashing windows**, while rendering a responsive obsidian-dark glassmorphic dashboard inside a borderless native application window.

---

## 🌟 Key Features

### 1. Direct Win32 Native ICMP Ping Engine
- **Kernel-Level Probing**: Invokes `IcmpCreateFile`, `IcmpSendEcho`, and `IcmpCloseHandle` directly from `iphlpapi.dll`.
- **Sub-Millisecond Accuracy**: Captures exact round-trip time (RTT), latency jitter, and standard IP status codes (Success, Destination Host Unreachable, Request Timed Out, Hardware Error).
- **Dual-Target Isolation**:
  - **Gateway Target** (`192.168.1.1` or auto-detected router IP): Pinpoints local physical cable faults, switch reboots, or NIC resets.
  - **Internet Target** (`1.1.1.1` / `8.8.8.8`): Isolates local physical cable drops from upstream ISP WAN outages.
- **Configurable Frequencies**: 250 ms, 500 ms, 1 000 ms (default), 2 000 ms, or custom polling intervals.

### 2. Native Physical Adapter & Link State Detection
- Calls `GetAdaptersAddresses` from the native Windows IP Helper API (zero PowerShell, zero WMI, zero CMD).
- Discovers active network interface name (e.g. *Realtek PCIe 2.5GbE Family Controller*), operational status, IPv4 address, link speed, and default gateway.
- Detects physical media disconnects (`IfOperStatusDown` / `Media Disconnected`) in real time.

### 3. Outage Telemetry & Precision Duration Tracking
- **State Transition Machine**: Tracks transition between `ONLINE`, `DISCONNECTED`, and `DEGRADED`.
- **Exact Outage Duration**: Measures the exact duration of each drop down to tenths of a second (e.g. `Duration: 4.2s`).
- **Real-Time Telemetry Metrics**:
  - Current continuous uptime streak (`00h 14m 32s`).
  - Total disconnect event counter.
  - Cumulative downtime duration.
  - Rolling packet loss percentage.
  - Minimum, average, and maximum latency sparklines.

### 4. 100% Backwards-Compatible Logging & Exports
- **`lan_disconnects.txt` Compatibility**: Maintains 100% format compatibility with classic batch ping scripts:
  ```text
  Monitoring started at 14-11-2026 18:55:00
  DISCONNECT at 14-11-2026 18:55:10 (Duration: 4.2s)
  ```
- **One-Click Native Log Opener**: Launches `lan_disconnects.txt` via native Win32 `ShellExecuteW` directly into the default text editor (zero command prompt).
- **Multi-Format Export**: One-click download of structured CSV (`lan_disconnect_report.csv`) and JSON event logs.

### 5. High-Tech Single-Window UI
- **Obsidian Dark Aesthetic**: Deep modern palette (`#070a12`), glassmorphic panels, and neon glowing indicators.
- **Live Latency Waveform**: Real-time rolling SVG sparkline displaying ping response times with visual drop markers.
- **Synthesized Audio Chimes**: Web Audio API alert sound triggered on disconnect and reconnection (with instant mute/unmute).
- **Embedded Static Assets**: HTML, CSS, SVG, and JavaScript are bundled directly into the compiled executable with `include_str!`.

### 6. Absolute Zero CMD Windows & Clean Lifecycle
- **Windows Subsystem**: `#![windows_subsystem = "windows"]` eliminates the Windows console window entirely.
- **App-Mode Launcher**: Launches Microsoft Edge or Chrome in `--app=http://127.0.0.1:<port>` with an isolated user profile (`%LOCALAPPDATA%\EtherPulse\profile`) and `CREATE_NO_WINDOW = 0x08000000`.
- **Active Port Persistence**: Saves active server port to `%LOCALAPPDATA%\EtherPulse\port.txt` upon startup; secondary instances read this port to seamlessly focus the existing window, and the file is cleaned up upon graceful shutdown.
- **Single-Instance Mutex**: Prevents duplicate processes (`Global\EtherPulseSingleInstanceMutex`).
- **Heartbeat Watchdog**: Shuts down the background daemon cleanly when the desktop window is closed.
- **100% Rust Repository**: Excludes vendored web assets via `.gitattributes` so GitHub Linguist detects the repository as 100% Rust.

---

## 🏗️ System Architecture

```
+-----------------------------------------------------------------------------------+
|                                  EtherPulse.exe                                   |
|                                 (100% Pure Rust)                                  |
|                                                                                   |
|  +-----------------------------------------------------------------------------+  |
|  |                 Native Windows GUI Subsystem (Zero CMD Window)              |  |
|  |                 #![windows_subsystem = "windows"]                           |  |
|  +-----------------------------------------------------------------------------+  |
|                                                                                   |
|  +-----------------------------+         +-------------------------------------+  |
|  |    Native Win32 ICMP Engine |         |       Lightweight HTTP Server       |  |
|  | - IcmpSendEcho (iphlpapi)   | <-----> | - Port 3000 (auto-fallback 3001+)   |  |
|  | - GetAdaptersAddresses      |         | - REST API + Heartbeat Watchdog     |  |
|  | - ShellExecuteW (shell32)   |         | - Embedded HTML/CSS/JS Assets       |  |
|  +-----------------------------+         +-------------------------------------+  |
|                 |                                           |                     |
|                 v                                           v                     |
|      +---------------------+               +-----------------------------------+  |
|      | lan_disconnects.txt |               | Dedicated Borderless App Window   |  |
|      +---------------------+               | (CREATE_NO_WINDOW = 0x08000000)   |  |
|                                            +-----------------------------------+  |
+-----------------------------------------------------------------------------------+
```

---

## 🚀 Getting Started

### Prerequisites
- **Operating System**: Windows 10 or Windows 11 (64-bit recommended)
- **Runtime**: None! Standalone single executable.
- **Compiler (for building from source)**: Rust 1.70+ (`cargo`, `rustc`) with `x86_64-pc-windows-msvc` or `x86_64-pc-windows-gnu`.

### Running the Application
Double-click `EtherPulse.exe` (or `EtherPulse.lnk`).
The application window opens instantly without flashing any console prompt.

---

## 🛠️ Building from Source

```powershell
# Clone or navigate to the repository
cd "Lan Cable logger"

# Build optimized release binary
cargo build --release

# The compiled single-file binary will be located at:
# target\release\ether-pulse.exe
```

To create the standalone deployment binary:
```powershell
Copy-Item "target\release\ether-pulse.exe" "EtherPulse.exe" -Force
```

---

## 📡 REST API Reference

EtherPulse embeds a low-overhead local HTTP server (default port `3000` with automatic fallback to `3001`–`3020` if occupied):

| Endpoint | Method | Description |
| :--- | :---: | :--- |
| `/` | `GET` | Serves the embedded single-page responsive dashboard. |
| `/api/status` | `GET` | Returns connection state, latency stats, uptime, packet loss %, and adapter info. |
| `/api/events` | `GET` | Returns disconnect and reconnect event logs with exact durations. |
| `/api/config` | `POST` | Updates target gateway IP, internet IP, ping interval (ms), or audio chime toggle. |
| `/api/action/clear` | `POST` | Clears event log in memory and resets current session counters. |
| `/api/action/open-log` | `POST` | Opens `lan_disconnects.txt` using native Win32 `ShellExecuteW` (zero CMD). |
| `/api/export?format=csv` | `GET` | Downloads structured event history in CSV format. |
| `/api/export?format=json` | `GET` | Downloads structured event history in JSON format. |
| `/api/exit` | `POST` | Gracefully terminates the background daemon. |

---

## 📁 File Structure

```
.
├── .gitattributes           # GitHub Linguist vendoring (guarantees 100% Rust)
├── .gitignore               # Build artifacts ignore rules
├── Cargo.toml               # Rust package manifest (name: ether-pulse)
├── Cargo.lock               # Cargo dependency lockfile
├── LICENSE                  # MIT License
├── README.md                # Project documentation
├── product.md               # Product requirements and technical specification
├── lan_disconnects.txt      # Real-time event log
├── EtherPulse.lnk           # Windows shortcut launcher
├── EtherPulse.exe           # Standalone release executable
└── src
    ├── main.rs              # Entry point, subsystem declaration, port persistence
    ├── icmp.rs              # Win32 IcmpSendEcho and GetAdaptersAddresses bindings
    ├── server.rs            # Lightweight embedded HTTP REST server
    ├── state.rs             # Thread-safe telemetry state and duration math
    ├── window.rs            # App-mode window launcher, mutex, ShellExecuteW
    └── web
        └── index.html       # Embedded single-page obsidian UI, sparkline, Web Audio
```

---

## 📄 License

EtherPulse is open-source software licensed under the [MIT License](LICENSE).
