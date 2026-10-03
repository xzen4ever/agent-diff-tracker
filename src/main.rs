mod aggregator;
mod cli;
mod models;
mod scanner;
mod ui;

use eframe::egui;
use ui::app::DiffTrackApp;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    // If explicit --gui or -g is specified, or no arguments are provided, launch GUI mode
    let is_gui = args.len() == 1 || args.iter().any(|a| a == "--gui" || a == "-g");

    if !is_gui {
        // Run via CLI interface
        return cli::run_cli(&args[1..]);
    }

    // Configure NativeOptions for standard Windowed mode
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("DiffTrack - AI Code Change Workbench")
            .with_inner_size([1260.0, 800.0])
            .with_min_inner_size([960.0, 620.0])
            .with_resizable(true)
            .with_active(true),
        ..Default::default()
    };

    println!("============================================================");
    println!("  DiffTrack - AI Code Change Workbench (Windowed Mode)");
    println!("  Scanning logs from Antigravity, Claude Code, Codex, Gemini");
    println!("============================================================");

    if let Err(e) = eframe::run_native(
        "DiffTrack",
        options,
        Box::new(|cc| Ok(Box::new(DiffTrackApp::new(cc)))),
    ) {
        eprintln!("Failed to launch GUI: {}", e);
    }

    Ok(())
}
