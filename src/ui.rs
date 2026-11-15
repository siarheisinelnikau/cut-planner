use crate::{
    export::size_label,
    i18n::{LANGUAGES, tr},
    model::*,
    solver::SolveError,
    storage::{Preferences, Theme},
    units::{Length, MM, Unit},
    updates::*,
};
use egui::{Color32, FontId, Id, RichText, Sense, Stroke, Vec2, vec2};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Inputs,
    Constraints,
    Plan,
    Settings,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pending {
    New,
    Open,
    Example,
    Import,
    Quit,
    Install,
}
#[derive(Clone, Copy)]
pub enum FileAction {
    Open,
    Import,
    ExportCsv,
    Report,
    Svg,
    Licenses,
}

pub enum Action {
    Continue(Pending),
    Save {
        after: Option<Pending>,
        save_as: bool,
    },
    File(FileAction),
    Solve {
        project: Project,
        generation: u64,
        cancel: Arc<AtomicBool>,
    },
    SavePreferences(Preferences),
    CheckUpdate,
    Download(AvailableUpdate),
    Quit,
}

pub enum Message {
    Solved {
        generation: u64,
        result: Result<Plan, SolveError>,
    },
    Progress {
        generation: u64,
        done: usize,
        total: usize,
    },
    Loaded(Result<(PathBuf, Project), String>),
    Imported(Result<Vec<Part>, String>),
    Saved {
        result: Result<Option<PathBuf>, String>,
        snapshot: Project,
        after: Option<Pending>,
    },
    Exported(Result<bool, String>),
    FileCancelled,
    PreferencesSaved(Result<(), String>),
    Checked(Result<Option<AvailableUpdate>, UpdateError>),
    Downloaded(Result<VerifiedUpdate, UpdateError>),
    DownloadProgress {
        read: u64,
        total: u64,
    },
    InstallFailed(UpdateError),
    InstallerPrepared(Result<(), UpdateError>),
}

pub enum UpdateState {
    Idle,
    Checking,
    Current,
    Available(AvailableUpdate),
    Downloading { fraction: f32 },
    Ready(VerifiedUpdate),
    Installing,
    Failed(UpdateError),
}

#[derive(Default)]
struct Fields {
    text: HashMap<Id, String>,
    invalid: HashMap<Id, &'static str>,
    edited: bool,
}

pub struct Workspace {
    pub project: Project,
    saved: Project,
    pub path: Option<PathBuf>,
    pub preferences: Preferences,
    pub stage: Stage,
    pub planned: Option<(Project, Plan)>,
    pub generation: u64,
    pub solving: Option<Arc<AtomicBool>>,
    progress: f32,
    pub file_busy: bool,
    pub update: UpdateState,
    fields: Fields,
    pending: Option<Pending>,
    notice: Option<(String, bool)>,
    sheet: usize,
    cut: Option<usize>,
    selected_node: Option<usize>,
    inspector: usize,
    zoom: f32,
    pan: Vec2,
    actions: Vec<Action>,
}

#[derive(Clone, Copy)]
struct Palette {
    background: Color32,
    surface: Color32,
    ink: Color32,
    muted: Color32,
    border: Color32,
    accent: Color32,
    canvas: Color32,
}
fn palette(theme: Theme) -> Palette {
    match theme {
        Theme::Light => Palette {
            background: Color32::from_rgb(246, 248, 245),
            surface: Color32::WHITE,
            ink: Color32::from_rgb(27, 48, 42),
            muted: Color32::from_rgb(83, 104, 95),
            border: Color32::from_rgb(209, 222, 215),
            accent: Color32::from_rgb(26, 104, 83),
            canvas: Color32::from_rgb(232, 239, 234),
        },
        Theme::Dark => Palette {
            background: Color32::from_rgb(20, 29, 27),
            surface: Color32::from_rgb(28, 40, 35),
            ink: Color32::from_rgb(231, 240, 233),
            muted: Color32::from_rgb(173, 192, 179),
            border: Color32::from_rgb(60, 79, 68),
            accent: Color32::from_rgb(110, 202, 164),
            canvas: Color32::from_rgb(17, 26, 22),
        },
    }
}

