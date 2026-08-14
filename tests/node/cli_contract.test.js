import assert from "node:assert/strict";
import test from "node:test";

import { assertCommon, assertHex32, scenario } from "../helpers/scenario_helpers.js";

test("scenarios expose the expected top-level contract", () => {
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
            assert.ok(Object.hasOwn(payload, key), `${name} does not include ${key}`);
        }
    }
});

test("settled scenarios serialize 32-byte identifiers", () => {
    for (const name of ["direct", "routed", "batch"]) {
        const payload = scenario(name);
        assertHex32(payload.intent_id);
        assertHex32(payload.open_tx);
        assertHex32(payload.settlement_tx);
    }
});

test("snapshot does not include intent activity", () => {
    const payload = scenario("snapshot");
    assert.equal(payload.intent_id, null);
    assert.equal(payload.open_tx, null);
    assert.equal(payload.settlement_tx, null);
});

test("structured commands expose quote, checkpoint and version", () => {
    const quote = scenario("quote");
    assert.equal(quote.scenario, "quote");
    assert.equal(quote.version, "1.0.0");
    assert.equal(quote.quote.effective_fee_bps, 86);

    const checkpoint = scenario("checkpoint");
    assert.equal(checkpoint.scenario, "checkpoint");
    assert.equal(checkpoint.checkpoint.sequence, 1);
    assertHex32(checkpoint.checkpoint.checkpoint_digest);

    const version = scenario("version");
    assert.equal(version.protocol, "ApexDTL");
    assert.equal(version.version, "1.0.0");
});
