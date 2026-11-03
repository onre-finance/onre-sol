// Run with pnpm test:compat. External sends and attestations use fixtures;
// SDK construction, serialization, signing, and command dispatch execute normally.
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { resolve } from "node:path";

const root = fileURLToPath(new URL("../../", import.meta.url));
const fixture = resolve(root, "scripts/tests/cctp-fixture.ts");
const amountArgs = ["--amount", "1000000", "--maxFee", "100", "--minFinalityThreshold", "2000"];

const cases = [
    { name: "CCTP v1 Solana to Ethereum", file: "cctp_v1/transfer_sol2eth.ts", args: [], solana: 1, evm: ["receiveMessage"] },
    { name: "CCTP v1 Ethereum to Solana", file: "cctp_v1/transfer_eth2sol.ts", args: [], solana: 1, evm: ["approve", "depositForBurn"] },
    { name: "CCTP v2 Solana to EVM", file: "cctp_v2/transfer-v2.ts", args: ["sol2evm", ...amountArgs], solana: 1, evm: ["receiveMessage"] },
    { name: "CCTP v2 Solana to EVM with hook", file: "cctp_v2/transfer-v2.ts", args: ["sol2evm", ...amountArgs, "--hookData", "0x1234"], solana: 1, evm: ["receiveMessage"] },
    { name: "CCTP v2 EVM to Solana", file: "cctp_v2/transfer-v2.ts", args: ["evm2sol", ...amountArgs], solana: 1, evm: ["approve", "depositForBurn"] },
    { name: "CCTP v2 EVM to Solana with hook", file: "cctp_v2/transfer-v2.ts", args: ["evm2sol", ...amountArgs, "--hookData", "0x1234"], solana: 1, evm: ["approve", "depositForBurnWithHook"] },
    { name: "CCTP v2 reclaim event account", file: "cctp_v2/transfer-v2.ts", args: ["reclaim", "--attestation", "0x1234", "--destinationMessage", "0x1234", "--messageSentEventAccount", "11111111111111111111111111111111"], solana: 1, evm: [] },
];

for (const scenario of cases) {
    test(scenario.name, () => {
        const result = spawnSync(process.execPath, ["--experimental-test-module-mocks", "--import", "tsx", "--import", fixture, resolve(root, "scripts/cross_chain_transfer", scenario.file), ...scenario.args], {
            cwd: root,
            env: { ...process.env, DOTENV_CONFIG_PATH: "/dev/null" },
            encoding: "utf8",
            timeout: 20_000,
        });
        assert.equal(result.status, 0, result.stdout + result.stderr);
        const captured = result.stdout.match(/COMPAT_CALLS=(.*)/);
        assert.ok(captured, result.stdout + result.stderr);
        const calls = JSON.parse(captured[1]) as { chain: string; method: string }[];
        assert.equal(calls.filter((c) => c.chain === "solana").length, scenario.solana);
        assert.deepEqual(calls.filter((c) => c.chain === "evm").map((c) => c.method), scenario.evm);
    });
}
