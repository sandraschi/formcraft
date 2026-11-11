//! formcraft-app: Native eframe/egui desktop studio for FormCraft.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use craft_biz::bus::Bus;
use eframe::egui::{self, Color32, RichText};
use formcraft_core::{dispatch, Command, Confinement, Ctx};
use formcraft_store::Store;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Forms,
    Responses,
    NpsAnalytics,
    BusSignals,
}

struct FormCraftApp {
    ctx: Ctx,
    view: View,
    selected_form: Option<String>,
    forms_list: Vec<Value>,
    responses_list: Vec<Value>,
    nps_summary: Option<Value>,
    status_msg: String,
}

impl FormCraftApp {
    fn new(ctx: Ctx) -> Self {
        let mut app = Self {
            ctx,
            view: View::Forms,
            selected_form: None,
            forms_list: Vec::new(),
            responses_list: Vec::new(),
            nps_summary: None,
            status_msg: "Ready".to_string(),
        };
        app.reload();
        app
    }

    fn reload(&mut self) {
        if let Ok(res) = dispatch(&Command::FormList, &mut self.ctx) {
            if let Some(arr) = res.get("forms").and_then(|v| v.as_array()) {
                self.forms_list = arr.clone();
                if self.selected_form.is_none() && !self.forms_list.is_empty() {
                    self.selected_form = self.forms_list[0]
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }
            }
        }
        if let Some(ref fid) = self.selected_form {
            if let Ok(res) = dispatch(
                &Command::ResponsesList {
                    form_id: fid.clone(),
                },
                &mut self.ctx,
            ) {
                if let Some(arr) = res.get("responses").and_then(|v| v.as_array()) {
                    self.responses_list = arr.clone();
                }
            }
            if let Ok(res) = dispatch(
                &Command::FormNps {
                    form_id: fid.clone(),
                },
                &mut self.ctx,
            ) {
                self.nps_summary = res.get("nps_report").cloned();
            }
        }
    }
}

