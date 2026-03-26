# Building OpenDeck on Windows

## Prerequisites

### 1. Microsoft C++ Build Tools

Install the [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/). During setup, select **Desktop development with C++**.

See the [Tauri prerequisites guide](https://tauri.app/start/prerequisites/) for full details.

### 2. WebView2

WebView2 is included with Windows 10 (1803+) and Windows 11. If you don't have it, install the Evergreen Bootstrapper from the [WebView2 download page](https://developer.microsoft.com/en-us/microsoft-edge/webview2/#download-section).

### 3. Rust

Install Rust via [rustup](https://rustup.rs/):

```powershell
winget install --id Rustlang.Rustup
```

After installing, ensure the MSVC toolchain is the default:

```powershell
rustup default stable-msvc
```

### 4. Deno

Install [Deno](https://deno.com/):

```powershell
irm https://deno.land/install.ps1 | iex
```

Restart your terminal after installing to pick up the new PATH entries.

## Building

Install frontend dependencies:

```
deno install
```

Run in development mode (debug build with hot-reload):

```
deno task tauri dev
```

Create a release build:

```
deno task tauri build
```

The release binary will be in `src-tauri/target/release/`.

## Pre-commit checks

Before committing, ensure:

1. `cargo clippy` -- no lint violations
2. `cargo fmt` -- Rust code formatted
3. `deno check` and `deno lint` -- TypeScript checked and linted
4. `deno task check` -- Svelte code linted
5. `deno fmt --unstable-component` -- frontend code formatted
