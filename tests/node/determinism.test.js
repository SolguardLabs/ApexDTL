import assert from "node:assert/strict";
import test from "node:test";

import { scenario } from "../helpers/scenario_helpers.js";

test("el mismo escenario produce digest estable", () => {
    const first = scenario("routed");
    const second = scenario("routed");
    assert.equal(first.state_digest, second.state_digest);
});

test("escenarios con flujos distintos producen digests distintos", () => {
    const direct = scenario("direct");
    const routed = scenario("routed");
    const batch = scenario("batch");

    assert.notEqual(direct.state_digest, routed.state_digest);
    assert.notEqual(routed.state_digest, batch.state_digest);
    assert.notEqual(direct.state_digest, batch.state_digest);
});

test("los ids de apertura y liquidacion no colisionan", () => {
    for (const name of ["direct", "routed", "batch"]) {
        const payload = scenario(name);
        assert.notEqual(payload.open_tx, payload.settlement_tx);
    }
});
