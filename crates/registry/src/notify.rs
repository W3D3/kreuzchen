//! Push notices about new tickets and new practice replies to Discord, Telegram and/or a JSON
//! webhook. A notice carries metadata and a link only, never the subject, a message or the reply
//! email. Delivery runs in the background and never affects the ticket request.

use std::sync::Arc;
use std::time::Duration;

use reqwest::{StatusCode, Url};
use serde_json::{json, Value};

use crate::tickets::{Kind, Ticket};

const TELEGRAM_API: &str = "https://api.telegram.org";

/// Channel settings, from the environment (`from_env`). Holds secrets: no `Debug`.
#[derive(Clone)]
pub struct Notify {
    /// Discord incoming webhook URL (contains its secret).
    pub discord_webhook: Option<String>,
    pub telegram: Option<Telegram>,
    /// Generic JSON webhook.
    pub webhook: Option<String>,
    /// Base URL of this server for links to `/admin`; without it notices have no link.
    pub public_url: Option<String>,
    /// Waits before each retry of a failed delivery.
    pub backoff: Vec<Duration>,
}

#[derive(Clone)]
pub struct Telegram {
    pub token: String,
    pub chat_id: String,
    /// `https://api.telegram.org`; tests point it at a mock.
    pub api_url: String,
}

impl Default for Notify {
    fn default() -> Self {
        Notify {
            discord_webhook: None,
            telegram: None,
            webhook: None,
            public_url: None,
            backoff: [2, 10, 30].map(Duration::from_secs).to_vec(),
        }
    }
}

impl Notify {
    /// Reads `KREUZCHEN_DISCORD_WEBHOOK_URL`, `KREUZCHEN_TELEGRAM_BOT_TOKEN` +
    /// `KREUZCHEN_TELEGRAM_CHAT_ID`, `KREUZCHEN_NOTIFY_WEBHOOK_URL` and `KREUZCHEN_PUBLIC_URL`
    /// through `var`; empty counts as unset. Errors name the variable, never its value.
    pub fn from_env(var: impl Fn(&str) -> Option<String>) -> Result<Notify, String> {
        let get = |k: &str| {
            var(k)
                .map(|v| v.trim().to_owned())
                .filter(|v| !v.is_empty())
        };
        let url = |k: &str| -> Result<Option<String>, String> {
            match get(k) {
                Some(v) if Url::parse(&v).is_ok_and(|u| matches!(u.scheme(), "http" | "https")) => {
                    Ok(Some(v))
                }
                Some(_) => Err(format!("{k} ist keine gültige http(s)-Adresse")),
                None => Ok(None),
            }
        };
        let telegram = match (
            get("KREUZCHEN_TELEGRAM_BOT_TOKEN"),
            get("KREUZCHEN_TELEGRAM_CHAT_ID"),
        ) {
            (Some(token), Some(chat_id)) => Some(Telegram {
                token,
                chat_id,
                api_url: TELEGRAM_API.into(),
            }),
            (None, None) => None,
            _ => return Err(
                "KREUZCHEN_TELEGRAM_BOT_TOKEN und KREUZCHEN_TELEGRAM_CHAT_ID nur zusammen setzen"
                    .into(),
            ),
        };
        Ok(Notify {
            discord_webhook: url("KREUZCHEN_DISCORD_WEBHOOK_URL")?,
            telegram,
            webhook: url("KREUZCHEN_NOTIFY_WEBHOOK_URL")?,
            public_url: url("KREUZCHEN_PUBLIC_URL")?,
            ..Notify::default()
        })
    }

