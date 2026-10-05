//! The node's own person's sign-in account, and the sessions sign-in proves.
//!
//! The node is the authorization server for its own person (the OAuth model).
//! A sign-in word and a secret prove the person once; the session that proves
//! it then lets a browser on another machine act with their authority. The
//! rules are `consent_grant::signin`; this module keeps the account and its
//! sessions in this node's own database and does the hashing.
//!
//! Private to this node: the account never travels (not on the DHT, a source
//! chain, the declaration, a sync or a projection). The secret is kept only
//! as an Argon2id verifier, with the `argon2` crate's defaults, the same the
//! doorway uses for its accounts (Argon2id v19, m = 19456 KiB, t = 2, p = 1,
//! a 16-byte salt, a 32-byte output, PHC string). A lost secret is set again
//! from the node's own machine: that is the deterministic floor.
//!
//! A session proven by sign-in is a row keyed by the SHA-256 of a random
//! 32-byte token the cookie carries. It lives `SESSION_LIFE_MICROS` (thirty
//! days) and ends at sign-out or when a new secret is set.

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use consent_grant::{SignInRefusal, SESSION_LIFE_MICROS};
use diesel::prelude::*;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::db::diesel_schema::{node_account, node_signin_sessions};
use crate::error::StorageError;

/// The longest secret accepted, so hashing work stays bounded.
pub const MAX_SECRET_LEN: usize = 1024;
/// The shortest secret accepted.
pub const MIN_SECRET_LEN: usize = 8;
/// The cookie a session proven by sign-in rides in: the node's one session
/// cookie, so `/auth/me` and the portal read either kind.
pub const SESSION_COOKIE: &str = "elohim_session";

/// Sign-in attempts, per source and for the account, for this process.
pub fn limiter() -> &'static consent_grant::AttemptLimiter {
    static LIMITER: std::sync::OnceLock<consent_grant::AttemptLimiter> = std::sync::OnceLock::new();
    LIMITER.get_or_init(consent_grant::AttemptLimiter::new)
}

/// How many secrets may be hashed at once: each takes about 19 MiB.
pub const VERIFY_PERMITS: usize = 2;

/// The permits for hashing a presented secret.
pub fn verify_permits() -> &'static tokio::sync::Semaphore {
    static PERMITS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(VERIFY_PERMITS);
    &PERMITS
}

/// The account, as stored.
#[derive(Debug, Clone, PartialEq, Eq, Queryable, Selectable)]
#[diesel(table_name = node_account)]
pub struct Account {
    pub id: i32,
    pub identifier: String,
    pub display_name: Option<String>,
    pub human_id: String,
    pub agent_pub_key: String,
    pub verifier: Option<String>,
    pub secret_set_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Account {
    pub fn secret_set(&self) -> bool {
        self.verifier.is_some()
    }
}

fn db(e: diesel::result::Error) -> StorageError {
    StorageError::Database(e.to_string())
}

fn now_text() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// The account, if this node has one.
pub fn account(conn: &mut SqliteConnection) -> Result<Option<Account>, StorageError> {
    use node_account::dsl as a;
    a::node_account
        .filter(a::id.eq(1))
        .select(Account::as_select())
        .first(conn)
        .optional()
        .map_err(db)
}

/// Record whose account this is: the word the person signs in with, their
/// name as given, and the identity it names. Keeps any secret already set,
/// unless the identity changed, which drops it.
pub fn record_identity(
    conn: &mut SqliteConnection,
    identifier: &str,
    display_name: Option<&str>,
    human_id: &str,
    agent_pub_key: &str,
) -> Result<(), StorageError> {
    use node_account::dsl as a;
    let now = now_text();
    let same_identity = account(conn)?
        .is_some_and(|acct| acct.human_id == human_id && acct.agent_pub_key == agent_pub_key);
    if same_identity {
        diesel::update(a::node_account.filter(a::id.eq(1)))
            .set((
                a::identifier.eq(identifier),
                a::display_name.eq(display_name),
                a::updated_at.eq(&now),
            ))
            .execute(conn)
            .map_err(db)?;
        return Ok(());
    }
    diesel::delete(a::node_account).execute(conn).map_err(db)?;
    end_all_sessions(conn)?;
    diesel::insert_into(a::node_account)
        .values((
            a::id.eq(1),
            a::identifier.eq(identifier),
            a::display_name.eq(display_name),
            a::human_id.eq(human_id),
            a::agent_pub_key.eq(agent_pub_key),
            a::verifier.eq(None::<String>),
            a::secret_set_at.eq(None::<String>),
            a::created_at.eq(&now),
            a::updated_at.eq(&now),
        ))
        .execute(conn)
        .map_err(db)?;
    Ok(())
}

/// Why a secret was not set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretRefusal {
    TooShort,
    TooLong,
    NoAccount,
}

