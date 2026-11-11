# FormCraft: Product Requirements Document

## 1. Executive Summary

`formcraft` is the survey management, structured response intake, and customer sentiment analytics engine of the Back-Office Craft Suite. It delivers:
- Local-first SQLite WAL storage for dynamic forms and submission payloads.
- Automated Net Promoter Score (NPS) breakdown across Promoters (9-10), Passives (7-8), and Detractors (0-6).
- Dual-language (German & English) sentiment lexicon scoring for qualitative responses.
- Automated churn risk mitigation by emitting `signal.negative` bus events directly to `gaincraft` for detractors.
- Native `egui 0.37` desktop workstation and stdio JSON-RPC MCP server.

## 2. Replaces & Cost Savings

- **Replaces:** Typeform (\$59/seat/mo), SurveyMonkey (\$99/seat/mo), Qualtrics CoreXM (\$150+/seat/mo).
- **Cost Reduction:** 100% cloud fee elimination; zero third-party telemetry, air-gapped GDPR compliance for EU enterprise surveys.

## 3. Architecture

- **`formcraft-store`:** SQLite WAL database schema with tables `forms` and `responses`.
- **`formcraft-core`:** Core dispatch, JSON schema validator, DE/EN sentiment lexicon analyzer, and bus router.
- **`formcraft-mcp`:** Single-binary CLI and stdio JSON-RPC MCP server.
- **`formcraft-app`:** `craft-biz-ui` / `egui 0.37` desktop application with forms manager, response viewer, and NPS analytics dock.

## 4. UI/UX Architecture & Menu Bar Standard

In accordance with [`CRAFT_MENU_BAR_STANDARD.md`](../../mcp-central-docs/standards/rust-artcraft/CRAFT_MENU_BAR_STANDARD.md):
- **Pattern 2 (Viewport-Centric Craft Workstation):** Persistent form builder / question list stage and live response analytics viewport with NPS gauges. Top quick button row acts as accelerators into the `View` menu. To add a new survey builder view or response chart:
  1. Register a visibility toggle in the `View` or `Window` menu.
  2. Add an entry in the sidebar Forms / Questions toolbox.
  3. Wire the action directly into the universal `Command::dispatch` bus.
- **Top Menu Bar Hierarchy:**
  - `File`: New Form (`Ctrl+N`), Open Form (`Ctrl+O`), Open Recent ▸, Save Form (`Ctrl+S`), Save As... (`Ctrl+Shift+S`), Export Responses / CSV (`Ctrl+E`), Close (`Ctrl+W`), Quit (`Ctrl+Q`).
  - `Edit`: Undo (`Ctrl+Z`), Redo (`Ctrl+Y`), Cut, Copy, Paste, Select All, Preferences (`Ctrl+,`).
  - `View`: Form Builder Canvas, Live Response Table, NPS Analytics Dashboard, Sentiment Breakdown, Theme (`Dark`/`Light`).
  - `Insert`: Add Question (Rating, Text, Multiple Choice), Insert Logic Branch, Import Response CSV.
  - `Modify`: Reorder Questions, Edit Validation Rules, Recalculate NPS / Sentiment.
  - `Commands`: Calculate NPS Score, Analyze Sentiment Lexicon, Emit Detractor Signal to GainCraft, Publish Form.
  - `Window`: Reset Layout, Form Builder Viewport, Responses Drawer, NPS Telemetry Dock.
  - `Help` (Mandatory 5-Dialog Suite): `Command Palette...` (`Ctrl+K`), `Keyboard Shortcuts...` (`F1`), `Project Files & Docs...`, `Licenses & Notices...`, `About FormCraft`.
- **Behavioral Mechanics:** Smooth `switch_on_hover` between top-level menus, zero-unsafe immediate-mode rendering, and headless `--smoke` parity.
