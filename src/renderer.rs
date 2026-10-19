//! Which graphics API the window draws with. Shared with the installer.
//!
//! On Windows it's Direct3D 12. Left to choose, wgpu may pick Vulkan, and some
//! Intel Vulkan drivers (HD Graphics 620, driver 31.0.101.2121) crash the whole
//! process a few seconds after the window opens. That's too late for the
//! OpenGL retry in `main` to help, and leaves no crash.log. Without Direct3D 12
//! the window fails to open cleanly instead, and `main` retries with OpenGL.
//!
//! `WGPU_BACKEND` (`vulkan`, `dx12`, `gl`…) still overrides the choice.

use eframe::egui_wgpu::{WgpuConfiguration, WgpuSetup, WgpuSetupCreateNew};
use eframe::wgpu::Backends;

/// `from_env` is what `WGPU_BACKEND` asks for, if it's set.
pub fn backends(from_env: Option<Backends>, windows: bool) -> Backends {
    match from_env {
        Some(backends) => backends,
        None if windows => Backends::DX12,
        // egui's own default.
        None => Backends::PRIMARY | Backends::GL,
    }
}

pub fn wgpu_options() -> WgpuConfiguration {
    let mut setup = WgpuSetupCreateNew::without_display_handle();
    setup.instance_descriptor.backends = backends(Backends::from_env(), cfg!(windows));
    WgpuConfiguration { wgpu_setup: WgpuSetup::CreateNew(setup), ..Default::default() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_draws_with_direct3d_12_only() {
        let chosen = backends(None, true);
        assert_eq!(chosen, Backends::DX12);
        assert!(!chosen.contains(Backends::VULKAN), "Vulkan crashes on some Intel drivers");
    }

    #[test]
    fn other_systems_keep_egui_default() {
        assert_eq!(backends(None, false), Backends::PRIMARY | Backends::GL);
    }

    #[test]
    fn wgpu_backend_variable_still_wins() {
        assert_eq!(backends(Some(Backends::VULKAN), true), Backends::VULKAN);
        assert_eq!(backends(Some(Backends::GL), false), Backends::GL);
    }

    #[test]
    fn options_carry_the_choice() {
        let WgpuSetup::CreateNew(setup) = wgpu_options().wgpu_setup else {
            panic!("expected a new wgpu setup");
        };
        assert_eq!(setup.instance_descriptor.backends, backends(Backends::from_env(), cfg!(windows)));
    }
}
