# EtherPulse - LAN Cable & Network Telemetry Desktop Application

## Overview
A modern, lightweight, 100% pure Rust single-window Windows desktop application for high-frequency LAN cable integrity monitoring, gateway ping telemetry, and instant disconnect logging. Engineered as a standalone single-file executable (`EtherPulse.exe`) with zero external runtime dependencies and **absolute zero command prompt / console windows**, providing real-time visibility into local Ethernet cable drops, gateway unreachability, and WAN outages.

---

## Core Features

### 1. 100% Pure Rust Win32 ICMP Ping Engine
- **Direct Native ICMP**: Utilizes native Windows `iphlpapi.dll` (`IcmpCreateFile`, `IcmpSendEcho`, `IcmpCloseHandle`) for sub-millisecond network probing without spawning cmd or `ping.exe` subprocesses.
- **Microsecond Precision**: Measures exact round-trip time (RTT), packet loss, and status codes (Success, Destination Host Unreachable, Request Timed Out, Hardware Error).
- **Dual-Target Isolation (LAN vs WAN)**:
  - **Gateway Target** (`192.168.1.1` / Auto-detected): Probes local router/switch to detect physical LAN cable slips, NIC disconnects, or switch reboots.
  - **Internet Target** (`1.1.1.1` / `8.8.8.8`): Probes public DNS to isolate whether an outage is a local cable drop or an external ISP blackout.
- **Configurable Intervals**: 250ms, 500ms, 1s (default), 2s, or custom polling frequencies.

### 2. Pure Rust Physical Adapter & Link State Detection
- Auto-detects local active network adapters directly via native Win32 `GetAdaptersAddresses` API (Zero PowerShell / Zero CMD).
- Inspects Ethernet interface status, interface name (e.g. *Realtek PCIe 2.5GbE Family Controller*), local IP (`192.168.1.15`), link speed, and Default Gateway (`192.168.1.1`).
- Identifies physical cable unplugs (`Media Disconnected` / `IfOperStatusDown`) versus router unreachability.

### 3. Outage Telemetry & Outage Duration Tracking
- **State Transition Tracking**: Accurately tracks when a connection drops and when it restores.
- **Exact Outage Duration**: Computes precise duration for every drop event (e.g., `Dropped at 18:55:10 - Restored at 18:55:14 [Outage: 4.2s]`).
- **Telemetry Counters**:
  - Live Connection State (`ONLINE`, `DISCONNECTED`, `DEGRADED`).
  - Total Disconnect Count.
  - Cumulative Downtime Duration.
  - Current Continuous Uptime Streak (`00h 00m 00s`).
  - Packet Loss Percentage over moving rolling window.
  - Average, Min, and Max Latency (ms).

### 4. Seamless Log Compatibility & Multi-Format Export
- **`lan_disconnects.txt` Compatibility**: Automatically appends disconnects to `lan_disconnects.txt` in real time, maintaining 100% backwards compatibility with the original batch script format:
  ```text
  Monitoring started at DD-MM-YYYY HH:MM:SS
  DISCONNECT at DD-MM-YYYY HH:MM:SS (Duration: 3.8s)
  ```
- **Export Capabilities**:
  - One-click CSV Export (`lan_disconnect_report.csv`).
  - One-click JSON Export with structured timestamps and durations.
  - One-click "Open Log File" button (via native Win32 `ShellExecuteW`, opening Notepad with zero CMD window).

### 5. High-Tech Single-Window UI
- **Obsidian Dark Aesthetic**: Deep modern palette (`#0a0e17`), glassmorphic panels, dynamic neon glow indicators (Emerald Green `#10b981`, Amber `#f59e0b`, Crimson `#ef4444`, Cyan `#06b6d4`).
- **Live Latency Waveform**: Real-time rolling SVG sparkline / chart displaying continuous ping response times with highlighted drop markers.
- **Instant Audio Chimes**: Synthesized Web Audio API alert sound on cable disconnect and recovery (with instant Mute / Unmute toggle).
- **Dedicated Desktop Window**: Borderless Edge/Chrome app-mode window launched with `CREATE_NO_WINDOW`, zero browser chrome, zero console prompt.

