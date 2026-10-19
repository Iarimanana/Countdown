# Countdown

A desktop app that keeps track of which devices are on your Wi-Fi and for how long. It's built for Windows and
also runs on Linux.

You enter each device's name, its name in your router's admin panel and its MAC address by hand, and choose
how long it may stay connected. When the time
runs out, Countdown rings an alarm, or shows a notification with a sound if you prefer. You remove the device
from your router yourself, then click **Retiré du réseau**. If you haven't done that after a few minutes, the
alert comes back.

Countdown doesn't connect to your router. It is only a tracker and timer.

The interface is in French by default. Switch to English with **FR / EN** at the top right or in **Réglages**.
The app follows your system's light or dark theme.

## Using it

- **Add a device:** fill in three fields, choose the connection time, then click **Ajouter l'appareil** or
  press Enter.
  - **Nom:** your own name for it, e.g. *Téléphone de Paul*.
  - **Nom dans la box:** the device's name as your router's admin page lists it, e.g. *iPhone 13 Pro Max*.
    It's shown in the device list and in alerts, so you can find the right device in the router when it's
    time to remove it.
  - **Adresse MAC:** hover over **(?)** to see where to find it.
  MAC addresses can be typed with `:`, `-` or nothing between the pairs. A hint under the field says whether
  the address is valid, and whether it's already saved.
- **Connection time:** pick a preset (30 min to 1 week), or use the **−** and **+** buttons for days, hours
  and minutes. You can also click a number and type it. The line below shows when the session will end.
- **While connected:** each device shows a countdown, its end time and a bar that drains. The bar turns amber
  in the last 5 minutes. **+30 min** and **+1 h** add time, and **Terminer** ends the session early.
- **When time is up:** the row turns red and the alert goes off. Remove the device from your router, then
  click **Retiré du réseau**. **Arrêter l'alarme** silences the alarm without removing the device.
- **A MAC address that's already saved:** a window opens and offers to reconnect the device, or to add time
  if it's still connected. The duration starts at what you set in the form, and you can change it there.
- **Past devices:** removed devices are kept in **Anciens appareils**. **Reconnecter…** opens the same window,
  and **Oublier** deletes a device for good after asking you to confirm.

## Installing on Windows

