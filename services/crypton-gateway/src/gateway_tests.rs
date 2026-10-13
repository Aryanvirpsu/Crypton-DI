//! Router-level gateway tests.
//!
//! These drive the real external/internal Axum routers (full middleware stack)
//! with `tower::ServiceExt::oneshot`, against a throwaway Redis + Postgres and a
//! local stub upstream. They need isolated infrastructure, so they are
//! `#[ignore]`d by default and run with:
//!
//!   GATEWAY_TEST_REDIS_URL=redis://127.0.0.1:16379 \
//!   GATEWAY_TEST_DATABASE_URL=postgres://postgres:<pw>@127.0.0.1:15432/gwtest \
//!   cargo test -- --ignored
//!
//! See `services/crypton-gateway/README.md` ("Integration tests"). Never point
//! these at a shared or production Redis/Postgres: the tests write keys and
//! tables.
//!
//! Tests named `known_*` pin CURRENT behaviour that looks like a defect or a
//! missing feature. They assert what the code does today so they flip loudly
//! when it is fixed; they are not a statement that the behaviour is desired.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::{
    body::Body,
    extract::{ConnectInfo, Request as AxRequest},
    http::{Request, Response, StatusCode},
    Router,
};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

use crate::state::{AppState, Metrics};
use crate::{build_external_router, build_internal_router};

const JWT_SECRET: &str = "gateway-test-secret-not-a-real-secret";
const INTERNAL_TOKEN: &str = "gateway-test-internal-token";

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

