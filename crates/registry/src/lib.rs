mod admin;
mod api;
mod config;
mod notify;
mod sync;
mod tickets;

pub use admin::MAX_UPLOAD;
pub use api::{credentials, router};
pub use config::{
    load, migrate, new_key, safe_id, Config, Entry, GitHub, Practice, Registry, Source,
};
pub use notify::{Channel, Notify, Telegram};
pub use tickets::{MAX_BODY as MAX_TICKET, PER_HOUR as TICKETS_PER_HOUR};
