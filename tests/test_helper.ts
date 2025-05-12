import { getAssociatedTokenAddressSync } from "@solana/spl-token";

import { ACCOUNT_SIZE, AccountLayout, TOKEN_PROGRAM_ID } from "@solana/spl-token";

import { MINT_SIZE, MintLayout } from "@solana/spl-token";
import { PublicKey } from "@solana/web3.js";
import { ProgramTestContext } from "solana-bankrun";
import { INITIAL_LAMPORTS } from "./test_constants";

export function createMint(context: ProgramTestContext, mintAuthority: PublicKey, supply: bigint = BigInt(100_000e9), decimals: number = 9, freezeAuthority: PublicKey = mintAuthority): PublicKey {
    const mintData = Buffer.alloc(MINT_SIZE);
    MintLayout.encode({
        mintAuthorityOption: 0,
        mintAuthority: mintAuthority,
        supply: supply,
        decimals: decimals,
        isInitialized: true,
        freezeAuthorityOption: 0,
        freezeAuthority: freezeAuthority,
    }, mintData)
    
    const mintAddress = PublicKey.unique();
    context.setAccount(mintAddress, {
        executable: false,
        data: mintData,
        lamports: INITIAL_LAMPORTS,
        owner: TOKEN_PROGRAM_ID,
    });

    return mintAddress
};

export function createTokenAccount(context: ProgramTestContext, mint: PublicKey, owner: PublicKey, amount: bigint, allowOwnerOffCurve: boolean = false): PublicKey {
    const tokenAccountData = Buffer.alloc(ACCOUNT_SIZE);
    AccountLayout.encode({
        mint: mint,
        owner: owner,
        amount: amount,
        delegateOption: 0,
        delegate: PublicKey.default,
        state: 1,
        isNativeOption: 0,
        isNative: BigInt(0),
        delegatedAmount: BigInt(0),
        closeAuthorityOption: 0,
        closeAuthority: PublicKey.default,
    }, tokenAccountData);

    const tokenAccountAddress = getAssociatedTokenAddressSync(mint, owner, allowOwnerOffCurve);

    context.setAccount(tokenAccountAddress, {
        executable: false,
        data: tokenAccountData,
        lamports: INITIAL_LAMPORTS,
        owner: TOKEN_PROGRAM_ID,
    });

    return tokenAccountAddress;
}