async fn state_with_upstream(identity_base: String) -> AppState {
    let (redis_url, pg_url) = infra();
    let client = redis::Client::open(redis_url).unwrap();
    let conn = client.get_connection_manager().await.unwrap();
    let pg = sqlx::PgPool::connect(&pg_url).await.unwrap();
    AppState {
        redis: Arc::new(conn),
        pg,
        http_client: reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap(),
        identity_base,
        metrics: Arc::new(Metrics::default()),
        policy_enforce: true,
        jwt_secret: JWT_SECRET.to_string(),
        internal_token: INTERNAL_TOKEN.to_string(),
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn token_for(sub: &str, role: Option<&str>, exp_offset: i64, secret: &str) -> String {
    let iat = now();
    let exp = (iat as i64 + exp_offset) as u64;
    let claims = json!({ "sub": sub, "exp": exp, "iat": iat, "role": role });
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap()
}

fn good_token(sub: &Uuid) -> String {
    token_for(&sub.to_string(), None, 3600, JWT_SECRET)
}

/// Request builder. Every request gets a fresh nonce + current timestamp unless
/// the caller overrides; `peer` is the TCP peer the listener would have seen.
struct Req {
    method: &'static str,
    path: String,
    bearer: Option<String>,
    nonce: Option<String>,
    timestamp: Option<String>,
    extra: Vec<(&'static str, String)>,
    peer: SocketAddr,
}

impl Req {
    fn get(path: &str) -> Self {
        Req {
            method: "GET",
            path: path.to_string(),
            bearer: None,
            nonce: Some(Uuid::new_v4().to_string()),
            timestamp: Some(now().to_string()),
            extra: vec![],
            // Random address in 100.64.0.0/10 (CGNAT): not loopback/RFC-1918, so
            // X-Forwarded-For is NOT trusted unless a test says otherwise, and
            // random so per-IP Redis counters never collide across tests.
            peer: random_public_peer(),
        }
    }
    fn bearer(mut self, t: String) -> Self {
        self.bearer = Some(t);
        self
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

fn random_public_peer() -> SocketAddr {
    let b = Uuid::new_v4();
    let b = b.as_bytes();
    SocketAddr::from(([100, 64 + (b[0] % 64), b[1], b[2].max(1)], 40000))
}

async fn send(app: &Router, req: Request<Body>) -> Response<Body> {
    app.clone().oneshot(req).await.unwrap()
}

async fn status_of(app: &Router, req: Req) -> StatusCode {
    send(app, req.build()).await.status()
}

async fn body_string(resp: Response<Body>) -> String {
    let b = axum::body::to_bytes(resp.into_body(), 1 << 20)
        .await
        .unwrap();
    String::from_utf8_lossy(&b).to_string()
}

async fn metrics_text(state: &AppState) -> String {
    let internal = build_internal_router(state.clone());
    let req = Request::builder()
        .uri("/internal/metrics")
        .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 1))))
        .body(Body::empty())
        .unwrap();
    body_string(send(&internal, req).await).await
}

fn metric(text: &str, name: &str) -> u64 {
    text.lines()
        .find_map(|l| l.strip_prefix(&format!("{name} ")))
        .unwrap_or_else(|| panic!("metric {name} missing"))
        .trim()
        .parse()
        .unwrap()
}

async fn redis_conn(state: &AppState) -> redis::aio::ConnectionManager {
    state.redis.as_ref().clone()
}

// ── 1. Authentication ─────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn health_is_open_and_everything_else_requires_auth() {
    let (up, seen) = spawn_upstream().await;
    let state = state_with_upstream(up).await;
    let app = build_external_router(state.clone());

    assert_eq!(status_of(&app, Req::get("/health")).await, StatusCode::OK);

    let sub = Uuid::new_v4();
    let cases: Vec<(&str, Req)> = vec![
        ("no authorization header", Req::get("/profile")),
        (
            "garbage bearer",
            Req::get("/profile").bearer("not.a.jwt".into()),
        ),
        (
            "wrong signing secret",
            Req::get("/profile").bearer(token_for(&sub.to_string(), None, 3600, "other-secret")),
        ),
        (
            "expired",
            Req::get("/profile").bearer(token_for(&sub.to_string(), None, -3600, JWT_SECRET)),
        ),
    ];
    for (label, req) in cases {
        let st = status_of(&app, req).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED, "{label}");
    }
    // None of the rejected requests reached the upstream.
    // (the opaque-token fallback calls POST /verify on the upstream for bad
    // bearer tokens, so only non-/verify paths count here.)
    assert!(
        seen.lock().unwrap().iter().all(|s| s.path == "/verify"),
        "a rejected request reached the upstream proxy path"
    );
    assert_eq!(
        metric(&metrics_text(&state).await, "gateway_auth_rejected_total"),
        4
    );
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn revoked_token_is_rejected() {
    let (up, _seen) = spawn_upstream().await;
    let state = state_with_upstream(up).await;
    let app = build_external_router(state.clone());

    let sub = Uuid::new_v4();
    let tok = good_token(&sub);
    // Recover iat from the token we just minted.
    let iat = {
        let payload = tok.split('.').nth(1).unwrap();
        let bytes = base64_url_decode(payload);
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        v["iat"].as_u64().unwrap()
    };
    let mut conn = redis_conn(&state).await;
    let _: () = redis::cmd("SET")
        .arg(format!("jwt:deny:{sub}:{iat}"))
        .arg(1)
        .arg("EX")
        .arg(60)
        .query_async(&mut conn)
        .await
        .unwrap();

    assert_eq!(
        status_of(&app, Req::get("/profile").bearer(tok)).await,
        StatusCode::UNAUTHORIZED
    );
}