impl SecretRefusal {
    pub fn code(&self) -> &'static str {
        match self {
            Self::TooShort => "secret_too_short",
            Self::TooLong => "secret_too_long",
            Self::NoAccount => "secret_no_identity",
        }
    }

    pub fn words(&self) -> String {
        match self {
            Self::TooShort => format!("a sign-in secret is at least {MIN_SECRET_LEN} characters"),
            Self::TooLong => format!("a sign-in secret is at most {MAX_SECRET_LEN} characters"),
            Self::NoAccount => "this node has no identity to sign in as yet; begin one first \
                                (epr identity begin)"
                .to_string(),
        }
    }
}

/// Whether `secret` is one this node will keep.
pub fn secret_fits(secret: &str) -> Result<(), SecretRefusal> {
    let n = secret.chars().count();
    if n < MIN_SECRET_LEN {
        return Err(SecretRefusal::TooShort);
    }
    if secret.len() > MAX_SECRET_LEN {
        return Err(SecretRefusal::TooLong);
    }
    Ok(())
}

/// The Argon2id verifier for `secret`, with a fresh salt.
pub fn hash_secret(secret: &str) -> Result<String, StorageError> {
    let mut salt = [0u8; 16];
    getrandom::fill(&mut salt).map_err(|e| StorageError::Internal(format!("salt: {e}")))?;
    let salt = SaltString::encode_b64(&salt)
        .map_err(|e| StorageError::Internal(format!("salt encoding: {e}")))?;
    Argon2::default()
        .hash_password(secret.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| StorageError::Internal(format!("hashing: {e}")))
}

fn verifies(secret: &str, verifier: &str) -> bool {
    PasswordHash::new(verifier).ok().is_some_and(|parsed| {
        Argon2::default()
            .verify_password(secret.as_bytes(), &parsed)
            .is_ok()
    })
}

/// Set the secret (already hashed). Every session it proved before ends.
/// Returns how many ended.
pub fn set_verifier(conn: &mut SqliteConnection, verifier: &str) -> Result<usize, SecretRefusal> {
    use node_account::dsl as a;
    let now = now_text();
    let updated = diesel::update(a::node_account.filter(a::id.eq(1)))
        .set((
            a::verifier.eq(verifier),
            a::secret_set_at.eq(&now),
            a::updated_at.eq(&now),
        ))
        .execute(conn)
        .map_err(|_| SecretRefusal::NoAccount)?;
    if updated == 0 {
        return Err(SecretRefusal::NoAccount);
    }
    end_all_sessions(conn).map_err(|_| SecretRefusal::NoAccount)
}

/// A verifier no secret matches, checked when the word is wrong so a wrong
/// word costs the same work as a wrong secret. Made once; the server makes it
/// at start, so the first wrong word does not take longer than a wrong secret.
pub fn decoy_verifier() -> &'static str {
    static DECOY: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    DECOY.get_or_init(|| {
        hash_secret("a decoy no one signs in with").unwrap_or_else(|_| {
            "$argon2id$v=19$m=19456,t=2,p=1$ZGVjb3lkZWNveWRlY295$\
             ZGVjb3lkZWNveWRlY295ZGVjb3lkZWNveWRlY295ZGU"
                .to_string()
        })
    })
}

