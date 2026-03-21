{
  description = "slimjelly dev/build environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        runtimeLibs = with pkgs; [
          wayland
          libxkbcommon
          mesa
          libglvnd
          vulkan-loader
        ];

        rpathLibs = builtins.concatStringsSep ":" (map (pkg: "${pkg}/lib") runtimeLibs);
        xkbRoot = "${pkgs.xkeyboard_config}/share/X11/xkb";
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            rustc
            cargo
            rustfmt
            clippy
            pkg-config
            xkeyboard_config
          ] ++ runtimeLibs;

          shellHook = ''
            export SLIMJELLY_RPATH_LIBS="${rpathLibs}"
            export SLIMJELLY_XKB_CONFIG_ROOT="${xkbRoot}"

            export XKB_CONFIG_ROOT="${xkbRoot}"

            echo "slimjelly dev shell ready"
            echo "- RPATH libs configured for release/debug builds"
            echo "- XKB_CONFIG_ROOT set to ${xkbRoot}"
          '';
        };
      }
    );
}
