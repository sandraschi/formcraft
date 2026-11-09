# FormCraft MCP Tools Reference

FormCraft exposes 11 Model Context Protocol (MCP) tools over stdio JSON-RPC for complete agent automation.

| Tool | Parameters | Description |
|---|---|---|
| `form_status` | none | Reports database status, forms count, and fleet actor. |
| `form_define` | `id`, `title`, `description`, `schema` | Creates or updates a form definition and JSON schema. |
| `form_validate` | `schema` | Validates a form schema definition structure. |
| `form_list` | none | Lists all defined survey forms. |
| `form_get` | `id` | Fetches details and schema for a single form. |
| `form_responses_import`| `form_id`, `responses` | Imports response records with optional ratings and comments. |
| `form_responses_list` | `form_id` | Lists all responses captured for a specific form. |
| `form_nps` | `form_id` | Calculates NPS, Promoters, Passives, Detractors, and sentiment. |
| `form_report` | `form_id` | Comprehensive survey analytics rollup and distribution. |
| `form_route_detractors`| `form_id` | Emits `signal.negative` bus events for low ratings or sentiment to `gaincraft`. |
| `form_inbox_drain` | none | Acknowledges and clears pending bus signals in `formcraft`. |
