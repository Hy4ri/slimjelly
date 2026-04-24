fn main() {
    #[cfg(target_os = "linux")]
    configure_linux_runtime();
}

#[cfg(target_os = "linux")]
fn configure_linux_runtime() {
    use std::path::Path;

    println!("cargo:rerun-if-env-changed=SLIMJELLY_RPATH_LIBS");
    println!("cargo:rerun-if-env-changed=SLIMJELLY_XKB_CONFIG_ROOT");

    let lib_dirs = std::env::var("SLIMJELLY_RPATH_LIBS").unwrap_or_default();
    for dir in lib_dirs.split(':').map(str::trim).filter(|s| !s.is_empty()) {
        if Path::new(dir).exists() {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}");
        }
    }

    if let Ok(xkb_root) = std::env::var("SLIMJELLY_XKB_CONFIG_ROOT") {
        let trimmed = xkb_root.trim();
        if !trimmed.is_empty() {
            println!("cargo:rustc-env=SLIMJELLY_XKB_CONFIG_ROOT_DEFAULT={trimmed}");
        }
    }
}
