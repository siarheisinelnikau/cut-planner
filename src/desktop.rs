use cut_planner::{
    csv, export, icon,
    model::Project,
    scheduler::Scheduler,
    solver::{SolveOptions, solve},
    storage,
    ui::{Action, FileAction, Message, Pending, UpdateState, Workspace},
    updates,
};
use egui_software_backend::{BufferMutRef, ColorFieldOrder, EguiSoftwareRender};
use serde::Serialize;
use std::{
    num::NonZeroU32,
    path::PathBuf,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Instant,
};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy, OwnedDisplayHandle},
    window::{Icon, Window, WindowId},
};

enum Event {
    Message(Box<Message>),
    Accessibility(egui_winit::accesskit_winit::Event),
}
impl From<egui_winit::accesskit_winit::Event> for Event {
    fn from(event: egui_winit::accesskit_winit::Event) -> Self {
        Self::Accessibility(event)
    }
}
fn send(proxy: &EventLoopProxy<Event>, message: Message) {
    let _ = proxy.send_event(Event::Message(Box::new(message)));
}

#[derive(Default, Serialize)]
struct Metrics {
    backend: &'static str,
    scene_runs: u64,
    egui_passes: u64,
    rasterizations: u64,
    presentations: u64,
    cached_presentations: u64,
    hidden_redraws_skipped: u64,
    input_invalidations: u64,
    worker_invalidations: u64,
    deadline_invalidations: u64,
    layout_ns: u64,
    raster_ns: u64,
    present_ns: u64,
    max_frame_ns: u64,
}

