{
  description = "alerts: dev shell (Haskell, Agda, just)";

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
            # The Haskell source (`just hs-test`): GHC with every library of the cabal packages,
            # so cabal builds offline from the pinned nixpkgs.
            (haskellPackages.ghcWithPackages (h: with h; [
              aeson http-client http-client-tls http-types sqlite-simple xml-conduit
            ]))
            cabal-install haskell-language-server ormolu # `just fmt`
            (agda.withPackages (p: [ p.standard-library ])) # `just spec`
            just jq
            cffconvert uv # `just reuse`
            zip # `just pack-verbs`
            miniserve # `just site-serve`
            nodejs # vendor/archify (`just figure`); Chrome comes from the host
            ptbr-tools.packages.${system}.default # `just reading-time`
            code-graph.packages.${system}.default # `just graph`
          ];
        };
        # `just site`: the Gleam site in site/.
        site = pkgs.mkShell {
          inputsFrom = [ gleam-website.devShells.${system}.default ];
          packages = [ code-graph.packages.${system}.default ptbr-tools.packages.${system}.default pkgs.jq ];
        };
      });
    };
}
