#!/usr/bin/env python3
"""Outside-in SBD cross-relay probe: does the MongoSignalBus deliver frames
CROSS-RELAY between the two public signal servers?

The end-to-end runtime proof of the D2 bus plane (`signal/bus_mongo.rs`)
that needs NO cluster access — it speaks the SBD wire protocol
(`signal/cmd.rs`: connect wss://<relay>/<pubkey-b64url>, answer `areq` with
an ed25519 signature, wait `srdy`, then send [32B dest pubkey][payload])
against the public relay endpoints. Complements the `#[ignore]` unit proof
`frame_published_on_a_is_drained_by_b_not_a`, which needs MONGODB_TEST_URI.

Legs:
  1  (control): both clients on relay A  -> local forward must work
  1b (control): both clients on relay B
  2: sender on A, receiver on B          -> requires bus cross-pod delivery
  3: sender on B, receiver on A          -> same, reverse direction

Run: python3 tools/sbd-cross-relay-probe.py   (needs: pip install pynacl websockets)

History: 2026-07-11 (dht-unity plan T1) — all four legs DELIVERED against
signal.doorway-alpha.elohim.host / signal.elohim.host, refuting the bus as
the cross-conductor-fetch blocker and pushing the seam below the relay
layer (tx5/ICE — see the plan's seam map).
"""
import asyncio, sys, time
import nacl.signing
import websockets
from base64 import urlsafe_b64encode

RELAY_A = "wss://signal.doorway-alpha.elohim.host"  # matthew side
RELAY_B = "wss://signal.elohim.host"                # adam side
CMD_PREFIX = b"\x00" * 28

def b64u(b): return urlsafe_b64encode(b).rstrip(b"=").decode()

class Client:
    def __init__(self, relay, name):
        self.relay, self.name = relay, name
        self.sk = nacl.signing.SigningKey.generate()
        self.pk = bytes(self.sk.verify_key)
        self.ws = None
        self.inbox = asyncio.Queue()

    async def connect(self):
        url = f"{self.relay}/{b64u(self.pk)}"
        self.ws = await websockets.connect(url, max_size=25_000, open_timeout=15)
        # handshake: consume lbrt/lidl, answer areq, wait srdy
        deadline = time.time() + 15
        while time.time() < deadline:
            raw = await asyncio.wait_for(self.ws.recv(), timeout=10)
            if len(raw) < 32: continue
            if raw[:28] == CMD_PREFIX:
                cmd = raw[28:32]
                if cmd == b"areq":
                    nonce = raw[32:64]
                    sig = self.sk.sign(nonce).signature
                    await self.ws.send(CMD_PREFIX + b"ares" + sig)
                elif cmd == b"srdy":
                    print(f"  {self.name} authenticated on {self.relay}")
                    asyncio.create_task(self._pump())
                    return True
                # lbrt / lidl: ignore
        raise TimeoutError(f"{self.name}: no srdy")

    async def _pump(self):
        try:
            async for raw in self.ws:
                if len(raw) >= 32 and raw[:28] != CMD_PREFIX:
                    await self.inbox.put((raw[:32], raw[32:]))
        except Exception:
            pass

    async def send_to(self, dest_pk, payload):
        await self.ws.send(dest_pk + payload)

    async def expect(self, payload, timeout):
        deadline = time.time() + timeout
        while time.time() < deadline:
            try:
                sender, body = await asyncio.wait_for(self.inbox.get(), timeout=max(0.1, deadline - time.time()))
                if body == payload:
                    return True
            except asyncio.TimeoutError:
                return False
        return False

async def leg(name, sender_relay, receiver_relay, timeout=12):
    s = Client(sender_relay, f"{name}-sender")
    r = Client(receiver_relay, f"{name}-receiver")
    await s.connect(); await r.connect()
    payload = f"dht-unity-T1-{name}-{time.time()}".encode()
    await s.send_to(r.pk, payload)
    ok = await r.expect(payload, timeout)
    print(f"LEG {name}: {'DELIVERED' if ok else 'NOT DELIVERED'} ({sender_relay} -> {receiver_relay})")
    await s.ws.close(); await r.ws.close()
    return ok

async def main():
    print("== leg 1: same-relay control on A ==")
    l1 = await leg("control-A", RELAY_A, RELAY_A, timeout=8)
    print("== leg 1b: same-relay control on B ==")
    l1b = await leg("control-B", RELAY_B, RELAY_B, timeout=8)
    print("== leg 2: cross A->B (needs mongo bus) ==")
    l2 = await leg("cross-AB", RELAY_A, RELAY_B)
    print("== leg 3: cross B->A (needs mongo bus) ==")
    l3 = await leg("cross-BA", RELAY_B, RELAY_A)
    print(f"\nVERDICT: control-A={l1} control-B={l1b} cross-AB={l2} cross-BA={l3}")

asyncio.run(main())
