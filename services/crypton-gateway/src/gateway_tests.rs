//! Router-level and end-to-end gateway tests.
//!
//! These drive the real external/internal Axum routers (full middleware stack)
//! with `tower::ServiceExt::oneshot` against a throwaway Redis + Postgres and a
//! local stub upstream. The `e2e_*` tests additionally run the REAL
//! `crypton-id` binary as the upstream and use tokens minted by its production
//! `issue_jwt`. All need isolated infrastructure, so they are `#[ignore]`d by
//! default and run with:
//!
//!   GATEWAY_TEST_REDIS_URL=redis://127.0.0.1:16379 \
//!   GATEWAY_TEST_DATABASE_URL=postgres://postgres:<pw>@127.0.0.1:15432/gwtest \
//!   cargo test -- --ignored
//!
//! See `services/crypton-gateway/README.md` ("Integration tests"). Never point
//! these at a shared or production Redis/Postgres: the tests write keys, create
//! and drop databases, and apply the identity migrations.
//!
//! Tests named `known_*` pin CURRENT behaviour that is a gap or limitation (not
//! a statement that it is desired), so they flip loudly when it changes.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::{
    body::Body,
    extract::{ConnectInfo, Request as AxRequest},
    http::{Method, Request, Response, StatusCode},
    Router,
};
use jsonwebtoken::{encode, Algorithm, DecodingKey, EncodingKey, Header};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use crate::state::{AppState, Metrics};
use crate::{build_external_router, build_internal_router};

const INTERNAL_TOKEN: &str = "gateway-test-internal-token";

// Fixed, NON-SECRET Ed25519 test keypairs. They are the same committed test
// keys as `crypton-id/src/jwt.rs` `test_keys` (used only for tests, never for
// anything real), so a token signed here is verifiable exactly as a token from
// `crypton-id` would be under the matching public key.
const SIGNING_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----\nMC4CAQAwBQYDK2VwBCIEIPCsvnOOwpwO3ny53kAPCm5XlSE5EGIvrgZbm/+R9Wn2\n-----END PRIVATE KEY-----\n";
const PUBLIC_KEY_PEM: &str = "-----BEGIN PUBLIC KEY-----\nMCowBQYDK2VwAyEAh206LonzWwDNeN7FFpFDgkFBuXf/NqtYZulHoqvWIt8=\n-----END PUBLIC KEY-----\n";
const OTHER_SIGNING_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----\nMC4CAQAwBQYDK2VwBCIEIDFfh6j6XSEabG1I0g5ZYbnicE9hTC4SI0nTDJ/N1Vyi\n-----END PRIVATE KEY-----\n";

// ── Harness ───────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct Seen {
    path: String,
    headers: Vec<(String, String)>,
}

impl Seen {
    fn header_all(&self, name: &str) -> Vec<String> {
        self.headers
            .iter()
            .filter(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
            .collect()
    }
}

/// Stub identity upstream: records every request it receives. `/boom` -> 500.
async fn spawn_upstream() -> (String, Arc<Mutex<Vec<Seen>>>) {
    let seen: Arc<Mutex<Vec<Seen>>> = Arc::new(Mutex::new(vec![]));
    let rec = seen.clone();
    let app = Router::new().fallback(move |req: AxRequest| {
        let rec = rec.clone();
        async move {
            let headers = req
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();
            let path = req.uri().path().to_string();
            rec.lock().unwrap().push(Seen {
                path: path.clone(),
                headers,
            });
            if path == "/boom" {
                (StatusCode::INTERNAL_SERVER_ERROR, "upstream-boom")
            } else {
                (StatusCode::OK, "upstream-ok")
            }
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}"), seen)
}

fn infra() -> (String, String) {
    let redis = std::env::var("GATEWAY_TEST_REDIS_URL")
        .expect("GATEWAY_TEST_REDIS_URL must point at a THROWAWAY redis");
    let pg = std::env::var("GATEWAY_TEST_DATABASE_URL")
        .expect("GATEWAY_TEST_DATABASE_URL must point at a THROWAWAY postgres");
    (redis, pg)
}

/// Create a brand-new empty database on the throwaway Postgres server.
async fn fresh_database() -> String {
    let (_, base) = infra();
    let admin = PgPool::connect(&base).await.unwrap();
    let name = format!("gwt_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE DATABASE {name}"))
        .execute(&admin)
        .await
        .unwrap();
    let idx = base.rfind('/').unwrap();
    format!("{}/{}", &base[..idx], name)
}

/// Apply the real `crypton-id` migrations once per database (they are not all
/// idempotent), serialised across tests with an advisory lock.
async fn apply_identity_migrations(pg: &PgPool) {
    let mut lock = pg.acquire().await.unwrap();
    sqlx::query("SELECT pg_advisory_lock(7770001)")
        .execute(&mut *lock)
        .await
        .unwrap();
    let applied: (Option<String>,) = sqlx::query_as("SELECT to_regclass('audit_logs')::text")
        .fetch_one(&mut *lock)
        .await
        .unwrap();
    if applied.0.is_none() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../crypton-id/migrations");
        let mut files: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().map(|e| e == "sql").unwrap_or(false))
            .collect();
        files.sort();
        for f in files {
            let sql = std::fs::read_to_string(&f).unwrap();
            sqlx::raw_sql(&sql)
                .execute(pg)
                .await
                .unwrap_or_else(|e| panic!("migration {f:?} failed: {e}"));
        }
    }
    sqlx::query("SELECT pg_advisory_unlock(7770001)")
        .execute(&mut *lock)
        .await
        .unwrap();
}

async fn build_state(redis_url: &str, pg_url: &str, identity_base: String) -> AppState {
    let client = redis::Client::open(redis_url.to_string()).unwrap();
    // Short retry/timeout budget so a dead Redis surfaces as an error quickly.
    let cfg = redis::aio::ConnectionManagerConfig::new()
        .set_number_of_retries(1)
        .set_connection_timeout(Duration::from_secs(2))
        .set_response_timeout(Duration::from_secs(2));
    let conn = redis::aio::ConnectionManager::new_with_config(client, cfg)
        .await
        .unwrap();
    let pg = PgPool::connect(pg_url).await.unwrap();
    AppState {
        redis: Arc::new(conn),
        pg,
        http_client: reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap(),
        identity_base,
        metrics: Arc::new(Metrics::default()),
        policy_enforce: true,
        jwt_decoding_key: DecodingKey::from_ed_pem(PUBLIC_KEY_PEM.as_bytes()).unwrap(),
        internal_token: INTERNAL_TOKEN.to_string(),
        gateway_secret: None,
    }
}

struct Env {
    state: AppState,
    ext: Router,
    int: Router,
    seen: Arc<Mutex<Vec<Seen>>>,
}

impl Env {
    fn from_state(state: AppState, seen: Arc<Mutex<Vec<Seen>>>) -> Env {
        Env {
            ext: build_external_router(state.clone()),
            int: build_internal_router(state.clone()),
            state,
            seen,
        }
    }

    /// Shared throwaway DB, stub upstream.
    async fn new() -> Env {
        let (redis, pg_url) = infra();
        let (up, seen) = spawn_upstream().await;
        let state = build_state(&redis, &pg_url, up).await;
        apply_identity_migrations(&state.pg).await;
        Env::from_state(state, seen)
    }

    /// Own empty database (migrated), stub upstream.
    async fn with_own_database() -> Env {
        let (redis, _) = infra();
        let url = fresh_database().await;
        let (up, seen) = spawn_upstream().await;
        let state = build_state(&redis, &url, up).await;
        apply_identity_migrations(&state.pg).await;
        Env::from_state(state, seen)
    }

    /// Redis reached through a killable TCP proxy.
    async fn with_killable_redis() -> (Env, RedisProxy) {
        let (redis, pg_url) = infra();
        let proxy = spawn_redis_proxy(&redis).await;
        let (up, seen) = spawn_upstream().await;
        let state = build_state(&proxy.url, &pg_url, up).await;
        apply_identity_migrations(&state.pg).await;
        (Env::from_state(state, seen), proxy)
    }

    async fn ident(&self) -> Ident {
        seed_identity(&self.state.pg).await
    }

    fn upstream_hits(&self, path: &str) -> usize {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .filter(|s| s.path == path)
            .count()
    }

    async fn status(&self, req: Req) -> StatusCode {
        self.send(req).await.status()
    }

