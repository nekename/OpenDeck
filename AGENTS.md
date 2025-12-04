# OpenDeck Development Guide

## Architecture Overview

OpenDeck is a Tauri desktop application for controlling Elgato Stream Deck devices. It's built with:
- **Backend**: Rust (Tauri v2) - device communication, plugin management, WebSocket/HTTP servers
- **Frontend**: SvelteKit + TypeScript + Tailwind CSS v4 - UI rendered in webview
- **Build Tool**: Deno (not Node.js) - manages tasks and dependencies

### Core Architecture Pattern

OpenDeck acts as a **host application** that communicates with **plugins** (separate processes):
1. Plugins connect via WebSocket (port dynamically allocated, starting from 57116)
2. Static assets served via `tiny_http` webserver (port = WebSocket port + 2)
3. Plugin property inspectors (HTML/JS) run in iframes and use separate WebSocket connections
4. Device button presses/releases trigger events sent to plugins via WebSocket

Key data flow: `Device (elgato-streamdeck crate) → Rust event handlers → WebSocket → Plugin process`

## Project Structure

```
src-tauri/src/              # Rust backend
├── main.rs                 # Entry point, Tauri setup, tray icon
├── elgato.rs               # Direct hardware communication (elgato-streamdeck crate)
├── plugins/                # Plugin lifecycle, WebSocket/HTTP servers
├── events/                 # Event routing (inbound/outbound/frontend)
├── store/                  # JSON file-based persistence (profiles, settings)
└── application_watcher.rs  # Auto-switch profiles based on active window

src/                        # SvelteKit frontend
├── lib/                    # TypeScript types mirroring Rust structs
├── components/             # Svelte UI components
└── routes/                 # SvelteKit routing (currently single-page app)

plugins/com.amansprojects.starterpack.sdPlugin/  # Plugin with basic actions
├── assets/manifest.json                         # Plugin metadata
├── assets/propertyInspector/                    # HTML UIs for action settings
└── src/                                         # Rust plugin using openaction crate
```

## Critical Workflows

### Development Commands

```bash
# Frontend dev server (Vite HMR on port 5173)
deno task dev

# Run Tauri app in dev mode (spawns frontend + Rust app)
deno task tauri dev

# Build production bundle
deno task tauri build
```

### Pre-commit Requirements

Before commits, **always** run:
1. `cargo clippy` (no violations allowed)
2. `cargo fmt` (in both `src-tauri/` and plugin directories)
3. `deno check`, `deno task check` and `deno lint` (no violations)

These are project standards, not suggestions.

### Built-in Plugins

Built-in plugins included in OpenDeck are Rust binaries. The `build.ts` script in each plugin compiles for multiple targets (x86_64/aarch64) and organizes binaries by OS.

## Key Conventions

### Type Synchronization

TypeScript types in `src/lib/` **must** mirror Rust structs in `src-tauri/src/shared.rs`:
- `Context`, `ActionInstance`, `ActionState`, `DeviceInfo`, `Profile`
- Changes to Rust structs require updating corresponding TypeScript types

### Context System

A `Context` identifies a button/encoder position:
```rust
struct Context {
    device: String,    // Device vendor prefix and serial number
    profile: String,   // Profile name
    controller: String, // "Keypad" or "Encoder"
    position: u8,      // Key index or encoder number
}
```

An `ActionContext` extends this with an action instance index for nested actions (e.g., multi-actions).

### State Management

- **Backend**:
  - `DEVICES` (DashMap): Thread-safe device registry, keyed by device ID
  - `CATEGORIES` (RwLock): Plugin actions organized by category for UI
  - `Store<T>`: Generic JSON persistence with file locking, backup, and atomic writes
  - Profile locks: Use `acquire_locks()` (read) or `acquire_locks_mut()` (write) before accessing profiles
- **Frontend**:
  - Svelte stores (`propertyInspector.ts`): `inspectedInstance`, `copiedContext`, `openContextMenu` for UI state
  - Tauri `invoke()` for backend calls - returns Promises with typed results
- **Persistence**: JSON files in config dir (see `store/mod.rs`), with `.temp` and `.bak` for crash recovery

### Plugin Communication

