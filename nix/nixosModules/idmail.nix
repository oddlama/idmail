{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (lib)
    filterAttrsRecursive
    getExe
    mkEnableOption
    mkIf
    mkOption
    mkPackageOption
    removeAttrs
    types
    ;

  cfg = config.services.idmail;
  defaultDataDir = "/var/lib/idmail";

  provisionWithoutNull = filterAttrsRecursive (_: v: v != null) (
    removeAttrs cfg.provision [ "enable" ]
  );
  provisionToml = (pkgs.formats.toml { }).generate "idmail-provision.toml" provisionWithoutNull;
in
{
  options.services.idmail = {
    enable = mkEnableOption "idmail";
    package = mkPackageOption pkgs "idmail" { };

    user = mkOption {
      default = "idmail";
      type = types.str;
      description = "The user as which the service will be executed. Only creates a user 'idmail' if left untouched.";
    };

    dataDir = mkOption {
      default = defaultDataDir;
      type = types.path;
      description = "The data directory where the database will be stored";
    };

    openFirewall = mkOption {
      type = types.bool;
      default = false;
      description = "Whether to open the relevant port for idmail in the firewall. It is recommended to use a reverse proxy with TLS termination instead.";
    };

    host = mkOption {
      type = types.str;
      description = "Host to bind to, must be an IP address.";
      default = "127.0.0.1";
    };

    port = mkOption {
      type = types.port;
      default = 3000;
      description = "Port to bind to";
    };

    provision = {
      enable = mkEnableOption "provisioning of idmail";

      users = mkOption {
        default = { };
        type = types.attrsOf (
          types.submodule {
            options = {
              password_hash = mkOption {
                type = types.str;
                description = ''
                  Password hash, should be a argon2id hash.
                  Can be generated with: `echo -n "whatever" | argon2 somerandomsalt -id`
                  Also accepts "%{file:/path/to/secret}%" to refer to the contents of a file.
                '';
              };
              admin = mkOption {
                type = types.bool;
                default = false;
                description = ''Whether the user should be an admin.'';
              };
              active = mkOption {
                type = types.bool;
                default = true;
                description = ''Whether the user should be active.'';
              };
            };
          }
        );
      };

      domains = mkOption {
        default = { };
        type = types.attrsOf (
          types.submodule {
            options = {
              owner = mkOption {
                type = types.str;
                description = ''
                  The user which owns this domain. Allows that user to modify
                  the catch all address and the domain's active state.
                  Creation and deletion of any domain is always restricted to admins only.
                '';
              };
              catch_all = mkOption {
                type = types.nullOr types.str;
                default = null;
                description = ''A catch-all address for this domain.'';
              };
              public = mkOption {
                type = types.bool;
                default = false;
                description = ''
                  Whether the domain should be available for use by any registered
                  user instead of just the owner. Admins can always use any domain,
                  regardless of this setting.
                '';
              };
              active = mkOption {
                type = types.bool;
                default = true;
                description = ''Whether the domain should be active.'';
              };
            };
          }
        );
      };

      mailboxes = mkOption {
        default = { };
        type = types.attrsOf (
          types.submodule {
            options = {
              password_hash = mkOption {
                type = types.str;
                description = ''
                  Password hash, should be a argon2id hash.
                  Can be generated with: `echo -n "whatever" | argon2 somerandomsalt -id`
                  Also accepts "%{file:/path/to/secret}%" to refer to the contents of a file.
                '';
              };
              owner = mkOption {
                type = types.str;
                description = ''The user which owns this mailbox. That user has full control over the mailbox and its aliases.'';
              };
              api_token = mkOption {
                type = types.nullOr types.str;
                default = null;
                description = ''
                  An API token for this mailbox to allow alias creation via the API endpoints.
                  Optional. Default: None (API access disabled)
                  Minimum length 16. Must be unique!
                  Also accepts "%{file:/path/to/secret}%" to refer to the contents of a file.
                '';
              };
              active = mkOption {
                type = types.bool;
                default = true;
                description = ''Whether the mailbox should be active.'';
              };
            };
          }
        );
      };

      aliases = mkOption {
        default = { };
        type = types.attrsOf (
          types.submodule {
            options = {
              target = mkOption {
                type = types.str;
                description = ''
                  The target address for this alias. The WebUI restricts users to only
                  target mailboxes they own. Admins and this provisioning file
                  have no such restrictions.
                '';
              };
              owner = mkOption {
                type = types.str;
                description = ''
                  The user/mailbox which owns this alias. If owned by a mailbox,
                  the user owning the mailbox transitively owns this.
                '';
              };
              comment = mkOption {
                type = types.nullOr types.str;
                default = null;
                description = ''A comment to store alongside this alias.'';
              };
              active = mkOption {
                type = types.bool;
                default = true;
                description = ''Whether the alias should be active.'';
              };
            };
          }
        );
      };
    };
  };

  config = mkIf cfg.enable {
    users = mkIf (cfg.user == "idmail") {
      groups.idmail = { };
      users.idmail = {
        isSystemUser = true;
        group = "idmail";
        home = defaultDataDir;
      };
    };

    networking.firewall.allowedTCPPorts = mkIf cfg.openFirewall [ cfg.port ];

    systemd.services.idmail = {
      description = "An email alias and account management interface for self-hosted mailservers";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];

      environment.LEPTOS_SITE_ADDR = "${cfg.host}:${toString cfg.port}";
      environment.IDMAIL_PROVISION = mkIf cfg.provision.enable provisionToml;

      serviceConfig = {
        Restart = "on-failure";
        ExecStart = getExe cfg.package;
        User = cfg.user;

        StateDirectory = mkIf (cfg.dataDir == defaultDataDir) "idmail";
        StateDirectoryMode = mkIf (cfg.dataDir == defaultDataDir) "750";
        WorkingDirectory = cfg.dataDir;
        ReadWriteDirectories = [ cfg.dataDir ];

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
        UMask = "0027";
      };
    };
  };
}