struct SettingsQueue {
    next: Option<storage::Preferences>,
    stop: bool,
}
struct SettingsWriter {
    shared: Arc<(Mutex<SettingsQueue>, Condvar)>,
    handle: Option<thread::JoinHandle<()>>,
}
impl SettingsWriter {
    fn new(path: PathBuf, proxy: EventLoopProxy<Event>) -> Result<Self, String> {
        let shared = Arc::new((
            Mutex::new(SettingsQueue {
                next: None,
                stop: false,
            }),
            Condvar::new(),
        ));
        let worker = shared.clone();
        let handle = thread::Builder::new()
            .name("preferences".into())
            .spawn(move || {
                loop {
                    let (mutex, condition) = &*worker;
                    let mut state = mutex.lock().unwrap_or_else(|e| e.into_inner());
                    while state.next.is_none() && !state.stop {
                        state = condition.wait(state).unwrap_or_else(|e| e.into_inner());
                    }
                    let Some(next) = state.next.take() else {
                        break;
                    };
                    drop(state);
                    send(
                        &proxy,
                        Message::PreferencesSaved(storage::save_preferences(&path, &next)),
                    );
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            shared,
            handle: Some(handle),
        })
    }
    fn save(&self, preferences: storage::Preferences) {
        let (mutex, condition) = &*self.shared;
        mutex.lock().unwrap_or_else(|e| e.into_inner()).next = Some(preferences);
        condition.notify_one();
    }
}
impl Drop for SettingsWriter {
    fn drop(&mut self) {
        let (mutex, condition) = &*self.shared;
        mutex.lock().unwrap_or_else(|e| e.into_inner()).stop = true;
        condition.notify_one();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

struct Desktop {
    ctx: egui::Context,
    proxy: EventLoopProxy<Event>,
    workspace: Workspace,
    window: Option<Arc<Window>>,
    input: Option<egui_winit::State>,
    surface: Option<softbuffer::Surface<OwnedDisplayHandle, Arc<Window>>>,
    renderer: EguiSoftwareRender,
    pixels: Vec<[u8; 4]>,
    dimensions: (u32, u32),
    scheduler: Scheduler,
    settings: Option<SettingsWriter>,
    preferences_path: Result<PathBuf, String>,
    metrics: Metrics,
    diagnostic_path: Option<PathBuf>,
    reopen: Option<PathBuf>,
    download_cancel: Arc<AtomicBool>,
}

pub fn run() -> Result<(), String> {
    let preferences_path = storage::preferences_path();
    let loaded = preferences_path
        .as_ref()
        .map_err(Clone::clone)
        .and_then(|p| storage::load_preferences(p));
    let mut workspace = Workspace::new(loaded.clone().unwrap_or_default());
    if let Err(error) = loaded {
        workspace.notify_error(error);
    }
    let event_loop = EventLoop::<Event>::with_user_event()
        .build()
        .map_err(|e| e.to_string())?;
    let proxy = event_loop.create_proxy();
    let settings = preferences_path.as_ref().ok().and_then(|p| {
        match SettingsWriter::new(p.clone(), proxy.clone()) {
            Ok(writer) => Some(writer),
            Err(error) => {
                workspace.notify_error(error);
                None
            }
        }
    });
    let mut diagnostic_path = None;
    let mut reopen = None;
    let mut update_error = None;
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--diagnostics" {
            diagnostic_path = Some(PathBuf::from(
                args.next().ok_or("--diagnostics requires a path")?,
            ));
        } else if arg == "--project" {
            reopen = Some(PathBuf::from(
                args.next().ok_or("--project requires a path")?,
            ));
        } else if arg == "--update-error" {
            update_error = Some(PathBuf::from(
                args.next().ok_or("--update-error requires a path")?,
            ));
        } else {
            return Err(format!("unknown argument: {}", arg.to_string_lossy()));
        }
    }
    if let Some(path) = update_error {
        let detail = storage::read_text(&path).unwrap_or_else(|e| e);
        workspace.notify_error(format!(
            "{} {detail}",
            cut_planner::i18n::tr(&workspace.preferences.language, "update_failed")
        ));
        workspace.stage = cut_planner::ui::Stage::Settings;
    }
    let mut desktop = Desktop {
        ctx: egui::Context::default(),
        proxy,
        workspace,
        window: None,
        input: None,
        surface: None,
        renderer: EguiSoftwareRender::new(ColorFieldOrder::Bgra),
        pixels: vec![],
        dimensions: (0, 0),
        scheduler: Scheduler::default(),
        settings,
        preferences_path,
        metrics: Metrics {
            backend: "egui_software_backend 0.0.2 / softbuffer 0.4.8 / Win32 GDI",
            ..Metrics::default()
        },
        diagnostic_path,
        reopen,
        download_cancel: Arc::new(AtomicBool::new(false)),
    };
    event_loop.run_app(&mut desktop).map_err(|e| e.to_string())
}

impl Desktop {
    fn invalidate(&mut self) {
        self.scheduler.invalidate();
        if self.scheduler.visible
            && let Some(window) = &self.window
        {
            window.request_redraw();
        }
    }
    fn spawn(&mut self, name: &str, job: impl FnOnce() -> Message + Send + 'static) {
        let proxy = self.proxy.clone();
        if let Err(error) = thread::Builder::new()
            .name(name.into())
            .spawn(move || send(&proxy, job()))
        {
            self.workspace.file_busy = false;
            self.workspace.solving = None;
            self.workspace.notify_error(error.to_string());
        }
    }
    fn dispatch(&mut self, event_loop: &ActiveEventLoop) {
        for action in self.workspace.take_actions() {
            match action {
                Action::Continue(pending) => match pending {
                    Pending::New => self.workspace.replace_project(None, Project::empty(), true),
                    Pending::Example => {
                        self.workspace
                            .replace_project(None, Project::example(), false)
                    }
                    Pending::Open => self.file(FileAction::Open),
                    Pending::Import => self.file(FileAction::Import),
                    Pending::Quit => {
                        self.workspace.cancel_solver();
                        self.download_cancel.store(true, Ordering::Relaxed);
                        event_loop.exit();
                    }
                    Pending::Install => {
                        if let UpdateState::Ready(update) = &self.workspace.update {
                            let update = update.clone();
                            let reopen = self.workspace.path.clone();
                            self.workspace.file_busy = true;
                            self.workspace.update = UpdateState::Installing;
                            self.spawn("prepare-installation", move || {
                                let result = std::env::current_exe()
                                    .map_err(|e| updates::UpdateError::new("update_failed", e))
                                    .and_then(|target| {
                                        updates::prepare_ticket(&update, &target, reopen)
                                    })
                                    .and_then(|ticket| updates::launch_helper(&ticket));
                                Message::InstallerPrepared(result)
                            });
                        }
                    }
                },
                Action::Save { after, save_as } => self.save(after, save_as),
                Action::File(action) => self.file(action),
                Action::Solve {
                    project,
                    generation,
                    cancel,
                } => {
                    let progress = self.proxy.clone();
                    self.spawn("guillotine-solver", move || {
                        let result = std::panic::catch_unwind(|| {
                            solve(&project, SolveOptions::default(), &cancel, |done, total| {
                                send(
                                    &progress,
                                    Message::Progress {
                                        generation,
                                        done,
                                        total,
                                    },
                                )
                            })
                        })
                        .unwrap_or_else(|_| {
                            Err(cut_planner::solver::SolveError::InvalidPlan(
                                "solver worker panicked".into(),
                            ))
                        });
                        Message::Solved { generation, result }
                    });
                }
                Action::SavePreferences(preferences) => {
                    if let Some(writer) = &self.settings {
                        writer.save(preferences);
                    } else if let Err(error) = &self.preferences_path {
                        self.workspace.notify_error(error.clone());
                    }
                }
                Action::CheckUpdate => self.spawn("update-check", || {
                    Message::Checked(updates::check(
                        &updates::HttpsTransport,
                        env!("CARGO_PKG_VERSION"),
                    ))
                }),
                Action::Download(update) => {
                    let cache = self
                        .preferences_path
                        .as_ref()
                        .map(|p| p.parent().unwrap().join("updates"))
                        .map_err(Clone::clone);
                    let proxy = self.proxy.clone();
                    let cancel = self.download_cancel.clone();
                    self.spawn("update-download", move || {
                        Message::Downloaded(match cache {
                            Ok(cache) => {
                                let mut last = 101_u64;
                                updates::download(
                                    &updates::HttpsTransport,
                                    &update,
                                    &cache,
                                    &cancel,
                                    |read, total| {
                                        let percent = read.saturating_mul(100) / total.max(1);
                                        if percent != last {
                                            last = percent;
                                            send(&proxy, Message::DownloadProgress { read, total });
                                        }
                                    },
                                )
                            }
                            Err(error) => Err(updates::UpdateError::new("update_failed", error)),
                        })
                    });
                }
                Action::Quit => event_loop.exit(),
            }
        }
    }
    fn save(&mut self, after: Option<Pending>, save_as: bool) {
        if self.workspace.file_busy {
            return;
        }
        if let Err(error) = self.workspace.project.validate() {
            self.workspace.notify_error(error.to_string());
            return;
        }
        self.workspace.file_busy = true;
        let snapshot = self.workspace.project.clone();
        let current = if save_as {
            None
        } else {
            self.workspace.path.clone()
        };
        let language = self.workspace.preferences.language.clone();
        let owner = self.window.clone();
        self.spawn("save-project", move || {
            let result = (|| {
                let path = current.or_else(|| {
                    file_dialog(
                        owner.as_deref(),
                        cut_planner::i18n::tr(&language, "save_as"),
                    )
                    .add_filter("Cut Planner", &["cutplan"])
                    .set_file_name("project.cutplan")
                    .save_file()
                });
                if let Some(path) = path {
                    storage::save_project(&path, &snapshot)?;
                    Ok(Some(path))
                } else {
                    Ok(None)
                }
            })();
            Message::Saved {
                result,
                snapshot,
                after,
            }
        });
    }
    fn file(&mut self, action: FileAction) {
        if self.workspace.file_busy {
            return;
        }
        self.workspace.file_busy = true;
        let project = self.workspace.project.clone();
        let planned = self.workspace.planned.clone();
        let sheet = self.workspace.selected_sheet();
        let language = self.workspace.preferences.language.clone();
        let owner = self.window.clone();
        self.spawn("project-files", move || {
            let t = |key| cut_planner::i18n::tr(&language, key);
            match action {
                FileAction::Open => {
                    let Some(path) = file_dialog(owner.as_deref(), t("open"))
                        .add_filter("Cut Planner", &["cutplan"])
                        .pick_file()
                    else {
                        return Message::FileCancelled;
                    };
                    Message::Loaded(storage::load_project(&path).map(|project| (path, project)))
                }
                FileAction::Import => {
                    let Some(path) = file_dialog(owner.as_deref(), t("import_csv"))
                        .add_filter("CSV", &["csv"])
                        .pick_file()
                    else {
                        return Message::FileCancelled;
                    };
                    Message::Imported(
                        storage::read_text(&path)
                            .and_then(|text| csv::import_parts(&text, project.unit)),
                    )
                }
                _ => {
                    let result = (|| {
                        let (data, extension, title) = match action {
                            FileAction::Licenses => (
                                format!(
                                    "{}\n\n{}",
                                    include_str!("../LICENSE"),
                                    include_str!("../THIRD_PARTY_NOTICES.md")
                                ),
                                "txt",
                                "export_licenses",
                            ),
                            FileAction::ExportCsv => {
                                project.validate().map_err(|e| e.to_string())?;
                                (
                                    csv::export_parts(&project.parts, project.unit),
                                    "csv",
                                    "export_csv",
                                )
                            }
                            FileAction::Report => {
                                let (project, plan) = planned.as_ref().ok_or("empty_plan")?;
                                (
                                    export::html_report(project, plan, &language)?,
                                    "html",
                                    "export_report",
                                )
                            }
                            FileAction::Svg => {
                                let (project, plan) = planned.as_ref().ok_or("empty_plan")?;
                                (
                                    export::sheet_svg(project, plan, sheet, &language)?,
                                    "svg",
                                    "export_svg",
                                )
                            }
                            _ => unreachable!(),
                        };
                        let path = file_dialog(owner.as_deref(), t(title))
                            .add_filter(extension, &[extension])
                            .set_file_name(format!("cut-plan.{extension}"))
                            .save_file();
                        if let Some(path) = path {
                            storage::atomic_write(&path, data.as_bytes())?;
                            Ok(true)
                        } else {
                            Ok(false)
                        }
                    })();
                    Message::Exported(result)
                }
            }
        });
    }

