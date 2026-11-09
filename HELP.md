# FormCraft Quick Reference

`formcraft` is a local-first survey engine, response store, and Net Promoter Score (NPS) analytics studio with bi-directional bus signal routing to GainCraft.

## Quick Start (CLI)

```bash
# Check status
formcraft status

# Define a survey
formcraft define onboarding "Customer Onboarding Survey" "First 30 days experience" '{"rating": "integer", "feedback": "string"}'

# List defined forms
formcraft list

# Get form details
formcraft get onboarding

# Calculate NPS and sentiment metrics
formcraft nps onboarding

# Comprehensive rollup report
formcraft report onboarding

# Route detractors (score <= 6 or negative sentiment) to GainCraft inbox
formcraft route-detractors onboarding

# Drain incoming bus signals
formcraft inbox-drain
```

## MCP Integration

Run as an MCP stdio server:
```bash
formcraft mcp
```

## Native Workstation

Run the native egui desktop app:
```bash
cargo run -p formcraft-app
```
Or with `--smoke` test:
```bash
cargo run -p formcraft-app -- --smoke
```
