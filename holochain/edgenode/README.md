# Elohim Edge Node

Holochain conductor for the Elohim Protocol, enabling browser-to-DHT communication.

## Architecture

```
Browser (Angular) ─── WebSocket ───> Edge Node (Docker) ───> DHT Network
                         │                   │
                         │                   └── holostrap.elohim.host (bootstrap)
                         │
              AdminWs (4444) for setup
              AppWs (4445) for zome calls
```

## Quick Start

```bash
# Start the Edge Node
cd docker/edge-node
docker compose up -d

# View logs
docker compose logs -f edgenode

# Stop
docker compose down
```

## Ports

| Port | Purpose | Protocol |
|------|---------|----------|
| 4444 | Admin WebSocket | ws:// |
| 4445 | App WebSocket | ws:// |

## Browser Connection Flow

1. **Connect to Admin interface**
   ```typescript
   const adminWs = await AdminWebsocket.connect({
     url: new URL("ws://localhost:4444"),
     wsClientOptions: { origin: "elohim-app" },
   });
   ```

2. **Generate agent key and install hApp**
   ```typescript
   const agentKey = await adminWs.generateAgentPubKey();
   await adminWs.installApp({
     agent_key: agentKey,
     path: "./path/to/app.happ",
     installed_app_id: "elohim-lamad",
   });
   await adminWs.enableApp({ installed_app_id: "elohim-lamad" });
   ```

3. **Attach app interface and get token**
   ```typescript
   await adminWs.attachAppInterface({
     port: 4445,
     allowed_origins: "elohim-app"
   });
   const { token } = await adminWs.issueAppAuthenticationToken({
     installed_app_id: "elohim-lamad",
   });
   ```

4. **Connect to App interface for zome calls**
   ```typescript
   const appWs = await AppWebsocket.connect({
     url: new URL("ws://localhost:4445"),
     token,
     wsClientOptions: { origin: "elohim-app" },
   });

   // Make zome calls
   const result = await appWs.callZome({
     cell_id,
     zome_name: "content_store",
     fn_name: "get_content",
     payload: { id: "some-content-id" },
   });
   ```

## Configuration

### Bootstrap Server

The conductor is configured to use `holostrap.elohim.host` for bootstrap and signaling.
Edit `conductor-config.yaml` to change:

```yaml
network:
  bootstrap_url: "https://holostrap.elohim.host"
  signal_url: "wss://holostrap.elohim.host"
```

### Allowed Origins

For production, restrict the admin interface to your domain:

```yaml
admin_interfaces:
  - driver:
      type: websocket
      port: 4444
      allowed_origins:
        - "https://elohim.host"
        - "https://alpha.elohim.host"
```

## Persistence

Data is stored in `./data/` directory:
- Holochain database
- Lair keystore
- Logs

## hApp Installation

After the conductor is running, install your hApp:

```bash
# Copy hApp to container
docker cp ./your-app.happ elohim-edgenode:/tmp/

# Install via admin WebSocket (from your browser app)
# Or use the happ_tool CLI inside the container:
docker exec elohim-edgenode happ_tool install /tmp/your-app.happ
```

## Troubleshooting

### Check conductor status
```bash
docker exec elohim-edgenode ps aux | grep holochain
```

### View logs
```bash
docker exec elohim-edgenode cat /data/logs/holochain.log
```

### Interactive shell
```bash
docker exec -it elohim-edgenode /bin/sh
```

## Version Compatibility

- Edge Node image: ghcr.io/holo-host/edgenode:latest
- Holochain: 0.6.0-dev.28
- @holochain/client: v0.20.x
