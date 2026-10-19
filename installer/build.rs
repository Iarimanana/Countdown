include!("../build/winres.rs");

use std::io::Write;

/// Finds the already-built app for the target being built, so it can be
/// embedded. Override with COUNTDOWN_APP_EXE.
fn find_app() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("COUNTDOWN_APP_EXE") {
        return Some(PathBuf::from(path));
    }
    let target = std::env::var("TARGET").unwrap();
    let host = std::env::var("HOST").unwrap();
    let exe = if target.contains("windows") { "countdown.exe" } else { "countdown" };
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let target_dir = std::env::var("CARGO_TARGET_DIR").map(PathBuf::from).unwrap_or_else(|_| manifest_dir.join("../target"));
    let mut candidates = vec![target_dir.join(&target).join("release").join(exe)];
    if target == host {
        candidates.push(target_dir.join("release").join(exe));
    }
    candidates.into_iter().find(|p| p.is_file())
}

fn main() {
    println!("cargo:rerun-if-changed=../build/winres.rs");
    println!("cargo:rerun-if-env-changed=COUNTDOWN_APP_EXE");
    embed_windows_resources("Installation de Countdown", Path::new("../assets/icon/countdown-setup.ico"));

    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("app.deflate");
    let app = match find_app() {
        Some(path) => {
            println!("cargo:rerun-if-changed={}", path.display());
            std::fs::read(&path).unwrap()
        }
        None if std::env::var("PROFILE").as_deref() == Ok("release") => panic!(
            "\n\nBuild the Countdown app before the installer:\n    \
             cargo build --release -p countdown --target {target}\n\
             (or set COUNTDOWN_APP_EXE to the app's path)\n",
            target = std::env::var("TARGET").unwrap()
        ),
        None => {
            // Debug builds and tests don't need the real app.
            println!("cargo:warning=Countdown app not built yet: the installer embeds a placeholder.");
            b"placeholder: build the app first".to_vec()
        }
    };
    let mut encoder = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::best());
    encoder.write_all(&app).unwrap();
    std::fs::write(out, encoder.finish().unwrap()).unwrap();
}
