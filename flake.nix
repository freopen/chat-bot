{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = { self, flake-utils, naersk, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = (import nixpkgs) {
          inherit system;
        };
        naersk' = pkgs.callPackage naersk { };
      in
      rec {
        # For `nix build` & `nix run`:
        packages.default = naersk'.buildPackage {
          src = ./.;
          nativeBuildInputs = with pkgs; [ pkg-config protobuf ];
          buildInputs = with pkgs; [ openssl ];
        };

        # For `nix develop`:
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [ rustc cargo ];
        };

        nixosModules.freopen_chat_bot = { config, lib, ... }: {
          options.services.freopen_chat_bot = {
            enable = lib.mkEnableOption "freopen_chat_bot";
            envFile = lib.mkOption {
              type = lib.types.str;
            };
          };
          config =
            let
              opts = config.services.freopen_chat_bot;
            in
            lib.mkIf opts.enable {
              users.groups.freopen_chat_bot = { };
              users.users.freopen_chat_bot = {
                isSystemUser = true;
                group = "freopen_chat_bot";
              };
              systemd.services.freopen_chat_bot = {
                wantedBy = [ "multi-user.target" ];
                after = [ "network.target" ];
                serviceConfig = {
                  User = "freopen_chat_bot";
                  ExecStart = "${self.packages."${system}".default}/bin/chat_bot";
                  EnvironmentFile = opts.envFile;
                  WorkingDirectory = "/var/lib/freopen_chat_bot";
                };
              };
            };
        };
      }
    );
}