impl Workspace {
    pub fn new(preferences: Preferences) -> Self {
        let project = Project::empty();
        Self {
            saved: project.clone(),
            project,
            path: None,
            preferences,
            stage: Stage::Inputs,
            planned: None,
            generation: 0,
            solving: None,
            progress: 0.0,
            file_busy: false,
            update: UpdateState::Idle,
            fields: Fields::default(),
            pending: None,
            notice: None,
            sheet: 0,
            cut: None,
            selected_node: None,
            inspector: 0,
            zoom: 1.0,
            pan: Vec2::ZERO,
            actions: vec![],
        }
    }
    pub fn dirty(&self) -> bool {
        self.project != self.saved || !self.fields.invalid.is_empty()
    }
    pub fn request(&mut self, pending: Pending) {
        if self.file_busy {
            return;
        }
        if self.dirty() {
            self.pending = Some(pending);
        } else {
            self.actions.push(Action::Continue(pending));
        }
    }
    pub fn replace_project(&mut self, path: Option<PathBuf>, project: Project, saved: bool) {
        self.cancel_solver();
        self.generation += 1;
        self.project = project;
        self.path = path;
        self.saved = if saved {
            self.project.clone()
        } else {
            Project::empty()
        };
        self.planned = None;
        self.fields = Fields::default();
        self.stage = Stage::Inputs;
        self.zoom = 1.0;
        self.pan = Vec2::ZERO;
        self.notice = None;
    }
    pub fn cancel_solver(&mut self) {
        if let Some(cancel) = &self.solving {
            cancel.store(true, Ordering::Relaxed);
        }
    }
    fn changed(&mut self) {
        self.generation += 1;
        self.planned = None;
        self.cut = None;
        self.selected_node = None;
    }
    pub fn notify_error(&mut self, error: String) {
        self.notice = Some((self.localize_error(&error), true));
    }
    fn localize_error(&self, error: &str) -> String {
        let lang = &self.preferences.language;
        let (key, detail) = error.split_once(" (").unwrap_or((error, ""));
        let text = tr(lang, key);
        if !detail.is_empty() {
            let detail = detail
                .strip_prefix("row ")
                .map(|row| format!("{} {row}", tr(lang, "row")))
                .unwrap_or_else(|| detail.to_owned());
            format!("{text} ({detail}")
        } else {
            text.to_string()
        }
    }
    pub fn handle(&mut self, message: Message) {
        let lang = self.preferences.language.clone();
        match message {
            Message::Solved { generation, result } => {
                // Never publish a result for different dimensions or a different project.
                self.solving = None;
                if generation != self.generation {
                    return;
                }
                match result {
                    Ok(plan) => {
                        if plan.cancelled {
                            self.notice = Some((tr(&lang, "cancelled").into(), false));
                        }
                        self.planned = Some((self.project.clone(), plan));
                        self.stage = Stage::Plan;
                        self.sheet = 0;
                        self.cut = None;
                        self.selected_node = None;
                        self.zoom = 1.0;
                        self.pan = Vec2::ZERO;
                    }
                    Err(SolveError::InvalidPlan(detail)) => {
                        self.notify_error(format!("{} {detail}", tr(&lang, "validating")))
                    }
                    Err(e) => self.notify_error(e.to_string()),
                }
            }
            Message::Progress {
                generation,
                done,
                total,
            } => {
                if generation == self.generation {
                    self.progress = done as f32 / total.max(1) as f32;
                }
            }
            Message::Loaded(result) => {
                self.file_busy = false;
                match result {
                    Ok((path, project)) => self.replace_project(Some(path), project, true),
                    Err(e) => self.notify_error(e),
                }
            }
            Message::Imported(result) => {
                self.file_busy = false;
                match result {
                    Ok(parts) => {
                        self.project.parts = parts;
                        self.fields = Fields::default();
                        self.changed();
                        self.stage = Stage::Inputs;
                    }
                    Err(e) => self.notify_error(e),
                }
            }
            Message::Saved {
                result,
                snapshot,
                after,
            } => {
                self.file_busy = false;
                match result {
                    Ok(Some(path)) => {
                        self.path = Some(path);
                        self.saved = snapshot;
                        self.notice = Some((tr(&lang, "saved").into(), false));
                        if !self.dirty()
                            && let Some(after) = after
                        {
                            self.actions.push(Action::Continue(after));
                        }
                    }
                    Ok(None) => {}
                    Err(e) => self.notify_error(e),
                }
            }
            Message::Exported(result) => {
                self.file_busy = false;
                match result {
                    Ok(true) => self.notice = Some((tr(&lang, "exported").into(), false)),
                    Ok(false) => {}
                    Err(e) => self.notify_error(e),
                }
            }
            Message::FileCancelled => self.file_busy = false,
            Message::PreferencesSaved(result) => {
                if let Err(e) = result {
                    self.notify_error(e);
                }
            }
            Message::Checked(result) => {
                self.update = match result {
                    Ok(Some(update)) => UpdateState::Available(update),
                    Ok(None) => UpdateState::Current,
                    Err(e) => UpdateState::Failed(e),
                };
            }
            Message::Downloaded(result) => {
                self.update = match result {
                    Ok(update) => UpdateState::Ready(update),
                    Err(e) => UpdateState::Failed(e),
                };
            }
            Message::DownloadProgress { read, total } => {
                self.update = UpdateState::Downloading {
                    fraction: (read as f32 / total.max(1) as f32).min(1.0),
                }
            }
            Message::InstallFailed(e) => self.update = UpdateState::Failed(e),
            Message::InstallerPrepared(result) => {
                self.file_busy = false;
                match result {
                    Ok(()) => self.actions.push(Action::Quit),
                    Err(e) => self.update = UpdateState::Failed(e),
                }
            }
        }
    }

