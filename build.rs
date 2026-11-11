//! The build script: Cargo compiles and runs it on this computer before compiling the app.
//! For Windows it builds the `.exe`'s icon and version details into the program.

use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() -> Result<(), Box<dyn Error>> {
    // Run this script again only when these change, not after every change to the code.
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=assets/icon.ico");
    println!("cargo::rerun-if-env-changed=WINDRES");

    // The system we build *for*. `cfg!(windows)` would tell us the system this script
    // runs *on*, which is Linux even when cross-compiling.
    if env::var("CARGO_CFG_TARGET_OS")? == "windows" {
        windows_resources()?;
    }
    Ok(())
}

/// Compiles a Windows resource file (icon + version details) and links it into the `.exe`.
fn windows_resources() -> Result<(), Box<dyn Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let icon = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?).join("assets/icon.ico");
    let version = env::var("CARGO_PKG_VERSION")?;
    // Windows wants the version as four numbers separated by commas: 0,2,0,0.
    let numbers = format!(
        "{},{},{},0",
        env::var("CARGO_PKG_VERSION_MAJOR")?,
        env::var("CARGO_PKG_VERSION_MINOR")?,
        env::var("CARGO_PKG_VERSION_PATCH")?,
    );

    let script = format!(
        r#"1 ICON "{icon}"
1 VERSIONINFO
FILEVERSION {numbers}
PRODUCTVERSION {numbers}
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "FileDescription", "chess-master"
      VALUE "ProductName", "chess-master"
      VALUE "FileVersion", "{version}"
      VALUE "ProductVersion", "{version}"
      VALUE "LegalCopyright", "GPL-3.0-or-later"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#,
        icon = icon.display(),
    );
    let rc_file = out_dir.join("chess-master.rc");
    let object = out_dir.join("chess-master-resources.o");
    fs::write(&rc_file, script)?;

    // windres (from mingw-w64) turns the script into an object file the linker understands.
    let windres = env::var("WINDRES").unwrap_or_else(|_| "x86_64-w64-mingw32-windres".to_string());
    let status = Command::new(&windres)
        .arg(&rc_file)
        .args(["--output-format", "coff", "--output"])
        .arg(&object)
        .status()
        .map_err(|error| {
            format!("could not run {windres}: {error} (is mingw-w64-gcc installed?)")
        })?;
    if !status.success() {
        return Err(format!("{windres} failed with {status}").into());
    }

    // Hand the object file to the linker, for the program (not for tests or the library).
    println!("cargo::rustc-link-arg-bins={}", object.display());
    Ok(())
}
