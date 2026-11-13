//! Asks GitHub whether a newer version of chess-master has been released.
//!
//! The check runs once, in the background, with the `curl` program that comes with Linux and
//! Windows 10+. Anything that goes wrong (offline, no curl, no release yet) just means no notice.

use std::fmt;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::sync::mpsc::{self, Receiver};
use std::thread;

/// Where people download a new version.
pub const RELEASES_PAGE: &str = "https://github.com/Ted2234/chess-master/releases/latest";

/// GitHub's API for the newest release; it answers with JSON.
const LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/Ted2234/chess-master/releases/latest";

/// A version number like `0.2.0`: major, minor and patch.
///
/// The derived `PartialOrd`/`Ord` compare the fields in the order they're written: first
/// `major`, then `minor`, then `patch`. That's exactly how versions compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

/// A text that isn't a version like `1.2.3` or `v1.2.3`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseVersionError(String);

impl fmt::Display for ParseVersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "'{}' is not a version like 1.2.3", self.0)
    }
}

impl std::error::Error for ParseVersionError {}

impl FromStr for Version {
    type Err = ParseVersionError;

    /// Reads `1.2.3`, or `v1.2.3` as written in git tags.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let error = || ParseVersionError(text.to_string());
        let digits = text.strip_prefix('v').unwrap_or(text);
        let mut parts = digits.split('.').map(|part| part.parse::<u32>());
        let (Some(Ok(major)), Some(Ok(minor)), Some(Ok(patch)), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(error());
        };
        Ok(Version {
            major,
            minor,
            patch,
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Returns the version of this program, from `Cargo.toml`.
pub fn current() -> Option<Version> {
    env!("CARGO_PKG_VERSION").parse().ok()
}

/// Finds the value of `"tag_name"` in GitHub's JSON answer, e.g. `v0.3.0`.
///
/// A real JSON parser would be more robust, but this one field is all we need, and its value
/// (a git tag) never contains a `"`.
fn tag_name(json: &str) -> Option<&str> {
    const KEY: &str = "\"tag_name\"";
    let after_key = &json[json.find(KEY)? + KEY.len()..];
    let value = after_key
        .trim_start()
        .strip_prefix(':')?
        .trim_start()
        .strip_prefix('"')?;
    value.split('"').next()
}

/// Returns the released version if it's newer than `current`.
pub fn newer_version(json: &str, current: Version) -> Option<Version> {
    let released: Version = tag_name(json)?.parse().ok()?;
    (released > current).then_some(released)
}

/// Starts the check on its own thread and returns at once.
///
/// The receiver gets one message, a newer version, if there is one. If there isn't, or the
/// check fails, the thread ends without a message. On Android there's no check at all: apps
/// there are updated from the APK.
pub fn check_in_background() -> Receiver<Version> {
    let (sender, receiver) = mpsc::channel();
    if !cfg!(target_os = "android") {
        thread::spawn(move || {
            if let Some(json) = latest_release_json()
                && let Some(current) = current()
                && let Some(version) = newer_version(&json, current)
            {
                // If the window is already closed, nobody needs to know.
                let _ = sender.send(version);
            }
        });
    }
    receiver
}

/// Downloads GitHub's answer with `curl`. `None` if anything goes wrong.
fn latest_release_json() -> Option<String> {
    let mut command = Command::new("curl");
    command
        .args(["--silent", "--fail", "--location", "--max-time", "5"])
        .args(["--header", "Accept: application/vnd.github+json"])
        .arg(LATEST_RELEASE_API)
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    crate::engine::hide_console_window(&mut command);
    let output = command.output().ok()?;
    if !output.status.success() {
        return None; // e.g. 404: no release yet
    }
    String::from_utf8(output.stdout).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(text: &str) -> Version {
        text.parse().unwrap()
    }

    #[test]
    fn versions_parse_with_or_without_v() {
        let expected = Version {
            major: 1,
            minor: 12,
            patch: 3,
        };
        assert_eq!(version("1.12.3"), expected);
        assert_eq!(version("v1.12.3"), expected);
        assert_eq!(expected.to_string(), "1.12.3");
        for bad in ["", "1.2", "1.2.3.4", "1.x.3", "v", "sf_19", "-1.2.3"] {
            assert!(bad.parse::<Version>().is_err(), "{bad} should be invalid");
        }
    }

    #[test]
    fn versions_compare_field_by_field() {
        assert!(version("0.10.0") > version("0.9.9"));
        assert!(version("1.0.0") > version("0.99.99"));
        assert!(version("0.2.1") > version("0.2.0"));
        assert_eq!(
            version("0.2.0").cmp(&version("0.2.0")),
            std::cmp::Ordering::Equal
        );
    }

    #[test]
    fn the_running_version_is_valid() {
        assert!(current().is_some());
    }

    #[test]
    fn newer_releases_are_found_in_the_json() {
        // Shortened from a real answer of GitHub's API.
        let json = r#"{"url":"...","html_url":"...","tag_name": "v0.3.0","name":"chess-master 0.3.0","draft":false}"#;
        assert_eq!(
            newer_version(json, version("0.2.0")),
            Some(version("0.3.0"))
        );
        assert_eq!(newer_version(json, version("0.3.0")), None);
        assert_eq!(newer_version(json, version("1.0.0")), None);
        assert_eq!(
            newer_version(r#"{"message": "Not Found"}"#, version("0.2.0")),
            None
        );
        assert_eq!(
            newer_version(r#"{"tag_name": "nightly"}"#, version("0.2.0")),
            None
        );
    }
}
