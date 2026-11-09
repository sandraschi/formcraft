# formcraft

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
