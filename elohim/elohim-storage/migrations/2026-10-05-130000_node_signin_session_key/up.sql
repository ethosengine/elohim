-- A sign-in session's bound key (RFC 9449 DPoP, adapted to a cookie-carried
-- session): `bound_key` (already present) holds the public JWK as given,
-- `bound_key_alg` its JWS algorithm, `bound_key_thumbprint` its RFC 7638
-- thumbprint, which proofs are compared by. All NULL for an unbound session.
-- Operational (Path C), private to this node, like the table it extends.
ALTER TABLE node_signin_sessions ADD COLUMN bound_key_alg TEXT;
ALTER TABLE node_signin_sessions ADD COLUMN bound_key_thumbprint TEXT;
