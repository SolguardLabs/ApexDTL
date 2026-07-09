use serde_json::Value;
use std::process::Command;

fn scenario(name: &str) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_apex_dtl"))
        .arg(name)
        .output()
        .expect("el escenario debe ejecutarse");

    assert!(
        output.status.success(),
        "el escenario {name} fallo: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("el escenario debe emitir json")
}

fn assert_hex32(value: &Value) {
    let text = value.as_str().expect("el valor debe ser texto");
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
fn escenario_directo_liquida_saldos_esperados() {
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
fn escenario_enrutado_aplica_saldos_de_ruta() {
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
fn escenario_batch_acumula_liquidaciones() {
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
fn escenario_snapshot_conserva_estado_inicial() {
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
fn digests_son_deterministas_y_distintos() {
    let first = scenario("routed");
    let second = scenario("routed");
    let direct = scenario("direct");
    let batch = scenario("batch");

    assert_eq!(first["state_digest"], second["state_digest"]);
    assert_ne!(first["state_digest"], direct["state_digest"]);
    assert_ne!(first["state_digest"], batch["state_digest"]);
}
