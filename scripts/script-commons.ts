import * as anchor from "@coral-xyz/anchor";
import { AnchorProvider, BN, Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { OnreApp } from "../target/types/onre_app";
import idl from "../target/idl/onre_app.json"; // assert { type: 'json' };

export const RPC_URL = process.env.SOL_MAINNET_RPC_URL || "";
// export const RPC_URL = "http://localhost:8899";

export const PROGRAM_ID = new PublicKey("onreuGhHHgVzMWSkj2oQDLDtvvGvoepBPkqyaubFcwe"); // PROD
// export const PROGRAM_ID = new PublicKey("J24jWEosQc5jgkdPm3YzNgzQ54CqNKkhzKy56XXJsLo2"); // TEST
// export const PROGRAM_ID = new PublicKey("devHfQHgiFNifkLW49RCXpyTUZMyKuBNnFSbrQ8XsbX"); // DEV

export async function initProgram() {
    const connection = new anchor.web3.Connection(RPC_URL);
    const wallet = new anchor.Wallet(anchor.web3.Keypair.generate());
    const provider = new AnchorProvider(connection, wallet);
    const program = new Program<OnreApp>(idl as OnreApp, provider);

    anchor.setProvider(provider);

    return program;
}

export async function getBossAccount(program: Program<OnreApp>) {
    const [statePda] = PublicKey.findProgramAddressSync([Buffer.from("state")], PROGRAM_ID);

    const stateAccount = await program.account.state.fetch(statePda);

    return stateAccount.boss;
}

export async function getOffer(offerId: BN, program: Program<OnreApp>) {
    const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from("offer"), offerId.toArrayLike(Buffer, "le", 8)], PROGRAM_ID);

    return program.account.offer.fetch(offerPda);
}
