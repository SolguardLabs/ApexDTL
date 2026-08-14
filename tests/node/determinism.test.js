import assert from "node:assert/strict";
import test from "node:test";

import { scenario } from "../helpers/scenario_helpers.js";

test("the same scenario produces a stable digest", () => {
    const first = scenario("routed");
    const second = scenario("routed");
    assert.equal(first.state_digest, second.state_digest);
});

test("different flows produce distinct digests", () => {
    const direct = scenario("direct");
    const routed = scenario("routed");
    const batch = scenario("batch");

    assert.notEqual(direct.state_digest, routed.state_digest);
    assert.notEqual(routed.state_digest, batch.state_digest);
    assert.notEqual(direct.state_digest, batch.state_digest);
});

test("open and settlement ids do not collide", () => {
    for (const name of ["direct", "routed", "batch"]) {
        const payload = scenario(name);
        assert.notEqual(payload.open_tx, payload.settlement_tx);
    }
});