    fn draw(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        if !self.scheduler.visible {
            self.metrics.hidden_redraws_skipped += 1;
            return Ok(());
        }
        let window = self.window.as_ref().ok_or("window unavailable")?.clone();
        if window.is_minimized() == Some(true) || window.is_visible() == Some(false) {
            self.scheduler.visibility(false);
            return Ok(());
        }
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            self.scheduler.visibility(false);
            return Ok(());
        }
        let count = u64::from(size.width) * u64::from(size.height);
        if count > 40_000_000 {
            return Err("window framebuffer exceeds 40 million pixels".into());
        }
        let frame_started = Instant::now();
        if self.dimensions != (size.width, size.height) {
            self.dimensions = (size.width, size.height);
            self.pixels.resize(count as usize, [0, 0, 0, 255]);
            self.scheduler.invalidate();
            self.surface
                .as_mut()
                .unwrap()
                .resize(
                    NonZeroU32::new(size.width).unwrap(),
                    NonZeroU32::new(size.height).unwrap(),
                )
                .map_err(|e| e.to_string())?;
        }
        let scene = self.scheduler.take_scene();
        if scene {
            let input = self.input.as_mut().unwrap();
            egui_winit::update_viewport_info(
                input
                    .egui_input_mut()
                    .viewports
                    .entry(egui::ViewportId::ROOT)
                    .or_default(),
                &self.ctx,
                &window,
                false,
            );
            let raw = input.take_egui_input(&window);
            let started = Instant::now();
            let before = self.ctx.cumulative_pass_nr();
            let output = self.ctx.run(raw, |ctx| self.workspace.show(ctx));
            self.metrics.scene_runs += 1;
            self.metrics.egui_passes += self.ctx.cumulative_pass_nr() - before;
            self.metrics.layout_ns += started.elapsed().as_nanos() as u64;
            let delay = output
                .viewport_output
                .get(&egui::ViewportId::ROOT)
                .map(|o| o.repaint_delay)
                .unwrap_or(std::time::Duration::MAX);
            self.scheduler.schedule(Instant::now(), delay);
            self.input
                .as_mut()
                .unwrap()
                .handle_platform_output(&window, output.platform_output);
            let primitives = self.ctx.tessellate(output.shapes, output.pixels_per_point);
            let started = Instant::now();
            self.renderer.render(
                &mut BufferMutRef::new(&mut self.pixels, size.width as usize, size.height as usize),
                &primitives,
                &output.textures_delta,
                output.pixels_per_point,
            );
            self.metrics.rasterizations += 1;
            self.metrics.raster_ns += started.elapsed().as_nanos() as u64;
        } else {
            self.metrics.cached_presentations += 1;
        }
        let started = Instant::now();
        let mut buffer = self
            .surface
            .as_mut()
            .unwrap()
            .buffer_mut()
            .map_err(|e| e.to_string())?;
        for (destination, pixel) in buffer.iter_mut().zip(&self.pixels) {
            *destination = u32::from_le_bytes(*pixel) & 0x00ff_ffff;
        }
        buffer.present().map_err(|e| e.to_string())?;
        self.metrics.presentations += 1;
        self.metrics.present_ns += started.elapsed().as_nanos() as u64;
        self.metrics.max_frame_ns = self
            .metrics
            .max_frame_ns
            .max(frame_started.elapsed().as_nanos() as u64);
        self.dispatch(event_loop);
        Ok(())
    }
}

