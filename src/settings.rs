use std::env;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The colors of the board squares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardTheme {
    Green,
    Wood,
    Ice,
    Slate,
}

impl BoardTheme {
    /// Every theme, in the order shown on the Settings screen.
    pub const ALL: [BoardTheme; 4] = [
        BoardTheme::Green,
        BoardTheme::Wood,
        BoardTheme::Ice,
        BoardTheme::Slate,
    ];

    /// Returns the name shown on the Settings screen and written to the settings file.
    pub fn name(self) -> &'static str {
        match self {
            BoardTheme::Green => "Green",
            BoardTheme::Wood => "Wood",
            BoardTheme::Ice => "Ice",
            BoardTheme::Slate => "Slate",
        }
    }

    /// Returns the theme with this name, or `None` if there is none.
    pub fn from_name(name: &str) -> Option<BoardTheme> {
        BoardTheme::ALL
            .into_iter()
            .find(|theme| theme.name() == name)
    }
}

/// Dark or light colors for the whole app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppTheme {
    Dark,
    Light,
}

impl AppTheme {
    /// Both themes, in the order shown on the Settings screen.
    pub const ALL: [AppTheme; 2] = [AppTheme::Dark, AppTheme::Light];

    /// Returns the name shown on the Settings screen and written to the settings file.
    pub fn name(self) -> &'static str {
        match self {
            AppTheme::Dark => "Dark",
            AppTheme::Light => "Light",
        }
    }

    /// Returns the theme with this name, or `None` if there is none.
    pub fn from_name(name: &str) -> Option<AppTheme> {
        AppTheme::ALL.into_iter().find(|theme| theme.name() == name)
    }
}

/// Everything chosen on the Settings screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    pub board_theme: BoardTheme,
    pub app_theme: AppTheme,
    /// Slide pieces to their new square instead of making them jump.
    pub animate_moves: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            board_theme: BoardTheme::Ice,
            app_theme: AppTheme::Dark,
            animate_moves: true,
        }
    }
}

/// Why the settings could not be saved.
#[derive(Debug)]
pub enum SettingsError {
    /// Neither `XDG_CONFIG_HOME`, `APPDATA` nor `HOME` is set.
    NoConfigFolder,
    /// Creating the folder or writing the file failed.
    Io(io::Error),
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SettingsError::NoConfigFolder => write!(f, "no folder found for the settings file"),
            SettingsError::Io(error) => write!(f, "could not save the settings: {error}"),
        }
    }
}

impl std::error::Error for SettingsError {}

impl From<io::Error> for SettingsError {
    fn from(error: io::Error) -> Self {
        SettingsError::Io(error)
    }
}

impl Settings {
    /// Loads the settings file. A missing or unreadable file gives the default settings.
    pub fn load() -> Self {
        settings_path()
            .map(|path| Self::read_from(&path))
            .unwrap_or_default()
    }

    /// Writes the settings file, creating its folder if needed.
    pub fn save(self) -> Result<(), SettingsError> {
        let path = settings_path().ok_or(SettingsError::NoConfigFolder)?;
        self.write_to(&path)
    }

    /// Reads settings from a file. A missing or unreadable file gives the default settings.
    fn read_from(path: &Path) -> Self {
        match fs::read_to_string(path) {
            Ok(text) => Self::from_text(&text),
            Err(_) => Self::default(),
        }
    }

    /// Writes the settings to a file, creating its folder if needed.
    fn write_to(self, path: &Path) -> Result<(), SettingsError> {
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder)?;
        }
        fs::write(path, self.to_text())?;
        Ok(())
    }

    /// Returns the settings as text: one `key = value` line per setting.
    pub fn to_text(self) -> String {
        format!(
            "board_theme = {}\napp_theme = {}\nanimate_moves = {}\n",
            self.board_theme.name(),
            self.app_theme.name(),
            self.animate_moves
        )
    }

    /// Reads settings from text made by `to_text`.
    ///
    /// Never fails: a missing line or a bad value keeps the default, and unknown keys are
    /// skipped. That way a file from an older or newer version of the app still loads.
    pub fn from_text(text: &str) -> Self {
        let mut settings = Settings::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue; // not a `key = value` line
            };
            let value = value.trim();
            match key.trim() {
                "board_theme" => {
                    if let Some(theme) = BoardTheme::from_name(value) {
                        settings.board_theme = theme;
                    }
                }
                "app_theme" => {
                    if let Some(theme) = AppTheme::from_name(value) {
                        settings.app_theme = theme;
                    }
                }
                "animate_moves" => {
                    if let Ok(on) = value.parse() {
                        settings.animate_moves = on;
                    }
                }
                _ => {} // a key this version doesn't know
            }
        }
        settings
    }
}

/// Returns where the settings file lives, for example
/// `~/.config/chess-master/settings.txt` on Linux or
/// `C:\Users\<name>\AppData\Roaming\chess-master\settings.txt` on Windows.
fn settings_path() -> Option<PathBuf> {
    let config_folder = env_path("XDG_CONFIG_HOME")
        .or_else(|| env_path("APPDATA"))
        .or_else(|| env_path("HOME").map(|home| home.join(".config")))?;
    Some(config_folder.join("chess-master").join("settings.txt"))
}

/// Reads an environment variable as a path. Unset or empty gives `None`.
fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_names_round_trip() {
        for theme in BoardTheme::ALL {
            assert_eq!(BoardTheme::from_name(theme.name()), Some(theme));
        }
        for theme in AppTheme::ALL {
            assert_eq!(AppTheme::from_name(theme.name()), Some(theme));
        }
        assert_eq!(BoardTheme::from_name("Purple"), None);
    }

    #[test]
    fn text_round_trip() {
        let settings = Settings {
            board_theme: BoardTheme::Wood,
            app_theme: AppTheme::Light,
            animate_moves: false,
        };
        assert_eq!(Settings::from_text(&settings.to_text()), settings);
    }

    #[test]
    fn bad_lines_and_unknown_keys_are_skipped() {
        let text = "hello\nsound = loud\nboard_theme = Purple\nanimate_moves = maybe\n";
        assert_eq!(Settings::from_text(text), Settings::default());

        let text = "  board_theme=Slate  \n";
        assert_eq!(Settings::from_text(text).board_theme, BoardTheme::Slate);
    }

    #[test]
    fn file_round_trip() {
        let path = env::temp_dir()
            .join("chess-master-test")
            .join("settings.txt");
        let settings = Settings {
            board_theme: BoardTheme::Green,
            app_theme: AppTheme::Dark,
            animate_moves: true,
        };
        settings.write_to(&path).unwrap();
        assert_eq!(Settings::read_from(&path), settings);

        fs::remove_file(&path).unwrap();
        assert_eq!(Settings::read_from(&path), Settings::default());
    }
}
