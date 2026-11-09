//! formcraft-store: SQLite storage layer for form definitions, answers, and NPS metrics.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FormError {
    #[error("sqlite error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("json serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("form not found: {0}")]
    NotFound(String),
    #[error("invalid argument: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormDef {
    pub id: String,
    pub title: String,
    pub description: String,
    pub schema: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseRecord {
    pub id: i64,
    pub form_id: String,
    pub account_id: Option<String>,
    pub answers: Value,
    pub nps_score: Option<i64>,
    pub sentiment: f64,
    pub submitted_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpsReport {
    pub form_id: String,
    pub total_responses: usize,
    pub promoters: usize,
    pub passives: usize,
    pub detractors: usize,
    pub nps: f64,
    pub avg_sentiment: f64,
}

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self, FormError> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let s = Self { conn };
        s.init_schema()?;
        Ok(s)
    }

    pub fn in_memory() -> Result<Self, FormError> {
        let conn = Connection::open_in_memory()?;
        let s = Self { conn };
        s.init_schema()?;
        Ok(s)
    }

    fn init_schema(&self) -> Result<(), FormError> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS forms (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                schema_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS responses (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                form_id TEXT NOT NULL REFERENCES forms(id) ON DELETE CASCADE,
                account_id TEXT,
                answers_json TEXT NOT NULL,
                nps_score INTEGER,
                sentiment REAL NOT NULL DEFAULT 0.0,
                submitted_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_responses_form ON responses(form_id);
            CREATE INDEX IF NOT EXISTS idx_responses_account ON responses(account_id);
            "#,
        )?;
        Ok(())
    }

    pub fn save_form(
        &mut self,
        id: &str,
        title: &str,
        desc: &str,
        schema: &Value,
    ) -> Result<FormDef, FormError> {
        let now = Utc::now().to_rfc3339();
        let schema_str = serde_json::to_string(schema)?;
        self.conn.execute(
            r#"
            INSERT INTO forms (id, title, description, schema_json, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?5)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                description = excluded.description,
                schema_json = excluded.schema_json,
                updated_at = excluded.updated_at
            "#,
            params![id, title, desc, schema_str, now],
        )?;
        Ok(FormDef {
            id: id.to_string(),
            title: title.to_string(),
            description: desc.to_string(),
            schema: schema.clone(),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn get_form(&self, id: &str) -> Result<FormDef, FormError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, description, schema_json, created_at, updated_at FROM forms WHERE id = ?1",
        )?;
        let row = stmt
            .query_row(params![id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                ))
            })
            .optional()?;
        let (id, title, desc, schema_str, created_at, updated_at) =
            row.ok_or_else(|| FormError::NotFound(id.to_string()))?;
        let schema = serde_json::from_str(&schema_str)?;
        Ok(FormDef {
            id,
            title,
            description: desc,
            schema,
            created_at,
            updated_at,
        })
    }

    pub fn list_forms(&self) -> Result<Vec<FormDef>, FormError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, description, schema_json, created_at, updated_at FROM forms ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
            ))
        })?;
        let mut out = Vec::new();
        for res in rows {
            let (id, title, desc, schema_str, created_at, updated_at) = res?;
            let schema = serde_json::from_str(&schema_str).unwrap_or(Value::Null);
            out.push(FormDef {
                id,
                title,
                description: desc,
                schema,
                created_at,
                updated_at,
            });
        }
        Ok(out)
    }

    pub fn add_response(
        &mut self,
        form_id: &str,
        account_id: Option<&str>,
        answers: &Value,
        nps_score: Option<i64>,
        sentiment: f64,
    ) -> Result<i64, FormError> {
        // Validate form existence
        self.get_form(form_id)?;
        let answers_str = serde_json::to_string(answers)?;
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            r#"
            INSERT INTO responses (form_id, account_id, answers_json, nps_score, sentiment, submitted_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![form_id, account_id, answers_str, nps_score, sentiment, now],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_responses(&self, form_id: &str) -> Result<Vec<ResponseRecord>, FormError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, form_id, account_id, answers_json, nps_score, sentiment, submitted_at
            FROM responses
            WHERE form_id = ?1
            ORDER BY id ASC
            "#,
        )?;
        let rows = stmt.query_map(params![form_id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<i64>>(4)?,
                r.get::<_, f64>(5)?,
                r.get::<_, String>(6)?,
            ))
        })?;
        let mut list = Vec::new();
        for r in rows {
            let (id, f_id, account_id, answers_str, nps_score, sentiment, submitted_at) = r?;
            let answers = serde_json::from_str(&answers_str).unwrap_or(Value::Null);
            list.push(ResponseRecord {
                id,
                form_id: f_id,
                account_id,
                answers,
                nps_score,
                sentiment,
                submitted_at,
            });
        }
        Ok(list)
    }

    pub fn nps_report(&self, form_id: &str) -> Result<NpsReport, FormError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT nps_score, sentiment
            FROM responses
            WHERE form_id = ?1 AND nps_score IS NOT NULL
            "#,
        )?;
        let rows = stmt.query_map(params![form_id], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)?))
        })?;
        let mut promoters = 0usize;
        let mut passives = 0usize;
        let mut detractors = 0usize;
        let mut total_sentiment = 0.0_f64;
        let mut count = 0usize;

        for res in rows {
            let (score, sent) = res?;
            count += 1;
            total_sentiment += sent;
            if score >= 9 {
                promoters += 1;
            } else if score >= 7 {
                passives += 1;
            } else {
                detractors += 1;
            }
        }

        let nps = if count > 0 {
            ((promoters as f64 - detractors as f64) / count as f64) * 100.0
        } else {
            0.0
        };
        let avg_sentiment = if count > 0 {
            total_sentiment / count as f64
        } else {
            0.0
        };

        Ok(NpsReport {
            form_id: form_id.to_string(),
            total_responses: count,
            promoters,
            passives,
            detractors,
            nps,
            avg_sentiment,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn store_crud_and_nps_report() {
        let mut s = Store::in_memory().unwrap();
        s.save_form(
            "csat-2026",
            "Customer Satisfaction",
            "Q4 CSAT Survey",
            &json!({"type": "object"}),
        )
        .unwrap();

        let f = s.get_form("csat-2026").unwrap();
        assert_eq!(f.title, "Customer Satisfaction");

        s.add_response(
            "csat-2026",
            Some("ACC-01"),
            &json!({"comment": "super"}),
            Some(10),
            0.8,
        )
        .unwrap();
        s.add_response(
            "csat-2026",
            Some("ACC-02"),
            &json!({"comment": "ok"}),
            Some(8),
            0.1,
        )
        .unwrap();
        s.add_response(
            "csat-2026",
            Some("ACC-03"),
            &json!({"comment": "bad"}),
            Some(4),
            -0.7,
        )
        .unwrap();

        let report = s.nps_report("csat-2026").unwrap();
        assert_eq!(report.total_responses, 3);
        assert_eq!(report.promoters, 1);
        assert_eq!(report.passives, 1);
        assert_eq!(report.detractors, 1);
        assert_eq!(report.nps, 0.0);
    }
}
