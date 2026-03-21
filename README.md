# slimjelly

Simple native Jellyfin wrapper focused on server management and external player playback.

## Build and Run

### NixOS / Wayland-first workflow

The recommended workflow is to build inside the provided flake dev shell so the
binary gets the required runtime search paths (RPATH) baked in.

1. Build release binary:

```bash
nix develop -c cargo build --release
```

2. Run from anywhere:

```bash
/absolute/path/to/slimJelly/target/release/slimjelly
```

You can still force backend selection when needed:

```bash
SLIMJELLY_UNIX_BACKEND=wayland /absolute/path/to/slimJelly/target/release/slimjelly
SLIMJELLY_UNIX_BACKEND=x11 /absolute/path/to/slimJelly/target/release/slimjelly
```

### Notes

- The build script reads:
  - `SLIMJELLY_RPATH_LIBS` (colon-separated library directories)
  - `SLIMJELLY_XKB_CONFIG_ROOT` (default XKB data root)
- The flake sets both values automatically.
- At runtime, if `XKB_CONFIG_ROOT` is unset, slimjelly uses the baked default.
