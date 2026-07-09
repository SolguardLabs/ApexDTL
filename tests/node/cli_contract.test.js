import assert from "node:assert/strict";
import test from "node:test";

import { assertCommon, assertHex32, scenario } from "../helpers/scenario_helpers.js";

test("los escenarios exponen el contrato superior esperado", () => {
    for (const name of ["direct", "routed", "batch", "snapshot"]) {
        const payload = scenario(name);
        assertCommon(payload, name);

        for (const key of [
            "scenario",
            "network_id",
            "asset",
            "intent_id",
            "open_tx",
            "settlement_tx",
            "balances",
            "total_supply",
            "state_digest",
            "conservation_ok",
        ]) {
            assert.ok(Object.hasOwn(payload, key), `${name} no incluye ${key}`);
        }
    }
});

test("los escenarios liquidados serializan identificadores de 32 bytes", () => {
    for (const name of ["direct", "routed", "batch"]) {
        const payload = scenario(name);
        assertHex32(payload.intent_id);
        assertHex32(payload.open_tx);
        assertHex32(payload.settlement_tx);
    }
});

test("snapshot no incluye actividad de intent", () => {
    const payload = scenario("snapshot");
    assert.equal(payload.intent_id, null);
    assert.equal(payload.open_tx, null);
    assert.equal(payload.settlement_tx, null);
});
