//! Support tickets from practices: one JSON file per ticket in `<data>/tickets/`.
//! Message bodies and emails are user text: they are stored and served, never logged.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::config::{safe_id, write_atomic};

/// Largest accepted ticket request body.
pub const MAX_BODY: usize = 64 * 1024;
/// Tickets one practice may send per hour.
pub const PER_HOUR: usize = 10;
const SUBJECT: usize = 200;
const MESSAGE: usize = 10_000;
const NOTE: usize = 10_000;
const META: usize = 200;

pub(crate) type Fail = (StatusCode, String);

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Question,
    Bug,
    Feedback,
    Wish,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Open,
    Done,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QRef {
    pub id: String,
    pub title: String,
    pub version: String,
    pub hash: String,
}

/// What a practice sends. Closed: anything else (answers, results, ids) is rejected.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NewTicket {
    #[serde(rename = "type")]
    pub kind: Kind,
    pub subject: String,
    pub message: String,
    #[serde(default)]
    pub reply_email: Option<String>,
    #[serde(default)]
    pub questionnaire: Option<QRef>,
    pub app_version: String,
    pub os: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Ticket {
    pub id: u64,
    /// UTC, RFC 3339.
    pub created_at: String,
    pub practice: String,
    #[serde(rename = "type")]
    pub kind: Kind,
    pub subject: String,
    pub message: String,
    pub reply_email: Option<String>,
    pub questionnaire: Option<QRef>,
    pub app_version: String,
    pub os: String,
    pub status: Status,
    /// Internal to the admins; practices never see it.
    pub admin_note: String,
}

/// An admin's change to a ticket; absent fields stay as they are.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Patch {
    pub status: Option<Status>,
    pub admin_note: Option<String>,
}

pub(crate) struct Tickets {
    dir: PathBuf,
    /// Held for each whole read-modify-write; also the per-practice send times for the rate limit.
    sent: Mutex<HashMap<String, Vec<Instant>>>,
}

fn bad(msg: impl Into<String>) -> Fail {
    (StatusCode::BAD_REQUEST, msg.into())
}

