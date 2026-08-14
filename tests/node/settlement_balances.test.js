import assert from "node:assert/strict";
import test from "node:test";

import { assertCommon, scenario } from "../helpers/scenario_helpers.js";

test("direct preserves supply and settles direct balances", () => {
    const payload = scenario("direct");
    assertCommon(payload, "direct");
    assert.deepEqual(payload.balances, {
        payer: 9_100,
        beneficiary: 900,
        solver: 0,
        integrator: 0,
        reserve: 0,
        sponsor: 0,
    });
});

test("routed preserves supply and distributes declared charges", () => {
    const payload = scenario("routed");
    assertCommon(payload, "routed");
    assert.deepEqual(payload.balances, {
        payer: 8_800,
        beneficiary: 1_185,
        solver: 12,
        integrator: 3,
        reserve: 0,
        sponsor: 0,
    });
});

test("batch accumulates two settlements in one ledger", () => {
    const payload = scenario("batch");
    assertCommon(payload, "batch");
    assert.deepEqual(payload.balances, {
        payer: 8_800,
        beneficiary: 1_180,
        solver: 8,
        integrator: 6,
        reserve: 6,
        sponsor: 0,
    });
});

test("snapshot preserves the funded initial state", () => {
    const payload = scenario("snapshot");
    assertCommon(payload, "snapshot");
    assert.deepEqual(payload.balances, {
        payer: 10_000,
        beneficiary: 0,
        solver: 0,
        integrator: 0,
        reserve: 0,
        sponsor: 0,
    });
});
