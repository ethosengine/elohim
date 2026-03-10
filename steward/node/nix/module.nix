# NixOS module for elohim-node
#
# Usage in your NixOS configuration:
#
#   imports = [ ./path/to/elohim-node/nix/module.nix ];
#
#   services.elohim-node = {
#     enable = true;
#     clusterName = "my-family";
#     clusterKey = "secret-key";
#     settings = {
#       storage.max_capacity = "1TB";
#     };
#   };

{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.elohim-node;

  # Generate TOML configuration from Nix attrset
  configFile = pkgs.writeText "elohim-node.toml" (
    generators.toTOML { } (recursiveUpdate defaultConfig cfg.settings)
  );

  defaultConfig = {
    node = {
      id = cfg.nodeId;
      data_dir = cfg.dataDir;
      cluster_name = cfg.clusterName;
    };
    sync = {
      max_document_size = 10485760;
      sync_interval_ms = 1000;
    };
    cluster = {
      mdns_enabled = cfg.mdnsEnabled;
    };
    p2p = {
      listen_addrs = cfg.listenAddrs;
      bootstrap_nodes = cfg.bootstrapNodes;
      relay_enabled = true;
      max_connections = 100;
    };
    storage = {
      max_capacity = "500GB";
      data_shards = 4;
      parity_shards = 3;
      cache_size = "1GB";
    };
    api = {
      http_enabled = cfg.httpApi.enable;
      http_port = cfg.httpApi.port;
      http_bind = cfg.httpApi.address;
      grpc_enabled = cfg.grpcApi.enable;
      grpc_port = cfg.grpcApi.port;
      grpc_bind = cfg.grpcApi.address;
      metrics_enabled = cfg.metricsEnabled;
    };
    logging = {
      level = cfg.logLevel;
      format = "pretty";
    };
  };

in {
  options.services.elohim-node = {
    enable = mkEnableOption "elohim-node infrastructure daemon";

    package = mkOption {
      type = types.package;
      default = pkgs.elohim-node or (pkgs.callPackage ./package.nix { });
      description = "The elohim-node package to use";
    };

    nodeId = mkOption {
      type = types.str;
      default = config.networking.hostName;
      description = "Unique identifier for this node";
    };

    dataDir = mkOption {
      type = types.path;
      default = "/var/lib/elohim";
      description = "Directory for persistent data storage";
    };

    clusterName = mkOption {
      type = types.str;
      description = "Name of the cluster this node belongs to";
    };

    clusterKey = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = "Shared secret for cluster authentication (use clusterKeyFile for production)";
    };

    clusterKeyFile = mkOption {
      type = types.nullOr types.path;
      default = null;
      description = "File containing the cluster key (more secure than clusterKey)";
    };

    mdnsEnabled = mkOption {
      type = types.bool;
      default = true;
      description = "Enable mDNS for local cluster discovery";
    };

    listenAddrs = mkOption {
      type = types.listOf types.str;
      default = [
        "/ip4/0.0.0.0/tcp/4001"
        "/ip4/0.0.0.0/udp/4001/quic-v1"
      ];
      description = "libp2p listen addresses";
    };

    bootstrapNodes = mkOption {
      type = types.listOf types.str;
      default = [ ];
      description = "Bootstrap nodes for peer discovery";
    };

    httpApi = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = "Enable HTTP management API";
      };

      port = mkOption {
        type = types.port;
        default = 8080;
        description = "HTTP API port";
      };

      address = mkOption {
        type = types.str;
        default = "127.0.0.1";
        description = "HTTP API bind address";
      };
    };

    grpcApi = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = "Enable gRPC API for device clients";
      };

      port = mkOption {
        type = types.port;
        default = 9090;
        description = "gRPC API port";
      };

      address = mkOption {
        type = types.str;
        default = "0.0.0.0";
        description = "gRPC API bind address";
      };
    };

    metricsEnabled = mkOption {
      type = types.bool;
      default = true;
      description = "Enable Prometheus metrics endpoint";
    };

    logLevel = mkOption {
      type = types.enum [ "trace" "debug" "info" "warn" "error" ];
      default = "info";
      description = "Logging level";
    };

    settings = mkOption {
      type = types.attrs;
      default = { };
      description = "Additional settings to merge into configuration";
    };

    openFirewall = mkOption {
      type = types.bool;
      default = false;
      description = "Open firewall ports for elohim-node";
    };
  };

  config = mkIf cfg.enable {
    # Create system user
    users.users.elohim = {
      isSystemUser = true;
      group = "elohim";
      home = cfg.dataDir;
      description = "elohim-node daemon user";
    };
    users.groups.elohim = { };

    # Create data directory
    systemd.tmpfiles.rules = [
      "d '${cfg.dataDir}' 0750 elohim elohim - -"
    ];

    # Systemd service
    systemd.services.elohim-node = {
      description = "Elohim Protocol Infrastructure Node";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];

      environment = {
        RUST_LOG = "info,elohim_node=${cfg.logLevel}";
      } // optionalAttrs (cfg.clusterKey != null) {
        ELOHIM_CLUSTER_KEY = cfg.clusterKey;
      };

      serviceConfig = {
        Type = "simple";
        User = "elohim";
        Group = "elohim";

        ExecStart = "${cfg.package}/bin/elohim-node --config ${configFile}";
        Restart = "always";
        RestartSec = "10s";

        # Security hardening
        NoNewPrivileges = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateTmp = true;
        PrivateDevices = true;
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectControlGroups = true;
        RestrictNamespaces = true;
        RestrictSUIDSGID = true;
        MemoryDenyWriteExecute = true;
        LockPersonality = true;

        # Allow network and data dir
        ReadWritePaths = [ cfg.dataDir ];
        CapabilityBoundingSet = [ "CAP_NET_BIND_SERVICE" ];
        AmbientCapabilities = [ "CAP_NET_BIND_SERVICE" ];
      } // optionalAttrs (cfg.clusterKeyFile != null) {
        LoadCredential = "cluster-key:${cfg.clusterKeyFile}";
      };
    };

    # Firewall rules
    networking.firewall = mkIf cfg.openFirewall {
      allowedTCPPorts = [
        4001                              # libp2p TCP
      ] ++ optional cfg.httpApi.enable cfg.httpApi.port
        ++ optional cfg.grpcApi.enable cfg.grpcApi.port;

      allowedUDPPorts = [
        4001                              # libp2p QUIC
        5353                              # mDNS
      ];
    };
  };
}
