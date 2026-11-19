use std::path::PathBuf;
use std::process::ExitCode;

use axum::extract::Request;
use axum::http::header;
use axum::middleware::{from_fn, Next};
use axum::response::Response;
use base64::Engine;
use kreuzchen_registry::Source;

const USAGE: &str = "usage: kreuzchen-registry serve [--data DIR] [--port N] | key";

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("key") if args.len() == 1 => {
            key();
            ExitCode::SUCCESS
        }
        Some("serve") => match serve(&args[1..]).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn key() {
    let (key, hash) = kreuzchen_registry::new_key();
    println!("key: {key}\nkey_hash: {hash}");
}

async fn serve(args: &[String]) -> Result<(), String> {
    let mut data = PathBuf::from("/data");
    let mut port: u16 = 8080;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let v = it
            .next()
            .ok_or_else(|| format!("{a}: Wert fehlt\n{USAGE}"))?;
        match a.as_str() {
            "--data" => data = v.into(),
            "--port" => port = v.parse().map_err(|_| format!("ungültiger Port {v:?}"))?,
            _ => return Err(format!("unbekannte Option {a:?}\n{USAGE}")),
        }
    }
    let reg = kreuzchen_registry::load(&data)?;
    let quelle = match &reg.source {
        Source::Lokal => "lokal".to_owned(),
        Source::Github(gh) => format!("github {}@{}", gh.repo, gh.branch),
    };
    let token = std::env::var("KREUZCHEN_ADMIN_TOKEN").ok();
    let admin = token.as_deref().is_some_and(|t| !t.is_empty());
    let github_token = std::env::var("KREUZCHEN_GITHUB_TOKEN").ok();
    let app = kreuzchen_registry::router(reg, token.as_deref(), github_token.as_deref())
        .layer(from_fn(log));
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .map_err(|e| format!("Port {port}: {e}"))?;
    eprintln!(
        "listening on 0.0.0.0:{port}, data {}, admin {}, quelle {quelle}",
        data.display(),
        if admin { "on" } else { "off" }
    );
    axum::serve(listener, app).await.map_err(|e| e.to_string())
}

/// Logs `user path status`; only the user part of the Basic header is ever decoded.
async fn log(req: Request, next: Next) -> Response {
    let user = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Basic "))
        .and_then(|b| base64::engine::general_purpose::STANDARD.decode(b).ok())
        .and_then(|b| String::from_utf8(b).ok())
        .and_then(|s| s.split_once(':').map(|(u, _)| u.to_owned()))
        .unwrap_or_else(|| "-".into());
    let path = req.uri().path().to_owned();
    let res = next.run(req).await;
    eprintln!("{}", log_line(&user, &path, res.status().as_u16()));
    res
}

/// The user comes from an unauthenticated header: control characters must not forge log lines.
fn log_line(user: &str, path: &str, status: u16) -> String {
    let clean = |s: &str| s.replace(|c: char| c.is_control(), "?");
    format!("{} {} {status}", clean(user), clean(path))
}

#[cfg(test)]
mod tests {
    use super::log_line;

    #[test]
    fn log_line_stays_single_line() {
        let l = log_line("a\nb\r\u{1b}c", "/x\ny", 401);
        assert_eq!(l, "a?b??c /x?y 401");
    }
}