    pub fn take_actions(&mut self) -> Vec<Action> {
        std::mem::take(&mut self.actions)
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        self.style(ctx);
        let lang = self.preferences.language.clone();
        let t = |key| tr(&lang, key);
        let p = palette(self.preferences.theme);
        let before_preferences = self.preferences.clone();
        let before = self.project.clone();
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::S))
            && self.fields.invalid.is_empty()
            && !self.file_busy
        {
            self.actions.push(Action::Save {
                after: None,
                save_as: false,
            });
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::O)) {
            self.request(Pending::Open);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::N)) {
            self.request(Pending::New);
        }
        egui::TopBottomPanel::top("header")
            .frame(
                egui::Frame::new()
                    .fill(p.surface)
                    .inner_margin(egui::Margin::symmetric(24, 15)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::hover());
                    ui.painter().rect_filled(rect, 8, p.accent);
                    let inner = rect.shrink(7.0);
                    ui.painter().rect_filled(
                        egui::Rect::from_min_max(
                            inner.min,
                            egui::pos2(inner.center().x - 1.5, inner.max.y),
                        ),
                        1,
                        Color32::from_rgb(221, 239, 214),
                    );
                    ui.painter().rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(inner.center().x + 1.5, inner.min.y),
                            inner.max,
                        ),
                        1,
                        Color32::from_rgb(238, 210, 163),
                    );
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Cut Planner").size(22.0).strong());
                        ui.label(RichText::new(t("tagline")).size(12.0).color(p.muted));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_enabled(
                                !self.file_busy && self.fields.invalid.is_empty(),
                                egui::Button::new(t("save")).min_size(vec2(82.0, 34.0)),
                            )
                            .clicked()
                        {
                            self.actions.push(Action::Save {
                                after: None,
                                save_as: false,
                            });
                        }
                        if ui
                            .add_enabled(
                                !self.file_busy,
                                egui::Button::new(t("open")).min_size(vec2(82.0, 34.0)),
                            )
                            .clicked()
                        {
                            self.request(Pending::Open);
                        }
                        ui.menu_button(t("project"), |ui| {
                            for (label, pending) in [
                                ("new", Pending::New),
                                ("example", Pending::Example),
                                ("import_csv", Pending::Import),
                            ] {
                                if ui.button(t(label)).clicked() {
                                    self.request(pending);
                                    ui.close();
                                }
                            }
                            if ui
                                .add_enabled(
                                    self.fields.invalid.is_empty() && !self.file_busy,
                                    egui::Button::new(t("save_as")),
                                )
                                .clicked()
                            {
                                self.actions.push(Action::Save {
                                    after: None,
                                    save_as: true,
                                });
                                ui.close();
                            }
                            if ui
                                .add_enabled(
                                    self.fields.invalid.is_empty()
                                        && self.project.validate().is_ok(),
                                    egui::Button::new(t("export_csv")),
                                )
                                .clicked()
                            {
                                self.actions.push(Action::File(FileAction::ExportCsv));
                                ui.close();
                            }
                        });
                    });
                });
            });
        egui::TopBottomPanel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(p.surface)
                    .inner_margin(egui::Margin::symmetric(24, 8)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(
                            self.path
                                .as_ref()
                                .and_then(|p| p.file_name())
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| t("project").into()),
                        )
                        .color(p.muted),
                    );
                    if self.dirty() {
                        ui.label(RichText::new(format!("• {}", t("unsaved"))).color(p.muted));
                    }
                    if let Some((notice, error)) = &self.notice {
                        ui.separator();
                        ui.colored_label(
                            if *error {
                                Color32::from_rgb(195, 76, 59)
                            } else {
                                p.accent
                            },
                            notice,
                        );
                    }
                    if self.notice.is_some()
                        && ui.small_button("×").on_hover_text(t("close")).clicked()
                    {
                        self.notice = None;
                    }
                });
            });
        egui::SidePanel::left("navigation")
            .exact_width(190.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(p.surface)
                    .inner_margin(egui::Margin::symmetric(16, 24)),
            )
            .show(ctx, |ui| {
                for (index, stage, key) in [
                    (1, Stage::Inputs, "inputs"),
                    (2, Stage::Constraints, "constraints"),
                    (3, Stage::Plan, "plan"),
                ] {
                    let selected = self.stage == stage;
                    let response = ui.add_sized(
                        [ui.available_width(), 48.0],
                        egui::Button::selectable(selected, format!("0{index}   {}", t(key))),
                    );
                    if response.clicked() {
                        self.stage = stage;
                    }
                    let a = ctx.animate_bool_with_time(
                        Id::new(("nav", index)),
                        selected,
                        if self.preferences.reduced_motion {
                            0.0
                        } else {
                            0.14
                        },
                    );
                    if a > 0.0 {
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(
                                response.rect.left_top(),
                                vec2(3.0, response.rect.height()),
                            ),
                            2,
                            p.accent.gamma_multiply(a),
                        );
                    }
                    ui.add_space(6.0);
                }
                ui.add_space(22.0);
                ui.separator();
                if ui
                    .add_sized(
                        [ui.available_width(), 42.0],
                        egui::Button::selectable(self.stage == Stage::Settings, t("settings")),
                    )
                    .clicked()
                {
                    self.stage = Stage::Settings;
                }
                if matches!(self.update, UpdateState::Available(_)) {
                    ui.colored_label(p.accent, t("update_available"));
                }
            });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(p.background)
                    .inner_margin(egui::Margin::same(24)),
            )
            .show(ctx, |ui| match self.stage {
                Stage::Inputs => self.inputs(ui, &lang),
                Stage::Constraints => self.constraints(ui, &lang),
                Stage::Plan => self.plan_view(ui, &lang),
                Stage::Settings => self.settings(ui, &lang),
            });
        if before != self.project || self.fields.edited {
            self.changed();
            self.fields.edited = false;
            for action in &mut self.actions {
                if let Action::Solve {
                    generation,
                    project,
                    ..
                } = action
                {
                    *generation = self.generation;
                    *project = self.project.clone();
                }
            }
        }
        if before_preferences != self.preferences {
            self.actions
                .push(Action::SavePreferences(self.preferences.clone()));
        }
        self.confirmation(ctx, &lang);
    }

    fn style(&self, ctx: &egui::Context) {
        let p = palette(self.preferences.theme);
        let mut visuals = match self.preferences.theme {
            Theme::Light => egui::Visuals::light(),
            Theme::Dark => egui::Visuals::dark(),
        };
        visuals.panel_fill = p.surface;
        visuals.window_fill = p.surface;
        visuals.override_text_color = Some(p.ink);
        visuals.faint_bg_color = p.background;
        visuals.extreme_bg_color = p.background;
        visuals.selection.bg_fill = p.accent.gamma_multiply(0.22);
        visuals.selection.stroke = Stroke::new(1.0, p.accent);
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
        visuals.widgets.inactive.bg_fill = p.surface;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, p.border);
        visuals.widgets.hovered.bg_fill = p.accent.gamma_multiply(0.15);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, p.accent);
        visuals.widgets.active.bg_fill = p.accent.gamma_multiply(0.22);
        visuals.text_cursor.blink = false;
        ctx.set_visuals(visuals);
        ctx.style_mut(|s| {
            s.spacing.item_spacing = vec2(10.0, 10.0);
            s.spacing.button_padding = vec2(14.0, 8.0);
            s.spacing.interact_size.y = 32.0;
            s.spacing.combo_height = 240.0;
            s.animation_time = if self.preferences.reduced_motion {
                0.0
            } else {
                0.14
            };
            s.text_styles
                .insert(egui::TextStyle::Body, FontId::proportional(14.0));
            s.text_styles
                .insert(egui::TextStyle::Button, FontId::proportional(14.0));
            s.text_styles
                .insert(egui::TextStyle::Heading, FontId::proportional(26.0));
        });
    }

    fn validation(&self, ui: &mut egui::Ui, lang: &str) -> bool {
        let color = Color32::from_rgb(195, 76, 59);
        if let Some(key) = self.fields.invalid.values().next() {
            ui.colored_label(color, tr(lang, key));
            return false;
        }
        if let Err(e) = self.project.validate() {
            let row = e
                .row
                .map(|r| format!(" · {} {}", tr(lang, "row"), r + 1))
                .unwrap_or_default();
            ui.colored_label(color, format!("{}{row}", tr(lang, e.key)));
            return false;
        }
        true
    }

    fn inputs(&mut self, ui: &mut egui::Ui, lang: &str) {
        let t = |key| tr(lang, key);
        ui.heading(t("inputs"));
        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .id_salt("inputs_scroll")
            .show(ui, |ui| {
                ui.add_enabled_ui(self.solving.is_none() && !self.file_busy, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            let label = ui.label(t("project_name"));
                            ui.add(
                                egui::TextEdit::singleline(&mut self.project.name)
                                    .desired_width(280.0),
                            )
                            .labelled_by(label.id);
                        });
                        ui.vertical(|ui| {
                            let label = ui.label(t("material"));
                            ui.add(
                                egui::TextEdit::singleline(&mut self.project.material)
                                    .desired_width(280.0),
                            )
                            .labelled_by(label.id);
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.label(t("unit"));
                        let old = self.project.unit;
                        ui.add_enabled_ui(self.fields.invalid.is_empty(), |ui| {
                            egui::ComboBox::from_id_salt("units")
                                .selected_text(old.symbol())
                                .width(85.0)
                                .show_ui(ui, |ui| {
                                    for unit in Unit::ALL {
                                        ui.selectable_value(
                                            &mut self.project.unit,
                                            unit,
                                            unit.symbol(),
                                        );
                                    }
                                });
                        });
                        if old != self.project.unit {
                            self.fields = Fields::default();
                        }
                    });
                    ui.label(
                        RichText::new(t("unit_note"))
                            .small()
                            .color(palette(self.preferences.theme).muted),
                    );
                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(t("stock")).size(19.0).strong());
                        if ui
                            .add_enabled(
                                self.project.stock.len() < MAX_ROWS,
                                egui::Button::new(t("add_stock")),
                            )
                            .clicked()
                        {
                            let id = next_row_id(self.project.stock.iter().map(|s| s.id));
                            self.project.stock.push(Stock {
                                id,
                                name: format!("{} {}", t("sheet"), self.project.stock.len() + 1),
                                width: 2440 * MM,
                                height: 1220 * MM,
                                quantity: 1,
                                grain: Grain::None,
                            });
                        }
                    });
                    let mut remove = None;
                    egui::ScrollArea::horizontal()
                        .id_salt("stock_horizontal")
                        .show(ui, |ui| {
                            egui::Grid::new("stock_grid")
                                .striped(true)
                                .spacing([10.0, 8.0])
                                .show(ui, |ui| {
                                    let mut headers = Vec::new();
                                    for key in ["name", "width", "height", "quantity", "grain", ""]
                                    {
                                        headers.push(ui.label(t(key)).id);
                                    }
                                    ui.end_row();
                                    for (i, stock) in self.project.stock.iter_mut().enumerate() {
                                        ui.add(
                                            egui::TextEdit::singleline(&mut stock.name)
                                                .id(Id::new(("stock", stock.id, "name")))
                                                .desired_width(155.0),
                                        )
                                        .labelled_by(headers[0]);
                                        self.fields
                                            .length(
                                                ui,
                                                self.project.unit,
                                                &mut stock.width,
                                                ("stock", stock.id, "width"),
                                                lang,
                                            )
                                            .labelled_by(headers[1]);
                                        self.fields
                                            .length(
                                                ui,
                                                self.project.unit,
                                                &mut stock.height,
                                                ("stock", stock.id, "height"),
                                                lang,
                                            )
                                            .labelled_by(headers[2]);
                                        self.fields
                                            .quantity(
                                                ui,
                                                &mut stock.quantity,
                                                ("stock", stock.id, "quantity"),
                                                lang,
                                            )
                                            .labelled_by(headers[3]);
                                        grain_picker(
                                            ui,
                                            &mut stock.grain,
                                            ("stock", stock.id),
                                            lang,
                                        )
                                        .labelled_by(headers[4]);
                                        if labelled_button(ui, "×", t("remove")).clicked() {
                                            remove = Some(i);
                                        }
                                        ui.end_row();
                                    }
                                });
                        });
                    if let Some(i) = remove {
                        let removed = self.project.stock.remove(i);
                        self.fields.remove_row("stock", removed.id);
                    }
                    ui.add_space(22.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(t("parts")).size(19.0).strong());
                        if ui
                            .add_enabled(
                                self.project.parts.len() < MAX_ROWS,
                                egui::Button::new(t("add_part")),
                            )
                            .clicked()
                        {
                            let id = next_row_id(self.project.parts.iter().map(|p| p.id));
                            self.project.parts.push(Part {
                                id,
                                name: format!("{} {}", t("name"), self.project.parts.len() + 1),
                                width: 600 * MM,
                                height: 300 * MM,
                                quantity: 1,
                                rotate: true,
                                grain: Grain::None,
                            });
                        }
                    });
                    let mut remove = None;
                    egui::ScrollArea::horizontal()
                        .id_salt("parts_horizontal")
                        .show(ui, |ui| {
                            egui::Grid::new("parts_grid")
                                .striped(true)
                                .spacing([10.0, 8.0])
                                .show(ui, |ui| {
                                    let mut headers = Vec::new();
                                    for key in [
                                        "name", "width", "height", "quantity", "rotate", "grain",
                                        "",
                                    ] {
                                        headers.push(ui.label(t(key)).id);
                                    }
                                    ui.end_row();
                                    for (i, part) in self.project.parts.iter_mut().enumerate() {
                                        ui.add(
                                            egui::TextEdit::singleline(&mut part.name)
                                                .id(Id::new(("part", part.id, "name")))
                                                .desired_width(155.0),
                                        )
                                        .labelled_by(headers[0]);
                                        self.fields
                                            .length(
                                                ui,
                                                self.project.unit,
                                                &mut part.width,
                                                ("part", part.id, "width"),
                                                lang,
                                            )
                                            .labelled_by(headers[1]);
                                        self.fields
                                            .length(
                                                ui,
                                                self.project.unit,
                                                &mut part.height,
                                                ("part", part.id, "height"),
                                                lang,
                                            )
                                            .labelled_by(headers[2]);
                                        self.fields
                                            .quantity(
                                                ui,
                                                &mut part.quantity,
                                                ("part", part.id, "quantity"),
                                                lang,
                                            )
                                            .labelled_by(headers[3]);
                                        ui.checkbox(&mut part.rotate, "")
                                            .on_hover_text(t("rotate"))
                                            .labelled_by(headers[4]);
                                        grain_picker(ui, &mut part.grain, ("part", part.id), lang)
                                            .labelled_by(headers[5]);
                                        if labelled_button(ui, "×", t("remove")).clicked() {
                                            remove = Some(i);
                                        }
                                        ui.end_row();
                                    }
                                });
                        });
                    if let Some(i) = remove {
                        let removed = self.project.parts.remove(i);
                        self.fields.remove_row("part", removed.id);
                    }
                    if self.project.stock.is_empty() && self.project.parts.is_empty() {
                        ui.add_space(18.0);
                        ui.label(t("empty_input"));
                        if ui.button(t("example")).clicked() {
                            self.request(Pending::Example);
                        }
                    }
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new(t("grain_help"))
                            .small()
                            .color(palette(self.preferences.theme).muted),
                    );
                    egui::CollapsingHeader::new("CSV").show(ui, |ui| {
                        ui.label(t("csv_help"));
                        ui.label(t("csv_units"));
                        if ui.button(t("import_csv")).clicked() {
                            self.request(Pending::Import);
                        }
                        if ui
                            .add_enabled(
                                self.fields.invalid.is_empty() && self.project.validate().is_ok(),
                                egui::Button::new(t("export_csv")),
                            )
                            .clicked()
                        {
                            self.actions.push(Action::File(FileAction::ExportCsv));
                        }
                    });
                });
                ui.add_space(18.0);
                let valid = self.validation(ui, lang);
                if ui
                    .add_enabled(
                        valid && !self.project.stock.is_empty() && !self.project.parts.is_empty(),
                        egui::Button::new(t("next")).min_size(vec2(160.0, 40.0)),
                    )
                    .clicked()
                {
                    self.stage = Stage::Constraints;
                }
            });
    }

    fn constraints(&mut self, ui: &mut egui::Ui, lang: &str) {
        let t = |key| tr(lang, key);
        ui.heading(t("constraints"));
        ui.add_space(18.0);
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_enabled_ui(self.solving.is_none(), |ui| {
                for (key, help) in [
                    ("kerf", "kerf_help"),
                    ("trim", "trim_help"),
                    ("min_offcut", "offcut_help"),
                ] {
                    ui.label(RichText::new(t(key)).size(17.0).strong());
                    ui.horizontal(|ui| {
                        let value = match key {
                            "kerf" => &mut self.project.constraints.kerf,
                            "trim" => &mut self.project.constraints.trim,
                            _ => &mut self.project.constraints.min_offcut,
                        };
                        self.fields.length(ui, self.project.unit, value, key, lang);
                        ui.label(self.project.unit.symbol());
                    });
                    ui.label(RichText::new(t(help)).color(palette(self.preferences.theme).muted));
                    ui.add_space(18.0);
                }
            });
            self.validation(ui, lang);
            ui.separator();
            ui.label(t("heuristic"));
            ui.add_space(10.0);
            self.calculate_button(ui, lang);
        });
    }

    fn calculate_button(&mut self, ui: &mut egui::Ui, lang: &str) {
        let t = |key| tr(lang, key);
        if let Some(cancel) = &self.solving {
            ui.label(t("calculating"));
            let fraction = ui.ctx().animate_value_with_time(
                Id::new("solver_progress"),
                self.progress,
                if self.preferences.reduced_motion {
                    0.0
                } else {
                    0.15
                },
            );
            ui.add(
                egui::ProgressBar::new(fraction)
                    .desired_width(260.0)
                    .show_percentage(),
            );
            if ui.button(t("cancel")).clicked() {
                cancel.store(true, Ordering::Relaxed);
            }
        } else {
            if self.project.parts.is_empty() || self.project.stock.is_empty() {
                ui.label(t("stock_and_parts_required"));
            }
            let valid = self.fields.invalid.is_empty()
                && self.project.validate().is_ok()
                && !self.project.parts.is_empty()
                && !self.project.stock.is_empty();
            if ui
                .add_enabled(
                    valid && !self.file_busy,
                    egui::Button::new(RichText::new(t("calculate")).strong().color(
                        if self.preferences.theme == Theme::Light {
                            Color32::WHITE
                        } else {
                            Color32::from_rgb(20, 40, 30)
                        },
                    ))
                    .fill(palette(self.preferences.theme).accent)
                    .min_size(vec2(220.0, 42.0)),
                )
                .clicked()
            {
                let cancel = Arc::new(AtomicBool::new(false));
                self.solving = Some(cancel.clone());
                self.progress = 0.0;
                self.actions.push(Action::Solve {
                    project: self.project.clone(),
                    generation: self.generation,
                    cancel,
                });
            }
        }
    }

    fn plan_view(&mut self, ui: &mut egui::Ui, lang: &str) {
        let t = |key| tr(lang, key);
        let Some((project, plan)) = self.planned.clone() else {
            ui.heading(t("plan"));
            ui.add_space(35.0);
            ui.label(RichText::new(t("empty_plan")).size(20.0));
            ui.add_space(15.0);
            self.validation(ui, lang);
            self.calculate_button(ui, lang);
            return;
        };
        ui.horizontal(|ui| {
            ui.heading(t("plan"));
            if !plan.sheets.is_empty() {
                egui::ComboBox::from_id_salt("sheet_picker")
                    .selected_text(format!(
                        "{} {} / {}",
                        t("sheet"),
                        self.sheet + 1,
                        plan.sheets.len()
                    ))
                    .width(155.0)
                    .show_ui(ui, |ui| {
                        for i in 0..plan.sheets.len() {
                            if ui
                                .selectable_value(
                                    &mut self.sheet,
                                    i,
                                    format!("{} {}", t("sheet"), i + 1),
                                )
                                .changed()
                            {
                                self.pan = Vec2::ZERO;
                                self.zoom = 1.0;
                                self.cut = None;
                                self.selected_node = None;
                            }
                        }
                    });
            }
            ui.menu_button(t("export_report"), |ui| {
                if ui.button(t("export_report")).clicked() {
                    self.actions.push(Action::File(FileAction::Report));
                    ui.close();
                }
                if ui
                    .add_enabled(!plan.sheets.is_empty(), egui::Button::new(t("export_svg")))
                    .clicked()
                {
                    self.actions.push(Action::File(FileAction::Svg));
                    ui.close();
                }
            });
        });
        ui.label(
            RichText::new(t("heuristic"))
                .small()
                .color(palette(self.preferences.theme).muted),
        );
        ui.add_space(10.0);
        ui.horizontal_wrapped(|ui| {
            for (label, value) in [
                ("sheets_used", plan.sheets.len().to_string()),
                (
                    "placed",
                    format!("{} / {}", plan.stats.placed, plan.stats.required),
                ),
                ("utilization", format!("{:.1}%", plan.stats.utilization())),
                (
                    "waste",
                    format!("{:.3} m²", plan.stats.waste_area() as f64 / 1e18),
                ),
            ] {
                ui.vertical(|ui| {
                    ui.label(RichText::new(value).size(24.0).strong());
                    ui.label(RichText::new(t(label)).small());
                });
                ui.add_space(12.0);
            }
        });
        ui.add_space(14.0);
        if plan.sheets.is_empty() {
            self.unplaced(ui, &project, &plan, lang);
            return;
        }
        self.sheet = self.sheet.min(plan.sheets.len() - 1);
        let sheet = &plan.sheets[self.sheet];
        ui.horizontal_top(|ui| {
            let canvas_width = (ui.available_width() - 296.0).max(260.0);
            ui.vertical(|ui| {
                ui.set_width(canvas_width);
                ui.horizontal(|ui| {
                    if ui.button(t("fit")).clicked() {
                        self.zoom = 1.0;
                        self.pan = Vec2::ZERO;
                    }
                    if ui.button("−").on_hover_text(t("zoom_out")).clicked() {
                        self.zoom = (self.zoom / 1.25).max(0.2);
                    }
                    if ui.button("+").on_hover_text(t("zoom_in")).clicked() {
                        self.zoom = (self.zoom * 1.25).min(20.0);
                    }
                    ui.label(format!("{:.0}%", self.zoom * 100.0));
                });
                self.canvas(ui, &project, sheet, lang);
                ui.label(RichText::new(t("canvas_help")).small());
                ui.label(RichText::new(t("keyboard_canvas")).small());
                ui.label(RichText::new(t("legend")).small());
            });
            ui.separator();
            ui.vertical(|ui| {
                ui.set_width(270.0);
                egui::ComboBox::from_id_salt("inspector_mode")
                    .selected_text(t(["cuts", "offcuts", "unplaced"][self.inspector]))
                    .width(245.0)
                    .show_ui(ui, |ui| {
                        for (i, key) in ["cuts", "offcuts", "unplaced"].into_iter().enumerate() {
                            ui.selectable_value(&mut self.inspector, i, t(key));
                        }
                    });
                let steps = sheet.steps();
                if self.inspector == 0 {
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(self.cut.is_some_and(|i| i > 0), egui::Button::new("←"))
                            .on_hover_text(t("back"))
                            .clicked()
                        {
                            self.cut = self.cut.map(|n| n - 1);
                            self.selected_node = None;
                        }
                        if ui
                            .add_enabled(
                                !steps.is_empty() && self.cut.is_none_or(|n| n + 1 < steps.len()),
                                egui::Button::new("→"),
                            )
                            .on_hover_text(t("next"))
                            .clicked()
                        {
                            self.cut = Some(self.cut.map_or(0, |n| n + 1));
                            self.selected_node = None;
                        }
                    });
                }
                egui::ScrollArea::vertical()
                    .id_salt("inspector")
                    .max_height((ui.available_height() - 15.0).max(220.0))
                    .show(ui, |ui| {
                        match self.inspector {
                            0 => {
                                for (i, step) in steps.iter().enumerate() {
                                    let label = format!(
                                        "{} {} · {}\n{}",
                                        t("step"),
                                        i + 1,
                                        t(match step.axis {
                                            Axis::Vertical => "vertical",
                                            Axis::Horizontal => "horizontal",
                                        }),
                                        project.unit.label(step.offset)
                                    );
                                    if ui
                                        .add_sized(
                                            [250.0, 54.0],
                                            egui::Button::selectable(self.cut == Some(i), label),
                                        )
                                        .clicked()
                                    {
                                        self.cut = Some(i);
                                        self.selected_node = None;
                                    }
                                    if self.cut == Some(i) {
                                        ui.label(format!(
                                            "{}: {}",
                                            t("workpiece"),
                                            size_label(project.unit, step.rect)
                                        ));
                                        ui.label(format!(
                                            "{}: {}, {}",
                                            t("origin"),
                                            project.unit.format(step.rect.x),
                                            project.unit.format(step.rect.y)
                                        ));
                                        ui.label(t("offset"));
                                        ui.label(format!(
                                            "{}: {}",
                                            t("kerf"),
                                            project.unit.label(step.kerf)
                                        ));
                                        ui.label(t(match step.purpose {
                                            Purpose::Trim => "trim_cut",
                                            Purpose::Separate => "separate_cut",
                                        }));
                                    }
                                }
                            }
                            1 => {
                                for (i, r) in
                                    sheet.offcuts(project.constraints.min_offcut).enumerate()
                                {
                                    ui.label(format!(
                                        "#{} · {}",
                                        i + 1,
                                        size_label(project.unit, r)
                                    ));
                                }
                            }
                            _ => self.unplaced(ui, &project, &plan, lang),
                        }
                        if let Some(n) = self.selected_node.and_then(|i| sheet.nodes.get(i)) {
                            ui.separator();
                            if let NodeKind::Part { id, copy, rotated } = n.kind {
                                if let Some(part) = project.parts.iter().find(|p| p.id == id) {
                                    ui.strong(format!("{} #{}", part.name, copy + 1));
                                    if rotated {
                                        ui.label(t("rotated"));
                                    }
                                }
                            } else {
                                ui.strong(t("offcuts"));
                            }
                            ui.label(size_label(project.unit, n.rect));
                            ui.label(format!(
                                "x: {} · y: {}",
                                project.unit.format(n.rect.x),
                                project.unit.format(n.rect.y)
                            ));
                        }
                    });
            });
        });
    }

    fn unplaced(&self, ui: &mut egui::Ui, project: &Project, plan: &Plan, lang: &str) {
        if plan.unplaced.is_empty() {
            ui.label(tr(lang, "none"));
        }
        for part in &project.parts {
            for reason in [
                UnplacedReason::Size,
                UnplacedReason::Grain,
                UnplacedReason::Stock,
                UnplacedReason::Cancelled,
            ] {
                let quantity = plan
                    .unplaced
                    .iter()
                    .filter(|m| m.id == part.id && m.reason == reason)
                    .count();
                if quantity > 0 {
                    ui.strong(format!("{} ×{quantity}", part.name));
                    ui.label(tr(lang, reason.key()));
                    ui.add_space(8.0);
                }
            }
        }
    }

    fn canvas(&mut self, ui: &mut egui::Ui, project: &Project, sheet: &SheetPlan, lang: &str) {
        let height = (ui.available_height() - 70.0).max(270.0);
        let (rect, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click_and_drag());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, true, tr(lang, "plan"))
        });
        let painter = ui.painter().with_clip_rect(rect);
        let p = palette(self.preferences.theme);
        painter.rect_filled(rect, 10, p.canvas);
        let r = sheet.nodes[0].rect;
        let base = ((rect.width() - 96.0) / r.w as f32).min((rect.height() - 96.0) / r.h as f32);
        if response.has_focus() {
            for (key, delta) in [
                (egui::Key::ArrowLeft, vec2(24.0, 0.0)),
                (egui::Key::ArrowRight, vec2(-24.0, 0.0)),
                (egui::Key::ArrowUp, vec2(0.0, 24.0)),
                (egui::Key::ArrowDown, vec2(0.0, -24.0)),
            ] {
                if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key)) {
                    self.pan += delta;
                }
            }
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Home)) {
                self.pan = Vec2::ZERO;
                self.zoom = 1.0;
            }
            painter.rect_stroke(
                rect.shrink(1.0),
                10,
                Stroke::new(2.0, p.accent),
                egui::StrokeKind::Inside,
            );
        }
        if response.hovered() || response.dragged() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let old = self.zoom;
                self.zoom = (self.zoom * (scroll * 0.002).exp()).clamp(0.2, 20.0);
                if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
                    let relative = pointer - rect.center();
                    self.pan = relative - (relative - self.pan) * (self.zoom / old);
                }
            }
            if response.dragged() {
                self.pan += response.drag_delta();
            }
        }
        let scale = base * self.zoom;
        let origin = rect.center() - vec2(r.w as f32, r.h as f32) * scale / 2.0 + self.pan;
        let to_rect = |r: Rect| {
            egui::Rect::from_min_size(
                origin + vec2(r.x as f32, r.y as f32) * scale,
                vec2(r.w as f32, r.h as f32) * scale,
            )
        };
        let board = to_rect(r);
        painter.rect_filled(board, 0, Color32::from_rgb(211, 220, 211));
        painter.rect_stroke(
            board,
            0,
            Stroke::new(1.0, p.muted),
            egui::StrokeKind::Inside,
        );
        for (index, node) in sheet.nodes.iter().enumerate() {
            let nrect = to_rect(node.rect);
            if !nrect.intersects(rect) {
                continue;
            }
            if let NodeKind::Part { id, copy, .. } = node.kind {
                let part_index = project.parts.iter().position(|p| p.id == id).unwrap();
                let colors = [
                    Color32::from_rgb(185, 223, 212),
                    Color32::from_rgb(189, 217, 235),
                    Color32::from_rgb(231, 216, 175),
                    Color32::from_rgb(215, 203, 229),
                    Color32::from_rgb(205, 221, 169),
                    Color32::from_rgb(239, 205, 183),
                ];
                painter.rect_filled(nrect, 1, colors[part_index % colors.len()]);
                painter.rect_stroke(
                    nrect,
                    1,
                    Stroke::new(0.8, Color32::from_rgb(67, 111, 91)),
                    egui::StrokeKind::Inside,
                );
                if nrect.width() > 55.0 && nrect.height() > 25.0 {
                    let clip = painter.with_clip_rect(nrect.shrink(4.0).intersect(rect));
                    let name = format!("{} #{}", project.parts[part_index].name, copy + 1);
                    clip.text(
                        nrect.center() - vec2(0.0, 6.0),
                        egui::Align2::CENTER_CENTER,
                        name,
                        FontId::proportional(13.0),
                        Color32::from_rgb(25, 49, 40),
                    );
                    if nrect.height() > 48.0 {
                        clip.text(
                            nrect.center() + vec2(0.0, 12.0),
                            egui::Align2::CENTER_CENTER,
                            size_label(project.unit, node.rect),
                            FontId::proportional(11.0),
                            Color32::from_rgb(45, 69, 59),
                        );
                    }
                }
            }
            if response.clicked()
                && !matches!(node.kind, NodeKind::Cut { .. })
                && response
                    .interact_pointer_pos()
                    .is_some_and(|point| nrect.contains(point))
            {
                self.selected_node = Some(index);
                self.cut = None;
            }
        }
        let steps = sheet.steps();
        let old_selection = (self.cut, self.selected_node);
        if response.clicked()
            && let Some(point) = response.interact_pointer_pos()
        {
            let hit = steps
                .iter()
                .enumerate()
                .filter_map(|(i, step)| {
                    let r = to_rect(step.rect);
                    let distance = match step.axis {
                        Axis::Vertical if (r.top() - 3.0..=r.bottom() + 3.0).contains(&point.y) => {
                            (point.x - r.left() - step.offset as f32 * scale).abs()
                        }
                        Axis::Horizontal
                            if (r.left() - 3.0..=r.right() + 3.0).contains(&point.x) =>
                        {
                            (point.y - r.top() - step.offset as f32 * scale).abs()
                        }
                        _ => return None,
                    };
                    if distance <= 5.0 {
                        Some((i, distance))
                    } else {
                        None
                    }
                })
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((i, _)) = hit {
                self.cut = Some(i);
                self.selected_node = None;
            }
        }
        if old_selection != (self.cut, self.selected_node) {
            ui.ctx().request_repaint();
        }
        for (i, step) in steps.iter().enumerate() {
            let sr = to_rect(step.rect);
            let (a, b) = match step.axis {
                Axis::Vertical => {
                    let x = sr.left() + step.offset as f32 * scale;
                    (egui::pos2(x, sr.top()), egui::pos2(x, sr.bottom()))
                }
                Axis::Horizontal => {
                    let y = sr.top() + step.offset as f32 * scale;
                    (egui::pos2(sr.left(), y), egui::pos2(sr.right(), y))
                }
            };
            if self.cut == Some(i) {
                painter.line_segment([a, b], Stroke::new(4.0, Color32::from_rgb(226, 143, 64)));
                painter.text(
                    a + vec2(8.0, 8.0),
                    egui::Align2::LEFT_TOP,
                    format!("{} {}", tr(lang, "step"), i + 1),
                    FontId::proportional(13.0),
                    p.ink,
                );
            }
        }
        if let Some(index) = self.selected_node {
            let selected = to_rect(sheet.nodes[index].rect);
            let alpha = ui.ctx().animate_bool_with_time(
                Id::new(("part_selection", index)),
                true,
                if self.preferences.reduced_motion {
                    0.0
                } else {
                    0.12
                },
            );
            painter.rect_stroke(
                selected,
                1,
                Stroke::new(3.0, Color32::from_rgb(226, 143, 64).gamma_multiply(alpha)),
                egui::StrokeKind::Inside,
            );
        }
        painter.text(
            egui::pos2(board.center().x, board.top() - 20.0),
            egui::Align2::CENTER_CENTER,
            project.unit.label(r.w),
            FontId::proportional(13.0),
            p.ink,
        );
        painter.text(
            egui::pos2(board.left() - 8.0, board.center().y),
            egui::Align2::RIGHT_CENTER,
            project.unit.label(r.h),
            FontId::proportional(13.0),
            p.ink,
        );
        let stock = project
            .stock
            .iter()
            .find(|s| s.id == sheet.stock_id)
            .unwrap();
        if stock.grain != Grain::None {
            painter.text(
                board.right_bottom() + vec2(0.0, 18.0),
                egui::Align2::RIGHT_CENTER,
                tr(lang, stock.grain.key()),
                FontId::proportional(12.0),
                p.muted,
            );
        }
    }

    fn settings(&mut self, ui: &mut egui::Ui, lang: &str) {
        let t = |key| tr(lang, key);
        ui.heading(t("settings"));
        ui.add_space(18.0);
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label(RichText::new(t("appearance")).size(19.0).strong());
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.preferences.theme, Theme::Light, t("light"));
                ui.selectable_value(&mut self.preferences.theme, Theme::Dark, t("dark"));
            });
            ui.add_space(10.0);
            ui.label(t("language"));
            let selected = LANGUAGES
                .iter()
                .find(|(code, _)| *code == self.preferences.language)
                .map(|(_, name)| *name)
                .unwrap_or("English");
            egui::ComboBox::from_id_salt("language")
                .selected_text(selected)
                .width(220.0)
                .show_ui(ui, |ui| {
                    for (code, name) in LANGUAGES {
                        ui.selectable_value(&mut self.preferences.language, code.to_string(), name);
                    }
                });
            ui.checkbox(&mut self.preferences.reduced_motion, t("reduced_motion"));
            ui.add_space(22.0);
            ui.separator();
            ui.add_space(12.0);
            ui.label(RichText::new(t("updates")).size(19.0).strong());
            ui.label(format!(
                "{}: {}",
                t("installed_version"),
                env!("CARGO_PKG_VERSION")
            ));
            ui.checkbox(
                &mut self.preferences.startup_update_check,
                t("startup_check"),
            );
            ui.label(RichText::new(t("update_note")).color(palette(self.preferences.theme).muted));
            ui.label(RichText::new(REPOSITORY).small());
            ui.add_space(10.0);
            match &self.update {
                UpdateState::Checking => {
                    ui.label(t("checking"));
                }
                UpdateState::Current => {
                    ui.label(t("up_to_date"));
                }
                UpdateState::Available(update) => {
                    ui.strong(format!("{}: {}", t("update_available"), update.version));
                    if ui.button(t("install_update")).clicked() {
                        let update = update.clone();
                        self.update = UpdateState::Downloading { fraction: 0.0 };
                        self.actions.push(Action::Download(update));
                    }
                }
                UpdateState::Downloading { fraction } => {
                    ui.label(t("downloading"));
                    ui.add(
                        egui::ProgressBar::new(*fraction)
                            .desired_width(300.0)
                            .show_percentage(),
                    );
                }
                UpdateState::Ready(_) => {
                    ui.label(t("update_ready"));
                    if ui.button(t("restart_update")).clicked() {
                        self.request(Pending::Install);
                    }
                }
                UpdateState::Installing => {
                    ui.label(t("installing"));
                }
                UpdateState::Failed(error) => {
                    ui.colored_label(Color32::from_rgb(195, 76, 59), t(error.key));
                    if !error.detail.is_empty() {
                        ui.label(RichText::new(&error.detail).small());
                    }
                }
                _ => {}
            }
            let checking = matches!(
                self.update,
                UpdateState::Checking | UpdateState::Downloading { .. } | UpdateState::Installing
            );
            if ui
                .add_enabled(!checking, egui::Button::new(t("check_updates")))
                .clicked()
            {
                self.update = UpdateState::Checking;
                self.actions.push(Action::CheckUpdate);
            }
            ui.add_space(24.0);
            egui::CollapsingHeader::new(t("licenses")).show(ui, |ui| {
                ui.label("Cut Planner · MIT · Copyright (c) 2026 siarheisinelnikau");
                if ui.button(t("export_licenses")).clicked() {
                    self.actions.push(Action::File(FileAction::Licenses));
                }
            });
        });
    }

    fn confirmation(&mut self, ctx: &egui::Context, lang: &str) {
        if self.pending.is_none() {
            return;
        }
        let t = |key| tr(lang, key);
        egui::Modal::new(Id::new("unsaved_confirmation")).show(ctx, |ui| {
            ui.set_max_width(490.0);
            ui.heading(t("unsaved"));
            ui.label(t("unsaved_help"));
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(
                        self.fields.invalid.is_empty() && self.project.validate().is_ok(),
                        egui::Button::new(t("save")),
                    )
                    .clicked()
                {
                    let after = self.pending.take();
                    self.actions.push(Action::Save {
                        after,
                        save_as: false,
                    });
                }
                if ui.button(t("discard")).clicked()
                    && let Some(pending) = self.pending.take()
                {
                    self.actions.push(Action::Continue(pending));
                }
                if ui.button(t("keep_editing")).clicked() {
                    self.pending = None;
                }
            });
        });
    }
    pub fn selected_sheet(&self) -> usize {
        self.sheet
    }
}

