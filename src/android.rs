//! Everything that only exists on Android: where the app starts, and keeping it clear of
//! the status bar and the navigation bar.

use eframe::egui::{self, SafeAreaInsets, epaint::MarginF32};
use winit::platform::android::activity::AndroidApp;

use crate::settings;
use crate::ui::app::ChessApp;

/// Where Android starts the app, instead of `main`.
#[unsafe(no_mangle)]
fn android_main(android: AndroidApp) {
    // Android apps get no home folder; the settings go into the app's private folder.
    if let Some(folder) = android.internal_data_path() {
        settings::use_folder(folder);
    }
    let options = eframe::NativeOptions {
        android_app: Some(android.clone()),
        ..Default::default()
    };
    let result = eframe::run_native(
        "chess-master",
        options,
        Box::new(|cc| {
            let chess = ChessApp::new(cc)?;
            Ok(Box::new(PhoneApp { chess, android }))
        }),
    );
    if let Err(error) = result {
        eprintln!("chess-master stopped: {error}");
    }
    // Android may call `android_main` again in this same process later, but the window
    // system can only be set up once per process. Ending the process avoids that.
    std::process::exit(0);
}

/// The chess app, plus the Android activity it runs in.
struct PhoneApp {
    chess: ChessApp,
    android: AndroidApp,
}

impl eframe::App for PhoneApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        // Draw the app only in the part of the screen the system bars leave free.
        let free = ui.ctx().content_rect();
        ui.scope_builder(egui::UiBuilder::new().max_rect(free), |ui| {
            self.chess.ui(ui, frame);
        });
    }

    /// Runs before every frame: turns Back into `Esc`, and tells egui which edges of the
    /// screen the system bars cover.
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        // The phone's Back button works like `Esc` on a computer: it closes the Settings
        // screen and pop-ups.
        for event in &mut raw_input.events {
            if let egui::Event::Key { key, .. } = event
                && *key == egui::Key::BrowserBack
            {
                *key = egui::Key::Escape;
            }
        }

        // `None` means "same size as last frame"; egui then keeps the last insets too.
        let Some(screen) = raw_input.screen_rect else {
            return;
        };
        // Android measures in pixels, egui in points.
        let pixels_per_point = raw_input.viewport().native_pixels_per_point.unwrap_or(1.0);
        let screen = screen.size() * pixels_per_point;
        // The part of the window not covered by system bars, in pixels.
        let content = self.android.content_rect();
        if content.right <= content.left || content.bottom <= content.top {
            return; // not known yet
        }

        let insets = MarginF32 {
            left: content.left as f32,
            top: content.top as f32,
            right: (screen.x - content.right as f32).max(0.0),
            bottom: (screen.y - content.bottom as f32).max(0.0),
        };
        raw_input.safe_area_insets = Some(SafeAreaInsets(insets / pixels_per_point));
    }
}
