//! Upgrades 0.4 definition files to the English names in place, then validates them:
//! `cargo run -p kreuzchen-engine --example upgrade -- <file.yaml>…`
use std::{fs, process::ExitCode};

fn main() -> ExitCode {
    let mut ok = true;
    for path in std::env::args().skip(1) {
        let text = match fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{path}: {e}");
                ok = false;
                continue;
            }
        };
        let text = match kreuzchen_engine::migrate::upgrade(&text) {
            Ok(Some(new)) => {
                fs::write(&path, &new).expect("write");
                println!("{path}: upgraded");
                new
            }
            Ok(None) => {
                println!("{path}: nothing to upgrade");
                text
            }
            Err(e) => {
                eprintln!("{path}: cannot upgrade: {e}");
                ok = false;
                continue;
            }
        };
        if let Err(errs) = kreuzchen_engine::load(&text) {
            errs.iter().for_each(|e| eprintln!("{path}: {e}"));
            ok = false;
        }
    }
    if ok { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}
