//! formcraft-core: Core logic, validation, sentiment estimation, and command dispatch.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use craft_biz::bus::Bus;
pub use craft_biz::confine::Confinement;
use craft_biz::mcp::tool;
use formcraft_store::Store;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const APP: &str = "formcraft";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Ctx {
    pub store: Store,
    pub confinement: Confinement,
    pub bus: Bus,
    pub actor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    Status,
    FormDefine {
        id: String,
        title: String,
        description: String,
        schema: Value,
    },
    FormValidate {
        schema: Value,
    },
    FormList,
    FormGet {
        id: String,
    },
    ResponsesImport {
        form_id: String,
        responses: Vec<NewResponse>,
    },
    ResponsesList {
        form_id: String,
    },
    FormNps {
        form_id: String,
    },
    FormReport {
        form_id: String,
    },
    RouteDetractors {
        form_id: String,
    },
    InboxDrain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewResponse {
    pub account_id: Option<String>,
    pub answers: Value,
    pub nps_score: Option<i64>,
}

fn s<'a>(a: &'a Value, k: &str) -> Result<&'a str, String> {
    a.get(k)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing string argument {k:?}"))
}

fn opt_s(a: &Value, k: &str) -> Option<String> {
    a.get(k)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|v| !v.trim().is_empty())
}

impl Command {
    pub fn from_tool(name: &str, a: &Value) -> Result<Command, String> {
        let name = craft_biz::mcp::normalize_tool_name(name);
        Ok(match name.as_str() {
            "form_status" => Command::Status,
            "form_define" => Command::FormDefine {
                id: s(a, "id")?.to_string(),
                title: s(a, "title")?.to_string(),
                description: opt_s(a, "description").unwrap_or_default(),
                schema: a.get("schema").cloned().unwrap_or(json!({})),
            },
            "form_validate" => Command::FormValidate {
                schema: a.get("schema").cloned().unwrap_or(json!({})),
            },
            "form_list" => Command::FormList,
            "form_get" => Command::FormGet {
                id: s(a, "id")?.to_string(),
            },
            "form_responses_import" => {
                let form_id = s(a, "form_id")?.to_string();
                let arr = a
                    .get("responses")
                    .and_then(Value::as_array)
                    .ok_or("responses must be an array")?;
                let mut resp_list = Vec::new();
                for r in arr {
                    let acc = opt_s(r, "account_id");
                    let answers = r.get("answers").cloned().unwrap_or(json!({}));
                    let nps_score = r.get("nps_score").and_then(Value::as_i64);
                    resp_list.push(NewResponse {
                        account_id: acc,
                        answers,
                        nps_score,
                    });
                }
                Command::ResponsesImport {
                    form_id,
                    responses: resp_list,
                }
            }
            "form_responses_list" => Command::ResponsesList {
                form_id: s(a, "form_id")?.to_string(),
            },
            "form_nps" => Command::FormNps {
                form_id: s(a, "form_id")?.to_string(),
            },
            "form_report" => Command::FormReport {
                form_id: s(a, "form_id")?.to_string(),
            },
            "form_route_detractors" => Command::RouteDetractors {
                form_id: s(a, "form_id")?.to_string(),
            },
            "form_inbox_drain" => Command::InboxDrain,
            other => return Err(format!("unknown formcraft tool: {other}")),
        })
    }
}

/// Simple DE + EN sentiment estimator for feedback comments.
pub fn estimate_sentiment(text: &str) -> f64 {
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();
    if words.is_empty() {
        return 0.0;
    }

    let positive_words = [
        "great",
        "good",
        "excellent",
        "super",
        "toll",
        "perfekt",
        "gut",
        "zufrieden",
        "love",
        "amazing",
        "fast",
        "schnell",
        "hilfreich",
        "helpful",
        "freundlich",
        "best",
        "beste",
        "danke",
        "thanks",
        "wunderbar",
        "großartig",
    ];
    let negative_words = [
        "bad",
        "terrible",
        "schlecht",
        "langsam",
        "slow",
        "broken",
        "kaputt",
        "fehler",
        "error",
        "bug",
        "furchtbar",
        "schrecklich",
        "enttäuscht",
        "disappointed",
        "poor",
        "unbrauchbar",
        "useless",
        "miserabel",
        "unfreundlich",
        "hate",
    ];

    let mut score = 0.0_f64;
    let mut is_negated = false;

    for w in words {
        let clean = w.trim_matches(|c: char| !c.is_alphabetic());
        if clean == "nicht"
            || clean == "not"
            || clean == "kein"
            || clean == "keine"
            || clean == "never"
        {
            is_negated = true;
            continue;
        }

        if positive_words.contains(&clean) {
            if is_negated {
                score -= 0.6;
            } else {
                score += 0.8;
            }
            is_negated = false;
        } else if negative_words.contains(&clean) {
            if is_negated {
                score += 0.5;
            } else {
                score -= 0.8;
            }
            is_negated = false;
        }
    }

    score.clamp(-1.0, 1.0)
}

