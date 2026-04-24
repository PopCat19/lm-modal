# home-manager.nix
#
# Purpose: Home Manager module for lm-modal integration
#
# This module:
# - Installs the lm-modal binary
# - Configures the API endpoint
# - Sets up config file

{ config, lib, pkgs, ... }:

let
  cfg = config.services.lm-modal;
in
{
  options.services.lm-modal = with lib; {
    enable = mkEnableOption "lm-modal Wayland LLM overlay";

    endpoint = mkOption {
      type = types.str;
      default = "http://localhost:8088";
      example = "http://localhost:11434/v1";
      description = "OpenAI-compatible API endpoint";
    };

    model = mkOption {
      type = types.nullOr types.str;
      default = null;
      example = "llama3";
      description = "Model name (null uses endpoint default)";
    };

    timeout = mkOption {
      type = types.int;
      default = 120;
      description = "Request timeout in seconds";
    };

    package = mkOption {
      type = types.package;
      description = "lm-modal package to use";
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = [ cfg.package ];

    xdg.configFile."lm-modal/config.toml".text = lib.generators.toYAML {} {
      endpoint = cfg.endpoint;
      model = cfg.model;
      timeout = cfg.timeout;
    };
  };
}