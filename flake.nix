{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    fup.url = "github:gytis-ivaskevicius/flake-utils-plus";
  };

  outputs = { self, nixpkgs, fup }@inputs:
    fup.lib.mkFlake {
      inherit self inputs;
      supportedSystems = [ "x86_64-linux" ];

      outputsBuilder = channels: {
        packages.default = channels.nixpkgs.runCommand "freopen_chat_bot" {
          src = ./.;
          __noChroot = true;
          nativeBuildInputs = with channels.nixpkgs; [ pkg-config protobuf cargo rustc gcc ];
          buildInputs = with channels.nixpkgs; [ openssl ];
        } ''
          export CARGO_HTTP_CAINFO="${channels.nixpkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
          export SSL_CERT_FILE=${channels.nixpkgs.cacert}/etc/ssl/certs/ca-bundle.crt
          cp -R $src/* .

          cargo build --release

          mkdir -p $out/bin
          cp target/release/chat_bot $out/bin 
          cp -R assets $out/assets
        '';
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
            pkg = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
          in
          lib.mkIf opts.enable {
            users.groups.freopen_chat_bot = { };
            users.users.freopen_chat_bot = {
              isSystemUser = true;
              group = "freopen_chat_bot";
            };
            systemd.services.freopen_chat_bot = {
              wantedBy = [ "multi-user.target" ];
              after = [ "network-online.target" ];
              wants = [ "network-online.target" ];
              serviceConfig = {
                User = "freopen_chat_bot";
                ExecStart = "${pkg}/bin/chat_bot";
                EnvironmentFile = opts.envFile;
                WorkingDirectory = "/var/lib/freopen_chat_bot";
                StateDirectory = "freopen_chat_bot";
                StateDirectoryMode = "0700";
                Restart = "always";
              };
            };
          };
      };
    };
}
