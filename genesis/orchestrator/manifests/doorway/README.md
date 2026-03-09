# Doorway Manifests

Doorway is the agency on-ramp — separate from the P2P StatefulSet, it provides five services in one Deployment:

1. **DNS/TLS gateway** — stateless HTTP/WebSocket routing
2. **Bootstrap/Signal** — agent discovery + WebRTC relay for the DHT
3. **Projection cache** — serves DHT content via REST API
4. **Identity host** — custodial agent keys for users transitioning from web2 to P2P
5. **Recovery registrar** — relationship-based identity recovery contracts

Connects to the edgenode StatefulSet via ClusterIP service for conductor and storage access.
Scaling is via the conductor pool behind doorway, not doorway replicas.

See `doorway/SCALING.md` for the full scaling model (graduation flywheel, conductor pool, human topology).
