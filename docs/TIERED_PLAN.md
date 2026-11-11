# FormCraft — Tiered Execution Plan

**Parent Standard:** [`RUST_ARTCRAFT_STANDARD.md`](../../mcp-central-docs/standards/RUST_ARTCRAFT_STANDARD.md)  
**Menu Bar Standard:** [`CRAFT_MENU_BAR_STANDARD.md`](../../mcp-central-docs/standards/rust-artcraft/CRAFT_MENU_BAR_STANDARD.md)  
**Architecture Model:** 4-Tier ArtCraft Incremental Delivery  
**Status:** Active  

---

## Architecture Flow

```
Tier 1: Core Engine ──> Tier 2: MCP / CLI ──> Tier 3: Desktop UI ──> Tier 4: Menu Bar, Help & Packaging
(formcraft-core)       (formcraft-mcp)       (formcraft-app)       (Hardened Workstation)
```

---

## Tier Specifications

### Tier 1 — Core Survey & Sentiment Engine (`formcraft-core`, `formcraft-store`)
- [x] L1 `Command::dispatch` universal survey execution bus.
- [x] SQLite WAL database schema (`forms`, `responses`).
- [x] JSON schema validator for question types (ratings, text, choices).
- [x] Net Promoter Score (NPS) computation engine (Promoters, Passives, Detractors).
- [x] DE/EN sentiment lexicon analyzer for open-text responses.
- [x] Cross-craft bus emitter (`signal.negative` -> GainCraft).
- [x] `#![forbid(unsafe_code)]` and zero-panic clippy lints.

### Tier 2 — Headless Driver & FastMCP (`formcraft-mcp`)
- [x] Unified CLI `formcraft-cli` with `--json` output.
- [x] FastMCP stdio server communicating over JSON-RPC 2.0.
- [x] Response export and sentiment analysis inspection.
- [x] Headless `--smoke` verification without window initialization.

### Tier 3 — Desktop Workstation UI (`formcraft-app`)
- [x] Immediate-mode `egui` 0.37 / `wgpu` workstation.
- [x] Drag-and-drop form question builder and live preview.
- [x] Response table and real-time NPS / sentiment telemetry gauges.

### Tier 4 — Craft Menu Bar Standard, Help Suite & Hardened Verification
- [ ] Universal 8-menu bar (`File`, `Edit`, `View`, `Insert`, `Modify`, `Commands`, `Window`, `Help`) with `switch_on_hover`.
- [ ] Mandatory 5-dialog Help suite (`Command Palette...`, `Keyboard Shortcuts...`, `Project Files & Docs...`, `Licenses & Notices...`, `About FormCraft`).
- [ ] Fleet UI/UX Pattern 2 adherence: viewport-centric navigation, top button row mapped to View toggles, panel visibility managed via `View` / `Window`.
- [ ] Tofu Guard verification (100% font glyph coverage, zero tofu placeholder boxes).
- [ ] Headless `--smoke` parity validation.
- [ ] Standalone release binary packaging and NSIS installer.
