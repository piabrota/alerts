{
  description = "alerts: dev shell (Rust workspace, maturin, just)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    # The tools of h0ffmann/ptbr-writer, from picasso's labs at the revision that ptbr-writer pins
    # (forks/picasso). `just pins` fails when the three revisions differ.
    # The site's toolchain: Gleam on JavaScript (Lustre), node, esbuild, just.
    gleam-website.url = "git+https://github.com/h0ffmann/picasso?dir=labs/gleam-website&rev=5da5237ba6f3fc51de0a10bc746a1ce0ad89e33e&shallow=1";
    # The code graph of this repository (graphify's graph.json and an interactive graph.html).
    code-graph.url = "git+https://github.com/h0ffmann/picasso?dir=labs/code-graph&rev=5da5237ba6f3fc51de0a10bc746a1ce0ad89e33e&shallow=1";
    # The reading-time calculator of ptbr-writer (`ptbr reading-time`).
    ptbr-tools.url = "git+https://github.com/h0ffmann/picasso?dir=labs/ptbr-tools&rev=5da5237ba6f3fc51de0a10bc746a1ce0ad89e33e&shallow=1";
  };

  outputs = { nixpkgs, gleam-website, code-graph, ptbr-tools, ... }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
      forAll = f: nixpkgs.lib.genAttrs systems (s: f s nixpkgs.legacyPackages.${s});
    in {
      devShells = forAll (system: pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            cargo rustc clippy rustfmt rust-analyzer
            cargo-deny maturin python3 just
            cffconvert uv
            jq
            zip # `just pack-verbs`
            miniserve # `just site-serve`
            nodejs # vendor/archify (`just figure`); Chrome comes from the host
            ptbr-tools.packages.${system}.default # `just reading-time`
            code-graph.packages.${system}.default # `just graph`
            trunk wasm-bindgen-cli lld # desktop/ui: the interface, built for wasm32
          ] ++ pkgs.lib.optional pkgs.stdenv.hostPlatform.isLinux pkgs.chromium # `just desmos-gif`; on macOS, CHROME names Chrome
          # desktop/app: Tauri 2 on WebKitGTK. On macOS, Tauri uses the system WebView.
          ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux (with pkgs; [
            pkg-config openssl glib glib-networking gtk3 webkitgtk_4_1 libsoup_3 librsvg cairo pango gdk-pixbuf atk
          ]);
          # WebKitGTK needs glib-networking for HTTPS (the map tiles).
          GIO_MODULE_DIR = pkgs.lib.optionalString pkgs.stdenv.hostPlatform.isLinux "${pkgs.glib-networking}/lib/gio/modules/";
        };
        # `just site`: the Gleam site in site/.
        site = pkgs.mkShell {
          inputsFrom = [ gleam-website.devShells.${system}.default ];
          packages = [ code-graph.packages.${system}.default ptbr-tools.packages.${system}.default pkgs.jq ];
        };
      });
    };
}
