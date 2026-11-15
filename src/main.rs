#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod capture;
mod desktop;

fn main() {
    let mut args = std::env::args_os().skip(1);
    if std::env::args_os().nth(1).is_some_and(|a| a == "--capture") {
        if let Err(error) = capture::run() {
            eprintln!("Cut Planner capture: {error}");
            std::process::exit(1);
        }
        return;
    }
    if let Some(first) = args.next()
        && first == "--apply-update"
    {
        if let Some(manifest) = args.next() {
            let _ = cut_planner::updates::run_helper(std::path::Path::new(&manifest));
        }
        return;
    }
    if let Err(error) = desktop::run() {
        // No modal error dialogs or retries. Startup errors are also returned to CLI callers.
        eprintln!("Cut Planner: {error}");
        std::process::exit(1);
    }
}
