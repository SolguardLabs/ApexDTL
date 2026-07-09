import assert from "node:assert/strict";
import test from "node:test";

import { assertCommon, scenario } from "../helpers/scenario_helpers.js";

test("direct conserva suministro y liquida saldo directo", () => {
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

test("routed conserva suministro y distribuye cargos declarados", () => {
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

test("batch acumula dos liquidaciones en el mismo ledger", () => {
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

test("snapshot conserva el estado inicial financiado", () => {
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