pub fn dispatch(cmd: &Command, ctx: &mut Ctx) -> Result<Value, String> {
    match cmd {
        Command::Status => {
            let forms = ctx.store.list_forms().map_err(|e| e.to_string())?;
            Ok(json!({
                "app": APP,
                "version": VERSION,
                "forms_count": forms.len(),
                "actor": ctx.actor,
                "status": "ok"
            }))
        }
        Command::FormDefine {
            id,
            title,
            description,
            schema,
        } => {
            let f = ctx
                .store
                .save_form(id, title, description, schema)
                .map_err(|e| e.to_string())?;
            Ok(json!({ "saved": true, "form": f }))
        }
        Command::FormValidate { schema } => {
            if !schema.is_object() {
                return Err("Form schema must be a JSON object".to_string());
            }
            Ok(
                json!({ "valid": true, "fields_count": schema.as_object().map(|m| m.len()).unwrap_or(0) }),
            )
        }
        Command::FormList => {
            let list = ctx.store.list_forms().map_err(|e| e.to_string())?;
            Ok(json!({ "forms": list, "count": list.len() }))
        }
        Command::FormGet { id } => {
            let f = ctx.store.get_form(id).map_err(|e| e.to_string())?;
            Ok(json!({ "form": f }))
        }
        Command::ResponsesImport { form_id, responses } => {
            let mut imported_ids = Vec::new();
            for r in responses {
                let comment = r.answers.to_string();
                let sentiment = estimate_sentiment(&comment);
                let id = ctx
                    .store
                    .add_response(
                        form_id,
                        r.account_id.as_deref(),
                        &r.answers,
                        r.nps_score,
                        sentiment,
                    )
                    .map_err(|e| e.to_string())?;
                imported_ids.push(id);
            }
            Ok(json!({ "imported_count": imported_ids.len(), "ids": imported_ids }))
        }
        Command::ResponsesList { form_id } => {
            let list = ctx
                .store
                .list_responses(form_id)
                .map_err(|e| e.to_string())?;
            Ok(json!({ "form_id": form_id, "responses": list, "count": list.len() }))
        }
        Command::FormNps { form_id } => {
            let r = ctx.store.nps_report(form_id).map_err(|e| e.to_string())?;
            Ok(json!({ "nps_report": r }))
        }
        Command::FormReport { form_id } => {
            let f = ctx.store.get_form(form_id).map_err(|e| e.to_string())?;
            let r = ctx.store.nps_report(form_id).map_err(|e| e.to_string())?;
            let responses = ctx
                .store
                .list_responses(form_id)
                .map_err(|e| e.to_string())?;
            Ok(json!({
                "form": f,
                "nps": r,
                "total_responses": responses.len()
            }))
        }
        Command::RouteDetractors { form_id } => {
            let responses = ctx
                .store
                .list_responses(form_id)
                .map_err(|e| e.to_string())?;
            let mut routed_count = 0usize;
            for r in responses {
                if let Some(score) = r.nps_score {
                    if score <= 6 || r.sentiment <= -0.4 {
                        if let Some(acc) = r.account_id {
                            // Emit signal.negative to gaincraft inbox
                            let payload = json!({
                                "account": acc,
                                "account_id": acc,
                                "source": "formcraft",
                                "form_id": form_id,
                                "score": 3,
                                "nps_score": score,
                                "sentiment": r.sentiment,
                                "response_id": r.id
                            });
                            let _ =
                                ctx.bus
                                    .emit("formcraft", "gaincraft", "signal.negative", payload);
                            routed_count += 1;
                        }
                    }
                }
            }
            Ok(json!({ "routed_detractors": routed_count }))
        }
        Command::InboxDrain => {
            let pending = ctx.bus.pending("formcraft").map_err(|e| e.to_string())?;
            let mut applied = 0usize;
            for evt in &pending {
                ctx.bus
                    .ack("formcraft", &evt.id)
                    .map_err(|e| e.to_string())?;
                applied += 1;
            }
            Ok(json!({ "applied": applied, "rejected": 0 }))
        }
    }
}

