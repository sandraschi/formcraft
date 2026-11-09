# FormCraft — AGENTS.md

## Architecture

Pure-Rust ArtCraft 3-tier workspace:
- **L1 `crates/formcraft-core` & `crates/formcraft-store`**: SQLite WAL store, sentiment evaluation, NPS calculations, and bus integration.
- **L2 `crates/formcraft-mcp`**: Unified CLI + MCP 2024-11-05 server.
- **L3 `crates/formcraft-app`**: Native `eframe`/`egui 0.37` desktop workstation.

## Hard Rules

- `unwrap_used`, `expect_used`, `panic`, `unimplemented`, and `todo` are **denied** at workspace level.
- Commit with `-c user.name="Sandra Schipal (<tool>)"`.
