# formcraft

![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)
![Rust 1.90+](https://img.shields.io/badge/rust-1.90%2B-orange?logo=rust)
![MCP stdio](https://img.shields.io/badge/MCP-stdio-purple)
![ArtCraft harness](https://img.shields.io/badge/ArtCraft-harness-green)

Survey, feedback response, and NPS analytics studio as a pure-Rust local-first craft app.
Part of the back-office craft suite (plan: `mcp-central-docs/projects/craft-fleet-expansion/BACKOFFICE_CRAFT_SUITE.md`).

Replaces Typeform, Google Forms, and Qualtrics for small teams.

## Surfaces

- **L1 `formcraft-core` & `formcraft-store`**: Form JSON schema validation, SQLite WAL response persistence, DE/EN sentiment estimation, and detractor routing to `gaincraft`.
- **L2 `formcraft-mcp`**: Unified CLI (`formcraft define`, `formcraft nps`) and stdio JSON-RPC MCP server.
- **L3 `formcraft-app`**: Native `eframe`/`egui 0.37` desktop workstation.

## Quick Start

```powershell
cargo run -p formcraft-mcp -- status
cargo run -p formcraft-app -- --smoke
```

## Collector

`formcraft serve --port 14133` listens on 127.0.0.1. Port 14133 is the registered collector port. It is not a web app and it does not bind every interface.

```powershell
cargo run -p formcraft-mcp -- collect-token nps-q4 a-long-token-value
cargo run -p formcraft-mcp -- serve --port 14133
```

A page posts JSON to `http://127.0.0.1:14133/v1/forms/nps-q4/responses`:

```json
{"token": "a-long-token-value", "answers": {"comment": "gut"}, "nps_score": 9, "company_website": ""}
```

Leave `company_website` empty. A value in that field is thrown away. The stored address is the /24 (IPv4) or /64 (IPv6), not the full address. The reply does not contain the token. Routing a detractor to GainCraft stays a separate command.