    async fn send(&self, req: Req) -> Response<Body> {
        self.ext.clone().oneshot(req.build()).await.unwrap()
    }

    async fn metrics(&self) -> String {
        let req = Request::builder()
            .uri("/internal/metrics")
            .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 1))))
            .body(Body::empty())
            .unwrap();
        body_string(self.int.clone().oneshot(req).await.unwrap()).await
    }

    /// Decision rows for one request id: `(outcome, user_id, credential_id, detail)`.
    async fn decisions(
        &self,
        request_id: &str,
    ) -> Vec<(String, Option<Uuid>, Option<Uuid>, serde_json::Value)> {
        sqlx::query_as(
            "SELECT outcome, user_id, credential_id, detail FROM audit_logs \
             WHERE actor = 'gateway' AND detail->>'request_id' = $1",
        )
        .bind(request_id)
        .fetch_all(&self.state.pg)
        .await
        .unwrap()
    }

    async fn redis(&self) -> redis::aio::ConnectionManager {
        self.state.redis.as_ref().clone()
    }
}

#[derive(Clone)]
struct Ident {
    user: Uuid,
    cred: Uuid,
    username: String,
}

async fn seed_identity(pg: &PgPool) -> Ident {
    let i = Ident {
        user: Uuid::new_v4(),
        cred: Uuid::new_v4(),
        username: format!("user-{}", Uuid::new_v4().simple()),
    };
    sqlx::query("INSERT INTO users (id, username) VALUES ($1, $2)")
        .bind(i.user)
        .bind(&i.username)
        .execute(pg)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO credentials (id, user_id, credential_id, public_key, status) \
         VALUES ($1, $2, $3, $4, 'active')",
    )
    .bind(i.cred)
    .bind(i.user)
    .bind(Uuid::new_v4().as_bytes().to_vec())
    .bind(vec![0u8; 8])
    .execute(pg)
    .await
    .unwrap();
    i
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn identity_header() -> Header {
    // Exactly what crypton-id's issue_jwt sets.
    let mut h = Header::new(Algorithm::EdDSA);
    h.kid = Some("v1".to_string());
    h
}

fn claims_json(i: &Ident, exp_offset: i64) -> serde_json::Value {
    let iat = now();
    json!({
        "sub": i.user.to_string(),
        "username": i.username,
        "cred_id": i.cred.to_string(),
        "iat": iat,
        "exp": (iat as i64 + exp_offset) as u64,
    })
}

/// A token shaped exactly like one `crypton-id` issues, signed with the test key.
fn mint(i: &Ident, exp_offset: i64) -> String {
    encode(
        &identity_header(),
        &claims_json(i, exp_offset),
        &EncodingKey::from_ed_pem(SIGNING_KEY_PEM.as_bytes()).unwrap(),
    )
    .unwrap()
}

fn token(i: &Ident) -> String {
    mint(i, 3600)
}

fn b64url(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16
            | (*c.get(1).unwrap_or(&0) as u32) << 8
            | *c.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        if c.len() > 1 {
            out.push(T[(n >> 6) as usize & 63] as char);
        }
        if c.len() > 2 {
            out.push(T[n as usize & 63] as char);
        }
    }
    out
}

fn b64url_decode(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let (mut buf, mut bits) = (0u32, 0);
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            _ => continue,
        } as u32;
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    out
}

fn rid() -> String {
    format!("t-{}", Uuid::new_v4())
}

/// Request builder. Every request gets a fresh nonce + current timestamp unless
/// overridden; `peer` is the TCP peer the listener would have seen.
struct Req {
    method: Method,
    path: String,
    bearer: Option<String>,
    nonce: Option<String>,
    timestamp: Option<String>,
    extra: Vec<(&'static str, String)>,
    peer: SocketAddr,
}

impl Req {
    fn new(method: Method, path: &str) -> Self {
        Req {
            method,
            path: path.to_string(),
            bearer: None,
            nonce: Some(Uuid::new_v4().to_string()),
            timestamp: Some(now().to_string()),
            extra: vec![],
            peer: random_public_peer(),
        }
    }
    fn get(path: &str) -> Self {
        Self::new(Method::GET, path)
    }
    fn post(path: &str) -> Self {
        Self::new(Method::POST, path)
    }
    fn bearer(mut self, t: String) -> Self {
        self.bearer = Some(t);
        self
    }
    fn auth(self, i: &Ident) -> Self {
        self.bearer(token(i))
    }
    fn nonce(mut self, n: Option<&str>) -> Self {
        self.nonce = n.map(str::to_string);
        self
    }
    fn timestamp(mut self, t: Option<String>) -> Self {
        self.timestamp = t;
        self
    }
    fn header(mut self, k: &'static str, v: &str) -> Self {
        self.extra.push((k, v.to_string()));
        self
    }
    fn request_id(self, id: &str) -> Self {
        self.header("x-request-id", id)
    }
    fn peer(mut self, p: SocketAddr) -> Self {
        self.peer = p;
        self
    }
    fn build(self) -> Request<Body> {
        let mut b = Request::builder().method(self.method).uri(&self.path);
        if let Some(t) = &self.bearer {
            b = b.header("authorization", format!("Bearer {t}"));
        }
        if let Some(n) = &self.nonce {
            b = b.header("x-nonce", n);
        }
        if let Some(t) = &self.timestamp {
            b = b.header("x-timestamp", t);
        }
        for (k, v) in &self.extra {
            b = b.header(*k, v);
        }
        b.extension(ConnectInfo(self.peer))
            .body(Body::empty())
            .unwrap()
    }
}

/// Random address in 100.64.0.0/10 (CGNAT): not loopback/RFC-1918, so
/// X-Forwarded-For is NOT trusted unless a test says otherwise, and random so
/// per-IP Redis counters never collide across tests.
fn random_public_peer() -> SocketAddr {
    let b = Uuid::new_v4();
    let b = b.as_bytes();
    SocketAddr::from(([100, 64 + (b[0] % 64), b[1], b[2].max(1)], 40000))
}

async fn body_string(resp: Response<Body>) -> String {
    let b = axum::body::to_bytes(resp.into_body(), 1 << 20)
        .await
        .unwrap();
    String::from_utf8_lossy(&b).to_string()
}

fn metric(text: &str, name: &str) -> u64 {
    text.lines()
        .find_map(|l| l.strip_prefix(&format!("{name} ")))
        .unwrap_or_else(|| panic!("metric {name} missing"))
        .trim()
        .parse()
        .unwrap()
}

/// A TCP forwarder in front of Redis that can be killed to simulate an outage
/// (no docker access needed from the test).
struct RedisProxy {
    url: String,
    tasks: Arc<Mutex<Vec<tokio::task::AbortHandle>>>,
}

impl RedisProxy {
    fn kill(&self) {
        for h in self.tasks.lock().unwrap().drain(..) {
            h.abort();
        }
    }
}

async fn spawn_redis_proxy(target_url: &str) -> RedisProxy {
    let target = target_url.trim_start_matches("redis://").to_string();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let tasks: Arc<Mutex<Vec<tokio::task::AbortHandle>>> = Arc::new(Mutex::new(vec![]));
    let t2 = tasks.clone();
    let accept = tokio::spawn(async move {
        loop {
            let Ok((mut inbound, _)) = listener.accept().await else {
                return;
            };
            let target = target.clone();
            let conn = tokio::spawn(async move {
                if let Ok(mut out) = tokio::net::TcpStream::connect(target).await {
                    let _ = tokio::io::copy_bidirectional(&mut inbound, &mut out).await;
                }
            });
            t2.lock().unwrap().push(conn.abort_handle());
        }
    });
    tasks.lock().unwrap().push(accept.abort_handle());
    RedisProxy {
        url: format!("redis://{addr}"),
        tasks,
    }
}

fn ingest_req(token: Option<&str>, ev: &serde_json::Value, request_id: &str) -> Request<Body> {
    let mut b = Request::builder()
        .method("POST")
        .uri("/internal/audit/ingest")
        .header("content-type", "application/json")
        .header("x-request-id", request_id)
        .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 1))));
    if let Some(t) = token {
        b = b.header("x-internal-token", t);
    }
    b.body(Body::from(ev.to_string())).unwrap()
}

async fn ingest(env: &Env, token: Option<&str>, ev: &serde_json::Value, rid: &str) -> StatusCode {
    env.int
        .clone()
        .oneshot(ingest_req(token, ev, rid))
        .await
        .unwrap()
        .status()
}

