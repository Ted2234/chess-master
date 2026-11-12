// In release builds on Windows, start without a console window behind the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;

/// The app icon (256x256), built into the program.
const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

fn main() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("chess-master")
        // Linux desktops match the window to its launcher entry (and its icon) by this id.
        .with_app_id("chess-master")
        .with_min_inner_size([800.0, 600.0]);
    // A broken icon is not worth refusing to start over.
    match app_icon() {
        Ok(icon) => viewport = viewport.with_icon(icon),
        Err(error) => eprintln!("could not load the app icon: {error}"),
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    chess_master::run(options)
}

/// Decodes the app icon for the window's title bar and the taskbar.
fn app_icon() -> Result<egui::IconData, image::ImageError> {
    let rgba = image::load_from_memory(ICON_PNG)?.to_rgba8();
    Ok(egui::IconData {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_decodes() {
        let icon = app_icon().expect("assets/icon.png should be a valid PNG");
        assert_eq!((icon.width, icon.height), (256, 256));
        assert_eq!(icon.rgba.len(), 256 * 256 * 4);
    }
}
