use serde_json::Value;
use std::process::Command;

fn scenario(name: &str) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_apex_dtl"))
        .arg(name)
        .output()
        .expect("scenario must execute");

    assert!(
        output.status.success(),
        "scenario {name} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("scenario must emit json")
}

fn command(name: &str) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_apex_dtl"))
        .arg(name)
        .output()
        .expect("command must execute");
    assert!(
        output.status.success(),
        "command {name} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("command must emit json")
}

fn assert_hex32(value: &Value) {
    let text = value.as_str().expect("value must be text");
    assert_eq!(text.len(), 64);
    assert!(text.chars().all(|ch| ch.is_ascii_hexdigit()));
}

fn assert_common(payload: &Value, name: &str) {
    assert_eq!(payload["scenario"], name);
    assert_eq!(payload["network_id"], 9_042);
    assert_eq!(payload["total_supply"], 10_000);
    assert_eq!(payload["conservation_ok"], true);
    assert_hex32(&payload["asset"]);
    assert_hex32(&payload["state_digest"]);
}

#[test]
fn direct_scenario_settles_expected_balances() {
    let payload = scenario("direct");
    assert_common(&payload, "direct");
    assert_hex32(&payload["intent_id"]);
    assert_hex32(&payload["open_tx"]);
    assert_hex32(&payload["settlement_tx"]);
    assert_eq!(payload["balances"]["payer"], 9_100);
    assert_eq!(payload["balances"]["beneficiary"], 900);
    assert_eq!(payload["balances"]["solver"], 0);
    assert_eq!(payload["balances"]["integrator"], 0);
    assert_eq!(payload["balances"]["reserve"], 0);
    assert_eq!(payload["balances"]["sponsor"], 0);
}

#[test]
fn routed_scenario_applies_route_balances() {
    let payload = scenario("routed");
    assert_common(&payload, "routed");
    assert_hex32(&payload["intent_id"]);
    assert_eq!(payload["balances"]["payer"], 8_800);
    assert_eq!(payload["balances"]["beneficiary"], 1_185);
    assert_eq!(payload["balances"]["solver"], 12);
    assert_eq!(payload["balances"]["integrator"], 3);
    assert_eq!(payload["balances"]["reserve"], 0);
    assert_eq!(payload["balances"]["sponsor"], 0);
}

#[test]
fn batch_scenario_accumulates_settlements() {
    let payload = scenario("batch");
    assert_common(&payload, "batch");
    assert_hex32(&payload["intent_id"]);
    assert_eq!(payload["balances"]["payer"], 8_800);
    assert_eq!(payload["balances"]["beneficiary"], 1_180);
    assert_eq!(payload["balances"]["solver"], 8);
    assert_eq!(payload["balances"]["integrator"], 6);
    assert_eq!(payload["balances"]["reserve"], 6);
    assert_eq!(payload["balances"]["sponsor"], 0);
}

#[test]
fn snapshot_scenario_preserves_initial_state() {
    let payload = scenario("snapshot");
    assert_common(&payload, "snapshot");
    assert!(payload["intent_id"].is_null());
    assert!(payload["open_tx"].is_null());
    assert!(payload["settlement_tx"].is_null());
    assert_eq!(payload["balances"]["payer"], 10_000);
    assert_eq!(payload["balances"]["beneficiary"], 0);
    assert_eq!(payload["balances"]["solver"], 0);
    assert_eq!(payload["balances"]["integrator"], 0);
    assert_eq!(payload["balances"]["reserve"], 0);
    assert_eq!(payload["balances"]["sponsor"], 0);
}

#[test]
fn digests_are_deterministic_and_distinct() {
    let first = scenario("routed");
    let second = scenario("routed");
    let direct = scenario("direct");
    let batch = scenario("batch");

    assert_eq!(first["state_digest"], second["state_digest"]);
    assert_ne!(first["state_digest"], direct["state_digest"]);
    assert_ne!(first["state_digest"], batch["state_digest"]);
}

#[test]
fn structured_commands_expose_quote_checkpoint_and_version() {
    let quote = command("quote");
    assert_eq!(quote["scenario"], "quote");
    assert_eq!(quote["version"], "1.0.0");
    assert_eq!(quote["quote"]["effective_fee_bps"], 86);

    let checkpoint = command("checkpoint");
    assert_eq!(checkpoint["scenario"], "checkpoint");
    assert_eq!(checkpoint["checkpoint"]["sequence"], 1);
    assert_hex32(&checkpoint["checkpoint"]["checkpoint_digest"]);

    let version = command("version");
    assert_eq!(version["protocol"], "ApexDTL");
    assert_eq!(version["version"], "1.0.0");
}
