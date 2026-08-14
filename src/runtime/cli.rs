use serde::Serialize;

use crate::{ApexError, ApexResult, VERSION};

#[derive(Serialize)]
struct VersionReport {
    protocol: &'static str,
    version: &'static str,
}

pub fn run() -> ApexResult<()> {
    let command = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "routed".to_owned());

    match command.as_str() {
        "direct" => print_json(&crate::runtime::scenarios::direct()?),
        "routed" => print_json(&crate::runtime::scenarios::routed()?),
        "batch" => print_json(&crate::runtime::scenarios::batch()?),
        "snapshot" => print_json(&crate::runtime::scenarios::snapshot()?),
        "quote" => print_json(&crate::runtime::scenarios::quote()?),
        "checkpoint" => print_json(&crate::runtime::scenarios::checkpoint()?),
        "version" => print_json(&VersionReport {
            protocol: "ApexDTL",
            version: VERSION,
        }),
        _ => Err(ApexError::Policy(format!(
            "unknown command {command}; expected direct, routed, batch, snapshot, quote, checkpoint or version"
        ))),
    }
}

fn print_json<T: Serialize>(value: &T) -> ApexResult<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .map_err(|error| ApexError::Serialization(error.to_string()))?
    );
    Ok(())
}
