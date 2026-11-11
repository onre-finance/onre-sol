import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { ASSOCIATED_TOKEN_PROGRAM_ID } from "@solana/spl-token";

import { ACTIVE_OFFERS, MINTS } from "../constants";
import { assertBigIntEq, assertBigIntGt, assertNumberGt } from "../assertions";
import { ata, configurableVaultAta, tokenProgramFor } from "../accounts";
import { ConfigurableVaultName, configurableVaultKinds, configurableVaultPda, PDAS } from "../pdas";
import { bn, SmokeRuntime, sendIxs } from "../runtime";

const VAULT_KINDS: ConfigurableVaultName[] = ["offerFee", "permissionlessOfferFee", "redemptionFee", "managementFee", "performanceFee", "propAmmBuyFee", "propAmmSellFee", "offerProceeds", "propAmmProceeds"];

export type VaultRecipients = Record<ConfigurableVaultName, PublicKey>;

export async function configureVaultRecipientsSmoke(runtime: SmokeRuntime): Promise<VaultRecipients> {
    console.log("\n== Configurable vault recipients ==");

    const recipients = Object.fromEntries(VAULT_KINDS.map((kind) => [kind, Keypair.generate().publicKey])) as VaultRecipients;

    const ixs = await Promise.all(
        VAULT_KINDS.map((kind) =>
            runtime.program.methods
                .setConfigurableVaultDestination(configurableVaultKinds[kind] as never, recipients[kind])
                .accountsPartial({
                    state: PDAS.state,
                    boss: runtime.authority.publicKey,
                    configurableVault: configurableVaultPda(kind),
                    systemProgram: SystemProgram.programId,
                })
                .instruction(),
        ),
    );
    await sendIxs(runtime, "set all configurable vault destinations", ixs);

    for (const kind of VAULT_KINDS) {
        const vault = await runtime.program.account.configurableVault.fetch(configurableVaultPda(kind));
        if (!vault.withdrawalDestination.equals(recipients[kind])) {
            throw new Error(`${kind} recipient was not stored`);
        }
        console.log(`  ok ${kind} recipient ${recipients[kind].toBase58()}`);
    }

    return recipients;
}

export async function withdrawNonZeroVaultsSmoke(runtime: SmokeRuntime, recipients: VaultRecipients): Promise<void> {
    console.log("\n== Configurable vault withdrawals ==");
    const candidateMints = [MINTS.onyc, ...ACTIVE_OFFERS.map((offer) => offer.mint)];
    let withdrawals = 0;

    for (const kind of VAULT_KINDS) {
        for (const mint of candidateMints) {
            const vaultTokenAccount = configurableVaultAta(kind, mint);
            const balance = await runtime.connection.getTokenAccountBalance(vaultTokenAccount).catch(() => null);
            const amount = BigInt(balance?.value.amount ?? "0");
            if (amount === 0n) {
                continue;
            }

            const destinationTokenAccount = ata(mint, recipients[kind]);
            const before = await runtime.connection.getTokenAccountBalance(destinationTokenAccount).catch(() => null);
            const beforeAmount = BigInt(before?.value.amount ?? "0");
            const ix = await runtime.program.methods
                .withdrawConfigurableVault(configurableVaultKinds[kind] as never, bn(0))
                .accountsPartial({
                    state: PDAS.state,
                    caller: runtime.authority.publicKey,
                    configurableVault: configurableVaultPda(kind),
                    vaultTokenAccount,
                    destination: recipients[kind],
                    destinationTokenAccount,
                    mint,
                    tokenProgram: tokenProgramFor(mint),
                    associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
                    systemProgram: SystemProgram.programId,
                })
                .instruction();
            await sendIxs(runtime, `withdraw full ${kind} ${mint.toBase58()}`, [ix]);

            const after = await runtime.connection.getTokenAccountBalance(destinationTokenAccount);
            const afterAmount = BigInt(after.value.amount);
            assertBigIntGt(afterAmount, beforeAmount, `${kind} withdrawal destination balance`);
            assertBigIntEq(afterAmount - beforeAmount, amount, `${kind} withdrawal destination delta`);
            const vaultAfter = await runtime.connection.getTokenAccountBalance(vaultTokenAccount).catch(() => null);
            assertBigIntEq(BigInt(vaultAfter?.value.amount ?? "0"), 0n, `${kind} vault drained after full withdrawal`);
            withdrawals += 1;
        }
    }

    assertNumberGt(withdrawals, 0, "non-zero configurable vault withdrawals checked");
    console.log(`  ok ${withdrawals} non-zero configurable vault withdrawal(s) checked`);
}