fn digest(text: &str) -> [u8; 32] {
    Sha256::digest(text.as_bytes()).into()
}

/// The sign-in verdict: whether `identifier` and `secret` prove this node's
/// person. The word is compared in constant time (as digests, so its length
/// is not compared either), and the secret is always hashed, against a decoy
/// when the word is wrong, so a wrong word and a wrong secret cost the same
/// and answer the same.
pub fn sign_in_verdict(
    account: Option<&Account>,
    identifier: &str,
    secret: &str,
) -> Result<(), SignInRefusal> {
    let Some(account) = account else {
        return Err(SignInRefusal::SecretUnset);
    };
    let Some(verifier) = account.verifier.as_deref() else {
        return Err(SignInRefusal::SecretUnset);
    };
    if secret.len() > MAX_SECRET_LEN {
        return Err(SignInRefusal::InvalidCredentials);
    }
    let word_matches: bool = digest(identifier.trim())
        .ct_eq(&digest(account.identifier.trim()))
        .into();
    let checked = if word_matches {
        verifier
    } else {
        decoy_verifier()
    };
    let secret_matches = verifies(secret, checked);
    if word_matches && secret_matches {
        Ok(())
    } else {
        Err(SignInRefusal::InvalidCredentials)
    }
}

/// Open a session proven by sign-in, returning the token for the cookie.
pub fn open_session(
    conn: &mut SqliteConnection,
    source: &str,
    now_micros: i64,
) -> Result<String, StorageError> {
    use node_signin_sessions::dsl as s;
    let mut raw = [0u8; 32];
    getrandom::fill(&mut raw).map_err(|e| StorageError::Internal(format!("token: {e}")))?;
    let token = URL_SAFE_NO_PAD.encode(raw);
    // bounded-work: expired sessions go before a new one is added.
    diesel::delete(s::node_signin_sessions.filter(s::expires_at_micros.le(now_micros)))
        .execute(conn)
        .map_err(db)?;
    diesel::insert_into(s::node_signin_sessions)
        .values((
            s::token_digest.eq(hex::encode(digest(&token))),
            s::created_at_micros.eq(now_micros),
            s::expires_at_micros.eq(now_micros.saturating_add(SESSION_LIFE_MICROS)),
            s::source.eq(source),
            s::proven_by.eq(PROVEN_BY_PASSWORD),
            s::bound_key.eq(None::<String>),
        ))
        .execute(conn)
        .map_err(db)?;
    Ok(token)
}

/// How a session was proven.
pub const PROVEN_BY_PASSWORD: &str = "password";

/// A live session proven by sign-in, as stored.
#[derive(Debug, Clone, PartialEq, Eq, Queryable, Selectable)]
#[diesel(table_name = node_signin_sessions)]
pub struct ProvenSession {
    pub expires_at_micros: i64,
    pub source: String,
    /// How the person proved themselves: `password` today.
    pub proven_by: String,
    /// A public key the session is bound to. Always `None` today; when a
    /// later step binds one, each signing request must also carry a
    /// signature from it, checked in [`request_proven`].
    pub bound_key: Option<String>,
}

/// The live session proven by sign-in that `token` names, if any.
pub fn proven_session(
    conn: &mut SqliteConnection,
    token: &str,
    now_micros: i64,
) -> Result<Option<ProvenSession>, StorageError> {
    use node_signin_sessions::dsl as s;
    s::node_signin_sessions
        .filter(s::token_digest.eq(hex::encode(digest(token))))
        .filter(s::expires_at_micros.gt(now_micros))
        .select(ProvenSession::as_select())
        .first(conn)
        .optional()
        .map_err(db)
}