fn base64_url_decode(s: &str) -> Vec<u8> {
    // Minimal URL-safe base64 decoder (no extra dependency for one test).
    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0;
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

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn known_gap_public_auth_routes_unreachable_without_a_bearer_token() {
    // `validate_jwt` wraps the whole proxy fallback, so an unauthenticated
    // caller cannot reach /auth/login or /auth/register through the gateway,
    // even though policy.rs ("without an authenticated session, only allow
    // public paths") implies public paths were intended. Pins current behaviour.
    let (up, seen) = spawn_upstream().await;
    let state = state_with_upstream(up).await;
    let app = build_external_router(state);

    for path in ["/auth/login/start", "/auth/register/start"] {
        assert_eq!(
            status_of(&app, Req::get(path)).await,
            StatusCode::UNAUTHORIZED,
            "{path}"
        );
    }
    assert!(seen.lock().unwrap().iter().all(|s| s.path == "/verify"));
}

// ── 2. Authorization ──────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn internal_diagnostics_require_admin_role() {
    let (up, _seen) = spawn_upstream().await;
    let state = state_with_upstream(up).await;
    let internal = build_internal_router(state);
    let target = Uuid::new_v4();
    let path = format!("/internal/diagnostics/user/{target}");
    let peer = SocketAddr::from(([127, 0, 0, 1], 1));

    let mk = |tok: Option<String>| {
        let mut b = Request::builder().uri(&path).extension(ConnectInfo(peer));
        if let Some(t) = tok {
            b = b.header("authorization", format!("Bearer {t}"));
        }
        b.body(Body::empty()).unwrap()
    };

    let sub = Uuid::new_v4().to_string();
    assert_eq!(
        send(&internal, mk(None)).await.status(),
        StatusCode::UNAUTHORIZED
    );
    let user_tok = token_for(&sub, Some("user"), 3600, JWT_SECRET);
    assert_eq!(
        send(&internal, mk(Some(user_tok))).await.status(),
        StatusCode::FORBIDDEN
    );
    let no_role = token_for(&sub, None, 3600, JWT_SECRET);
    assert_eq!(
        send(&internal, mk(Some(no_role))).await.status(),
        StatusCode::FORBIDDEN
    );
    let admin = token_for(&sub, Some("admin"), 3600, JWT_SECRET);
    assert_eq!(
        send(&internal, mk(Some(admin))).await.status(),
        StatusCode::OK
    );
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn locked_account_is_denied_by_policy_after_audit_failures() {
    // End-to-end: audit events (login_failed x6) -> Redis lock -> risk engine
    // account_locked -> policy blocks the proxied request.
    let (up, seen) = spawn_upstream().await;
    let state = state_with_upstream(up).await;
    let external = build_external_router(state.clone());
    let internal = build_internal_router(state.clone());
    let user = Uuid::new_v4();

    // Baseline: this user is allowed.
    let before = status_of(&external, Req::get("/profile").bearer(good_token(&user))).await;
    assert_eq!(before, StatusCode::OK, "baseline allow for a clean user");

    for _ in 0..6 {
        let ev = json!({"event_type":"login_failed","user_id":user.to_string(),"details":{}});
        let _ = send(&internal, ingest_req(Some(INTERNAL_TOKEN), &ev)).await;
    }
    let mut conn = redis_conn(&state).await;
    let locked: bool = redis::cmd("EXISTS")
        .arg(format!("login:locked:{user}"))
        .query_async(&mut conn)
        .await
        .unwrap();
    assert!(locked, "6 login_failed events should set login:locked");

    let after = status_of(&external, Req::get("/profile").bearer(good_token(&user))).await;
    assert!(
        after == StatusCode::FORBIDDEN || after == StatusCode::UNAUTHORIZED,
        "locked account must not reach upstream, got {after}"
    );
    let upstream_hits = seen
        .lock()
        .unwrap()
        .iter()
        .filter(|s| s.path == "/profile")
        .count();
    assert_eq!(
        upstream_hits, 1,
        "only the pre-lock request reached upstream"
    );
    assert!(
        metric(
            &metrics_text(&state).await,
            "gateway_policy_blocked_locked_total"
        ) >= 1
    );
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn revoked_credential_for_the_authenticated_user_is_blocked_but_another_users_is_ignored() {
    let (up, _seen) = spawn_upstream().await;
    let state = state_with_upstream(up).await;
    apply_identity_migrations(&state).await;
    let external = build_external_router(state.clone());

    let owner = Uuid::new_v4();
    let other = Uuid::new_v4();
    let revoked_dev = Uuid::new_v4();
    for (u, name) in [(owner, "owner"), (other, "other")] {
        sqlx::query("INSERT INTO users (id, username) VALUES ($1, $2)")
            .bind(u)
            .bind(format!("{name}-{u}"))
            .execute(&state.pg)
            .await
            .unwrap();
    }
    sqlx::query(
        "INSERT INTO credentials (id, user_id, credential_id, public_key, status) \
         VALUES ($1, $2, $3, $4, 'revoked')",
    )
    .bind(revoked_dev)
    .bind(owner)
    .bind(Uuid::new_v4().as_bytes().to_vec())
    .bind(vec![0u8; 8])
    .execute(&state.pg)
    .await
    .unwrap();

    // Control: the same user with no device header is allowed, so the block
    // below is attributable to the revoked credential, not to the user.
    assert_eq!(
        status_of(&external, Req::get("/profile").bearer(good_token(&owner))).await,
        StatusCode::OK
    );

    // Owner presenting their revoked device: not allowed through.
    let st = status_of(
        &external,
        Req::get("/profile")
            .bearer(good_token(&owner))
            .header("x-device-id", &revoked_dev.to_string()),
    )
    .await;
    assert!(
        st == StatusCode::FORBIDDEN || st == StatusCode::UNAUTHORIZED,
        "revoked credential must be blocked, got {st}"
    );
    eprintln!("revoked credential -> {st}");

    // A different user presenting the owner's device id gets no DeviceContext
    // (lookup is bound to the authenticated user), so the owner's revoked status
    // is neither applied to them nor leaked: they are treated like no device.
    let st2 = status_of(
        &external,
        Req::get("/profile")
            .bearer(good_token(&other))
            .header("x-device-id", &revoked_dev.to_string()),
    )
    .await;
    assert_eq!(st2, StatusCode::OK);
}

async fn apply_identity_migrations(state: &AppState) {
    // Serialise across tests and apply only once: the identity migrations are
    // not all idempotent (e.g. 004 re-adds a named constraint).
    let mut lock = state.pg.acquire().await.unwrap();
    sqlx::query("SELECT pg_advisory_lock(7770001)")
        .execute(&mut *lock)
        .await
        .unwrap();
    let applied: (Option<String>,) = sqlx::query_as("SELECT to_regclass('audit_logs')::text")
        .fetch_one(&mut *lock)
        .await
        .unwrap();
    if applied.0.is_none() {
        apply_identity_migrations_inner(state).await;
    }
    sqlx::query("SELECT pg_advisory_unlock(7770001)")
        .execute(&mut *lock)
        .await
        .unwrap();
}

async fn apply_identity_migrations_inner(state: &AppState) {
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
            .execute(&state.pg)
            .await
            .unwrap_or_else(|e| panic!("migration {f:?} failed: {e}"));
    }
}

// ── 3. Authorized request reaches upstream ────────────────────────────────────

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn authorized_request_reaches_upstream_with_trusted_client_ip() {
    let (up, seen) = spawn_upstream().await;
    let state = state_with_upstream(up).await;
    let app = build_external_router(state.clone());
    let user = Uuid::new_v4();
    let peer = SocketAddr::from(([100, 100, 113, 77], 5555));

    let resp = send(
        &app,
        Req::get("/profile")
            .bearer(good_token(&user))
            .peer(peer)
            // Spoof attempt from a public peer: must not be believed or forwarded.
            .header("x-forwarded-for", "10.0.0.1")
            .header("cf-ipcountry", "XX")
            .build(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(body_string(resp).await, "upstream-ok");

    {
        let seen = seen.lock().unwrap();
        let hit = seen
            .iter()
            .find(|s| s.path == "/profile")
            .expect("upstream hit");
        assert_eq!(
            hit.header_all("x-forwarded-for"),
            vec!["100.100.113.77".to_string()]
        );
        assert!(hit.header_all("cf-ipcountry").is_empty());
        assert_eq!(hit.header_all("authorization").len(), 1);
    }

    let m = metrics_text(&state).await;
    assert_eq!(metric(&m, "gateway_auth_jwt_accepted_total"), 1);
    assert_eq!(metric(&m, "gateway_proxy_upstream_2xx_total"), 1);
}

// ── 4. Rate limiting ──────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn per_user_rate_limit_applies_to_sensitive_paths_only() {
    let (up, _seen) = spawn_upstream().await;
    let state = state_with_upstream(up).await;
    let app = build_external_router(state.clone());
    let user = Uuid::new_v4();
    let tok = good_token(&user);

    // Control: a non-sensitive path is never rate limited, even past the limit.
    for _ in 0..70 {
        let st = status_of(&app, Req::get("/profile").bearer(tok.clone())).await;
        assert_ne!(st, StatusCode::TOO_MANY_REQUESTS);
    }

    // Sensitive path: user limit is 60 / 60 s. Each request uses a fresh peer IP
    // so the (larger) IP limit cannot be what trips.
    let mut first_429 = None;
    for i in 1..=65 {
        let st = status_of(&app, Req::get("/auth/ping").bearer(tok.clone())).await;
        if st == StatusCode::TOO_MANY_REQUESTS && first_429.is_none() {
            first_429 = Some(i);
        }
    }
    assert_eq!(
        first_429,
        Some(61),
        "61st sensitive request should be limited"
    );
    assert!(
        metric(
            &metrics_text(&state).await,
            "gateway_rate_limit_block_user_total"
        ) >= 1
    );
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn per_ip_rate_limit_applies_across_users() {
    let (up, _seen) = spawn_upstream().await;
    let state = state_with_upstream(up).await;
    let app = build_external_router(state.clone());
    let peer = random_public_peer();

    let mut first_429 = None;
    for i in 1..=105 {
        // New user each time: the user limit can never trip, only the IP one.
        let st = status_of(
            &app,
            Req::get("/auth/ping")
                .bearer(good_token(&Uuid::new_v4()))
                .peer(peer),
        )
        .await;
        if st == StatusCode::TOO_MANY_REQUESTS && first_429.is_none() {
            first_429 = Some(i);
        }
    }
    assert_eq!(first_429, Some(101));
    assert!(
        metric(
            &metrics_text(&state).await,
            "gateway_rate_limit_block_ip_total"
        ) >= 1
    );
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn per_device_rate_limit_applies() {
    let (up, _seen) = spawn_upstream().await;
    let state = state_with_upstream(up).await;
    let app = build_external_router(state.clone());
    let device = Uuid::new_v4().to_string();

    let mut first_429 = None;
    for i in 1..=55 {
        let st = status_of(
            &app,
            Req::get("/auth/ping")
                .bearer(good_token(&Uuid::new_v4()))
                .header("x-device-id", &device),
        )
        .await;
        if st == StatusCode::TOO_MANY_REQUESTS && first_429.is_none() {
            first_429 = Some(i);
        }
    }
    assert_eq!(first_429, Some(51));
}

// ── 5. Nonce / replay protection ─────────────────────────────────────────────

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn nonce_and_timestamp_are_enforced_and_replays_rejected() {
    let (up, seen) = spawn_upstream().await;
    let state = state_with_upstream(up).await;
    let app = build_external_router(state.clone());
    let user = Uuid::new_v4();
    let tok = good_token(&user);

    // Missing nonce / timestamp / stale / malformed timestamp -> 400.
    assert_eq!(
        status_of(&app, Req::get("/profile").bearer(tok.clone()).nonce(None)).await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        status_of(
            &app,
            Req::get("/profile").bearer(tok.clone()).timestamp(None)
        )
        .await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        status_of(
            &app,
            Req::get("/profile")
                .bearer(tok.clone())
                .timestamp(Some((now() - 3600).to_string()))
        )
        .await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        status_of(
            &app,
            Req::get("/profile")
                .bearer(tok.clone())
                .timestamp(Some("yesterday".into()))
        )
        .await,
        StatusCode::BAD_REQUEST
    );

    // First use of a nonce passes; replay of the same nonce is rejected 409 and
    // does not reach the upstream a second time.
    let nonce = Uuid::new_v4().to_string();
    assert_eq!(
        status_of(
            &app,
            Req::get("/profile").bearer(tok.clone()).nonce(Some(&nonce))
        )
        .await,
        StatusCode::OK
    );
    assert_eq!(
        status_of(
            &app,
            Req::get("/profile").bearer(tok.clone()).nonce(Some(&nonce))
        )
        .await,
        StatusCode::CONFLICT
    );
    let hits = seen
        .lock()
        .unwrap()
        .iter()
        .filter(|s| s.path == "/profile")
        .count();
    assert_eq!(hits, 1);

    let m = metrics_text(&state).await;
    assert!(metric(&m, "gateway_nonce_replay_blocked_total") >= 2); // missing-nonce + replay
    assert_eq!(metric(&m, "gateway_nonce_timestamp_missing_total"), 1);
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn known_gap_nonce_is_global_not_bound_to_the_caller() {
    // The nonce key is `nonce:{value}` with no user/token component. A nonce
    // consumed by user A is rejected for user B. This is replay-safe but means
    // one caller can burn another caller's chosen nonce values (DoS-by-
    // collision, only if nonces are guessable). Pins current behaviour.
    let (up, _seen) = spawn_upstream().await;
    let state = state_with_upstream(up).await;
    let app = build_external_router(state);
    let nonce = Uuid::new_v4().to_string();
    assert_eq!(
        status_of(
            &app,
            Req::get("/profile")
                .bearer(good_token(&Uuid::new_v4()))
                .nonce(Some(&nonce))
        )
        .await,
        StatusCode::OK
    );
    assert_eq!(
        status_of(
            &app,
            Req::get("/profile")
                .bearer(good_token(&Uuid::new_v4()))
                .nonce(Some(&nonce))
        )
        .await,
        StatusCode::CONFLICT
    );
}

// ── 6. Audit ingest and metrics ──────────────────────────────────────────────

fn ingest_req(token: Option<&str>, ev: &serde_json::Value) -> Request<Body> {
    let mut b = Request::builder()
        .method("POST")
        .uri("/internal/audit/ingest")
        .header("content-type", "application/json")
        .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 1))));
    if let Some(t) = token {
        b = b.header("x-internal-token", t);
    }
    b.body(Body::from(ev.to_string())).unwrap()
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn audit_ingest_authenticates_validates_and_updates_signals() {
    let (up, _seen) = spawn_upstream().await;
    let state = state_with_upstream(up).await;
    let internal = build_internal_router(state.clone());
    let user = Uuid::new_v4();
    let ev = json!({"event_type":"login_failed","user_id":user.to_string(),"details":{}});

    assert_eq!(
        send(&internal, ingest_req(None, &ev)).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(&internal, ingest_req(Some("wrong"), &ev))
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let bad = json!({"event_type":"login_failed","user_id":"not-a-uuid","details":{}});
    assert_eq!(
        send(&internal, ingest_req(Some(INTERNAL_TOKEN), &bad))
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );

    let mut conn = redis_conn(&state).await;
    // Rejected events must have had no effect.
    let c: Option<i64> = redis::cmd("GET")
        .arg(format!("velocity:failed:{user}:1h"))
        .query_async(&mut conn)
        .await
        .unwrap();
    assert_eq!(c, None);

    // Accepted event updates the Redis signals the policy engine reads.
    let _ = send(&internal, ingest_req(Some(INTERNAL_TOKEN), &ev)).await;
    let c: Option<i64> = redis::cmd("GET")
        .arg(format!("velocity:failed:{user}:1h"))
        .query_async(&mut conn)
        .await
        .unwrap();
    assert_eq!(c, Some(1));
    // login_success clears the lock/failure counters.
    let ok = json!({"event_type":"login_success","user_id":user.to_string(),"details":{}});
    let _ = send(&internal, ingest_req(Some(INTERNAL_TOKEN), &ok)).await;
    let failed: Option<i64> = redis::cmd("GET")
        .arg(format!("login:failed:{user}"))
        .query_async(&mut conn)
        .await
        .unwrap();
    assert_eq!(failed, None);
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn known_defect_audit_ingest_persists_to_a_table_identity_never_creates() {
    // ingest.rs INSERTs into `audit_events`; crypton-id's migrations create
    // `audit_logs` and nothing named `audit_events`. Against the real schema the
    // INSERT fails and the endpoint returns 500, so no audit row is ever stored
    // (Redis side effects still apply). Pins current behaviour.
    let (up, _seen) = spawn_upstream().await;
    let state = state_with_upstream(up).await;
    apply_identity_migrations(&state).await;
    let internal = build_internal_router(state.clone());

    let has = |name: &'static str| {
        let pg = state.pg.clone();
        async move {
            let r: (Option<String>,) = sqlx::query_as("SELECT to_regclass($1)::text")
                .bind(name)
                .fetch_one(&pg)
                .await
                .unwrap();
            r.0.is_some()
        }
    };
    assert!(has("audit_logs").await, "identity schema has audit_logs");
    assert!(
        !has("audit_events").await,
        "identity schema has no audit_events"
    );

    let ev = json!({"event_type":"device_added","user_id":null,"details":{"device_id":"d1"}});
    let resp = send(&internal, ingest_req(Some(INTERNAL_TOKEN), &ev)).await;
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn known_gap_gateway_emits_no_audit_record_of_its_own_decisions() {
    // The only record of allow/deny decisions is the in-memory /internal/metrics
    // counters (reset on restart, no per-request detail, no user/path). There is
    // no persisted audit trail of gateway decisions. Pins current behaviour by
    // showing decisions are reflected in counters and nowhere durable.
    let (up, _seen) = spawn_upstream().await;
    let state = state_with_upstream(up).await;
    apply_identity_migrations(&state).await;
    let app = build_external_router(state.clone());
    let user = Uuid::new_v4();

    let _ = status_of(&app, Req::get("/profile").bearer(good_token(&user))).await; // allow
    let _ = status_of(&app, Req::get("/profile")).await; // deny (no auth)

    let m = metrics_text(&state).await;
    assert_eq!(metric(&m, "gateway_proxy_upstream_2xx_total"), 1);
    assert_eq!(metric(&m, "gateway_auth_rejected_total"), 1);

    let n: (i64,) = sqlx::query_as("SELECT count(*) FROM audit_logs")
        .fetch_one(&state.pg)
        .await
        .unwrap();
    assert_eq!(n.0, 0, "gateway decisions are not written to audit_logs");
}

// ── 7. Request IDs ───────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn request_ids_are_generated_unique_propagated_upstream_and_on_rejections() {
    let (up, seen) = spawn_upstream().await;
    let state = state_with_upstream(up).await;
    let app = build_external_router(state);
    let user = Uuid::new_v4();

    // Generated on a normal request, echoed on the response, same value upstream.
    let r1 = send(&app, Req::get("/profile").bearer(good_token(&user)).build()).await;
    assert_eq!(r1.status(), StatusCode::OK);
    let id1 = r1
        .headers()
        .get("x-request-id")
        .expect("response request id")
        .to_str()
        .unwrap()
        .to_string();
    assert!(
        Uuid::parse_str(&id1).is_ok(),
        "generated id should be a UUID, got {id1}"
    );
    {
        let seen = seen.lock().unwrap();
        let hit = seen.iter().find(|s| s.path == "/profile").unwrap();
        assert_eq!(hit.header_all("x-request-id"), vec![id1.clone()]);
    }

    // A second request gets a different id.
    let r2 = send(&app, Req::get("/profile").bearer(good_token(&user)).build()).await;
    let id2 = r2
        .headers()
        .get("x-request-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert_ne!(id1, id2);

    // Rejected requests (auth failure, nonce failure, /health) still carry one.
    for req in [
        Req::get("/profile"),
        Req::get("/profile").bearer(good_token(&user)).nonce(None),
        Req::get("/health"),
    ] {
        let r = send(&app, req.build()).await;
        assert!(r.headers().contains_key("x-request-id"), "{}", r.status());
    }

    // A caller-supplied id is preserved end to end (documented behaviour: the
    // layer does not overwrite an existing x-request-id; callers can choose it).
    let r = send(
        &app,
        Req::get("/profile")
            .bearer(good_token(&user))
            .header("x-request-id", "caller-chosen-id")
            .build(),
    )
    .await;
    assert_eq!(r.headers().get("x-request-id").unwrap(), "caller-chosen-id");
}

// ── 8. Upstream failures do not bypass controls ──────────────────────────────

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn upstream_down_still_enforces_every_control_and_reports_bad_gateway_only_when_authorized() {
    // Port 1 on loopback: connection refused.
    let state = state_with_upstream("http://127.0.0.1:1".to_string()).await;
    let app = build_external_router(state.clone());
    let user = Uuid::new_v4();
    let tok = good_token(&user);

    // Unauthenticated: 401, not 502 (controls run before the proxy).
    assert_eq!(
        status_of(&app, Req::get("/profile")).await,
        StatusCode::UNAUTHORIZED
    );

    // Authorized: reaches the proxy, which reports 502.
    let nonce = Uuid::new_v4().to_string();
    assert_eq!(
        status_of(
            &app,
            Req::get("/profile").bearer(tok.clone()).nonce(Some(&nonce))
        )
        .await,
        StatusCode::BAD_GATEWAY
    );
    // Replay protection still holds even though the first attempt failed upstream.
    assert_eq!(
        status_of(&app, Req::get("/profile").bearer(tok).nonce(Some(&nonce))).await,
        StatusCode::CONFLICT
    );
    assert_eq!(
        metric(
            &metrics_text(&state).await,
            "gateway_proxy_upstream_error_total"
        ),
        1
    );
}

#[tokio::test]
#[ignore = "needs throwaway redis+postgres (see module docs)"]
async fn upstream_error_responses_are_relayed_and_counted() {
    let (up, _seen) = spawn_upstream().await;
    let state = state_with_upstream(up).await;
    let app = build_external_router(state.clone());
    let resp = send(
        &app,
        Req::get("/boom")
            .bearer(good_token(&Uuid::new_v4()))
            .build(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body_string(resp).await, "upstream-boom");
    assert_eq!(
        metric(
            &metrics_text(&state).await,
            "gateway_proxy_upstream_5xx_total"
        ),
        1
    );
}

// ── 9. Observations that are NOT covered ─────────────────────────────────────
//
// Not tested here (no isolated way, or no behaviour to test):
//  * Redis outage => nonce 503 / policy degraded-challenge: needs stopping Redis.
//  * Identity-issued tokens: identity signs EdDSA (crypton-id/src/jwt.rs) and has
//    no /verify route; the gateway verifies HS256 with a shared JWT_SECRET and
//    falls back to POST {identity}/verify. Verified by source inspection, not by
//    a runtime test against a real identity service.
//  * Step-up completion path, org/tenant policy overrides, GeoIP/breach signals
//    (the gateway deliberately does not enrich).
