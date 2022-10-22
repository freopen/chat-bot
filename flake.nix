{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    fup.url = "github:gytis-ivaskevicius/flake-utils-plus";
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, fup, naersk }@inputs:
    fup.lib.mkFlake {
      inherit self inputs;
      supportedSystems = [ "x86_64-linux" ];

      outputsBuilder = channels: {
        packages.freopen_chat_bot = (channels.nixpkgs.callPackage naersk { }).buildPackage {
          src = ./.;
          nativeBuildInputs = with channels.nixpkgs; [ pkg-config protobuf ];
          buildInputs = with channels.nixpkgs; [ openssl ];
          postInstall = "cp -R assets $out/assets";
        };

      };

      nixosModules.freopen_chat_bot = { config, pkgs, lib, ... }: {
        options.services.freopen_chat_bot = {
          enable = lib.mkEnableOption "freopen_chat_bot";
          envFile = lib.mkOption {
            type = lib.types.str;
          };
        };
        config =
          let
            opts = config.services.freopen_chat_bot;
            pkg = self.packages.${pkgs.stdenv.hostPlatform.system}.freopen_chat_bot;
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
                ExecStart = "${pkg}/bin/chat_bot";
                EnvironmentFile = opts.envFile;
                WorkingDirectory = "/var/lib/freopen_chat_bot";
                StateDirectory = "freopen_chat_bot";
                StateDirectoryMode = "0700";
              };
            };
          };
      };
    };
}
