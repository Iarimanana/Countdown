// Shared by the app's and the installer's build scripts, pulled in with `include!`.
//
// Embeds the icon, the version information shown in the file's Properties,
// and a manifest into Windows builds. It never fails the build: without a
// resource compiler the program simply has the default icon.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Escapes a path for a string inside an .rc file.
fn rc_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "\\\\").replace('"', "\"\"")
}

/// `icon` is relative to the crate being built.
fn embed_windows_resources(description: &str, icon: &Path) {
    let icon = Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join(icon);
    let icon = icon.as_path();
    println!("cargo:rerun-if-changed={}", icon.display());
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let version = std::env::var("CARGO_PKG_VERSION").unwrap();
    let mut parts = version.split('.').map(|p| p.parse::<u16>().unwrap_or(0));
    let (major, minor, patch) = (parts.next().unwrap_or(0), parts.next().unwrap_or(0), parts.next().unwrap_or(0));

    // asInvoker stops Windows from asking for administrator rights just
    // because a program's name contains "setup" or "install". Per-monitor
    // DPI awareness keeps text sharp on high-resolution screens.
    let manifest = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity type="win32" name="Countdown" version="{major}.{minor}.{patch}.0"/>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security><requestedPrivileges><requestedExecutionLevel level="asInvoker" uiAccess="false"/></requestedPrivileges></security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application><supportedOS Id="{{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}}"/></application>
  </compatibility>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
    </windowsSettings>
  </application>
  <dependency>
    <dependentAssembly>
      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0"
        processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>
    </dependentAssembly>
  </dependency>
</assembly>
"#
    );
    let manifest_path = out.join("app.manifest");
    std::fs::write(&manifest_path, manifest).unwrap();

    let rc = format!(
        r#"1 ICON "{icon}"
1 24 "{manifest}"
1 VERSIONINFO
FILEVERSION {major},{minor},{patch},0
PRODUCTVERSION {major},{minor},{patch},0
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040C04B0"
    BEGIN
      VALUE "CompanyName", "Countdown"
      VALUE "FileDescription", "{description}"
      VALUE "FileVersion", "{version}"
      VALUE "ProductName", "Countdown"
      VALUE "ProductVersion", "{version}"
      VALUE "LegalCopyright", "Countdown"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x040C, 1200
  END
END
"#,
        icon = rc_path(icon),
        manifest = rc_path(&manifest_path),
    );
    let rc_file = out.join("app.rc");
    std::fs::write(&rc_file, rc).unwrap();

    // Cross-builds from Linux link with Zig (cargo-zigbuild), whose linker
    // accepts a compiled .res file directly, and Zig ships a resource compiler.
    let linker_is_zig = std::env::vars().any(|(k, v)| k.ends_with("_LINKER") && v.contains("zig"));
    if linker_is_zig {
        let res = out.join("app.res");
        let zig_commands: [&[&str]; 3] = [&["zig"], &["python3", "-m", "ziglang"], &["python", "-m", "ziglang"]];
        for cmd in zig_commands {
            let ok = Command::new(cmd[0])
                .args(&cmd[1..])
                .args(["rc", "/fo"])
                .arg(&res)
                .arg(&rc_file)
                .status()
                .is_ok_and(|s| s.success());
            if ok {
                println!("cargo:rustc-link-arg={}", res.display());
                return;
            }
        }
    }

    // Native Windows builds: let embed-resource find rc.exe (MSVC) or windres (MinGW).
    if let Err(e) = embed_resource::compile(&rc_file, embed_resource::NONE).manifest_optional() {
        println!("cargo:warning=No icon embedded in the .exe ({e}). The program still works.");
    }
}