impl eframe::App for FormCraftApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.horizontal(|ui| {
            ui.heading(
                RichText::new("📋 FormCraft Studio")
                    .color(Color32::from_rgb(56, 189, 248))
                    .strong(),
            );
            ui.separator();
            if ui
                .selectable_label(self.view == View::Forms, "📝 Forms")
                .clicked()
            {
                self.view = View::Forms;
            }
            if ui
                .selectable_label(self.view == View::Responses, "📥 Responses")
                .clicked()
            {
                self.view = View::Responses;
            }
            if ui
                .selectable_label(self.view == View::NpsAnalytics, "📊 NPS Analytics")
                .clicked()
            {
                self.view = View::NpsAnalytics;
            }
            if ui
                .selectable_label(self.view == View::BusSignals, "⚡ Bus Signals")
                .clicked()
            {
                self.view = View::BusSignals;
            }
            ui.separator();
            if ui.button("🔄 Refresh").clicked() {
                self.reload();
            }
            ui.label(
                RichText::new(&self.status_msg)
                    .monospace()
                    .color(Color32::from_rgb(52, 211, 153)),
            );
        });

        ui.separator();

        match self.view {
            View::Forms => {
                let mut switch_to = None;
                ui.horizontal(|ui| {
                    ui.label("Selected Form:");
                    for f in &self.forms_list {
                        let id = f.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        let title = f.get("title").and_then(|v| v.as_str()).unwrap_or(id);
                        let is_sel = self.selected_form.as_deref() == Some(id);
                        if ui.selectable_label(is_sel, title).clicked() {
                            switch_to = Some(id.to_string());
                        }
                    }
                });
                if let Some(new_id) = switch_to {
                    self.selected_form = Some(new_id);
                    self.reload();
                }
                ui.add_space(8.0);
                egui::Grid::new("forms_table")
                    .striped(true)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new("ID").strong());
                        ui.label(RichText::new("Title").strong());
                        ui.label(RichText::new("Description").strong());
                        ui.end_row();

                        for f in &self.forms_list {
                            let id = f.get("id").and_then(|v| v.as_str()).unwrap_or("");
                            let title = f.get("title").and_then(|v| v.as_str()).unwrap_or("");
                            let desc = f.get("description").and_then(|v| v.as_str()).unwrap_or("");
                            ui.monospace(id);
                            ui.label(title);
                            ui.label(desc);
                            ui.end_row();
                        }
                    });
            }
            View::Responses => {
                ui.heading(format!(
                    "Responses for {}",
                    self.selected_form.as_deref().unwrap_or("none")
                ));
                egui::Grid::new("responses_grid")
                    .striped(true)
                    .spacing([14.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new("ID").strong());
                        ui.label(RichText::new("Account").strong());
                        ui.label(RichText::new("NPS").strong());
                        ui.label(RichText::new("Sentiment").strong());
                        ui.label(RichText::new("Submitted At").strong());
                        ui.end_row();

                        for r in &self.responses_list {
                            let id = r.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
                            let acc = r.get("account_id").and_then(|v| v.as_str()).unwrap_or("-");
                            let nps = r
                                .get("nps_score")
                                .and_then(|v| v.as_i64())
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "-".to_string());
                            let sent = r.get("sentiment").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            let sub = r.get("submitted_at").and_then(|v| v.as_str()).unwrap_or("");

                            ui.monospace(id.to_string());
                            ui.monospace(acc);
                            ui.label(nps);
                            ui.label(format!("{sent:.2}"));
                            ui.label(sub);
                            ui.end_row();
                        }
                    });
            }
            View::NpsAnalytics => {
                ui.heading(format!(
                    "NPS & Sentiment Scorecard: {}",
                    self.selected_form.as_deref().unwrap_or("none")
                ));
                if let Some(ref report) = self.nps_summary {
                    let total = report
                        .get("total_responses")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let nps = report.get("nps").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let sent = report
                        .get("avg_sentiment")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    let p = report
                        .get("promoters")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let pass = report.get("passives").and_then(|v| v.as_u64()).unwrap_or(0);
                    let d = report
                        .get("detractors")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);

                    ui.group(|ui| {
                        ui.label(format!("Total Responses: {total}"));
                        ui.label(RichText::new(format!("Net Promoter Score: {nps:.1}")).size(18.0).color(Color32::from_rgb(56, 189, 248)).strong());
                        ui.label(format!("Average Sentiment: {sent:.2}"));
                        ui.label(format!("Breakdown: {p} Promoters (9-10) · {pass} Passives (7-8) · {d} Detractors (0-6)"));
                    });

                    if ui.button("⚡ Route Detractors to GainCraft").clicked() {
                        if let Some(ref fid) = self.selected_form {
                            if let Ok(res) = dispatch(
                                &Command::RouteDetractors {
                                    form_id: fid.clone(),
                                },
                                &mut self.ctx,
                            ) {
                                let routed = res
                                    .get("routed_detractors")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                                self.status_msg =
                                    format!("Routed {routed} detractors to GainCraft bus inbox");
                            }
                        }
                    }
                } else {
                    ui.label("No responses available for this form.");
                }
            }
            View::BusSignals => {
                ui.heading("FormCraft Bus Dock");
                ui.label("Events emitted: `signal.negative` -> `gaincraft`");
                if ui.button("Drain FormCraft Inbox").clicked() {
                    let _ = dispatch(&Command::InboxDrain, &mut self.ctx);
                    self.status_msg = "Inbox drained".to_string();
                }
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--smoke") {
        let store = match Store::in_memory() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("smoke failed: {e}");
                std::process::exit(1);
            }
        };
        let mut ctx = Ctx {
            store,
            confinement: Confinement(None),
            bus: Bus::new(PathBuf::from("target/smoke_bus")),
            actor: "smoke".to_string(),
        };
        if dispatch(&Command::Status, &mut ctx).is_ok() {
            println!("smoke: ok (formcraft-app)");
            std::process::exit(0);
        } else {
            std::process::exit(1);
        }
    }

    let default_db = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("formcraft")
        .join("formcraft.db");
    let store = match Store::open(&default_db) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("formcraft-app: cannot open {}: {e}", default_db.display());
            std::process::exit(1);
        }
    };
    let ctx = Ctx {
        store,
        confinement: Confinement(None),
        bus: Bus::new(Bus::default_root()),
        actor: "desktop".to_string(),
    };

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("FormCraft - Survey, Response & NPS Studio")
            .with_inner_size([1100.0, 720.0]),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "formcraft-app",
        native_options,
        Box::new(|_cc| Ok(Box::new(FormCraftApp::new(ctx)))),
    );
}