- **WebSocket protocol**: Plugins/PIs connect to `localhost:PORT_BASE`, send JSON messages with `event` field
- **Message routing**: `inbound::InboundEventType` enum handles all incoming events, `outbound::` modules send to plugins
- **Outbound event types**: `willAppear`, `keyDown`, `keyUp`, `dialRotate`, etc. (Stream Deck SDK compatible)
- **Authentication**: Context validation ensures plugins can only access their own action instances
- **Plugin manifests** (`manifest.json`: Stream Deck SDK format + extensions):
  - `CodePathLin`: Linux binary path
  - `CodePaths`: Map of Rust target triples to binaries
  - Platform overrides: `manifest.{os}.json` files merged via `json-patch`
- **Property inspectors**: Communicate with plugins via `sendToPlugin`/`sendToPropertyInspector`

### Cross-Platform Considerations

- **Wine support**: Plugins compiled for Windows can run on Linux/macOS via Wine (spawned as child processes)
- **Device access**: Linux requires udev rules (`40-streamdeck.rules`), installed automatically with .deb/.rpm
- **Flatpak**: Special handling for paths (`is_flatpak()` checks), Wine must be installed natively

## Common Patterns

### Adding a New Tauri Command

1. Define handler in `src-tauri/src/events/frontend.rs`
2. Add to `invoke_handler![]` macro in `main.rs`
3. Call from frontend: `await invoke<ReturnType>("command_name", { arg })`

### Profile Management

Profiles are device-specific JSON files in `<config_dir>/<device_id>/<profile_name>.json`:
```rust
// Read profile
let locks = crate::store::profiles::acquire_locks().await;
let profile = locks.profile_stores.get_profile_store(&device, "Default")?;

// Modify profile
let mut locks = crate::store::profiles::acquire_locks_mut().await;
let slot = crate::store::profiles::get_slot_mut(&context, &mut locks).await?;
*slot = Some(new_instance);
crate::store::profiles::save_profile(&device.id, &mut locks).await?;
```

Auto-switching: `application_watcher.rs` polls active window every 250ms, triggers profile changes via `SwitchProfileEvent` emitted to frontend.

### Event Flow Examples

**Button press**: `elgato.rs` → `outbound::keypad::key_down()` → WebSocket → Plugin's `key_down` handler
**Set image**: Plugin sends `setImage` → `inbound::states::set_image()` → `elgato::update_image()` → Device hardware
**Property inspector**: User edits in iframe → `sendToPlugin` → Plugin updates → `setSettings` → Profile saved

## Integration Points

### External Dependencies

- `elgato-streamdeck`: Async hardware communication via HID, image format conversion for different device types
- `tauri-plugin-*`: Dialog (file picker), logging (to file), autostart, single-instance, deep-link (opendeck:// URLs)
- `tokio-tungstenite`: WebSocket server for plugin communication
- `tiny_http`: Static file server for plugin assets (icons, property inspectors)
- `image`: Image loading/manipulation, format conversion for device displays
- `enigo`: Keyboard/mouse input simulation (starter pack plugin)
- `active-win-pos-rs`: Detect focused application for profile switching (polls every 250ms)
- `sysinfo`: Process monitoring for ApplicationsToMonitor feature

### WebSocket Protocol

Port allocation: `PORT_BASE` (WebSocket), `PORT_BASE + 2` (HTTP static files)
- Dynamic port selection: Tries ports starting at 57116 until both WebSocket and HTTP ports are available
- Registration: Plugins send `RegisterEvent::RegisterPlugin { uuid }`, property inspectors send `RegisterPropertyInspector`
- Message queuing: `PLUGIN_QUEUES` buffers messages until plugin connects
- Separate socket collections: `PLUGIN_SOCKETS` and `PROPERTY_INSPECTOR_SOCKETS` (HashMap of uuid → WebSocket sink)
- Plugin lifecycle: Socket registered → messages processed → socket removed on disconnect

### File Locations

```
Config: ~/.config/opendeck/ (Linux) / ~/Library/Application Support/opendeck/ (macOS)
Logs:   ~/.local/share/opendeck/logs/ (Linux) / ~/Library/Logs/opendeck/ (macOS)
Plugins: <config_dir>/plugins/
```

Flatpak uses different paths with `~/.var/app/me.amankhanna.opendeck/` prefix.

## Testing & Debugging

- Run from terminal to see live logs: `deno task tauri dev`
- Plugin logs: Check `<log_dir>/plugins/<uuid>.log` (stdout/stderr captured from plugin processes)
- Debug logging: Uses Rust `log` crate (`log::debug!`)
- Frontend: Tauri devtools accessible via right-click → "Inspect Element"