    /// The configured channels.
    pub fn channels(&self) -> Vec<Channel> {
        [
            (self.discord_webhook.is_some(), Channel::Discord),
            (self.telegram.is_some(), Channel::Telegram),
            (self.webhook.is_some(), Channel::Webhook),
        ]
        .into_iter()
        .filter_map(|(on, name)| on.then_some(name))
        .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Event {
    NewTicket,
    NewReply,
}

/// What one notice says. Built from ticket metadata only (`notice`), so the channels' bodies
/// cannot contain user text.
#[derive(Debug)]
pub(crate) struct Notice {
    pub text: String,
    pub url: Option<String>,
    /// The generic webhook's JSON body.
    pub data: Value,
}

fn kind_label(k: Kind) -> &'static str {
    match k {
        Kind::Question => "Frage",
        Kind::Bug => "Fehler",
        Kind::Feedback => "Feedback",
        Kind::Wish => "Wunsch",
    }
}

fn link(public_url: Option<&str>, hash: &str) -> Option<String> {
    public_url.map(|u| format!("{}/admin#{hash}", u.trim_end_matches('/')))
}

/// The notice for a ticket event: number, practice, type and questionnaire, plus the link.
pub(crate) fn notice(event: Event, t: &Ticket, public_url: Option<&str>) -> Notice {
    let url = link(public_url, &format!("ticket-{}", t.id));
    let text = match event {
        Event::NewReply => format!("Neue Antwort auf Ticket #{} – {}", t.id, t.practice),
        Event::NewTicket => format!(
            "Neues Ticket #{} – {} – {} – {}",
            t.id,
            t.practice,
            kind_label(t.kind),
            t.questionnaire
                .as_ref()
                .map_or("Allgemein".to_owned(), |q| format!(
                    "{} (v{})",
                    q.title, q.version
                ))
        ),
    };
    let data = json!({
        "event": if event == Event::NewReply { "new_reply" } else { "new_ticket" },
        "ticket_id": t.id,
        "practice": t.practice,
        "type": t.kind,
        "questionnaire": t.questionnaire.as_ref().map(|q| json!({"id": q.id, "title": q.title, "version": q.version})),
        "url": url,
    });
    Notice { text, url, data }
}

pub(crate) fn test_notice(public_url: Option<&str>) -> Notice {
    let url = link(public_url, "tickets");
    Notice {
        text: "Testnachricht vom Bibliotheksserver – Benachrichtigungen sind eingerichtet.".into(),
        data: json!({"event": "test", "ticket_id": null, "practice": null, "type": null, "questionnaire": null, "url": url}),
        url,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Channel {
    Discord,
    Telegram,
    Webhook,
}

impl Channel {
    pub fn name(self) -> &'static str {
        match self {
            Channel::Discord => "discord",
            Channel::Telegram => "telegram",
            Channel::Webhook => "webhook",
        }
    }
}

/// A failed delivery: a message without secrets, and whether trying again may help.
type Failed = (String, bool);

// No Debug: it holds the secrets.
pub(crate) struct Notifier {
    cfg: Notify,
    client: reqwest::Client,
}

impl Notifier {
    pub fn new(cfg: Notify) -> Self {
        // Same HTTP stack as the GitHub sync; a second install attempt is a harmless Err.
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("kreuzchen-registry")
            .build()
            .expect("HTTP client");
        Notifier { cfg, client }
    }

    /// Which channels are set up; never the secrets.
    pub fn status_json(&self) -> Value {
        let channels: Vec<_> = self.cfg.channels().into_iter().map(Channel::name).collect();
        json!({"channels": channels, "public_url": self.cfg.public_url})
    }

    /// Target URL and JSON body for one channel, `None` when it is not configured.
    pub(crate) fn request(&self, ch: Channel, n: &Notice) -> Option<(String, Value)> {
        let content = match &n.url {
            Some(u) => format!("{}\n{u}", n.text),
            None => n.text.clone(),
        };
        Some(match ch {
            Channel::Discord => (
                self.cfg.discord_webhook.clone()?,
                // Practice-sent titles must not ping anyone.
                json!({"content": content, "allowed_mentions": {"parse": []}}),
            ),
            Channel::Telegram => {
                let t = self.cfg.telegram.as_ref()?;
                let mut url = Url::parse(&t.api_url).ok()?;
                url.path_segments_mut()
                    .ok()?
                    .pop_if_empty()
                    .extend([format!("bot{}", t.token).as_str(), "sendMessage"]);
                (
                    url.into(),
                    json!({"chat_id": t.chat_id, "text": content, "disable_web_page_preview": true}),
                )
            }
            Channel::Webhook => (self.cfg.webhook.clone()?, n.data.clone()),
        })
    }

    /// One attempt. The URL holds secrets (webhook token, bot token): errors never include it.
    async fn post(&self, ch: Channel, n: &Notice) -> Result<(), Failed> {
        let (url, body) = self
            .request(ch, n)
            .ok_or(("nicht eingerichtet".to_owned(), false))?;
        let resp = self
            .client
            .post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.to_string())
            .send()
            .await
            .map_err(|e| (format!("nicht erreichbar: {}", e.without_url()), true))?;
        let status = resp.status();
        if status.is_success() {
            return Ok(());
        }
        let hint = match (ch, status.as_u16()) {
            (Channel::Telegram, 401 | 404) => " – Bot-Token prüfen",
            (Channel::Telegram, 400 | 403) => " – Chat-ID prüfen und dem Bot einmal schreiben",
            (_, 401 | 403 | 404) => " – Webhook-Adresse prüfen",
            _ => "",
        };
        let retry = status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS;
        Err((format!("antwortet {status}{hint}"), retry))
    }

    /// Sends in the background to every channel, retrying with backoff; failures are logged.
    pub fn ticket(self: &Arc<Self>, event: Event, t: &Ticket) {
        let n = Arc::new(notice(event, t, self.cfg.public_url.as_deref()));
        for ch in self.cfg.channels() {
            let (me, n, id) = (self.clone(), n.clone(), t.id);
            tokio::spawn(async move {
                let mut waits = me.cfg.backoff.iter();
                let mut tries = 1;
                loop {
                    match me.post(ch, &n).await {
                        Ok(()) => return,
                        Err((msg, retry)) => match waits.next().filter(|_| retry) {
                            Some(w) => {
                                tokio::time::sleep(*w).await;
                                tries += 1;
                            }
                            None => {
                                eprintln!(
                                    "notify {}: Ticket #{id} nicht zugestellt ({tries} Versuche): {msg}",
                                    ch.name()
                                );
                                return;
                            }
                        },
                    }
                }
            });
        }
    }

    /// One attempt per configured channel, for the admin's test button: `(channel, error)`.
    pub async fn test(&self) -> Vec<(Channel, Option<String>)> {
        let n = test_notice(self.cfg.public_url.as_deref());
        let mut out = Vec::new();
        for ch in self.cfg.channels() {
            out.push((ch, self.post(ch, &n).await.err().map(|(m, _)| m)));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tickets::{Author, QRef, Reply, Status};

    const SECRETS: [&str; 4] = [
        "GEHEIMER BETREFF",
        "Nachricht mit Patientendaten",
        "antwort@example.org",
        "Antworttext der Praxis",
    ];

    fn ticket(questionnaire: bool) -> Ticket {
        Ticket {
            id: 12,
            created_at: "2026-11-26T10:00:00Z".into(),
            practice: "praxis_mitsche".into(),
            kind: Kind::Bug,
            subject: SECRETS[0].into(),
            message: SECRETS[1].into(),
            reply_email: Some(SECRETS[2].into()),
            questionnaire: questionnaire.then(|| QRef {
                id: "srs2".into(),
                title: "SRS-2 4–18".into(),
                version: "2".into(),
                hash: "abc".into(),
            }),
            app_version: "0.7.0".into(),
            os: "linux".into(),
            status: Status::Open,
            admin_note: "interne Notiz".into(),
            replies: vec![Reply {
                id: 1,
                author: Author::Practice,
                created_at: "2026-11-26T11:00:00Z".into(),
                message: SECRETS[3].into(),
            }],
        }
    }

    fn all_channels(public_url: Option<&str>) -> Notifier {
        Notifier::new(Notify {
            discord_webhook: Some("https://discord.example/api/webhooks/1/tok".into()),
            telegram: Some(Telegram {
                token: "123:bot-secret".into(),
                chat_id: "42".into(),
                api_url: "https://tg.example".into(),
            }),
            webhook: Some("https://hook.example/x".into()),
            public_url: public_url.map(str::to_owned),
            ..Notify::default()
        })
    }

    #[test]
    fn texts_and_links() {
        let n = notice(
            Event::NewTicket,
            &ticket(true),
            Some("https://bib.example/"),
        );
        assert_eq!(
            n.text,
            "Neues Ticket #12 – praxis_mitsche – Fehler – SRS-2 4–18 (v2)"
        );
        assert_eq!(
            n.url.as_deref(),
            Some("https://bib.example/admin#ticket-12")
        );
        let n = notice(Event::NewReply, &ticket(true), None);
        assert_eq!(n.text, "Neue Antwort auf Ticket #12 – praxis_mitsche");
        assert_eq!(n.url, None);
        assert!(notice(Event::NewTicket, &ticket(false), None)
            .text
            .ends_with("– Fehler – Allgemein"));
    }

    #[test]
    fn bodies_carry_metadata_only() {
        for public_url in [None, Some("https://bib.example")] {
            let notifier = all_channels(public_url);
            for q in [true, false] {
                for n in [
                    notice(Event::NewTicket, &ticket(q), public_url),
                    notice(Event::NewReply, &ticket(q), public_url),
                    test_notice(public_url),
                ] {
                    for ch in [Channel::Discord, Channel::Telegram, Channel::Webhook] {
                        let (_, body) = notifier.request(ch, &n).unwrap();
                        let body = body.to_string();
                        for s in SECRETS.iter().chain(&["interne Notiz", "0.7.0"]) {
                            assert!(!body.contains(s), "{} leaks {s}: {body}", ch.name());
                        }
                        assert_eq!(body.contains("/admin#"), public_url.is_some(), "{body}");
                    }
                }
            }
        }
    }

    #[test]
    fn request_shapes() {
        let notifier = all_channels(Some("https://bib.example"));
        let n = notice(Event::NewTicket, &ticket(true), Some("https://bib.example"));
        let (url, body) = notifier.request(Channel::Telegram, &n).unwrap();
        assert_eq!(url, "https://tg.example/bot123:bot-secret/sendMessage");
        assert_eq!(body["chat_id"], "42");
        assert_eq!(
            body["text"],
            "Neues Ticket #12 – praxis_mitsche – Fehler – SRS-2 4–18 (v2)\nhttps://bib.example/admin#ticket-12"
        );
        let (_, body) = notifier.request(Channel::Discord, &n).unwrap();
        assert_eq!(body["content"], n_text(&n));
        assert_eq!(body["allowed_mentions"]["parse"], json!([]));
        let (_, body) = notifier.request(Channel::Webhook, &n).unwrap();
        assert_eq!(
            body,
            json!({"event": "new_ticket", "ticket_id": 12, "practice": "praxis_mitsche", "type": "bug",
                   "questionnaire": {"id": "srs2", "title": "SRS-2 4–18", "version": "2"},
                   "url": "https://bib.example/admin#ticket-12"})
        );
    }

    fn n_text(n: &Notice) -> String {
        format!("{}\n{}", n.text, n.url.as_deref().unwrap())
    }

    #[test]
    fn env_parsing() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |k: &str| {
                pairs
                    .iter()
                    .find(|(n, _)| *n == k)
                    .map(|(_, v)| v.to_string())
            }
        };
        let n = Notify::from_env(env(&[])).unwrap();
        assert!(n.channels().is_empty() && n.public_url.is_none());
        let n = Notify::from_env(env(&[
            ("KREUZCHEN_DISCORD_WEBHOOK_URL", "https://discord.example/x"),
            ("KREUZCHEN_TELEGRAM_BOT_TOKEN", "1:abc"),
            ("KREUZCHEN_TELEGRAM_CHAT_ID", "42"),
            ("KREUZCHEN_NOTIFY_WEBHOOK_URL", " "),
        ]))
        .unwrap();
        assert_eq!(n.channels(), [Channel::Discord, Channel::Telegram]);
        let only_token = Notify::from_env(env(&[("KREUZCHEN_TELEGRAM_BOT_TOKEN", "1:abc")]));
        assert!(only_token.is_err_and(|e| !e.contains("1:abc")));
        let bad = Notify::from_env(env(&[("KREUZCHEN_DISCORD_WEBHOOK_URL", "geheim")]));
        assert!(bad
            .is_err_and(|e| e.contains("KREUZCHEN_DISCORD_WEBHOOK_URL") && !e.contains("geheim")));
    }
}