impl Fields {
    fn length(
        &mut self,
        ui: &mut egui::Ui,
        unit: Unit,
        value: &mut Length,
        key: impl std::hash::Hash,
        lang: &str,
    ) -> egui::Response {
        let id = Id::new(key);
        let text = self.text.entry(id).or_insert_with(|| unit.format(*value));
        let response = ui.add(egui::TextEdit::singleline(text).id(id).desired_width(92.0));
        if response.changed() {
            self.edited = true;
            match unit.parse(text) {
                Ok(parsed) => {
                    *value = parsed;
                    self.invalid.remove(&id);
                }
                Err(key) => {
                    self.invalid.insert(id, key);
                }
            }
        }
        if let Some(key) = self.invalid.get(&id) {
            ui.painter().rect_stroke(
                response.rect,
                3,
                Stroke::new(1.5, Color32::from_rgb(195, 76, 59)),
                egui::StrokeKind::Inside,
            );
            response.clone().on_hover_text(tr(lang, key));
        }
        response
    }
    fn quantity(
        &mut self,
        ui: &mut egui::Ui,
        value: &mut u32,
        key: impl std::hash::Hash,
        lang: &str,
    ) -> egui::Response {
        let id = Id::new(key);
        let text = self.text.entry(id).or_insert_with(|| value.to_string());
        let response = ui.add(egui::TextEdit::singleline(text).id(id).desired_width(50.0));
        if response.changed() {
            self.edited = true;
            match text.parse::<u32>() {
                Ok(parsed) if parsed > 0 => {
                    *value = parsed;
                    self.invalid.remove(&id);
                }
                _ => {
                    self.invalid.insert(id, "positive_quantity");
                }
            }
        }
        if let Some(key) = self.invalid.get(&id) {
            response.clone().on_hover_text(tr(lang, key));
        }
        response
    }

    fn remove_row(&mut self, kind: &str, row: u64) {
        for field in ["width", "height", "quantity"] {
            let id = Id::new((kind, row, field));
            self.text.remove(&id);
            self.invalid.remove(&id);
        }
    }
}

fn grain_picker(
    ui: &mut egui::Ui,
    value: &mut Grain,
    key: impl std::hash::Hash,
    lang: &str,
) -> egui::Response {
    egui::ComboBox::from_id_salt(key)
        .selected_text(tr(lang, value.key()))
        .width(140.0)
        .show_ui(ui, |ui| {
            for grain in Grain::ALL {
                ui.selectable_value(value, grain, tr(lang, grain.key()));
            }
        })
        .response
}

fn labelled_button(ui: &mut egui::Ui, text: &str, label: &str) -> egui::Response {
    let response = ui.button(text).on_hover_text(label);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    response
}