Download `Countdown-Setup-<version>.exe` from the [Releases](https://github.com/Iarimanana/Countdown/releases)
page and run it. It works offline (the app is inside it) and needs no administrator rights:

1. Choose whether you want a desktop shortcut and whether Countdown opens at the end, then click **Installer**.
   **Modifier** changes the install folder (default: `%LOCALAPPDATA%\Programs\Countdown`).
2. A progress bar shows each step.
3. **Countdown est installé**: it's in the Start menu, and on the desktop if you chose that.

Running a newer installer later offers **Mettre à jour**. It replaces the app in the same folder and keeps the
device list and settings. If Countdown is open, the installer asks you to close it and offers **Réessayer**.

To uninstall, go to **Settings → Apps → Countdown → Uninstall**. The uninstaller removes the app and its
shortcuts. It keeps the device list unless you tick **Supprimer aussi la liste des appareils et les réglages**.
It only deletes files it installed, so anything else in the folder stays.

Windows may show a SmartScreen warning ("Windows protected your PC") the first time, because the installer
isn't code-signed yet (see [Code signing policy](#code-signing-policy)). Click **More info → Run anyway**.

Advanced: `Countdown-Setup.exe --silent` installs with default choices and no window.
`uninstall.exe --uninstall --silent` uninstalls and keeps the data.

## Settings (Réglages)

- **Quand le temps est écoulé:**
  - *Faire sonner une alarme* (default): beeps for 1 minute and brings the window forward.
  - *Envoyer une notification avec un son*: a notification that stays on screen, plus a short chime.
- **Rappel:** how long to wait before alerting again if a device still hasn't been removed: 2, 5 (default),
  10, 15 or 30 minutes, or never.
- **Tester l'alerte:** plays the chosen alert, so you can check the volume.

## Things to know

- Alerts only fire while Countdown is open. Minimized is fine. Closing the window with devices connected asks
  for confirmation first. Timers keep counting while the app is closed: anything that ran out is flagged, and
  the alert rings, as soon as you open it again.
- Only one copy of Countdown can run at a time. Opening it again shows a message instead of a second window.
- Data is saved after every change, to:
  - **Windows:** `%APPDATA%\Countdown\data\`
  - **Linux:** `~/.local/share/countdown/`

  The folder contains:
  - `devices.json`: the device list.
  - `devices.json.bak`: the previous save. Countdown uses it automatically if `devices.json` is damaged.
  - `crash.log`: written if the app ever crashes. Send it along when reporting a problem.
- On Windows, Countdown and its installer draw with Direct3D 12, not Vulkan: some Intel Vulkan drivers crash
  the app a few seconds after it opens. To try another one, set `WGPU_BACKEND` (`vulkan`, `dx12` or `gl`).
- If the window can't open (an old graphics driver, for example), Countdown retries once with OpenGL. If that
  also fails, it explains what happened in a message box.

## Building

You need [Rust](https://rustup.rs). The project is a Cargo workspace with two programs: the app (`countdown`, in
`src/`) and its installer (`countdown-setup`, in `installer/`). The installer embeds the finished app, so the
app is built first.

### The Windows installer

On Windows (Rust with the default MSVC setup), in PowerShell:

```
.\build-windows.ps1
```

From Linux, with Zig as the cross-linker (no sudo needed):

```
rustup target add x86_64-pc-windows-gnu
pip install ziglang
cargo install cargo-zigbuild
./build-windows.sh
```

Either way the result is `dist/Countdown-Setup-<version>.exe`, a single file to give to whoever will use the
app. The version comes from `Cargo.toml` (`[workspace.package] version`).

The icon, version information and a manifest (which keeps Windows from asking for administrator rights) are
embedded by `build/winres.rs`. If no resource compiler is found, the build still works and the `.exe` has the
default icon.

The icons are drawn by `assets/icon/make_icon.py` (`python3 assets/icon/make_icon.py assets/icon`):
`countdown.ico` for the app (a stopwatch) and `countdown-setup.ico` for the installer (the stopwatch with a
download badge).

### Just the app

```
cargo build --release -p countdown                                    # Linux, or Windows natively
cargo zigbuild --release -p countdown --target x86_64-pc-windows-gnu  # Windows .exe from Linux
```

On Linux, sounds are played with `pw-play`, `paplay` or `aplay`, whichever is installed.

## Tests

```
cargo test --workspace
```

The suite covers:

- **Data model:** every device state change, reminders, clock changes and overflow on huge values.
- **Randomized sequences:** thousands of random operations, checking that ids and MAC addresses stay unique
  and that saving and reloading changes nothing.
- **Files:** saving, loading, damaged files, backup recovery, and a disk that refuses to save.
- **Interface:** headless tests that type and click through the real window, covering every flow above.
- **Random clicking:** a "monkey test" that clicks random buttons while time jumps forward.
- **Installer:** installing, updating, uninstalling with and without data, the app being open, damaged
  payloads, unwritable folders, refusing to delete folders that aren't Countdown's, and the installer window's
  screens. The tests run against a fake Windows, so they work on any machine.

### Testing the Windows version from Linux

With Wine installed, after `./build-windows.sh`:

```
./test-windows.sh
```

The script runs both test suites compiled for Windows. It then runs the real installer through install,
update, an update refused while the app is open, and uninstall, checking the files, shortcuts and registry
entry at each step. It uses its own Wine prefix in `target/wine-test` and turns off Wine's desktop
integration, so nothing appears on your real desktop or in your app menu.

The UI tests run one at a time under Wine. Wine's DirectWrite crashes when several threads lay out text at
once. Real Windows doesn't have this problem, and the app draws all its text on one thread.

### Tests that only run on request

Two tests only run on request:

```
cargo test -- --ignored player_starts                 # plays a real sound
SCREENSHOT_DIR=shots cargo test screenshots -- --ignored   # saves screenshots of the main screens
SCREENSHOT_DIR=shots cargo test -p countdown-setup screenshots -- --ignored   # same for the installer
```

## Code signing policy

Releases aren't code-signed yet. We plan to sign them through the [SignPath Foundation](https://signpath.org),
which offers free code signing to open source projects.

Once that's in place, only these files are signed, built by GitHub Actions from this repository's source:
`Countdown.exe`, and `Countdown-Setup-<version>.exe` (which is also installed as `uninstall.exe`). Nothing
built elsewhere, and no third-party program, is signed with the project's certificate.

Team roles:

- Committers and reviewers: [Iarimanana](https://github.com/Iarimanana)
- Approvers: [Iarimanana](https://github.com/Iarimanana)

Every change to the source and the build scripts goes through this repository, and every signed release is
approved by hand.

### Privacy

This program will not transfer any information to other networked systems unless specifically requested by
the user or the person installing or operating it. The device list and settings stay on your computer, in the
folder listed under [Things to know](#things-to-know).

## Fonts

The app embeds [Atkinson Hyperlegible](https://www.brailleinstitute.org/freefont/) and Atkinson Hyperlegible Mono
by the Braille Institute, under the SIL Open Font License (`assets/fonts/OFL.txt`). Emoji use egui's bundled
Noto Emoji. Other scripts (Arabic, Chinese…) use the operating system's fonts.

## License

Countdown is free and open source, under the [MIT License](LICENSE). The fonts keep their own license (above).
