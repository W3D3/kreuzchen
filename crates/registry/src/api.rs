use std::sync::{Arc, RwLock};

use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, SaltString};
use argon2::{Argon2, PasswordVerifier};
use axum::extract::{Path, Request, State};
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::middleware::{from_fn_with_state, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Extension, Json, Router};
use base64::Engine;
use serde_json::json;
use tokio::sync::{Mutex, Semaphore};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::admin::{self, AdminState};
use crate::sync::Sync;
use crate::{Practice, Registry, Source};

#[derive(Clone)]
struct AppState {
    /// Swapped whole after each admin write; practices see changes on their next request.
    reg: Arc<RwLock<Arc<Registry>>>,
    /// Verified against for unknown users so timing matches known ones.
    dummy_hash: Arc<str>,
    /// Bounds concurrent argon2 verifications (memory-hard).
    verify_slots: Arc<Semaphore>,
}

/// `admin_token`: `None` (or empty) leaves every `/admin` route unmounted, so they answer 404.
/// With a `type: github` source this also starts the sync loop (needs a Tokio runtime); `github_token`
/// is optional for public repos.
pub fn router(reg: Registry, admin_token: Option<&str>, github_token: Option<&str>) -> Router {
    let source = reg.source.clone();
    let reg = Arc::new(RwLock::new(Arc::new(reg)));
    let write = Arc::new(Mutex::new(()));
    let sync = match source {
        Source::Local => None,
        Source::Github(gh) => {
            let sync = Arc::new(Sync::new(gh, github_token, reg.clone(), write.clone()));
            sync.clone().spawn();
            Some(sync)
        }
    };
    let dummy_hash = Argon2::default()
        .hash_password(b"dummy", &SaltString::generate(&mut OsRng))
        .expect("hash dummy")
        .to_string();
    let state = AppState {
        reg: reg.clone(),
        dummy_hash: dummy_hash.into(),
        verify_slots: Arc::new(Semaphore::new(4)),
    };
    let v1 = Router::new()
        .route("/questionnaires", get(list))
        .route("/questionnaires/{id}", get(download))
        // shortcut: the 0.4.0 app's paths; remove once no 0.4.0 clients are left.
        .route("/fragebogen", get(list))
        .route("/fragebogen/{id}", get(download))
        .fallback(|| async { StatusCode::NOT_FOUND })
        .layer(from_fn_with_state(state.clone(), auth))
        .with_state(state);
    let mut app = Router::new()
        .route("/healthz", get(|| async {}))
        .nest("/v1", v1);
    if let Some(token) = admin_token.filter(|t| !t.is_empty()) {
        app = app.nest(
            "/admin",
            admin::router(AdminState {
                reg,
                token: admin::digest(token),
                write,
                sync,
            }),
        );
    }
    app.layer(SetResponseHeaderLayer::overriding(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store"),
    ))
}

/// `(user, key)` from a Basic `Authorization` header; empty strings when absent or malformed.
pub fn credentials(headers: &HeaderMap) -> (String, String) {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Basic "))
        .and_then(|b| base64::engine::general_purpose::STANDARD.decode(b).ok())
        .and_then(|b| String::from_utf8(b).ok())
        .and_then(|s| s.split_once(':').map(|(u, k)| (u.to_owned(), k.to_owned())))
        .unwrap_or_default()
}

// No WWW-Authenticate: the app sends credentials preemptively and browsers must not prompt.
async fn auth(State(st): State<AppState>, mut req: Request, next: Next) -> Response {
    let (user, key) = credentials(req.headers());
    let reg = st.reg.read().unwrap().clone();
    let practice = reg.practices.iter().find(|p| p.user == user).cloned();
    let hash = practice
        .as_ref()
        .map_or_else(|| st.dummy_hash.to_string(), |p| p.key_hash.clone());
    let ok = {
        let _slot = st.verify_slots.acquire().await.ok();
        tokio::task::spawn_blocking(move || {
            PasswordHash::new(&hash)
                .map(|h| {
                    Argon2::default()
                        .verify_password(key.as_bytes(), &h)
                        .is_ok()
                })
                .unwrap_or(false)
        })
        .await
        .unwrap_or(false)
    };
    match practice {
        Some(p) if ok => {
            req.extensions_mut().insert(p);
            next.run(req).await
        }
        _ => StatusCode::UNAUTHORIZED.into_response(),
    }
}

async fn list(
    State(st): State<AppState>,
    Extension(p): Extension<Practice>,
) -> Json<Vec<serde_json::Value>> {
    let reg = st.reg.read().unwrap().clone();
    Json(
        reg.entries
            .values()
            .filter(|e| p.questionnaires.contains(&e.id))
            .map(|e| json!({"id": e.id, "title": e.title, "version": e.version, "hash": e.hash}))
            .collect(),
    )
}

async fn download(
    State(st): State<AppState>,
    Extension(p): Extension<Practice>,
    Path(id): Path<String>,
) -> Response {
    let reg = st.reg.read().unwrap().clone();
    match reg
        .entries
        .get(&id)
        .filter(|e| p.questionnaires.contains(&e.id))
    {
        Some(e) => (
            [
                (header::CONTENT_TYPE, "application/yaml".to_string()),
                (HeaderName::from_static("x-kreuzchen-hash"), e.hash.clone()),
            ],
            e.bytes.clone(),
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
