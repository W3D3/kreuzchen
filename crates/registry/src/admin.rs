use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, Request, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::middleware::{from_fn_with_state, Next};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, put};
use axum::{Json, Router};
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;
use tower_http::set_header::SetResponseHeaderLayer;

use crate::config::{load, parse_entry, write_atomic, write_config};
use crate::{Entry, Registry};

/// Largest accepted questionnaire upload.
pub const MAX_UPLOAD: usize = 1024 * 1024;

#[derive(Clone)]
pub(crate) struct AdminState {
    pub reg: Arc<RwLock<Arc<Registry>>>,
    /// SHA-256 of the admin token; comparing digests hides the token length.
    pub token: [u8; 32],
    /// Held for each whole read-modify-write, so admin requests never interleave.
    pub write: Arc<Mutex<()>>,
}

type Fail = (StatusCode, String);

pub(crate) fn router(st: AdminState) -> Router {
    let api = Router::new()
        .route("/fragebogen", get(list).post(upload))
        .route("/fragebogen/{id}", put(replace).delete(remove))
        .route("/fragebogen/{id}/praxen", put(assign))
        .fallback(|| async { StatusCode::NOT_FOUND })
        .layer(DefaultBodyLimit::max(MAX_UPLOAD))
        .layer(from_fn_with_state(st.clone(), auth))
        .with_state(st);
    Router::new()
        .route(
            "/",
            get(|| async { Html(include_str!("admin.html")) }).layer(
                SetResponseHeaderLayer::overriding(
                    header::CONTENT_SECURITY_POLICY,
                    HeaderValue::from_static(
                        "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; connect-src 'self'; frame-ancestors 'none'",
                    ),
                ),
            ),
        )
        .nest("/api", api)
}

pub(crate) fn digest(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

// Bearer only: Basic (practice) credentials never reach the admin handlers.
async fn auth(State(st): State<AdminState>, req: Request, next: Next) -> Response {
    let given = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(digest);
    let ok =
        given.is_some_and(|g| g.iter().zip(st.token).fold(0, |acc, (a, b)| acc | (a ^ b)) == 0);
    if ok {
        next.run(req).await
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}

fn current(st: &AdminState) -> Arc<Registry> {
    st.reg.read().unwrap().clone()
}

fn internal(e: impl ToString) -> Fail {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

/// Re-reads the data dir and serves it. Called before each write too, so hand edits made since
/// the last load are the base of the write (not overwritten), and a broken one stops it early.
fn reload(st: &AdminState) -> Result<Arc<Registry>, Fail> {
    let reg = Arc::new(load(&current(st).dir).map_err(internal)?);
    *st.reg.write().unwrap() = reg.clone();
    Ok(reg)
}

fn entry_json(reg: &Registry, e: &Entry) -> serde_json::Value {
    let praxen: Vec<_> = reg
        .practices
        .iter()
        .filter(|p| p.fragebogen.contains(&e.id))
        .map(|p| &p.user)
        .collect();
    json!({"id": e.id, "title": e.title, "version": e.version, "hash": e.hash, "praxen": praxen})
}

async fn list(State(st): State<AdminState>) -> Json<serde_json::Value> {
    let reg = current(&st);
    let praxen: Vec<_> = reg.practices.iter().map(|p| &p.user).collect();
    let fragebogen: Vec<_> = reg.entries.values().map(|e| entry_json(&reg, e)).collect();
    Json(json!({"praxen": praxen, "fragebogen": fragebogen}))
}

fn parse_upload(body: Bytes) -> Result<Entry, Fail> {
    parse_entry("Datei", body.to_vec(), PathBuf::new()).map_err(|e| (StatusCode::BAD_REQUEST, e))
}

async fn upload(State(st): State<AdminState>, body: Bytes) -> Result<Response, Fail> {
    let new = parse_upload(body)?;
    let _w = st.write.lock().await;
    let reg = reload(&st)?;
    let path = reg
        .dir
        .join("definitionen")
        .join(format!("{}.yaml", new.id));
    if reg.entries.contains_key(&new.id) || path.exists() {
        return Err((
            StatusCode::CONFLICT,
            format!(
                "Fragebogen {:?} gibt es schon; zum Aktualisieren „Ersetzen“ verwenden",
                new.id
            ),
        ));
    }
    write_atomic(&path, &new.bytes).map_err(internal)?;
    let reg = reload(&st)?;
    Ok((
        StatusCode::CREATED,
        Json(entry_json(&reg, &reg.entries[&new.id])),
    )
        .into_response())
}

async fn replace(
    State(st): State<AdminState>,
    Path(id): Path<String>,
    body: Bytes,
) -> Result<Json<serde_json::Value>, Fail> {
    let new = parse_upload(body)?;
    if new.id != id {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Datei hat die id {:?}, ersetzt werden soll {id:?}", new.id),
        ));
    }
    let _w = st.write.lock().await;
    let reg = reload(&st)?;
    let old = reg
        .entries
        .get(&id)
        .ok_or((StatusCode::NOT_FOUND, String::new()))?;
    write_atomic(&bak(&old.path), &old.bytes).map_err(internal)?;
    write_atomic(&old.path, &new.bytes).map_err(internal)?;
    let reg = reload(&st)?;
    Ok(Json(entry_json(&reg, &reg.entries[&id])))
}

async fn remove(State(st): State<AdminState>, Path(id): Path<String>) -> Result<StatusCode, Fail> {
    let _w = st.write.lock().await;
    let reg = reload(&st)?;
    let old = reg
        .entries
        .get(&id)
        .ok_or((StatusCode::NOT_FOUND, String::new()))?;
    // Config first: a crash in between leaves an unassigned file, which still loads.
    if reg.practices.iter().any(|p| p.fragebogen.contains(&id)) {
        let mut practices = reg.practices.clone();
        practices
            .iter_mut()
            .for_each(|p| p.fragebogen.retain(|f| *f != id));
        write_config(&reg.dir, practices).map_err(internal)?;
    }
    std::fs::rename(&old.path, bak(&old.path)).map_err(internal)?;
    reload(&st)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn assign(
    State(st): State<AdminState>,
    Path(id): Path<String>,
    Json(users): Json<Vec<String>>,
) -> Result<Json<serde_json::Value>, Fail> {
    let _w = st.write.lock().await;
    let reg = reload(&st)?;
    if !reg.entries.contains_key(&id) {
        return Err((StatusCode::NOT_FOUND, String::new()));
    }
    if let Some(u) = users
        .iter()
        .find(|u| !reg.practices.iter().any(|p| &p.user == *u))
    {
        return Err((StatusCode::BAD_REQUEST, format!("unbekannte Praxis {u:?}")));
    }
    let mut practices = reg.practices.clone();
    for p in &mut practices {
        let has = p.fragebogen.contains(&id);
        if users.contains(&p.user) && !has {
            p.fragebogen.push(id.clone());
        } else if !users.contains(&p.user) && has {
            p.fragebogen.retain(|f| *f != id);
        }
    }
    write_config(&reg.dir, practices).map_err(internal)?;
    let reg = reload(&st)?;
    Ok(Json(entry_json(&reg, &reg.entries[&id])))
}

/// The single kept previous version of a definition file.
fn bak(path: &std::path::Path) -> PathBuf {
    let mut p = path.as_os_str().to_owned();
    p.push(".bak");
    p.into()
}