/// Whether `token` names a live session proven by sign-in.
pub fn session_proven(
    conn: &mut SqliteConnection,
    token: &str,
    now_micros: i64,
) -> Result<bool, StorageError> {
    Ok(proven_session(conn, token, now_micros)?.is_some())
}

/// The token a request's session cookie carries, if any.
pub fn session_token(headers: &hyper::HeaderMap) -> Option<String> {
    let cookie = headers.get(hyper::header::COOKIE)?.to_str().ok()?;
    cookie.split(';').find_map(|pair| {
        let value = pair
            .trim()
            .strip_prefix(SESSION_COOKIE)?
            .strip_prefix('=')?;
        let value = value.trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

/// THE question every signing route asks: does this request carry a session
/// proven by sign-in for this node's own person? Today the cookie's session
/// is the whole proof. A session bound to a key (`bound_key`) would also need
/// the request signed by that key; this is where that check goes, so no
/// route changes when it does. No such session exists today, so a bound one
/// is refused here rather than accepted on its cookie alone.
pub fn request_proven(
    conn: &mut SqliteConnection,
    headers: &hyper::HeaderMap,
    now_micros: i64,
) -> Result<bool, StorageError> {
    let Some(token) = session_token(headers) else {
        return Ok(false);
    };
    Ok(match proven_session(conn, &token, now_micros)? {
        Some(session) => session.bound_key.is_none(),
        None => false,
    })
}

/// End the session `token` names. Whether one was ended.
pub fn end_session(conn: &mut SqliteConnection, token: &str) -> Result<bool, StorageError> {
    use node_signin_sessions::dsl as s;
    let n = diesel::delete(
        s::node_signin_sessions.filter(s::token_digest.eq(hex::encode(digest(token)))),
    )
    .execute(conn)
    .map_err(db)?;
    Ok(n > 0)
}

fn end_all_sessions(conn: &mut SqliteConnection) -> Result<usize, StorageError> {
    diesel::delete(node_signin_sessions::table)
        .execute(conn)
        .map_err(db)
}

/// The `Set-Cookie` value for a session proven by sign-in. `Secure` when the
/// sign-in arrived over TLS; `Max-Age` only when the person asked to be
/// remembered, otherwise the browser forgets it when it closes (the session
/// itself still ends in thirty days at most).
pub fn session_cookie(token: &str, secure: bool, remember: bool) -> String {
    let mut cookie = format!("{SESSION_COOKIE}={token}; HttpOnly; SameSite=Strict; Path=/");
    if remember {
        cookie.push_str(&format!("; Max-Age={}", SESSION_LIFE_MICROS / 1_000_000));
    }
    if secure {
        cookie.push_str("; Secure");
    }
    cookie
}

/// The `Set-Cookie` value that ends the cookie in the browser.
pub fn cleared_cookie(secure: bool) -> String {
    let mut cookie = format!("{SESSION_COOKIE}=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0");
    if secure {
        cookie.push_str("; Secure");
    }
    cookie
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::Connection;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn conn() -> SqliteConnection {
        let mut c = SqliteConnection::establish(":memory:").unwrap();
        c.run_pending_migrations(MIGRATIONS).unwrap();
        c
    }

    fn with_secret(c: &mut SqliteConnection, secret: &str) -> Account {
        record_identity(c, "matthew", Some("Matthew"), "h-1", "uhCAkme").unwrap();
        set_verifier(c, &hash_secret(secret).unwrap()).unwrap();
        account(c).unwrap().unwrap()
    }

    #[test]
    fn the_account_is_the_lasting_home_of_the_sign_in_word() {
        let mut c = conn();
        assert_eq!(account(&mut c).unwrap(), None);
        record_identity(&mut c, "matthew", Some("Matthew"), "h-1", "uhCAkme").unwrap();
        let a = account(&mut c).unwrap().unwrap();
        assert_eq!((a.identifier.as_str(), a.secret_set()), ("matthew", false));
        // The same identity keeps its secret; another identity drops it.
        set_verifier(&mut c, &hash_secret("correct horse").unwrap()).unwrap();
        record_identity(&mut c, "matt", Some("Matthew"), "h-1", "uhCAkme").unwrap();
        let a = account(&mut c).unwrap().unwrap();
        assert!(a.secret_set() && a.identifier == "matt");
        record_identity(&mut c, "other", None, "h-2", "uhCAkme").unwrap();
        assert!(!account(&mut c).unwrap().unwrap().secret_set());
    }

    #[test]
    fn a_wrong_word_and_a_wrong_secret_answer_the_same() {
        let mut c = conn();
        let a = with_secret(&mut c, "correct horse");
        assert_eq!(
            sign_in_verdict(Some(&a), "matthew", "correct horse"),
            Ok(())
        );
        assert_eq!(
            sign_in_verdict(Some(&a), " matthew ", "correct horse"),
            Ok(())
        );
        let wrong_secret = sign_in_verdict(Some(&a), "matthew", "battery staple");
        let wrong_word = sign_in_verdict(Some(&a), "someone", "correct horse");
        assert_eq!(wrong_secret, Err(SignInRefusal::InvalidCredentials));
        assert_eq!(wrong_word, wrong_secret);
        assert_eq!(
            sign_in_verdict(None, "matthew", "x"),
            Err(SignInRefusal::SecretUnset)
        );
        let mut unset = a.clone();
        unset.verifier = None;
        assert_eq!(
            sign_in_verdict(Some(&unset), "matthew", "correct horse"),
            Err(SignInRefusal::SecretUnset)
        );
        assert!(a
            .verifier
            .unwrap()
            .starts_with("$argon2id$v=19$m=19456,t=2,p=1$"));
    }

    #[test]
    fn a_session_lives_thirty_days_and_ends_at_sign_out_or_a_new_secret() {
        let mut c = conn();
        with_secret(&mut c, "correct horse");
        let now = 1_000_000;
        let token = open_session(&mut c, "10.0.0.9", now).unwrap();
        assert!(token.len() >= 43);
        assert!(session_proven(&mut c, &token, now + 1).unwrap());
        let found = proven_session(&mut c, &token, now + 1).unwrap().unwrap();
        assert_eq!(
            (found.proven_by.as_str(), found.bound_key),
            (PROVEN_BY_PASSWORD, None)
        );
        let mut headers = hyper::HeaderMap::new();
        headers.insert(
            hyper::header::COOKIE,
            format!("theme=dark; {SESSION_COOKIE}={token}")
                .parse()
                .unwrap(),
        );
        assert!(request_proven(&mut c, &headers, now + 1).unwrap());
        assert!(!request_proven(&mut c, &hyper::HeaderMap::new(), now + 1).unwrap());
        assert!(!session_proven(&mut c, &token, now + SESSION_LIFE_MICROS).unwrap());
        assert!(!session_proven(&mut c, "guess", now).unwrap());
        assert!(end_session(&mut c, &token).unwrap());
        assert!(!session_proven(&mut c, &token, now + 1).unwrap());
        let again = open_session(&mut c, "10.0.0.9", now).unwrap();
        assert_eq!(
            set_verifier(&mut c, &hash_secret("new secret!").unwrap()),
            Ok(1)
        );
        assert!(!session_proven(&mut c, &again, now + 1).unwrap());
    }

    #[test]
    fn the_cookie_is_strict_http_only_and_secure_over_tls() {
        let c = session_cookie("t", true, true);
        assert!(c.contains("HttpOnly") && c.contains("SameSite=Strict") && c.contains("; Secure"));
        assert!(c.contains("Max-Age=2592000"));
        let plain = session_cookie("t", false, false);
        assert!(!plain.contains("Secure") && !plain.contains("Max-Age"));
        assert!(cleared_cookie(false).contains("Max-Age=0"));
        assert_eq!(secret_fits("short"), Err(SecretRefusal::TooShort));
        assert_eq!(secret_fits("long enough"), Ok(()));
    }
}
