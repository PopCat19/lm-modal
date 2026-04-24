# home-manager.nix
#
# Purpose: Home Manager module for lm-modal integration
#
# This module:
# - Installs the lm-modal binary
# - Configures systemd user services if needed
# - Sets up keybind suggestions

{ self }:

{ config, lib, pkgs, ... }:

let
  cfg = config.services.lm-modal;
in
{
  options.services.lm-modal = {
    enable = lib.mkEnableOption "lm-modal Wayland LLM overlay";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.system}.default;
      description = "lm-modal package to use";
    };

    endpoint = lib.mkOption {
      type = lib.types.str;
      default = "http://localhost:8088";
      example = "http://localhost:11434/v1";
      description = "OpenAI-compatible API endpoint";
    };

    model = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      example = "llama3";
      description = "Model name (null uses endpoint default)";
    };

    timeout = lib.mkOption {
      type = lib.types.int;
      default = 120;
      description = "Request timeout in seconds";
    };

    keyCommand = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      example = "pass show api/openai";
      description = "Command to retrieve API key";
    };

    threadsDir = lib.mkOption {
      type = lib.types.path;
      default = "${config.xdg.dataHome}/lm-modal/threads";
      description = "Thread storage directory";
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = [ cfg.package ];

    xdg.configFile."lm-modal/config.toml".text = lib.generators.toYAML {} {
      endpoint = cfg.endpoint;
      model = cfg.model;
      timeout = cfg.timeout;
    };

    home.sessionVariables = lib.mkIf (cfg.keyCommand != null) {
      LM_MODAL_ENDPOINT = cfg.endpoint;
      LM_MODAL_MODEL = lib.mkIf (cfg.model != null) cfg.model;
    };

    # Example keybind for hyprland (user must add manually)
    # wayland.windowManager.hyprland.settings.bind = [
    #   "SUPER, P, exec, lm-modal"
    # ];
  };
}