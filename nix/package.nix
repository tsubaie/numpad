# NumPad for Nix: `nix build`, `nix run`, or add this flake's overlay to a NixOS config.
{
  lib,
  rustPlatform,
  pkg-config,
  fontconfig,
  wayland,
  libxkbcommon,
  vulkan-loader,
  libGL,
  libx11,
  libxcursor,
  libxi,
  libxrandr,
  libxcb,
}:
rustPlatform.buildRustPackage {
  pname = "numpad";
  version = (lib.importTOML ../Cargo.toml).package.version;

  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.difference ../. (
      lib.fileset.unions [
        ../nix
        ../flake.nix
        (lib.fileset.maybeMissing ../flake.lock)
        ../docs
      ]
    );
  };
  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ fontconfig ];

  postInstall = ''
    install -Dm644 packaging/numpad.desktop $out/share/applications/numpad.desktop
    install -Dm644 ${../docs/images/icon.png} $out/share/icons/hicolor/256x256/apps/numpad.png
  '';

  # winit/wgpu load these with dlopen() at runtime, which Nix can't see
  postFixup = ''
    patchelf --add-rpath ${
      lib.makeLibraryPath [
        wayland
        libxkbcommon
        vulkan-loader
        libGL
        fontconfig
        libx11
        libxcursor
        libxi
        libxrandr
        libxcb
      ]
    } $out/bin/numpad
  '';

  meta = {
    description = "Offline calculation tape with notes and live totals";
    homepage = "https://github.com/tsubaie/numpad";
    license = lib.licenses.mit;
    mainProgram = "numpad";
    platforms = lib.platforms.linux;
  };
}
