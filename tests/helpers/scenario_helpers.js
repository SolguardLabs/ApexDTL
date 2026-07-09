import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const cache = new Map();
const cargo = process.platform === "win32" ? "cargo.exe" : "cargo";

export function scenario(name) {
    if (!cache.has(name)) {
        const stdout = execFileSync(cargo, ["run", "--quiet", "--", name], {
            cwd: root,
            encoding: "utf8",
            env: { ...process.env, CARGO_TERM_COLOR: "never" },
            timeout: 120_000,
        });
        cache.set(name, JSON.parse(stdout));
    }

    return cache.get(name);
}

export function assertHex32(value) {
    assert.equal(typeof value, "string");
    assert.match(value, /^[0-9a-f]{64}$/u);
}

export function assertCommon(payload, name) {
    assert.equal(payload.scenario, name);
    assert.equal(payload.network_id, 9_042);
    assert.equal(payload.total_supply, 10_000);
    assert.equal(payload.conservation_ok, true);
    assertHex32(payload.asset);
    assertHex32(payload.state_digest);
}
