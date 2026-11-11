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
          librsvg cairo pango gdk-pixbuf atk dbus
        ];
        GIO_MODULE_DIR = "${pkgs.glib-networking}/lib/gio/modules";
      };
    };
}
