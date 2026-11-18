{
  description = "Kreuzchen dev shell";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      pkgs = nixpkgs.legacyPackages.x86_64-linux;
    in {
      devShells.x86_64-linux.default = pkgs.mkShell {
        nativeBuildInputs = [ pkgs.pkg-config ];
        buildInputs = with pkgs; [
          rustc cargo clippy rustfmt rust-analyzer
          nodejs_24
          webkitgtk_4_1 gtk3 libsoup_3 glib glib-networking openssl
          librsvg cairo pango gdk-pixbuf atk dbus gsettings-desktop-schemas
        ];
        GIO_MODULE_DIR = "${pkgs.glib-networking}/lib/gio/modules";
        # GTK file dialogs abort without the GSettings schemas, which wrapGAppsHook adds for packaged apps.
        shellHook = with pkgs; ''
          export XDG_DATA_DIRS=${glib.getSchemaDataDirPath gsettings-desktop-schemas}:${glib.getSchemaDataDirPath gtk3}''${XDG_DATA_DIRS:+:$XDG_DATA_DIRS}
        '';
      };
    };
}