fn internal(e: impl ToString) -> Fail {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

/// Errors name the field, never its content.
fn check_len(field: &str, s: &str, max: usize) -> Result<(), Fail> {
    let n = s.chars().count();
    if n == 0 || n > max {
        return Err(bad(format!("{field}: 1–{max} Zeichen erlaubt")));
    }
    Ok(())
}

fn check_line(field: &str, s: &str, max: usize) -> Result<(), Fail> {
    check_len(field, s, max)?;
    if s.chars().any(char::is_control) {
        return Err(bad(format!("{field}: Steuerzeichen nicht erlaubt")));
    }
    Ok(())
}

fn valid_email(e: &str) -> bool {
    e.len() <= 254
        && !e.chars().any(|c| c.is_whitespace() || c.is_control())
        && e.split_once('@')
            .is_some_and(|(l, d)| !l.is_empty() && d.contains('.') && !d.contains('@'))
}

/// Trims and validates; an empty reply email becomes `None`.
fn normalize(mut t: NewTicket) -> Result<NewTicket, Fail> {
    t.subject = t.subject.trim().to_owned();
    t.message = t.message.trim().to_owned();
    check_line("subject", &t.subject, SUBJECT)?;
    check_len("message", &t.message, MESSAGE)?;
    check_line("app_version", &t.app_version, META)?;
    check_line("os", &t.os, META)?;
    t.reply_email = t
        .reply_email
        .map(|e| e.trim().to_owned())
        .filter(|e| !e.is_empty());
    if t.reply_email.as_deref().is_some_and(|e| !valid_email(e)) {
        return Err(bad("reply_email: keine gültige E-Mail-Adresse"));
    }
    if let Some(q) = &t.questionnaire {
        if !safe_id(&q.id) || q.id.len() > META {
            return Err(bad("questionnaire.id: ungültig"));
        }
        check_line("questionnaire.title", &q.title, META)?;
        check_line("questionnaire.version", &q.version, META)?;
        check_line("questionnaire.hash", &q.hash, META)?;
    }
    Ok(t)
}

impl Tickets {
    pub fn new(data: &std::path::Path) -> Self {
        Self {
            dir: data.join("tickets"),
            sent: Mutex::default(),
        }
    }

    fn path(&self, id: u64) -> PathBuf {
        self.dir.join(format!("{id:06}.json"))
    }

    fn ids(&self) -> Result<Vec<u64>, Fail> {
        let rd = match fs::read_dir(&self.dir) {
            Ok(rd) => rd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(internal(e)),
        };
        Ok(rd
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                e.file_name()
                    .to_str()?
                    .strip_suffix(".json")?
                    .parse::<u64>()
                    .ok()
            })
            .collect())
    }

    fn read(&self, id: u64) -> Result<Ticket, Fail> {
        let bytes = fs::read(self.path(id)).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => (StatusCode::NOT_FOUND, String::new()),
            _ => internal(e),
        })?;
        serde_json::from_slice(&bytes).map_err(|e| internal(format!("Ticket {id}: {e}")))
    }

    fn write(&self, t: &Ticket) -> Result<(), Fail> {
        fs::create_dir_all(&self.dir).map_err(internal)?;
        let json = serde_json::to_vec_pretty(t).map_err(internal)?;
        write_atomic(&self.path(t.id), &json).map_err(internal)
    }

    pub async fn create(&self, practice: &str, new: NewTicket) -> Result<Ticket, Fail> {
        let new = normalize(new)?;
        let mut sent = self.sent.lock().await;
        let now = Instant::now();
        let times = sent.entry(practice.to_owned()).or_default();
        times.retain(|t| now.duration_since(*t) < Duration::from_secs(3600));
        if times.len() >= PER_HOUR {
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                "Zu viele Anfragen – bitte später erneut versuchen".into(),
            ));
        }
        let t = Ticket {
            id: self.ids()?.into_iter().max().unwrap_or(0) + 1,
            created_at: rfc3339(SystemTime::now()),
            practice: practice.to_owned(),
            kind: new.kind,
            subject: new.subject,
            message: new.message,
            reply_email: new.reply_email,
            questionnaire: new.questionnaire,
            app_version: new.app_version,
            os: new.os,
            status: Status::Open,
            admin_note: String::new(),
        };
        self.write(&t)?;
        times.push(now);
        Ok(t)
    }

    /// Every ticket, newest first; a file that does not parse is skipped (named in the log).
    pub async fn list(&self) -> Result<Vec<Ticket>, Fail> {
        let _w = self.sent.lock().await;
        let mut out: Vec<Ticket> = self
            .ids()?
            .into_iter()
            .filter_map(|id| {
                self.read(id)
                    .inspect_err(|_| eprintln!("tickets: {id:06}.json nicht lesbar, übersprungen"))
                    .ok()
            })
            .collect();
        out.sort_by_key(|t| std::cmp::Reverse(t.id));
        Ok(out)
    }

    pub async fn update(&self, id: u64, patch: Patch) -> Result<Ticket, Fail> {
        if let Some(n) = &patch.admin_note {
            if n.chars().count() > NOTE {
                return Err(bad(format!("admin_note: höchstens {NOTE} Zeichen")));
            }
        }
        let _w = self.sent.lock().await;
        let mut t = self.read(id)?;
        if let Some(s) = patch.status {
            t.status = s;
        }
        if let Some(n) = patch.admin_note {
            t.admin_note = n;
        }
        self.write(&t)?;
        Ok(t)
    }
}

/// `YYYY-MM-DDTHH:MM:SSZ` (civil-from-days, H. Hinnant); avoids a date crate for one timestamp.
fn rfc3339(at: SystemTime) -> String {
    let secs = at.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (days, rem) = ((secs / 86_400) as i64, secs % 86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem / 60 % 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339_dates() {
        let at = |s| rfc3339(UNIX_EPOCH + Duration::from_secs(s));
        assert_eq!(at(0), "1970-01-01T00:00:00Z");
        assert_eq!(at(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(at(1_767_225_599), "2025-12-31T23:59:59Z");
        assert_eq!(at(1_767_225_600), "2026-01-01T00:00:00Z");
    }

    #[test]
    fn emails() {
        assert!(valid_email("a@b.at"));
        for e in ["a", "@b.at", "a@b", "a b@c.at", "a@b@c.at"] {
            assert!(!valid_email(e), "{e}");
        }
    }
}