---

## Architecture & Single-File Execution

```
+-------------------------------------------------------------------------+
|                             EtherPulse.exe                              |
|                            (100% Pure Rust)                             |
|                                                                         |
|  +-------------------------------------------------------------------+  |
|  |           Native Windows GUI Subsystem (Zero CMD Window)          |  |
|  |           #![windows_subsystem = "windows"]                       |  |
|  +-------------------------------------------------------------------+  |
|                                                                         |
|  +---------------------------+       +-------------------------------+  |
|  |  Native Win32 ICMP Engine |       |    Lightweight HTTP Server    |  |
|  | - IcmpSendEcho (iphlpapi) | <---> | - Port 3000 (auto-fallback)   |  |
|  | - GetAdaptersAddresses    |       | - REST API + Watchdog         |  |
|  | - ShellExecuteW (shell32) |       | - Embedded HTML/CSS/JS Assets |  |
|  +---------------------------+       +-------------------------------+  |
|                |                                    |                   |
|                v                                    v                   |
|     +----------------------+       +---------------------------------+  |
|     | lan_disconnects.txt  |       | Dedicated Single App Window     |  |
|     +----------------------+       | (CREATE_NO_WINDOW Flag)         |  |
|                                    +---------------------------------+  |
+-------------------------------------------------------------------------+
```

### Key Technical Attributes
1. **100% Pure Rust Architecture**: Backend ICMP ping engine, network adapter scanner, and embedded frontend UI are 100% Rust compiled into a single standalone Windows executable (`EtherPulse.exe`). All web assets are vendored and excluded via `.gitattributes` (`src/web/** linguist-vendored=true`, `*.html linguist-detectable=false`) guaranteeing 100% Rust language detection on GitHub.
2. **Absolute Zero Console Window**:
   - Compiles with `#![windows_subsystem = "windows"]` to ensure the OS never allocates a console for the executable.
   - Spawns background desktop window with `CREATE_NO_WINDOW = 0x08000000`.
   - Uses native Win32 `ShellExecuteW` for opening log files or URLs, completely eliminating `cmd.exe` or `powershell.exe`.
3. **Application Mode Launcher**: Automatically launches Microsoft Edge or Chrome in `--app=http://127.0.0.1:<port>` with an isolated profile (`%LOCALAPPDATA%\EtherPulse\profile`), giving a native frameless single-window application feel.
4. **Active Port Persistence**:
   - The active HTTP server port is persisted to `%LOCALAPPDATA%\EtherPulse\port.txt` immediately upon server start.
   - Secondary instance attempts detect `Global\EtherPulseSingleInstanceMutex`, read the active port from `port.txt`, and seamlessly focus/reopen the existing dashboard.
   - The port file is cleanly deleted during graceful application shutdown.
5. **Graceful Auto-Shutdown**: Watchdog process monitors the desktop window; when the user closes the window or clicks Exit, the background server cleanly flushes the log file, removes the active port file, and terminates.
6. **Single-Instance Mutex**: Running `EtherPulse.exe` while an instance is already active focuses the existing window rather than spawning duplicate instances.

---

## REST & Streaming API Endpoints

- `GET /` : Serves the embedded single-page responsive application.
- `GET /api/status` : Returns current connection health, latency, uptime streak, packet loss %, adapter info, and target configurations.
- `GET /api/events` : Returns the history of disconnect and reconnection events.
- `POST /api/config` : Updates ping interval (ms), gateway target IP, internet target IP, or sound alert preferences.
- `POST /api/action/clear` : Clears the in-memory event history and resets session counters.
- `POST /api/action/open-log` : Opens `lan_disconnects.txt` via native Win32 `ShellExecuteW` without opening any command prompt.
- `GET /api/export?format=csv|json` : Downloads structured telemetry logs.
- `POST /api/exit` : Gracefully shuts down the background process.

---

## Usage
- **One-Click Run**: Double-click `EtherPulse.exe`. Zero CMD prompt. Single window.