pub fn tools() -> Vec<Value> {
    vec![
        tool(
            "form_status",
            "Report formcraft status, forms count and store health",
            json!({}),
            &[],
        ),
        tool(
            "form_define",
            "Create or update a form definition",
            json!({
                "id": { "type": "string" },
                "title": { "type": "string" },
                "description": { "type": "string" },
                "schema": { "type": "object" }
            }),
            &["id", "title", "schema"],
        ),
        tool(
            "form_validate",
            "Validate a form JSON schema",
            json!({
                "schema": { "type": "object" }
            }),
            &["schema"],
        ),
        tool("form_list", "List all defined forms", json!({}), &[]),
        tool(
            "form_get",
            "Get a single form definition by ID",
            json!({
                "id": { "type": "string" }
            }),
            &["id"],
        ),
        tool(
            "form_responses_import",
            "Import responses for a form",
            json!({
                "form_id": { "type": "string" },
                "responses": { "type": "array" }
            }),
            &["form_id", "responses"],
        ),
        tool(
            "form_responses_list",
            "List all responses submitted for a form",
            json!({
                "form_id": { "type": "string" }
            }),
            &["form_id"],
        ),
        tool(
            "form_nps",
            "Calculate Net Promoter Score (NPS) and sentiment analysis",
            json!({
                "form_id": { "type": "string" }
            }),
            &["form_id"],
        ),
        tool(
            "form_report",
            "Comprehensive form response rollup and sentiment report",
            json!({
                "form_id": { "type": "string" }
            }),
            &["form_id"],
        ),
        tool(
            "form_route_detractors",
            "Emit negative sentiment and detractor signals to gaincraft",
            json!({
                "form_id": { "type": "string" }
            }),
            &["form_id"],
        ),
        tool(
            "form_inbox_drain",
            "Process pending incoming signals in the formcraft inbox",
            json!({}),
            &[],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn sentiment_analysis_de_and_en() {
        assert!(estimate_sentiment("Das Produkt ist super und schnell") > 0.5);
        assert!(estimate_sentiment("Sehr schlecht und kaputt") < -0.5);
        assert!(estimate_sentiment("Nicht schlecht, ganz gut") > 0.0);
        assert!(estimate_sentiment("Not bad, very helpful") > 0.0);
    }

    #[test]
    fn dispatch_form_lifecycle_and_detractor_routing() {
        let store = Store::in_memory().unwrap();
        let mut ctx = Ctx {
            store,
            confinement: Confinement(None),
            bus: Bus::new(PathBuf::from("target/test_bus")),
            actor: "test".to_string(),
        };

        let def_cmd = Command::FormDefine {
            id: "nps-q4".to_string(),
            title: "Q4 Customer Survey".to_string(),
            description: "Quarterly NPS".to_string(),
            schema: json!({"rating": "integer"}),
        };
        let res = dispatch(&def_cmd, &mut ctx).unwrap();
        assert!(res.get("saved").unwrap().as_bool().unwrap());

        let import_cmd = Command::ResponsesImport {
            form_id: "nps-q4".to_string(),
            responses: vec![
                NewResponse {
                    account_id: Some("ACC-1".to_string()),
                    answers: json!({"comment": "Love it"}),
                    nps_score: Some(10),
                },
                NewResponse {
                    account_id: Some("ACC-2".to_string()),
                    answers: json!({"comment": "Broken and bad"}),
                    nps_score: Some(3),
                },
            ],
        };
        let imp_res = dispatch(&import_cmd, &mut ctx).unwrap();
        assert_eq!(imp_res.get("imported_count").unwrap().as_u64().unwrap(), 2);

        let nps_res = dispatch(
            &Command::FormNps {
                form_id: "nps-q4".to_string(),
            },
            &mut ctx,
        )
        .unwrap();
        let nps_val = nps_res
            .pointer("/nps_report/nps")
            .unwrap()
            .as_f64()
            .unwrap();
        assert_eq!(nps_val, 0.0); // 1 promoter, 1 detractor -> 0 NPS

        let route_res = dispatch(
            &Command::RouteDetractors {
                form_id: "nps-q4".to_string(),
            },
            &mut ctx,
        )
        .unwrap();
        assert_eq!(
            route_res
                .get("routed_detractors")
                .unwrap()
                .as_u64()
                .unwrap(),
            1
        );
    }
}