impl ApplicationHandler<Event> for Desktop {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let created = (|| -> Result<(), String> {
            let icon = Icon::from_rgba(icon::rgba(64), 64, 64).map_err(|e| e.to_string())?;
            let window = Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_title("Cut Planner")
                            .with_inner_size(LogicalSize::new(1280, 850))
                            .with_min_inner_size(LogicalSize::new(960, 640))
                            .with_visible(false)
                            .with_window_icon(Some(icon)),
                    )
                    .map_err(|e| e.to_string())?,
            );
            let mut input = egui_winit::State::new(
                self.ctx.clone(),
                egui::ViewportId::ROOT,
                &window,
                Some(window.scale_factor() as f32),
                None,
                Some(4096),
            );
            // AccessKit must be initialized before the window first becomes visible.
            input.init_accesskit(event_loop, &window, self.proxy.clone());
            let context = softbuffer::Context::new(event_loop.owned_display_handle())
                .map_err(|e| e.to_string())?;
            let surface =
                softbuffer::Surface::new(&context, window.clone()).map_err(|e| e.to_string())?;
            self.input = Some(input);
            self.surface = Some(surface);
            self.window = Some(window.clone());
            window.set_visible(true);
            window.request_redraw();
            Ok(())
        })();
        if let Err(error) = created {
            eprintln!("Cut Planner: {error}");
            event_loop.exit();
            return;
        }
        if updates::should_check_at_startup(&self.workspace.preferences) {
            self.workspace.update = UpdateState::Checking;
            self.spawn("startup-update-check", || {
                Message::Checked(updates::check(
                    &updates::HttpsTransport,
                    env!("CARGO_PKG_VERSION"),
                ))
            });
        }
        if let Some(path) = self.reopen.take() {
            self.workspace.file_busy = true;
            self.spawn("reopen-project", move || {
                Message::Loaded(storage::load_project(&path).map(|project| (path, project)))
            });
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(window) = &self.window else {
            return;
        };
        if window.id() != id {
            return;
        }
        let repaint = self
            .input
            .as_mut()
            .unwrap()
            .on_window_event(window, &event)
            .repaint;
        match event {
            WindowEvent::CloseRequested => {
                if self.workspace.dirty() && !self.workspace.file_busy {
                    window.set_minimized(false);
                    window.set_visible(true);
                    self.scheduler.visibility(true);
                }
                self.workspace.request(Pending::Quit);
                self.dispatch(event_loop);
                self.invalidate();
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.draw(event_loop) {
                    self.workspace.notify_error(error);
                    self.scheduler.visibility(false);
                }
            }
            WindowEvent::Occluded(occluded) => {
                self.scheduler.visibility(!occluded);
                if !occluded {
                    self.invalidate();
                }
            }
            WindowEvent::Resized(size) => {
                self.scheduler.visibility(
                    size.width > 0 && size.height > 0 && window.is_minimized() != Some(true),
                );
                self.invalidate();
            }
            WindowEvent::ScaleFactorChanged { .. } => self.invalidate(),
            _ => {
                if repaint {
                    self.metrics.input_invalidations += 1;
                    self.invalidate();
                }
            }
        }
    }
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: Event) {
        match event {
            Event::Message(message) => {
                self.workspace.handle(*message);
                self.metrics.worker_invalidations += 1;
                self.dispatch(event_loop);
                self.invalidate();
            }
            Event::Accessibility(event) => {
                match event.window_event {
                    egui_winit::accesskit_winit::WindowEvent::InitialTreeRequested => {
                        self.ctx.enable_accesskit()
                    }
                    egui_winit::accesskit_winit::WindowEvent::ActionRequested(request) => self
                        .input
                        .as_mut()
                        .unwrap()
                        .on_accesskit_action_request(request),
                    egui_winit::accesskit_winit::WindowEvent::AccessibilityDeactivated => {}
                }
                self.invalidate();
            }
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.scheduler.wake_deadline(Instant::now()) {
            self.metrics.deadline_invalidations += 1;
        }
        if self.scheduler.visible
            && self.scheduler.dirty
            && let Some(window) = &self.window
        {
            window.request_redraw();
        }
        event_loop.set_control_flow(
            match self.scheduler.deadline.filter(|_| self.scheduler.visible) {
                Some(deadline) => ControlFlow::WaitUntil(deadline),
                None => ControlFlow::Wait,
            },
        );
    }
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.workspace.cancel_solver();
        self.download_cancel.store(true, Ordering::Relaxed);
        if let Some(path) = &self.diagnostic_path
            && let Ok(bytes) = serde_json::to_vec_pretty(&self.metrics)
        {
            let _ = storage::atomic_write(path, &bytes);
        }
    }
}

fn file_dialog(owner: Option<&Window>, title: &str) -> rfd::FileDialog {
    let dialog = rfd::FileDialog::new().set_title(title);
    if let Some(owner) = owner {
        dialog.set_parent(owner)
    } else {
        dialog
    }
}
