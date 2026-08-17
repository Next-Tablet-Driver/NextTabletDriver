self: { config, lib, pkgs, ... }:

let
  cfg = config.services.nexttabletdriver;
in
{
  options.services.nexttabletdriver = {
    enable = lib.mkEnableOption "NextTabletDriver background service";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.system}.default;
      description = "The NextTabletDriver package to use.";
    };

    user = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      example = "alice";
      description = ''
        Username to add to the `input` group as a fallback for sessions
        without systemd-logind (headless setups, some minimal window
        managers). Not required for normal desktop sessions: the udev
        rules already grant the logged-in user access instantly via
        logind's dynamic ACLs.
      '';
    };

    extraArgs = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      example = [ "--minimized" ];
      description = "Extra command-line arguments passed to the background service.";
    };
  };

  config = lib.mkIf cfg.enable {
    services.udev.packages = [ cfg.package ];

    boot.kernelModules = [ "uinput" ];

    users.users = lib.optionalAttrs (cfg.user != null) {
      ${cfg.user}.extraGroups = [ "input" ];
    };

    systemd.user.services.next-tablet-driver = {
      description = "NextTabletDriver background service";
      partOf = [ "graphical-session.target" ];
      wantedBy = [ "graphical-session.target" ];

      serviceConfig = {
        ExecStart = "${lib.getExe' cfg.package "next_tablet_driver"} ${lib.escapeShellArgs cfg.extraArgs}";
        Restart = "on-failure";
      };
    };
  };
}
