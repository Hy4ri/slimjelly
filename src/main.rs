mod app;
mod config;
mod error;
mod jellyfin;
mod secure_store;
mod seerr;
mod subtitles;

use std::sync::Arc;

use app::SlimJellyApp;
use config::{AppConfig, AppPaths, load_or_create};

#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LinuxBackend {
    Auto,
    X11,
    Wayland,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    #[cfg(target_os = "linux")]
    ensure_linux_runtime_env();

    let runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()?,
    );

    let (config, paths) = load_or_create()?;
    let app_title = config.client.app_name.clone();

    #[cfg(target_os = "linux")]
    let requested_backend = requested_linux_backend();
    #[cfg(target_os = "linux")]
    let selected_backend = select_linux_backend(requested_backend);

    #[cfg(target_os = "linux")]
    let native_options = build_native_options(selected_backend);
    #[cfg(not(target_os = "linux"))]
    let native_options = eframe::NativeOptions::default();

    let run_result = run_app(
        &app_title,
        native_options,
        runtime.clone(),
        config.clone(),
        paths.clone(),
    );

    if let Err(err) = run_result {
        #[cfg(target_os = "linux")]
        print_linux_error_hint(&err, selected_backend);

        return Err(Box::new(err));
    }

    Ok(())
}

fn run_app(
    app_title: &str,
    native_options: eframe::NativeOptions,
    runtime: Arc<tokio::runtime::Runtime>,
    config: AppConfig,
    paths: AppPaths,
) -> eframe::Result {
    eframe::run_native(
        app_title,
        native_options,
        Box::new(move |_cc| {
            Ok(Box::new(SlimJellyApp::new(
                runtime.clone(),
                config.clone(),
                paths.clone(),
            )))
        }),
    )
}

#[cfg(target_os = "linux")]
fn requested_linux_backend() -> LinuxBackend {
    std::env::var("SLIMJELLY_UNIX_BACKEND")
        .ok()
        .and_then(|value| parse_linux_backend(&value))
        .or_else(|| {
            std::env::var("WINIT_UNIX_BACKEND")
                .ok()
                .and_then(|value| parse_linux_backend(&value))
        })
        .unwrap_or(LinuxBackend::Auto)
}

#[cfg(target_os = "linux")]
fn ensure_linux_runtime_env() {
    if std::env::var_os("XKB_CONFIG_ROOT").is_some() {
        return;
    }

    let Some(default_path) = option_env!("SLIMJELLY_XKB_CONFIG_ROOT_DEFAULT") else {
        return;
    };

    if default_path.is_empty() {
        return;
    }

    // SAFETY: This runs in main before background threads are started.
    unsafe {
        std::env::set_var("XKB_CONFIG_ROOT", default_path);
    }
}

#[cfg(target_os = "linux")]
fn select_linux_backend(requested: LinuxBackend) -> LinuxBackend {
    match requested {
        LinuxBackend::Auto => {
            if env_has_non_empty("WAYLAND_DISPLAY") || env_has_non_empty("WAYLAND_SOCKET") {
                LinuxBackend::Wayland
            } else if env_has_non_empty("DISPLAY") {
                LinuxBackend::X11
            } else {
                LinuxBackend::Auto
            }
        }
        explicit => explicit,
    }
}

#[cfg(target_os = "linux")]
fn env_has_non_empty(name: &str) -> bool {
    std::env::var(name)
        .map(|value| !value.is_empty())
        .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn parse_linux_backend(value: &str) -> Option<LinuxBackend> {
    match value.trim().to_ascii_lowercase().as_str() {
        "auto" => Some(LinuxBackend::Auto),
        "x11" => Some(LinuxBackend::X11),
        "wayland" => Some(LinuxBackend::Wayland),
        _ => None,
    }
}

#[cfg(target_os = "linux")]
fn build_native_options(backend: LinuxBackend) -> eframe::NativeOptions {
    use winit::platform::{
        wayland::EventLoopBuilderExtWayland as _, x11::EventLoopBuilderExtX11 as _,
    };

    let mut native_options = eframe::NativeOptions::default();
    native_options.event_loop_builder = Some(Box::new(move |builder| match backend {
        LinuxBackend::Auto => {}
        LinuxBackend::X11 => {
            builder.with_x11();
        }
        LinuxBackend::Wayland => {
            builder.with_wayland();
        }
    }));

    native_options
}

#[cfg(target_os = "linux")]
fn print_linux_error_hint(err: &eframe::Error, backend: LinuxBackend) {
    let debug = format!("{err:?}");
    if debug.contains("NoWaylandLib") {
        eprintln!(
            "Wayland runtime libraries are missing. On NixOS run inside a shell with wayland + libxkbcommon + xkeyboard-config + mesa + libglvnd."
        );
    } else if debug.contains("XNotSupported") {
        eprintln!(
            "X11 runtime libraries are missing. Install libX11 or use Wayland backend if available."
        );
    } else if debug.contains("XKBNotFound") {
        eprintln!(
            "Wayland keyboard data missing (XKBNotFound). Add xkeyboard-config and set XKB_CONFIG_ROOT to <store>/share/X11/xkb."
        );
    } else if debug.contains("NoGlutinConfigs") {
        eprintln!(
            "No GL config found. Add mesa + libglvnd (EGL/OpenGL runtime) to your environment."
        );
    } else {
        eprintln!(
            "Startup failed on backend {:?}. Set SLIMJELLY_UNIX_BACKEND=x11 or wayland to override.",
            backend
        );
    }
}
