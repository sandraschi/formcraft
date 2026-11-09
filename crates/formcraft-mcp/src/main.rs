//! `formcraft`: CLI and MCP stdio server in one binary.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use clap::{Parser, Subcommand};
use craft_biz::bus::Bus;
use craft_biz::mcp::{serve_stdio, ToolHost};
use formcraft_core::{dispatch, tools, Command, Confinement, Ctx, APP, VERSION};
use formcraft_store::Store;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "formcraft",
    version,
    about = "formcraft: survey, response and NPS studio. CLI + MCP."
)]
struct Cli {
    /// SQLite database. Default: $FORMCRAFT_DB or %LOCALAPPDATA%\formcraft\formcraft.db
    #[arg(long, global = true)]
    db: Option<PathBuf>,
    /// Print machine-readable JSON only.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Status and counts.
    Status,
    /// Define or update a form.
    Define {
        id: String,
        title: String,
        #[arg(default_value = "")]
        description: String,
        #[arg(default_value = "{}")]
        schema: String,
    },
    /// List all forms.
    List,
    /// Get a form by ID.
    Get { id: String },
    /// Calculate NPS report for a form.
    Nps { form_id: String },
    /// Comprehensive report rollup for a form.
    Report { form_id: String },
    /// Route detractors and negative sentiment signals to gaincraft.
    RouteDetractors { form_id: String },
    /// Drain incoming bus signals.
    InboxDrain,
    /// Print tool specifications (or markdown docs).
    Tools {
        #[arg(long)]
        markdown: bool,
    },
    /// Run as MCP stdio server.
    Mcp,
}

struct Host {
    ctx: Ctx,
}

impl ToolHost for Host {
    fn server_name(&self) -> &str {
        APP
    }
    fn server_version(&self) -> &str {
        VERSION
    }
    fn tools(&self) -> Vec<Value> {
        tools()
    }
    fn call(&mut self, name: &str, args: &Value) -> Result<Value, String> {
        let cmd = Command::from_tool(name, args)?;
        dispatch(&cmd, &mut self.ctx)
    }
}

fn default_db() -> PathBuf {
    if let Ok(p) = std::env::var("FORMCRAFT_DB") {
        if !p.trim().is_empty() {
            return PathBuf::from(p);
        }
    }
    let base = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join("formcraft").join("formcraft.db")
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let db_path = cli.db.unwrap_or_else(default_db);

    let is_mcp = matches!(cli.cmd, Some(Cmd::Mcp));
    if !is_mcp {
        if let Some(parent) = db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
    }

    let store = match Store::open(&db_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error opening database at {}: {e}", db_path.display());
            return ExitCode::FAILURE;
        }
    };

    let actor = if is_mcp {
        "agent".to_string()
    } else {
        std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_else(|_| "cli".into())
    };

    let ctx = Ctx {
        store,
        confinement: Confinement(None),
        bus: Bus::new(Bus::default_root()),
        actor,
    };

    if is_mcp {
        let mut host = Host { ctx };
        return match serve_stdio(&mut host) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("formcraft mcp: {e}");
                ExitCode::FAILURE
            }
        };
    }

    let mut ctx = ctx;

    let cmd_target = match cli.cmd.unwrap_or(Cmd::Status) {
        Cmd::Mcp => unreachable!(),
        Cmd::Tools { markdown } => {
            if markdown {
                for t in tools() {
                    let name = t.get("name").and_then(Value::as_str).unwrap_or("");
                    let desc = t.get("description").and_then(Value::as_str).unwrap_or("");
                    println!("- `{name}`: {desc}");
                }
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&tools()).unwrap_or_default()
                );
            }
            return ExitCode::SUCCESS;
        }
        Cmd::Status => Command::Status,
        Cmd::List => Command::FormList,
        Cmd::Get { id } => Command::FormGet { id },
        Cmd::Define {
            id,
            title,
            description,
            schema,
        } => {
            let parsed = serde_json::from_str(&schema).unwrap_or(json!({}));
            Command::FormDefine {
                id,
                title,
                description,
                schema: parsed,
            }
        }
        Cmd::Nps { form_id } => Command::FormNps { form_id },
        Cmd::Report { form_id } => Command::FormReport { form_id },
        Cmd::RouteDetractors { form_id } => Command::RouteDetractors { form_id },
        Cmd::InboxDrain => Command::InboxDrain,
    };

    match dispatch(&cmd_target, &mut ctx) {
        Ok(val) => {
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
            } else {
                println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}
