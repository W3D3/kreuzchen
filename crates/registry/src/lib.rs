mod admin;
mod api;
mod config;
mod sync;

pub use admin::MAX_UPLOAD;
pub use api::router;
pub use config::{
    hash, load, new_key, safe_id, Config, Entry, GitHub, Practice, Registry, Source,
};
