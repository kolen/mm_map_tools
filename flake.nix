{
  description = "Dev environment for mm_map_tools";

  inputs = {
    nixpkgs.url = "nixpkgs/22.11";
  };

  outputs = { self, nixpkgs }: {
    devShells.x86_64-darwin.default = let
      pkgs = import nixpkgs { system = "x86_64-darwin"; };
    in
      pkgs.mkShell {
        nativeBuildInputs = with pkgs; [ pkg-config gtk4 glib gdk-pixbuf pango ];
      };
  };
}
