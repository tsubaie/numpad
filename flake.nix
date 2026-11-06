{
  description = "NumPad: offline calculation tape with notes and live totals";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [
        "x86_64-linux"
        "aarch64-linux"
      ];
    in
    {
      packages = forAllSystems (system: rec {
        numpad = nixpkgs.legacyPackages.${system}.callPackage ./nix/package.nix { };
        default = numpad;
      });

      # For NixOS configs: nixpkgs.overlays = [ inputs.numpad.overlays.default ]; then pkgs.numpad
      overlays.default = final: _prev: { numpad = final.callPackage ./nix/package.nix { }; };
    };
}
