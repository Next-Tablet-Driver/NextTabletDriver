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

    extraArgs = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      example = [ "--minimized" ];
      description = "Extra command-line arguments passed to the background service.";
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = [ cfg.package ];

    systemd.user.services.next-tablet-driver = {
      Unit = {
        Description = "NextTabletDriver background service";
        PartOf = [ "graphical-session.target" ];
      };

      Service = {
        ExecStart = "${lib.getExe' cfg.package "next_tablet_driver"} ${lib.escapeShellArgs cfg.extraArgs}";
        Restart = "on-failure";
      };

      Install = {
        WantedBy = [ "graphical-session.target" ];
      };
    };
  };
}
