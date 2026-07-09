use crate::ApexResult;

pub fn run() -> ApexResult<()> {
    let scenario = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "routed".to_owned());

    let report = match scenario.as_str() {
        "direct" => crate::runtime::scenarios::direct()?,
        "routed" => crate::runtime::scenarios::routed()?,
        "batch" => crate::runtime::scenarios::batch()?,
        "snapshot" => crate::runtime::scenarios::snapshot()?,
        _ => crate::runtime::scenarios::routed()?,
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| crate::ApexError::Serialization(error.to_string()))?
    );
    Ok(())
}
