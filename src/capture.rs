//! Isolated, finite client-area capture for later authorized visual review.
//! This mode never initializes winit, dialogs, preferences, workers or network access.
use cut_planner::{
    model::Project,
    solver::{SolveOptions, solve},
    storage::{self, Preferences, Theme},
    ui::{Message, Stage, Workspace},
};
use egui_software_backend::{BufferMutRef, ColorFieldOrder, EguiSoftwareRender};
use std::{
    path::PathBuf,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

pub fn run() -> Result<(), String> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--capture")) {
        return Err("expected --capture".into());
    }
    let path = PathBuf::from(
        args.next()
            .ok_or("--capture requires an absolute PNG output path")?,
    );
    if !path.is_absolute() {
        return Err("capture output must be an absolute path".into());
    }
    let mut language = "en".to_owned();
    let mut theme = Theme::Light;
    let mut stage = Stage::Plan;
    let mut logical_width = 1280_u32;
    let mut logical_height = 850_u32;
    let mut scale = 1.0_f32;
    let mut scenario = "populated".to_owned();
    while let Some(argument) = args.next() {
        let value = args
            .next()
            .ok_or("missing capture option value")?
            .to_string_lossy()
            .to_string();
        match argument.to_str().ok_or("invalid capture option")? {
            "--screen" => {
                stage = match value.as_str() {
                    "inputs" => Stage::Inputs,
                    "constraints" => Stage::Constraints,
                    "plan" => Stage::Plan,
                    "settings" => Stage::Settings,
                    _ => return Err("unknown screen".into()),
                }
            }
            "--language" => {
                if !cut_planner::i18n::LANGUAGES
                    .iter()
                    .any(|(code, _)| *code == value)
                {
                    return Err("unknown language".into());
                }
                language = value;
            }
            "--theme" => {
                theme = match value.as_str() {
                    "light" => Theme::Light,
                    "dark" => Theme::Dark,
                    _ => return Err("unknown theme".into()),
                }
            }
            "--width" => logical_width = value.parse().map_err(|_| "invalid width")?,
            "--height" => logical_height = value.parse().map_err(|_| "invalid height")?,
            "--scale" => scale = value.parse().map_err(|_| "invalid scale")?,
            "--scenario" => {
                if !["populated", "empty", "error"].contains(&value.as_str()) {
                    return Err("unknown scenario".into());
                }
                scenario = value;
            }
            _ => return Err("unknown capture option".into()),
        }
    }
    if !(960..=2560).contains(&logical_width)
        || !(640..=1600).contains(&logical_height)
        || !scale.is_finite()
        || !(1.0..=2.0).contains(&scale)
    {
        return Err("capture viewport is outside supported bounds".into());
    }
    let started = Instant::now();
    let mut workspace = Workspace::new(Preferences {
        language,
        theme,
        startup_update_check: false,
        reduced_motion: true,
    });
    if scenario != "empty" {
        workspace.replace_project(None, Project::example(), false);
        if stage == Stage::Plan {
            let result = solve(
                &workspace.project,
                SolveOptions::default(),
                &AtomicBool::new(false),
                |_, _| {},
            );
            workspace.handle(Message::Solved {
                generation: workspace.generation,
                result,
            });
        }
    }
    workspace.stage = stage;
    if scenario == "error" {
        workspace.notify_error("positive_size".into());
    }
    let ctx = egui::Context::default();
    let (width, height) = (
        (logical_width as f32 * scale).round() as u32,
        (logical_height as f32 * scale).round() as u32,
    );
    let mut pixels = vec![[0, 0, 0, 255]; (width * height) as usize];
    let mut renderer = EguiSoftwareRender::new(ColorFieldOrder::Bgra);
    // Egui may need a sizing pass and a follow-up font atlas upload. Four settled frames are a hard bound.
    for frame in 0..4 {
        if started.elapsed() > Duration::from_secs(15) {
            return Err("capture exceeded the 15 second readiness deadline".into());
        }
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(logical_width as f32, logical_height as f32),
            )),
            time: Some(frame as f64 * 0.25),
            focused: false,
            ..Default::default()
        };
        input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(scale);
        let output = ctx.run(input, |ctx| workspace.show(ctx));
        let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
        renderer.render(
            &mut BufferMutRef::new(&mut pixels, width as usize, height as usize),
            &primitives,
            &output.textures_delta,
            output.pixels_per_point,
        );
        // Ignore actions deliberately: no saves, dialogs, updater checks or workers in capture mode.
        workspace.take_actions();
    }
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for pixel in pixels {
        rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], 255]);
    }
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(&rgba).map_err(|e| e.to_string())?;
    }
    storage::atomic_write(&path, &bytes)?;
    Ok(())
}
