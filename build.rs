include!("build/winres.rs");

fn main() {
    println!("cargo:rerun-if-changed=build/winres.rs");
    embed_windows_resources("Countdown", Path::new("assets/icon/countdown.ico"));
}
