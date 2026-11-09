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
