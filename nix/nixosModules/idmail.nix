{
  config,
  lib,
  pkgs,
  ...
}: let
  inherit
    (lib)
    getExe
    mkEnableOption
    mkIf
    mkOption
    mkPackageOption
    types
    ;

  cfg = config.services.idmail;
  dataDir = "/var/lib/idmail";
in {
  options.services.idmail = {
    enable = mkEnableOption "idmail";
    package = mkPackageOption pkgs "idmail" {};

    openFirewall = mkOption {
      type = types.bool;
      default = false;
      description = "Whether to open the relevant port for idmail in the firewall. It is recommended to use a reverse proxy with TLS termination instead.";
    };

    host = mkOption {
      type = types.str;
      description = "Host to bind to";
      default = "localhost";
    };

    port = mkOption {
      type = types.port;
      default = 3000;
      description = "Port to bind to";
    };
  };

  config = mkIf cfg.enable {
    users.groups.idmail = {};
    users.users.idmail = {
      isSystemUser = true;
      group = "idmail";
      home = dataDir;
    };

    networking.firewall.allowedTCPPorts = mkIf cfg.openFirewall [cfg.port];

    systemd.services.idmail = {
      description = "An email alias and account management interface for self-hosted mailservers";
      wantedBy = ["multi-user.target"];
      after = ["network.target"];

      environment.LEPTOS_SITE_ADDR = "${cfg.host}:${toString cfg.port}";
      serviceConfig = {
        Restart = "on-failure";
        ExecStart = getExe cfg.package;
        User = "idmail";
        Group = "idmail";

        WorkingDirectory = dataDir;
        StateDirectory = "idmail";
        StateDirectoryMode = "0750";

        # Hardening
        CapabilityBoundingSet = "";
        LockPersonality = true;
        MemoryDenyWriteExecute = true;
        NoNewPrivileges = true;
        PrivateUsers = true;
        PrivateTmp = true;
        PrivateDevices = true;
        PrivateMounts = true;
        ProtectClock = true;
        ProtectControlGroups = true;
        ProtectHome = true;
        ProtectHostname = true;
        ProtectKernelLogs = true;
        ProtectKernelModules = true;
        ProtectKernelTunables = true;
        ProtectProc = "invisible";
        ProtectSystem = "strict";
        RemoveIPC = true;
        RestrictAddressFamilies = [
          "AF_INET"
          "AF_INET6"
          "AF_UNIX"
        ];
        RestrictNamespaces = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;
        SystemCallArchitectures = "native";
        SystemCallFilter = [
          "@system-service"
          "~@privileged"
        ];
        UMask = "0077";
      };
    };
  };
}