async fn redis_get(env: &Env, key: &str) -> Option<i64> {
    redis::cmd("GET")
        .arg(key)
        .query_async(&mut env.redis().await)
        .await
        .unwrap()
}

// ── 1. Identity token contract ────────────────────────────────────────────────

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn health_is_open_and_unauthenticated_protected_requests_fail_closed() {
    let env = Env::new().await;
    assert_eq!(env.status(Req::get("/health")).await, StatusCode::OK);
    let id = rid();
    assert_eq!(
        env.status(Req::get("/profile").request_id(&id)).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(env.upstream_hits("/profile"), 0);
    let d = env.decisions(&id).await;
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].0, "deny");
    assert_eq!(d[0].3["reason"], "missing_authorization_header");
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn valid_identity_token_is_accepted_and_invalid_ones_are_rejected_with_reasons() {
    let env = Env::new().await;
    let i = env.ident().await;

    // Control: a well-formed token for an active credential is accepted.
    assert_eq!(
        env.status(Req::get("/profile").auth(&i)).await,
        StatusCode::OK
    );

    let other = EncodingKey::from_ed_pem(OTHER_SIGNING_KEY_PEM.as_bytes()).unwrap();
    let own = EncodingKey::from_ed_pem(SIGNING_KEY_PEM.as_bytes()).unwrap();
    let good = token(&i);
    let parts: Vec<&str> = good.split('.').collect();

    // Tampered payload with the original signature: change the subject.
    let mut forged_claims: serde_json::Value =
        serde_json::from_slice(&b64url_decode(parts[1])).unwrap();
    forged_claims["sub"] = json!(Uuid::new_v4().to_string());
    let tampered = format!(
        "{}.{}.{}",
        parts[0],
        b64url(forged_claims.to_string().as_bytes()),
        parts[2]
    );

    // alg confusion: HS256 token whose HMAC secret is the identity PUBLIC key.
    let mut hs = Header::new(Algorithm::HS256);
    hs.kid = Some("v1".into());
    let hs256_confusion = encode(
        &hs,
        &claims_json(&i, 3600),
        &EncodingKey::from_secret(PUBLIC_KEY_PEM.as_bytes()),
    )
    .unwrap();

    // alg none (unsigned).
    let none = format!(
        "{}.{}.",
        b64url(br#"{"alg":"none","typ":"JWT","kid":"v1"}"#),
        b64url(claims_json(&i, 3600).to_string().as_bytes())
    );

    let mut no_kid = identity_header();
    no_kid.kid = None;
    let mut wrong_kid = identity_header();
    wrong_kid.kid = Some("v2".into());

    let mut missing_cred = claims_json(&i, 3600);
    missing_cred.as_object_mut().unwrap().remove("cred_id");
    let mut bad_sub = claims_json(&i, 3600);
    bad_sub["sub"] = json!("not-a-uuid");
    let mut bad_cred = claims_json(&i, 3600);
    bad_cred["cred_id"] = json!("not-a-uuid");

    let cases: Vec<(&str, String, &str)> = vec![
        ("garbage", "not.a.jwt".into(), "token_malformed"),
        (
            "wrong signing key",
            encode(&identity_header(), &claims_json(&i, 3600), &other).unwrap(),
            "token_signature_invalid",
        ),
        ("tampered payload", tampered, "token_signature_invalid"),
        ("expired", mint(&i, -3600), "token_expired"),
        (
            "hs256 key confusion",
            hs256_confusion,
            "token_algorithm_not_allowed",
        ),
        ("alg none", none, "token_malformed"), /* jsonwebtoken cannot even parse alg=none */
        (
            "missing kid",
            encode(&no_kid, &claims_json(&i, 3600), &own).unwrap(),
            "token_unknown_key_id",
        ),
        (
            "unknown kid",
            encode(&wrong_kid, &claims_json(&i, 3600), &own).unwrap(),
            "token_unknown_key_id",
        ),
        (
            "missing cred_id claim",
            encode(&identity_header(), &missing_cred, &own).unwrap(),
            "token_claims_invalid",
        ),
        (
            "non-uuid subject",
            encode(&identity_header(), &bad_sub, &own).unwrap(),
            "token_subject_not_uuid",
        ),
        (
            "non-uuid cred_id",
            encode(&identity_header(), &bad_cred, &own).unwrap(),
            "token_credential_not_uuid",
        ),
    ];

    for (label, tok, reason) in cases {
        let id = rid();
        let st = env
            .status(Req::get("/profile").bearer(tok).request_id(&id))
            .await;
        assert_eq!(st, StatusCode::UNAUTHORIZED, "{label}");
        let d = env.decisions(&id).await;
        assert_eq!(d.len(), 1, "{label}: exactly one decision row");
        assert_eq!(d[0].0, "deny", "{label}");
        assert_eq!(d[0].3["reason"], reason, "{label}");
    }
    // Only the control request ever reached the upstream.
    assert_eq!(env.upstream_hits("/profile"), 1);

    // Non-Bearer scheme.
    let req = Request::builder()
        .uri("/profile")
        .header("authorization", format!("Basic {}", token(&i)))
        .header("x-nonce", Uuid::new_v4().to_string())
        .header("x-timestamp", now().to_string())
        .extension(ConnectInfo(random_public_peer()))
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        env.ext.clone().oneshot(req).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn revoked_lost_foreign_and_deleted_credentials_are_rejected() {
    let env = Env::new().await;
    let pg = &env.state.pg;

    let a = env.ident().await;
    assert_eq!(
        env.status(Req::get("/profile").auth(&a)).await,
        StatusCode::OK
    );

    for status in ["revoked", "lost"] {
        let i = env.ident().await;
        sqlx::query("UPDATE credentials SET status = $1 WHERE id = $2")
            .bind(status)
            .bind(i.cred)
            .execute(pg)
            .await
            .unwrap();
        let id = rid();
        assert_eq!(
            env.status(Req::get("/profile").auth(&i).request_id(&id))
                .await,
            StatusCode::UNAUTHORIZED,
            "{status}"
        );
        assert_eq!(
            env.decisions(&id).await[0].3["reason"],
            "credential_not_active"
        );
    }

    // Token for user A naming user B's (active) credential.
    let b = env.ident().await;
    let mut crossed = a.clone();
    crossed.cred = b.cred;
    assert_eq!(
        env.status(Req::get("/profile").auth(&crossed)).await,
        StatusCode::UNAUTHORIZED
    );

    // Credential row deleted.
    let d = env.ident().await;
    sqlx::query("DELETE FROM credentials WHERE id = $1")
        .bind(d.cred)
        .execute(pg)
        .await
        .unwrap();
    assert_eq!(
        env.status(Req::get("/profile").auth(&d)).await,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn logged_out_token_is_rejected_and_other_tokens_are_not() {
    let env = Env::new().await;
    let i = env.ident().await;
    let tok = token(&i);
    let iat = {
        let p: serde_json::Value =
            serde_json::from_slice(&b64url_decode(tok.split('.').nth(1).unwrap())).unwrap();
        p["iat"].as_u64().unwrap()
    };
    // The key crypton-id's logout writes.
    let _: () = redis::cmd("SET")
        .arg(format!("jwt:deny:{}:{}", i.user, iat))
        .arg(1)
        .arg("EX")
        .arg(60)
        .query_async(&mut env.redis().await)
        .await
        .unwrap();
    let id = rid();
    assert_eq!(
        env.status(Req::get("/profile").bearer(tok).request_id(&id))
            .await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(env.decisions(&id).await[0].3["reason"], "token_revoked");
    // A different user is unaffected.
    let other = env.ident().await;
    assert_eq!(
        env.status(Req::get("/profile").auth(&other)).await,
        StatusCode::OK
    );
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn redis_outage_fails_closed_for_authenticated_requests() {
    let (env, proxy) = Env::with_killable_redis().await;
    let i = env.ident().await;
    assert_eq!(
        env.status(Req::get("/profile").auth(&i)).await,
        StatusCode::OK
    );
    let before = env.upstream_hits("/profile");

    proxy.kill();
    let id = rid();
    let st = env
        .status(Req::get("/profile").auth(&i).request_id(&id))
        .await;
    assert_eq!(st, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        env.upstream_hits("/profile"),
        before,
        "no request reached upstream"
    );
    assert_eq!(
        env.decisions(&id).await[0].3["reason"],
        "revocation_check_unavailable"
    );
}

// ── 2. Public authentication routes ───────────────────────────────────────────

const PUBLIC: [(&str, &str); 7] = [
    ("POST", "/auth/register/start"),
    ("POST", "/auth/register/finish"),
    ("POST", "/auth/login/start"),
    ("POST", "/auth/login/finish"),
    ("POST", "/recovery/public/start"),
    ("GET", "/recovery/public/status"),
    ("POST", "/recovery/public/complete"),
];

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn allowlisted_bootstrap_routes_work_without_a_token() {
    let env = Env::new().await;
    for (m, p) in PUBLIC {
        let id = rid();
        let st = env
            .status(Req::new(m.parse().unwrap(), p).request_id(&id))
            .await;
        assert_eq!(st, StatusCode::OK, "{m} {p}");
        let d = env.decisions(&id).await;
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].0, "allow");
        assert_eq!(d[0].3["reason"], "allowed_public_route");
        assert!(d[0].1.is_none());
    }
    assert_eq!(env.upstream_hits("/auth/login/start"), 1);
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn near_miss_routes_cannot_bypass_authentication() {
    let env = Env::new().await;
    let attempts: Vec<(Method, &str)> = vec![
        (Method::GET, "/auth/login/start"),
        (Method::PUT, "/auth/login/start"),
        (Method::DELETE, "/auth/login/start"),
        (Method::PATCH, "/auth/login/start"),
        (Method::POST, "/auth/login/start/"),
        (Method::POST, "/auth/login/start/extra"),
        (Method::POST, "/auth/login"),
        (Method::POST, "/AUTH/login/start"),
        (Method::POST, "/auth//login/start"),
        (Method::POST, "/auth/login/start/../../me"),
        (Method::POST, "/auth/login/%73tart"),
        (Method::POST, "/%61uth/login/start"),
        (Method::POST, "/auth/login/start%2f..%2f..%2fme"),
        (Method::POST, "/api/auth/login/start"),
        (Method::POST, "/auth/logout"),
        (Method::POST, "/recovery/start"),
        (Method::POST, "/recovery/public/status"),
        (Method::GET, "/recovery/public/start"),
        (Method::GET, "/me"),
        (Method::GET, "/"),
    ];
    for (m, p) in attempts {
        let st = env.status(Req::new(m.clone(), p)).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED, "{m} {p}");
    }
    assert!(
        env.seen.lock().unwrap().is_empty(),
        "no near-miss request reached the upstream"
    );
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn public_routes_keep_nonce_timestamp_and_rate_limit_protections() {
    let env = Env::new().await;
    let path = "/auth/login/start";
    assert_eq!(
        env.status(Req::post(path).nonce(None)).await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        env.status(Req::post(path).timestamp(None)).await,
        StatusCode::BAD_REQUEST
    );

    // Replay from the same peer is rejected (anonymous scope: ip+method+path).
    let peer = random_public_peer();
    let n = Uuid::new_v4().to_string();
    assert_eq!(
        env.status(Req::post(path).peer(peer).nonce(Some(&n))).await,
        StatusCode::OK
    );
    assert_eq!(
        env.status(Req::post(path).peer(peer).nonce(Some(&n))).await,
        StatusCode::CONFLICT
    );
    // Same nonce value from another peer is a different anonymous request.
    assert_eq!(
        env.status(Req::post(path).peer(random_public_peer()).nonce(Some(&n)))
            .await,
        StatusCode::OK
    );

    // Per-IP limit (100/min) applies to the public bootstrap route.
    let peer = random_public_peer();
    let mut first_429 = None;
    for i in 1..=105 {
        if env.status(Req::post(path).peer(peer)).await == StatusCode::TOO_MANY_REQUESTS
            && first_429.is_none()
        {
            first_429 = Some(i);
        }
    }
    assert_eq!(first_429, Some(101));
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn a_presented_token_on_a_public_route_must_still_be_valid() {
    let env = Env::new().await;
    let i = env.ident().await;
    let path = "/auth/register/start";

    // Valid token: accepted and attributed to the user.
    let id = rid();
    assert_eq!(
        env.status(Req::post(path).auth(&i).request_id(&id)).await,
        StatusCode::OK
    );
    let d = env.decisions(&id).await;
    assert_eq!((d[0].0.as_str(), d[0].1), ("allow", Some(i.user)));

    // Invalid token is not silently downgraded to anonymous.
    assert_eq!(
        env.status(Req::post(path).bearer(mint(&i, -3600))).await,
        StatusCode::UNAUTHORIZED
    );
    let revoked = env.ident().await;
    sqlx::query("UPDATE credentials SET status = 'revoked' WHERE id = $1")
        .bind(revoked.cred)
        .execute(&env.state.pg)
        .await
        .unwrap();
    assert_eq!(
        env.status(Req::post(path).auth(&revoked)).await,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn cors_preflight_for_a_public_route_is_answered_without_a_token() {
    let env = Env::new().await;
    let req = Request::builder()
        .method("OPTIONS")
        .uri("/auth/login/start")
        .header("origin", "http://localhost:3000")
        .header("access-control-request-method", "POST")
        .extension(ConnectInfo(random_public_peer()))
        .body(Body::empty())
        .unwrap();
    let resp = env.ext.clone().oneshot(req).await.unwrap();
    assert!(resp.status().is_success());
    assert_eq!(
        resp.headers().get("access-control-allow-origin").unwrap(),
        "http://localhost:3000"
    );
    assert!(env.seen.lock().unwrap().is_empty());
}

// ── 3. Authorization ──────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn internal_diagnostics_require_admin_role_from_the_database() {
    let env = Env::new().await;
    let path = |u: Uuid| format!("/internal/diagnostics/user/{u}");
    let call = |p: String, tok: Option<String>| {
        let mut b = Request::builder()
            .uri(p)
            .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 1))));
        if let Some(t) = tok {
            b = b.header("authorization", format!("Bearer {t}"));
        }
        b.body(Body::empty()).unwrap()
    };
    let status = |r: Request<Body>| {
        let int = env.int.clone();
        async move { int.oneshot(r).await.unwrap().status() }
    };

    let user = env.ident().await;
    assert_eq!(
        status(call(path(user.user), None)).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        status(call(path(user.user), Some(token(&user)))).await,
        StatusCode::FORBIDDEN
    );

    let admin = env.ident().await;
    sqlx::query("UPDATE users SET role = 'admin' WHERE id = $1")
        .bind(admin.user)
        .execute(&env.state.pg)
        .await
        .unwrap();
    assert_eq!(
        status(call(path(user.user), Some(token(&admin)))).await,
        StatusCode::OK
    );

    // A role claim inside the token is never trusted: role is read from the DB.
    let mut forged = claims_json(&user, 3600);
    forged["role"] = json!("admin");
    let tok = encode(
        &identity_header(),
        &forged,
        &EncodingKey::from_ed_pem(SIGNING_KEY_PEM.as_bytes()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        status(call(path(user.user), Some(tok))).await,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn locked_account_is_denied_by_policy_after_audit_failures() {
    // End to end: ingested login_failed events -> Redis lock -> risk engine
    // account_locked -> policy blocks the proxied request.
    let env = Env::new().await;
    let i = env.ident().await;
    assert_eq!(
        env.status(Req::get("/profile").auth(&i)).await,
        StatusCode::OK
    );

    for _ in 0..6 {
        let ev = json!({"event_type":"login_failed","user_id":i.user.to_string(),"details":{}});
        assert_eq!(
            ingest(&env, Some(INTERNAL_TOKEN), &ev, &rid()).await,
            StatusCode::ACCEPTED
        );
    }
    let id = rid();
    let after = env
        .status(Req::get("/profile").auth(&i).request_id(&id))
        .await;
    assert!(
        after == StatusCode::FORBIDDEN || after == StatusCode::UNAUTHORIZED,
        "locked account must not reach upstream, got {after}"
    );
    assert_eq!(
        env.upstream_hits("/profile"),
        1,
        "only the pre-lock request reached upstream"
    );
    let d = env.decisions(&id).await;
    assert_eq!(d[0].0, "deny");
    assert!(d[0].3["reason"].as_str().unwrap().starts_with("policy_"));
    assert_eq!(d[0].1, Some(i.user), "denial attributed to the user");
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn revoked_device_header_is_blocked_but_another_users_device_is_ignored() {
    let env = Env::new().await;
    let owner = env.ident().await;
    let other = env.ident().await;
    // A second, revoked credential owned by `owner`.
    let revoked_dev = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO credentials (id, user_id, credential_id, public_key, status) \
         VALUES ($1, $2, $3, $4, 'revoked')",
    )
    .bind(revoked_dev)
    .bind(owner.user)
    .bind(Uuid::new_v4().as_bytes().to_vec())
    .bind(vec![0u8; 8])
    .execute(&env.state.pg)
    .await
    .unwrap();

    assert_eq!(
        env.status(Req::get("/profile").auth(&owner)).await,
        StatusCode::OK,
        "control"
    );
    let st = env
        .status(
            Req::get("/profile")
                .auth(&owner)
                .header("x-device-id", &revoked_dev.to_string()),
        )
        .await;
    assert!(
        st == StatusCode::FORBIDDEN || st == StatusCode::UNAUTHORIZED,
        "got {st}"
    );
    assert_eq!(
        env.status(
            Req::get("/profile")
                .auth(&other)
                .header("x-device-id", &revoked_dev.to_string())
        )
        .await,
        StatusCode::OK
    );
}

// ── 4. Authorized request reaches the upstream ───────────────────────────────

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn authorized_request_reaches_upstream_with_trusted_client_ip() {
    let env = Env::new().await;
    let i = env.ident().await;
    let peer = SocketAddr::from(([100, 100, 113, 77], 5555));
    let resp = env
        .send(
            Req::get("/profile")
                .auth(&i)
                .peer(peer)
                .header("x-forwarded-for", "10.0.0.1")
                .header("cf-ipcountry", "XX"),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(body_string(resp).await, "upstream-ok");
    {
        let seen = env.seen.lock().unwrap();
        let hit = seen.iter().find(|s| s.path == "/profile").unwrap();
        assert_eq!(
            hit.header_all("x-forwarded-for"),
            vec!["100.100.113.77".to_string()]
        );
        assert!(hit.header_all("cf-ipcountry").is_empty());
        assert_eq!(hit.header_all("authorization").len(), 1);
    }
    let m = env.metrics().await;
    assert_eq!(metric(&m, "gateway_auth_jwt_accepted_total"), 1);
    assert_eq!(metric(&m, "gateway_proxy_upstream_2xx_total"), 1);
}

// ── 5. Rate limiting ──────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn per_user_rate_limit_applies_to_sensitive_paths_only() {
    let env = Env::new().await;
    let i = env.ident().await;
    for _ in 0..70 {
        assert_ne!(
            env.status(Req::get("/profile").auth(&i)).await,
            StatusCode::TOO_MANY_REQUESTS
        );
    }
    let mut first_429 = None;
    for n in 1..=65 {
        if env.status(Req::get("/auth/ping").auth(&i)).await == StatusCode::TOO_MANY_REQUESTS
            && first_429.is_none()
        {
            first_429 = Some(n);
        }
    }
    assert_eq!(first_429, Some(61));
    assert!(metric(&env.metrics().await, "gateway_rate_limit_block_user_total") >= 1);
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn per_device_rate_limit_applies() {
    let env = Env::new().await;
    let device = Uuid::new_v4().to_string();
    let mut first_429 = None;
    for n in 1..=55 {
        // Public route: no token needed; a fresh peer each time so only the
        // device counter can trip.
        if env
            .status(Req::post("/auth/login/start").header("x-device-id", &device))
            .await
            == StatusCode::TOO_MANY_REQUESTS
            && first_429.is_none()
        {
            first_429 = Some(n);
        }
    }
    assert_eq!(first_429, Some(51));
}

// ── 6. Nonce / replay protection ─────────────────────────────────────────────

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn nonce_and_timestamp_are_required_and_same_user_replay_is_rejected() {
    let env = Env::new().await;
    let i = env.ident().await;
    assert_eq!(
        env.status(Req::get("/profile").auth(&i).nonce(None)).await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        env.status(Req::get("/profile").auth(&i).timestamp(None))
            .await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        env.status(
            Req::get("/profile")
                .auth(&i)
                .timestamp(Some((now() - 3600).to_string()))
        )
        .await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        env.status(
            Req::get("/profile")
                .auth(&i)
                .timestamp(Some("yesterday".into()))
        )
        .await,
        StatusCode::BAD_REQUEST
    );
    // Malformed nonces (charset / length).
    for bad in ["has space", "star*", &"a".repeat(129)] {
        assert_eq!(
            env.status(Req::get("/profile").auth(&i).nonce(Some(bad)))
                .await,
            StatusCode::BAD_REQUEST,
            "{bad}"
        );
    }

    let n = Uuid::new_v4().to_string();
    assert_eq!(
        env.status(Req::get("/profile").auth(&i).nonce(Some(&n)))
            .await,
        StatusCode::OK
    );
    let id = rid();
    assert_eq!(
        env.status(
            Req::get("/profile")
                .auth(&i)
                .nonce(Some(&n))
                .request_id(&id)
        )
        .await,
        StatusCode::CONFLICT
    );
    assert_eq!(env.upstream_hits("/profile"), 1);
    let d = env.decisions(&id).await;
    assert_eq!(
        (d[0].3["reason"].as_str().unwrap(), d[0].1),
        ("nonce_replayed", Some(i.user))
    );

    // Expiry is bounded: the key exists with a TTL no larger than 300 s, which
    // exceeds the 2x60 s window in which the timestamp check would still pass.
    let ttl: i64 = redis::cmd("TTL")
        .arg(format!("nonce:u:{}:{n}", i.user))
        .query_async(&mut env.redis().await)
        .await
        .unwrap();
    assert!((1..=300).contains(&ttl), "ttl {ttl}");
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn nonce_is_scoped_to_the_principal_and_switching_identity_does_not_enable_replay() {
    let env = Env::new().await;
    let a = env.ident().await;
    let b = env.ident().await;
    let n = Uuid::new_v4().to_string();

    assert_eq!(
        env.status(Req::get("/profile").auth(&a).nonce(Some(&n)))
            .await,
        StatusCode::OK
    );
    // Design: a different principal using the same nonce value is a different
    // request, accepted, and it does not burn or disturb A's nonce space...
    assert_eq!(
        env.status(Req::get("/profile").auth(&b).nonce(Some(&n)))
            .await,
        StatusCode::OK
    );
    // ...while replaying A's captured request (A's token) from anywhere is still
    // A's scope and is rejected.
    assert_eq!(
        env.status(
            Req::get("/profile")
                .auth(&a)
                .nonce(Some(&n))
                .peer(random_public_peer())
        )
        .await,
        StatusCode::CONFLICT
    );
    // And B cannot replay either.
    assert_eq!(
        env.status(Req::get("/profile").auth(&b).nonce(Some(&n)))
            .await,
        StatusCode::CONFLICT
    );
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn concurrent_reuse_of_one_nonce_admits_exactly_one_request() {
    let env = Env::new().await;
    let i = env.ident().await;
    let tok = token(&i);
    let n = Uuid::new_v4().to_string();
    let mut tasks = vec![];
    for _ in 0..24 {
        let app = env.ext.clone();
        let req = Req::get("/profile")
            .bearer(tok.clone())
            .nonce(Some(&n))
            .build();
        tasks.push(tokio::spawn(async move {
            app.oneshot(req).await.unwrap().status()
        }));
    }
    let mut ok = 0;
    let mut conflict = 0;
    for t in tasks {
        match t.await.unwrap() {
            StatusCode::OK => ok += 1,
            StatusCode::CONFLICT => conflict += 1,
            other => panic!("unexpected {other}"),
        }
    }
    assert_eq!((ok, conflict), (1, 23));
    assert_eq!(env.upstream_hits("/profile"), 1);
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn redis_outage_does_not_disable_replay_protection() {
    let (env, proxy) = Env::with_killable_redis().await;
    let i = env.ident().await;
    let n = Uuid::new_v4().to_string();
    assert_eq!(
        env.status(Req::get("/profile").auth(&i).nonce(Some(&n)))
            .await,
        StatusCode::OK
    );

    proxy.kill();
    // Replayed and brand-new nonces are both refused, never silently allowed.
    for nonce in [n.clone(), Uuid::new_v4().to_string()] {
        let st = env
            .status(Req::get("/profile").auth(&i).nonce(Some(&nonce)))
            .await;
        assert_eq!(st, StatusCode::SERVICE_UNAVAILABLE);
    }
    assert_eq!(env.upstream_hits("/profile"), 1);
}

// ── 7. Audit ingest ──────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn audit_ingest_authenticates_validates_persists_and_applies_signals() {
    let env = Env::new().await;
    let i = env.ident().await;
    let ev = json!({"event_type":"login_failed","user_id":i.user.to_string(),"details":{"ip":"192.0.2.1"}});

    assert_eq!(
        ingest(&env, None, &ev, &rid()).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        ingest(&env, Some("wrong"), &ev, &rid()).await,
        StatusCode::FORBIDDEN
    );
    let bad_user = json!({"event_type":"login_failed","user_id":"not-a-uuid","details":{}});
    assert_eq!(
        ingest(&env, Some(INTERNAL_TOKEN), &bad_user, &rid()).await,
        StatusCode::BAD_REQUEST
    );
    let bad_type = json!({"event_type":"Bad Type","user_id":null,"details":{}});
    assert_eq!(
        ingest(&env, Some(INTERNAL_TOKEN), &bad_type, &rid()).await,
        StatusCode::BAD_REQUEST
    );
    let bad_dev = json!({"event_type":"device_added","user_id":null,"details":{"device_id":"x:y"}});
    assert_eq!(
        ingest(&env, Some(INTERNAL_TOKEN), &bad_dev, &rid()).await,
        StatusCode::BAD_REQUEST
    );

    // Rejected events had no effect anywhere.
    assert_eq!(
        redis_get(&env, &format!("velocity:failed:{}:1h", i.user)).await,
        None
    );
    let n: (i64,) = sqlx::query_as("SELECT count(*) FROM audit_logs WHERE user_id = $1")
        .bind(i.user)
        .fetch_one(&env.state.pg)
        .await
        .unwrap();
    assert_eq!(n.0, 0);

    // Accepted: persisted to audit_logs (the identity table) AND applied to Redis.
    let id = rid();
    assert_eq!(
        ingest(&env, Some(INTERNAL_TOKEN), &ev, &id).await,
        StatusCode::ACCEPTED
    );
    let row: (String, String, Option<Uuid>, serde_json::Value) = sqlx::query_as(
        "SELECT action, outcome, user_id, detail FROM audit_logs \
         WHERE actor = 'gateway-ingest' AND detail->>'request_id' = $1",
    )
    .bind(&id)
    .fetch_one(&env.state.pg)
    .await
    .unwrap();
    assert_eq!(
        (row.0.as_str(), row.1.as_str(), row.2),
        ("login_failed", "recorded", Some(i.user))
    );
    assert_eq!(row.3["event"]["ip"], "192.0.2.1");
    assert_eq!(
        redis_get(&env, &format!("velocity:failed:{}:1h", i.user)).await,
        Some(1)
    );
    assert_eq!(
        redis_get(&env, &format!("login:failed:{}", i.user)).await,
        Some(1)
    );

    // login_success clears the lock/failure counters.
    let ok = json!({"event_type":"login_success","user_id":i.user.to_string(),"details":{}});
    assert_eq!(
        ingest(&env, Some(INTERNAL_TOKEN), &ok, &rid()).await,
        StatusCode::ACCEPTED
    );
    assert_eq!(
        redis_get(&env, &format!("login:failed:{}", i.user)).await,
        None
    );

    // Unknown (but well-formed) user: row stored with NULL user_id, id kept in detail.
    let ghost = Uuid::new_v4();
    let gid = rid();
    let ev = json!({"event_type":"login_failed","user_id":ghost.to_string(),"details":{}});
    assert_eq!(
        ingest(&env, Some(INTERNAL_TOKEN), &ev, &gid).await,
        StatusCode::ACCEPTED
    );
    let row: (Option<Uuid>, serde_json::Value) =
        sqlx::query_as("SELECT user_id, detail FROM audit_logs WHERE detail->>'request_id' = $1")
            .bind(&gid)
            .fetch_one(&env.state.pg)
            .await
            .unwrap();
    assert_eq!(row.0, None);
    assert_eq!(row.1["claimed_user_id"], ghost.to_string());
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn audit_ingest_leaves_no_partial_state_when_redis_fails() {
    let (env, proxy) = Env::with_killable_redis().await;
    let i = env.ident().await;
    proxy.kill();
    let id = rid();
    let ev = json!({"event_type":"login_failed","user_id":i.user.to_string(),"details":{}});
    assert_eq!(
        ingest(&env, Some(INTERNAL_TOKEN), &ev, &id).await,
        StatusCode::SERVICE_UNAVAILABLE
    );
    let n: (i64,) =
        sqlx::query_as("SELECT count(*) FROM audit_logs WHERE detail->>'request_id' = $1")
            .bind(&id)
            .fetch_one(&env.state.pg)
            .await
            .unwrap();
    assert_eq!(
        n.0, 0,
        "audit row must be rolled back when the Redis effect failed"
    );
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn audit_ingest_leaves_no_partial_state_when_postgres_fails() {
    let env = Env::with_own_database().await;
    let i = env.ident().await;
    sqlx::query("DROP TABLE audit_logs")
        .execute(&env.state.pg)
        .await
        .unwrap();
    let ev = json!({"event_type":"login_failed","user_id":i.user.to_string(),"details":{}});
    assert_eq!(
        ingest(&env, Some(INTERNAL_TOKEN), &ev, &rid()).await,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(
        redis_get(&env, &format!("velocity:failed:{}:1h", i.user)).await,
        None
    );
    assert_eq!(
        redis_get(&env, &format!("login:failed:{}", i.user)).await,
        None
    );
}

// ── 8. Decision trail ────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn every_request_has_one_accurate_durable_decision_without_secrets() {
    let env = Env::new().await;
    let i = env.ident().await;

    let allow_id = rid();
    let tok = token(&i);
    let resp = env
        .send(
            Req::get("/profile?secret=abc")
                .bearer(tok.clone())
                .request_id(&allow_id),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let d = env.decisions(&allow_id).await;
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].0, "allow");
    assert_eq!((d[0].1, d[0].2), (Some(i.user), Some(i.cred)));
    assert_eq!(d[0].3["reason"], "allowed");
    assert_eq!(d[0].3["path"], "/profile", "query string is never stored");
    assert_eq!(d[0].3["method"], "GET");
    let blob = d[0].3.to_string();
    for secret in [tok.as_str(), "abc", "authorization", "Bearer"] {
        assert!(!blob.contains(secret), "decision detail leaked {secret}");
    }

    // Denials by each control, with the right reason and attribution.
    let n = Uuid::new_v4().to_string();
    let _ = env
        .status(Req::get("/profile").auth(&i).nonce(Some(&n)))
        .await;
    let cases: Vec<(&str, Req, &str, bool)> = vec![
        (
            "nonce replay",
            Req::get("/profile").auth(&i).nonce(Some(&n)),
            "nonce_replayed",
            true,
        ),
        (
            "missing nonce",
            Req::get("/profile").auth(&i).nonce(None),
            "nonce_missing",
            true,
        ),
        (
            "stale timestamp",
            Req::get("/profile").auth(&i).timestamp(Some("1".into())),
            "timestamp_out_of_window",
            true,
        ),
        (
            "no token",
            Req::get("/profile"),
            "missing_authorization_header",
            false,
        ),
    ];
    for (label, req, reason, attributed) in cases {
        let id = rid();
        let st = env.status(req.request_id(&id)).await;
        assert!(st.is_client_error(), "{label}");
        let d = env.decisions(&id).await;
        assert_eq!(d.len(), 1, "{label}");
        assert_eq!(
            (d[0].0.as_str(), d[0].3["reason"].as_str().unwrap()),
            ("deny", reason),
            "{label}"
        );
        assert_eq!(d[0].1.is_some(), attributed, "{label}");
        assert_eq!(d[0].3["status"], st.as_u16(), "{label}");
    }

    // Rate limit denial.
    let limited = env.ident().await;
    let mut last = None;
    for _ in 0..62 {
        let id = rid();
        if env
            .status(Req::get("/auth/ping").auth(&limited).request_id(&id))
            .await
            == StatusCode::TOO_MANY_REQUESTS
        {
            last = Some(id);
        }
    }
    let d = env.decisions(&last.expect("rate limited")).await;
    assert_eq!(d[0].3["reason"], "rate_limited_user");
    assert_eq!(d[0].1, Some(limited.user));

    // Counters match what was recorded.
    let m = env.metrics().await;
    assert!(metric(&m, "gateway_audit_decisions_allowed_total") >= 1);
    assert!(metric(&m, "gateway_audit_decisions_denied_total") >= 5);
    assert_eq!(metric(&m, "gateway_audit_write_failed_total"), 0);
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn allow_fails_closed_when_the_decision_cannot_be_recorded_but_denials_still_hold() {
    let env = Env::with_own_database().await;
    let i = env.ident().await;
    assert_eq!(
        env.status(Req::get("/profile").auth(&i)).await,
        StatusCode::OK,
        "control"
    );
    let before = env.upstream_hits("/profile");

    sqlx::query("DROP TABLE audit_logs")
        .execute(&env.state.pg)
        .await
        .unwrap();

    // Allowed-by-policy request is refused rather than executed unaudited.
    assert_eq!(
        env.status(Req::get("/profile").auth(&i)).await,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(env.upstream_hits("/profile"), before);
    // A denial is still enforced even though it cannot be recorded.
    assert_eq!(
        env.status(Req::get("/profile")).await,
        StatusCode::UNAUTHORIZED
    );
    assert!(metric(&env.metrics().await, "gateway_audit_write_failed_total") >= 2);
}

// ── 9. Request ids ───────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn request_ids_are_generated_unique_propagated_and_recorded() {
    let env = Env::new().await;
    let i = env.ident().await;

    let r1 = env.send(Req::get("/profile").auth(&i)).await;
    assert_eq!(r1.status(), StatusCode::OK);
    let id1 = r1
        .headers()
        .get("x-request-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(
        Uuid::parse_str(&id1).is_ok(),
        "generated id should be a UUID, got {id1}"
    );
    {
        let seen = env.seen.lock().unwrap();
        let hit = seen.iter().find(|s| s.path == "/profile").unwrap();
        assert_eq!(hit.header_all("x-request-id"), vec![id1.clone()]);
    }
    assert_eq!(
        env.decisions(&id1).await.len(),
        1,
        "the generated id is in the decision row"
    );

    let r2 = env.send(Req::get("/profile").auth(&i)).await;
    assert_ne!(
        id1,
        r2.headers().get("x-request-id").unwrap().to_str().unwrap()
    );

    for req in [
        Req::get("/profile"),
        Req::get("/profile").auth(&i).nonce(None),
        Req::get("/health"),
    ] {
        assert!(env.send(req).await.headers().contains_key("x-request-id"));
    }
    let r = env
        .send(Req::get("/profile").auth(&i).request_id("caller-chosen-id"))
        .await;
    assert_eq!(r.headers().get("x-request-id").unwrap(), "caller-chosen-id");
}

// ── 10. Upstream failures do not bypass controls ─────────────────────────────

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn upstream_down_still_enforces_every_control() {
    let (redis, pg_url) = infra();
    let state = build_state(&redis, &pg_url, "http://127.0.0.1:1".to_string()).await;
    apply_identity_migrations(&state.pg).await;
    let env = Env::from_state(state, Arc::new(Mutex::new(vec![])));
    let i = env.ident().await;

    assert_eq!(
        env.status(Req::get("/profile")).await,
        StatusCode::UNAUTHORIZED
    );
    let n = Uuid::new_v4().to_string();
    assert_eq!(
        env.status(Req::get("/profile").auth(&i).nonce(Some(&n)))
            .await,
        StatusCode::BAD_GATEWAY
    );
    // The nonce was consumed even though the upstream failed.
    assert_eq!(
        env.status(Req::get("/profile").auth(&i).nonce(Some(&n)))
            .await,
        StatusCode::CONFLICT
    );
    assert_eq!(
        metric(&env.metrics().await, "gateway_proxy_upstream_error_total"),
        1
    );
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn upstream_error_responses_are_relayed_and_counted() {
    let env = Env::new().await;
    let i = env.ident().await;
    let resp = env.send(Req::get("/boom").auth(&i)).await;
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body_string(resp).await, "upstream-boom");
    assert_eq!(
        metric(&env.metrics().await, "gateway_proxy_upstream_5xx_total"),
        1
    );
}

// ── 11. End to end with crypton-id's real token code ─────────────────────────
//
// Tokens are minted by crypton-id's production `issue_jwt`, and the upstream
// authenticates requests with crypton-id's real `AuthUser` extractor, `/me`
// handler, Redis denylist and credential check (see
// `tests-support/identity-src`, which compiles those source files verbatim).
// This is NOT the full crypton-id binary: that cannot be built on every host
// (webauthn-rs needs OpenSSL dev libraries), so the passkey ceremony, register/
// login handlers and the identity audit log are not exercised.

struct Identity {
    base: String,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Identity {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn start_identity(pg: PgPool, redis_url: &str) -> Identity {
    let state = identity_src::AppState::new(pg, redis_url, PUBLIC_KEY_PEM).unwrap();
    let app = identity_src::identity_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    Identity {
        base: format!("http://{addr}"),
        task,
    }
}

/// Mint with crypton-id's own `issue_jwt`.
fn mint_with_identity(i: &Ident) -> String {
    identity_src::issue_jwt(
        i.user,
        &i.username,
        i.cred,
        &EncodingKey::from_ed_pem(SIGNING_KEY_PEM.as_bytes()).unwrap(),
    )
    .unwrap_or_else(|_| panic!("issue_jwt failed"))
}

struct E2e {
    env: Env,
    identity: Identity,
    i: Ident,
    token: String,
}

async fn e2e() -> E2e {
    let (redis, _) = infra();
    let url = fresh_database().await;
    let pg = PgPool::connect(&url).await.unwrap();
    apply_identity_migrations(&pg).await;
    let identity = start_identity(pg, &redis).await;
    let state = build_state(&redis, &url, identity.base.clone()).await;
    let env = Env::from_state(state, Arc::new(Mutex::new(vec![])));
    let i = env.ident().await;
    let token = mint_with_identity(&i);
    E2e {
        env,
        identity,
        i,
        token,
    }
}

async fn direct(
    identity: &Identity,
    method: reqwest::Method,
    path: &str,
    tok: Option<&str>,
) -> (StatusCode, String) {
    let mut r = reqwest::Client::new().request(method, format!("{}{path}", identity.base));
    if let Some(t) = tok {
        r = r.bearer_auth(t);
    }
    let resp = r.send().await.unwrap();
    (
        StatusCode::from_u16(resp.status().as_u16()).unwrap(),
        resp.text().await.unwrap(),
    )
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn e2e_token_issued_by_crypton_id_authorizes_a_request_through_the_gateway() {
    let e = e2e().await;

    // Header and claims are what the gateway pins: EdDSA, kid v1.
    let hdr: serde_json::Value =
        serde_json::from_slice(&b64url_decode(e.token.split('.').next().unwrap())).unwrap();
    assert_eq!(
        (hdr["alg"].as_str(), hdr["kid"].as_str()),
        (Some("EdDSA"), Some(identity_src::JWT_KID))
    );

    // Through the gateway to crypton-id's real /me: the gateway and identity
    // independently accept the token.
    let id = rid();
    let resp = e
        .env
        .send(Req::get("/me").bearer(e.token.clone()).request_id(&id))
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body_string(resp).await).unwrap();
    assert_eq!(body["data"]["user_id"], e.i.user.to_string());
    assert_eq!(body["data"]["role"], "user");
    let d = e.env.decisions(&id).await;
    assert_eq!(
        (d[0].0.as_str(), d[0].1, d[0].2),
        ("allow", Some(e.i.user), Some(e.i.cred))
    );
    assert_eq!(
        direct(&e.identity, reqwest::Method::GET, "/me", Some(&e.token))
            .await
            .0,
        StatusCode::OK
    );
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn e2e_tampered_foreign_expired_and_unauthenticated_requests_fail_closed() {
    let e = e2e().await;
    let parts: Vec<&str> = e.token.split('.').collect();
    let mut c: serde_json::Value = serde_json::from_slice(&b64url_decode(parts[1])).unwrap();
    c["sub"] = json!(Uuid::new_v4().to_string());
    let tampered = format!(
        "{}.{}.{}",
        parts[0],
        b64url(c.to_string().as_bytes()),
        parts[2]
    );
    let foreign = encode(
        &identity_header(),
        &claims_json(&e.i, 3600),
        &EncodingKey::from_ed_pem(OTHER_SIGNING_KEY_PEM.as_bytes()).unwrap(),
    )
    .unwrap();

    for (label, req) in [
        ("no token", Req::get("/me")),
        ("tampered", Req::get("/me").bearer(tampered)),
        ("foreign key", Req::get("/me").bearer(foreign)),
        ("expired", Req::get("/me").bearer(mint(&e.i, -3600))),
    ] {
        assert_eq!(e.env.status(req).await, StatusCode::UNAUTHORIZED, "{label}");
    }
    // Replay is rejected at the gateway even with a perfectly valid real token.
    let n = Uuid::new_v4().to_string();
    assert_eq!(
        e.env
            .status(Req::get("/me").bearer(e.token.clone()).nonce(Some(&n)))
            .await,
        StatusCode::OK
    );
    assert_eq!(
        e.env
            .status(Req::get("/me").bearer(e.token.clone()).nonce(Some(&n)))
            .await,
        StatusCode::CONFLICT
    );
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn e2e_logout_through_the_gateway_revokes_the_token_at_gateway_and_identity() {
    let e = e2e().await;
    assert_eq!(
        e.env.status(Req::get("/me").bearer(e.token.clone())).await,
        StatusCode::OK
    );

    // identity's real deny_token writes the key the gateway then honours.
    let resp = e
        .env
        .send(Req::post("/auth/logout").bearer(e.token.clone()))
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let id = rid();
    assert_eq!(
        e.env
            .status(Req::get("/me").bearer(e.token.clone()).request_id(&id))
            .await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(e.env.decisions(&id).await[0].3["reason"], "token_revoked");
    assert_eq!(
        direct(&e.identity, reqwest::Method::GET, "/me", Some(&e.token))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn e2e_revoked_credential_is_rejected_by_gateway_and_identity_alike() {
    let e = e2e().await;
    assert_eq!(
        e.env.status(Req::get("/me").bearer(e.token.clone())).await,
        StatusCode::OK
    );
    sqlx::query("UPDATE credentials SET status = 'revoked' WHERE id = $1")
        .bind(e.i.cred)
        .execute(&e.env.state.pg)
        .await
        .unwrap();
    assert_eq!(
        e.env.status(Req::get("/me").bearer(e.token.clone())).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        direct(&e.identity, reqwest::Method::GET, "/me", Some(&e.token))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn e2e_public_bootstrap_requests_are_forwarded_to_identity_without_a_token() {
    // The stand-in identity has no /auth/login/start route (that handler needs
    // WebAuthn/OpenSSL), so an upstream 404 here is the proof that the gateway
    // forwarded the request instead of answering 401 itself.
    let e = e2e().await;
    let id = rid();
    let st = e
        .env
        .status(Req::post("/auth/login/start").request_id(&id))
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    assert_eq!(
        e.env.decisions(&id).await[0].3["reason"],
        "allowed_public_route"
    );
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn known_gap_identity_is_reachable_directly_and_gateway_only_controls_are_bypassable() {
    // Deployment fact, not a code bug: if clients can reach crypton-id's port,
    // they skip every gateway-only control (nonce/replay, rate limits, risk
    // policy, decision trail). Identity still validates the token itself, so
    // authentication is not bypassed, but nothing else is. Production must not
    // expose identity except through the gateway (private networking / network
    // policy); this repository does not enforce or verify that.
    let e = e2e().await;
    assert_eq!(
        e.env
            .status(
                Req::get("/me")
                    .bearer(e.token.clone())
                    .nonce(None)
                    .timestamp(None)
            )
            .await,
        StatusCode::BAD_REQUEST,
        "through the gateway: nonce and timestamp are required"
    );
    assert_eq!(
        direct(&e.identity, reqwest::Method::GET, "/me", Some(&e.token))
            .await
            .0,
        StatusCode::OK,
        "directly to identity: no nonce, no timestamp, no gateway involvement"
    );
    assert_eq!(
        direct(&e.identity, reqwest::Method::GET, "/me", None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
}

// ── 12. Runtime-integration additions ────────────────────────────────────────

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn gateway_secret_is_forwarded_and_a_client_supplied_one_is_stripped() {
    let (redis, pg_url) = infra();
    let (up, seen) = spawn_upstream().await;
    let mut state = build_state(&redis, &pg_url, up).await;
    apply_identity_migrations(&state.pg).await;
    state.gateway_secret = Some("s3cret-from-gateway".to_string());
    let env = Env::from_state(state, seen);
    let i = env.ident().await;

    let st = env
        .status(
            Req::get("/profile")
                .auth(&i)
                .header("x-gateway-secret", "attacker-supplied"),
        )
        .await;
    assert_eq!(st, StatusCode::OK);
    let seen = env.seen.lock().unwrap();
    let hit = seen.iter().find(|s| s.path == "/profile").unwrap();
    assert_eq!(
        hit.header_all("x-gateway-secret"),
        vec!["s3cret-from-gateway".to_string()],
        "only the gateway's own secret reaches the upstream"
    );
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn policy_blocks_are_counted_by_verdict_and_other_denials_do_not_touch_them() {
    let env = Env::new().await;
    let i = env.ident().await;
    for _ in 0..6 {
        let ev = json!({"event_type":"login_failed","user_id":i.user.to_string(),"details":{}});
        assert_eq!(
            ingest(&env, Some(INTERNAL_TOKEN), &ev, &rid()).await,
            StatusCode::ACCEPTED
        );
    }
    let m0 = env.metrics().await;
    assert!(
        !m0.contains("gateway_policy_blocked_locked_total"),
        "the misleading all-purpose counter is gone"
    );
    assert_eq!(metric(&m0, "gateway_policy_blocked_deny_total"), 0);

    // Not a policy denial: must leave the policy counters alone.
    assert_eq!(
        env.status(Req::get("/profile")).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        env.status(Req::get("/profile").auth(&i).nonce(None)).await,
        StatusCode::BAD_REQUEST
    );
    let m1 = env.metrics().await;
    for name in [
        "gateway_policy_blocked_deny_total",
        "gateway_policy_blocked_challenge_total",
        "gateway_policy_blocked_hold_total",
    ] {
        assert_eq!(metric(&m1, name), 0, "{name} moved on a non-policy denial");
    }

    // A real policy denial (locked account) moves exactly the deny counter.
    let st = env.status(Req::get("/profile").auth(&i)).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let m2 = env.metrics().await;
    assert_eq!(metric(&m2, "gateway_policy_blocked_deny_total"), 1);
    assert_eq!(metric(&m2, "gateway_policy_blocked_challenge_total"), 0);
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn known_gap_ingest_commit_failure_leaves_the_signal_without_an_audit_row() {
    // The ingest is NOT fully atomic across Postgres and Redis. Order is: insert
    // the audit row in a transaction, apply the Redis effect, COMMIT. A failure at
    // COMMIT (forced here with a deferred constraint trigger) leaves the Redis
    // signal applied and no audit row, and the caller sees a 500 and may retry
    // (which would apply the signal twice). The order is deliberate: a security
    // signal that is applied without its audit row is safer than the reverse.
    let env = Env::with_own_database().await;
    let i = env.ident().await;
    sqlx::query(
        "CREATE FUNCTION fail_at_commit() RETURNS trigger LANGUAGE plpgsql AS \
         $$ BEGIN RAISE EXCEPTION 'forced commit-time failure'; END $$",
    )
    .execute(&env.state.pg)
    .await
    .unwrap();
    sqlx::query(
        "CREATE CONSTRAINT TRIGGER audit_fail AFTER INSERT ON audit_logs \
         DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION fail_at_commit()",
    )
    .execute(&env.state.pg)
    .await
    .unwrap();

    let id = rid();
    let ev = json!({"event_type":"login_failed","user_id":i.user.to_string(),"details":{}});
    assert_eq!(
        ingest(&env, Some(INTERNAL_TOKEN), &ev, &id).await,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(
        redis_get(&env, &format!("velocity:failed:{}:1h", i.user)).await,
        Some(1),
        "Redis effect was applied"
    );
    let n: (i64,) =
        sqlx::query_as("SELECT count(*) FROM audit_logs WHERE detail->>'request_id' = $1")
            .bind(&id)
            .fetch_one(&env.state.pg)
            .await
            .unwrap();
    assert_eq!(n.0, 0, "but no audit row was committed");
}
