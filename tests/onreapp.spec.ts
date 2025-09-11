import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Keypair, PublicKey, TransactionMessage, VersionedTransaction } from "@solana/web3.js";
import { OnreApp } from "../target/types/onre_app";
import {
    createAssociatedTokenAccount,
    createAssociatedTokenAccountInstruction,
    createMint,
    getAssociatedTokenAddress,
    getAssociatedTokenAddressSync,
    mintTo,
    TOKEN_PROGRAM_ID,
} from "@solana/spl-token";

async function airdropLamports(provider: anchor.AnchorProvider, publicKey: anchor.web3.PublicKey, amount: number) {
    const signature = await provider.connection.requestAirdrop(publicKey, amount);
    await provider.connection.confirmTransaction({ signature, ...(await provider.connection.getLatestBlockhash()) });
    return signature;
}

async function mintToAddress(
    provider: anchor.AnchorProvider,
    payer: anchor.web3.Signer | anchor.web3.Keypair,
    mint: anchor.web3.PublicKey,
    destination: anchor.web3.PublicKey,
    authority: anchor.web3.PublicKey | anchor.web3.Signer,
    amount: number | bigint,
) {
    await mintTo(provider.connection, payer, mint, destination, authority, amount);
}

async function createATA(provider: anchor.AnchorProvider, payer: anchor.web3.Signer | anchor.web3.Keypair, mint: anchor.web3.PublicKey, owner: anchor.web3.PublicKey) {
    return await createAssociatedTokenAccount(provider.connection, payer, mint, owner);
}

async function createAndSendTransaction(provider: anchor.AnchorProvider, payer: anchor.Wallet, instructions: anchor.web3.TransactionInstruction[]) {
    const tx = new VersionedTransaction(
        new TransactionMessage({
            payerKey: payer.publicKey,
            recentBlockhash: (await provider.connection.getLatestBlockhash()).blockhash,
            instructions,
        }).compileToLegacyMessage(),
    );
    const versionedTransaction = await payer.signTransaction(tx);
    const signedTransactionBytes = versionedTransaction.serialize();
    const signature = await provider.connection.sendRawTransaction(signedTransactionBytes);
    await provider.connection.confirmTransaction({
        signature,
        ...(await provider.connection.getLatestBlockhash()),
    });
    return signature;
}

async function createIntermediaryAccountsIfNeeded(provider: anchor.AnchorProvider, payer: anchor.Wallet, buyTokenMint: PublicKey, sellTokenMint: PublicKey, programId: PublicKey) {
    const [intermediaryAuthority] = PublicKey.findProgramAddressSync([Buffer.from("permissionless-1")], programId);
    const intermediaryBuyTokenAccount = await getAssociatedTokenAddress(buyTokenMint, intermediaryAuthority, true);
    const intermediarySellTokenAccount = await getAssociatedTokenAddress(sellTokenMint, intermediaryAuthority, true);

    const instructions: anchor.web3.TransactionInstruction[] = [];

    // Check if intermediary buy token account exists
    const buyTokenAccountInfo = await provider.connection.getAccountInfo(intermediaryBuyTokenAccount);
    if (!buyTokenAccountInfo) {
        instructions.push(createAssociatedTokenAccountInstruction(payer.publicKey, intermediaryBuyTokenAccount, intermediaryAuthority, buyTokenMint));
    }

    // Check if intermediary sell token account exists
    const sellTokenAccountInfo = await provider.connection.getAccountInfo(intermediarySellTokenAccount);
    if (!sellTokenAccountInfo) {
        instructions.push(createAssociatedTokenAccountInstruction(payer.publicKey, intermediarySellTokenAccount, intermediaryAuthority, sellTokenMint));
    }

    // Only send transaction if there are instructions to execute
    if (instructions.length > 0) {
        await createAndSendTransaction(provider, payer, instructions);
    }
}

describe("onreapp", () => {
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);
    const initialBoss = provider.wallet as anchor.Wallet;

    const program = anchor.workspace.OnreApp as Program<OnreApp>;
    let sellTokenMint: PublicKey;
    let buyToken1Mint: PublicKey;
    let buyToken2Mint: PublicKey;
    let bossSellTokenAccount: PublicKey;
    let bossBuyTokenAccount1: PublicKey;
    let bossBuyTokenAccount2: PublicKey;
    let offerPda: PublicKey;
    let offerBuyToken1Pda: PublicKey;
    let offerId = new anchor.BN(123123123);
    let statePda: PublicKey;
    let offerAuthority: PublicKey;

    beforeAll(async () => {
        await airdropLamports(provider, initialBoss.publicKey, anchor.web3.LAMPORTS_PER_SOL * 20);

        sellTokenMint = await createMint(provider.connection, initialBoss.payer, initialBoss.publicKey, null, 9);
        buyToken1Mint = await createMint(provider.connection, initialBoss.payer, initialBoss.publicKey, null, 9);
        buyToken2Mint = await createMint(provider.connection, initialBoss.payer, initialBoss.publicKey, null, 9);

        bossSellTokenAccount = await createATA(provider, initialBoss.payer, sellTokenMint, initialBoss.publicKey);
        bossBuyTokenAccount1 = await createATA(provider, initialBoss.payer, buyToken1Mint, initialBoss.publicKey);
        bossBuyTokenAccount2 = await createATA(provider, initialBoss.payer, buyToken2Mint, initialBoss.publicKey);

        await mintToAddress(provider, initialBoss.payer, sellTokenMint, bossSellTokenAccount, initialBoss.publicKey, 10000e9);
        await mintToAddress(provider, initialBoss.payer, buyToken1Mint, bossBuyTokenAccount1, initialBoss.publicKey, 10000e9);
        await mintToAddress(provider, initialBoss.payer, buyToken2Mint, bossBuyTokenAccount2, initialBoss.publicKey, 10000e9);

        [offerPda] = PublicKey.findProgramAddressSync([Buffer.from("offer"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        [statePda] = PublicKey.findProgramAddressSync([Buffer.from("state")], program.programId);
        [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from("offer_authority"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        await getAssociatedTokenAddress(sellTokenMint, offerAuthority, true);
        offerBuyToken1Pda = await getAssociatedTokenAddress(buyToken1Mint, offerAuthority, true);
        await getAssociatedTokenAddress(buyToken2Mint, offerAuthority, true);
    });

    it("Initialize onre with right boss account", async () => {
        await program.methods.initialize().accounts({ boss: initialBoss.publicKey }).rpc();
        const currentBoss = await program.account.state.fetch(statePda);
        expect(currentBoss.boss).toEqual(initialBoss.publicKey);
    });

    it("Set boss account sets a new boss account", async () => {
        const newBoss = new anchor.Wallet(Keypair.generate());
        await program.methods.setBoss(newBoss.publicKey).accounts({ state: statePda }).rpc();
        const currentBoss = await program.account.state.fetch(statePda);
        expect(currentBoss.boss).toEqual(newBoss.publicKey);

        await airdropLamports(provider, newBoss.publicKey, anchor.web3.LAMPORTS_PER_SOL * 200);
        const setBossInstruction = await program.methods.setBoss(initialBoss.publicKey).accountsPartial({ state: statePda, boss: newBoss.publicKey }).instruction();
        await createAndSendTransaction(provider, newBoss, [setBossInstruction]);

        const finalBoss = await program.account.state.fetch(statePda);
        expect(finalBoss.boss).toEqual(initialBoss.publicKey);
    });

    it("Makes an offer", async () => {
        const offerSellTokenAccountInstruction = createAssociatedTokenAccountInstruction(
            initialBoss.payer.publicKey,
            getAssociatedTokenAddressSync(sellTokenMint, offerAuthority, true),
            offerAuthority,
            sellTokenMint,
        );
        const buyToken1AccountInstruction = createAssociatedTokenAccountInstruction(
            initialBoss.payer.publicKey,
            getAssociatedTokenAddressSync(buyToken1Mint, offerAuthority, true),
            offerAuthority,
            buyToken1Mint,
        );

        await program.methods
            .makeOfferOne(
                offerId,
                new anchor.BN(500e9),
                new anchor.BN(200e9),
                new anchor.BN(400e9),
                new anchor.BN(Date.now()),
                new anchor.BN(Date.now() + 7200),
                new anchor.BN(3600),
            )
            .accounts({ sellTokenMint, buyToken1Mint, state: statePda })
            .preInstructions([buyToken1AccountInstruction, offerSellTokenAccountInstruction])
            .rpc();

        const offerAccount = await program.account.offer.fetch(offerPda);
        expect(offerAccount.offerId.eq(offerId)).toBe(true);
        expect(offerAccount.sellTokenMint.toBase58()).toEqual(sellTokenMint.toBase58());
        expect(offerAccount.buyToken1.mint.toBase58()).toEqual(buyToken1Mint.toBase58());
        expect(offerAccount.sellTokenEndAmount.eq(new anchor.BN(400e9))).toBe(true);
        expect(offerAccount.buyToken1.amount.eq(new anchor.BN(500e9))).toBe(true);

        const bossBuyTokenAccountInfo = await provider.connection.getTokenAccountBalance(bossBuyTokenAccount1);
        expect(+bossBuyTokenAccountInfo.value.amount).toEqual(9500e9);

        const offerBuyTokenAccountInfo = await provider.connection.getTokenAccountBalance(offerBuyToken1Pda);
        expect(+offerBuyTokenAccountInfo.value.amount).toEqual(500e9);
    });

    it("Make offer fails on boss account with non boss signature", async () => {
        const newBoss = new anchor.Wallet(Keypair.generate());
        await airdropLamports(provider, newBoss.publicKey, anchor.web3.LAMPORTS_PER_SOL * 20);
        await expect(
            program.methods
                .makeOfferOne(
                    offerId,
                    new anchor.BN(500e9),
                    new anchor.BN(200e9),
                    new anchor.BN(400e9),
                    new anchor.BN(Date.now()),
                    new anchor.BN(Date.now() + 1),
                    new anchor.BN(3600),
                )
                .accountsPartial({ bossBuyToken1Account: bossBuyTokenAccount1, sellTokenMint, buyToken1Mint, state: statePda })
                .signers([newBoss.payer])
                .rpc(),
        ).rejects.toThrow();
    });

    it("Make offer fails on non boss account with boss signature", async () => {
        const newBoss = new anchor.Wallet(Keypair.generate());
        await expect(
            program.methods
                .makeOfferOne(
                    offerId,
                    new anchor.BN(500e9),
                    new anchor.BN(200e9),
                    new anchor.BN(400e9),
                    new anchor.BN(Date.now()),
                    new anchor.BN(Date.now() + 1),
                    new anchor.BN(3600),
                )
                .accountsPartial({
                    bossBuyToken1Account: bossBuyTokenAccount1,
                    sellTokenMint,
                    buyToken1Mint,
                    state: statePda,
                    boss: newBoss.publicKey,
                })
                .signers([initialBoss.payer])
                .rpc(),
        ).rejects.toThrow();
    });

    it("Make offer fails on non boss account with non boss signature", async () => {
        const newBoss = new anchor.Wallet(Keypair.generate());
        await expect(
            program.methods
                .makeOfferOne(
                    offerId,
                    new anchor.BN(500e9),
                    new anchor.BN(200e9),
                    new anchor.BN(400e9),
                    new anchor.BN(Date.now()),
                    new anchor.BN(Date.now() + 1),
                    new anchor.BN(3600),
                )
                .accountsPartial({
                    bossBuyToken1Account: bossBuyTokenAccount1,
                    sellTokenMint,
                    buyToken1Mint,
                    state: statePda,
                    boss: newBoss.publicKey,
                })
                .signers([newBoss.payer])
                .rpc(),
        ).rejects.toThrow();
    });

    it("Replace an offer", async () => {
        const newOfferId = new anchor.BN(123123124);
        const [newOfferAuthorityPda] = PublicKey.findProgramAddressSync([Buffer.from("offer_authority"), newOfferId.toArrayLike(Buffer, "le", 8)], program.programId);
        const newOfferSellTokenPda = await getAssociatedTokenAddress(sellTokenMint, newOfferAuthorityPda, true);
        const newOfferBuyTokenPda = await getAssociatedTokenAddress(buyToken1Mint, newOfferAuthorityPda, true);

        const closeInstruction = await program.methods
            .closeOfferOne()
            .accounts({
                offer: offerPda,
                state: statePda,
            })
            .instruction();
        const offerSellTokenAccountInstruction = createAssociatedTokenAccountInstruction(
            initialBoss.payer.publicKey,
            getAssociatedTokenAddressSync(sellTokenMint, newOfferAuthorityPda, true),
            newOfferAuthorityPda,
            sellTokenMint,
        );
        const buyToken1AccountInstruction = createAssociatedTokenAccountInstruction(
            initialBoss.payer.publicKey,
            getAssociatedTokenAddressSync(buyToken1Mint, newOfferAuthorityPda, true),
            newOfferAuthorityPda,
            buyToken1Mint,
        );
        const makeOfferInstruction = await program.methods
            .makeOfferOne(
                newOfferId,
                new anchor.BN(500e9),
                new anchor.BN(200e9),
                new anchor.BN(400e9),
                new anchor.BN(Date.now()),
                new anchor.BN(Date.now() + 7200),
                new anchor.BN(3600),
            )
            .accounts({ sellTokenMint, buyToken1Mint, state: statePda })
            .instruction();

        await createAndSendTransaction(provider, initialBoss, [closeInstruction, offerSellTokenAccountInstruction, buyToken1AccountInstruction, makeOfferInstruction]);

        const [newOfferPda] = PublicKey.findProgramAddressSync([Buffer.from("offer"), newOfferId.toArrayLike(Buffer, "le", 8)], program.programId);
        const offerAccount = await program.account.offer.fetch(newOfferPda);
        expect(offerAccount.offerId.eq(newOfferId)).toBe(true);
        expect(offerAccount.sellTokenMint.toBase58()).toEqual(sellTokenMint.toBase58());
        expect(offerAccount.buyToken1.mint.toBase58()).toEqual(buyToken1Mint.toBase58());
        expect(offerAccount.sellTokenStartAmount.eq(new anchor.BN(200e9))).toBe(true);
        expect(offerAccount.sellTokenEndAmount.eq(new anchor.BN(400e9))).toBe(true);
        expect(offerAccount.buyToken1.amount.eq(new anchor.BN(500e9))).toBe(true);

        const bossSellTokenAccountInfo = await provider.connection.getTokenAccountBalance(bossSellTokenAccount);
        const offerSellTokenAccountInfo = await provider.connection.getTokenAccountBalance(newOfferSellTokenPda);
        const offerBuyToken1AccountInfo = await provider.connection.getTokenAccountBalance(newOfferBuyTokenPda);
        expect(+bossSellTokenAccountInfo.value.amount).toEqual(10000e9);
        expect(+offerSellTokenAccountInfo.value.amount).toEqual(0);
        expect(+offerBuyToken1AccountInfo.value.amount).toEqual(500e9);
    });

    it("Create and take offer", async () => {
        const offerId = new anchor.BN(1);
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from("offer_authority"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from("offer"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const offerSellTokenPda = await getAssociatedTokenAddress(sellTokenMint, offerAuthority, true);
        const offerBuyToken1Pda = await getAssociatedTokenAddress(buyToken1Mint, offerAuthority, true);
        const offerBuyToken2Pda = await getAssociatedTokenAddress(buyToken2Mint, offerAuthority, true);

        const offerSellTokenAccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerSellTokenPda, offerAuthority, sellTokenMint);
        const offerBuyToken1AccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerBuyToken1Pda, offerAuthority, buyToken1Mint);
        const offerBuyToken2AccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerBuyToken2Pda, offerAuthority, buyToken2Mint);

        const currentTime = Date.now() / 1000;

        await program.methods
            .makeOfferTwo(
                offerId,
                new anchor.BN(100e9), // buy token 1
                new anchor.BN(20e9), // buy token 2
                new anchor.BN(240e9), // sell token start
                new anchor.BN(240e9), // sell token end
                new anchor.BN(currentTime), // offer start
                new anchor.BN(currentTime + 7200), // offer end
                new anchor.BN(3600), // offer interval
            )
            .accounts({ sellTokenMint, buyToken1Mint, buyToken2Mint, state: statePda })
            .preInstructions([offerSellTokenAccountInstruction, offerBuyToken1AccountInstruction, offerBuyToken2AccountInstruction])
            .rpc();

        const user1 = new anchor.Wallet(Keypair.generate());
        await airdropLamports(provider, user1.publicKey, anchor.web3.LAMPORTS_PER_SOL * 20);
        const user1SellTokenAccount = await createATA(provider, user1.payer, sellTokenMint, user1.publicKey);
        await mintToAddress(provider, initialBoss.payer, sellTokenMint, user1SellTokenAccount, initialBoss.publicKey, 1000e9);

        const takeOfferInstruction = await program.methods
            .takeOfferTwo(new anchor.BN(120e9))
            .accountsPartial({ userSellTokenAccount: user1SellTokenAccount, offer: offerPda, user: user1.publicKey })
            .instruction();
        const createUser1BuyToken1AccountInstruction = createAssociatedTokenAccountInstruction(
            user1.payer.publicKey,
            getAssociatedTokenAddressSync(buyToken1Mint, user1.publicKey, true),
            user1.publicKey,
            buyToken1Mint,
        );
        const createUser1BuyToken2AccountInstruction = createAssociatedTokenAccountInstruction(
            user1.payer.publicKey,
            getAssociatedTokenAddressSync(buyToken2Mint, user1.publicKey, true),
            user1.publicKey,
            buyToken2Mint,
        );

        await createAndSendTransaction(provider, user1, [createUser1BuyToken1AccountInstruction, createUser1BuyToken2AccountInstruction, takeOfferInstruction]);

        const offerBuyToken1Info = await provider.connection.getTokenAccountBalance(offerBuyToken1Pda);
        const offerBuyToken2Info = await provider.connection.getTokenAccountBalance(offerBuyToken2Pda);
        const offerSellTokenInfo = await provider.connection.getTokenAccountBalance(offerSellTokenPda);
        expect(+offerBuyToken1Info.value.amount).toEqual(50e9);
        expect(+offerBuyToken2Info.value.amount).toEqual(10e9);
        expect(+offerSellTokenInfo.value.amount).toEqual(120e9);

        const user2 = new anchor.Wallet(Keypair.generate());
        await airdropLamports(provider, user2.publicKey, anchor.web3.LAMPORTS_PER_SOL * 20);
        const user2SellTokenAccount = await createATA(provider, user2.payer, sellTokenMint, user2.publicKey);
        await mintToAddress(provider, initialBoss.payer, sellTokenMint, user2SellTokenAccount, initialBoss.publicKey, 1000e9);

        const createUser2BuyToken1AccountInstruction = createAssociatedTokenAccountInstruction(
            user2.payer.publicKey,
            getAssociatedTokenAddressSync(buyToken1Mint, user2.publicKey, true),
            user2.publicKey,
            buyToken1Mint,
        );
        const createUser2BuyToken2AccountInstruction = createAssociatedTokenAccountInstruction(
            user2.payer.publicKey,
            getAssociatedTokenAddressSync(buyToken2Mint, user2.publicKey, true),
            user2.publicKey,
            buyToken2Mint,
        );
        const takeOfferInstruction2 = await program.methods
            .takeOfferTwo(new anchor.BN(24e9))
            .accountsPartial({ userSellTokenAccount: user2SellTokenAccount, offer: offerPda, user: user2.publicKey })
            .instruction();

        await createAndSendTransaction(provider, user2, [createUser2BuyToken1AccountInstruction, createUser2BuyToken2AccountInstruction, takeOfferInstruction2]);

        const offerBuyToken1Info2 = await provider.connection.getTokenAccountBalance(offerBuyToken1Pda);
        const offerBuyToken2Info2 = await provider.connection.getTokenAccountBalance(offerBuyToken2Pda);
        const offerSellTokenInfo2 = await provider.connection.getTokenAccountBalance(offerSellTokenPda);
        expect(+offerSellTokenInfo2.value.amount).toEqual(144e9);
        expect(+offerBuyToken1Info2.value.amount).toEqual(40e9);
        expect(+offerBuyToken2Info2.value.amount).toEqual(8e9);
    });

    it("Takes an offer with one buy token successfully", async () => {
        const offerId = new anchor.BN(2);
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from("offer_authority"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from("offer"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const offerSellTokenPda = await getAssociatedTokenAddress(sellTokenMint, offerAuthority, true);
        const offerBuyToken1Pda = await getAssociatedTokenAddress(buyToken1Mint, offerAuthority, true);

        const offerSellTokenAccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerSellTokenPda, offerAuthority, sellTokenMint);
        const offerBuyToken1AccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerBuyToken1Pda, offerAuthority, buyToken1Mint);

        const currentTime = Date.now() / 1000;

        await program.methods
            .makeOfferOne(
                offerId,
                new anchor.BN(100e9), // buy token 1 amount
                new anchor.BN(200e9), // sell token start
                new anchor.BN(200e9), // sell token end
                new anchor.BN(currentTime), // offer start
                new anchor.BN(currentTime + 7200), // offer end
                new anchor.BN(3600), // offer interval
            )
            .accounts({ sellTokenMint, buyToken1Mint, state: statePda })
            .preInstructions([offerSellTokenAccountInstruction, offerBuyToken1AccountInstruction])
            .rpc();

        const user = new anchor.Wallet(Keypair.generate());
        await airdropLamports(provider, user.publicKey, anchor.web3.LAMPORTS_PER_SOL * 20);
        const userSellTokenAccount = await createATA(provider, user.payer, sellTokenMint, user.publicKey);
        await mintToAddress(provider, initialBoss.payer, sellTokenMint, userSellTokenAccount, initialBoss.publicKey, 100e9);

        const createUserBuyToken1AccountInstruction = createAssociatedTokenAccountInstruction(
            user.payer.publicKey,
            getAssociatedTokenAddressSync(buyToken1Mint, user.publicKey, true),
            user.publicKey,
            buyToken1Mint,
        );
        const takeOfferInstruction = await program.methods
            .takeOfferOne(new anchor.BN(50e9))
            .accountsPartial({
                offer: offerPda,
                offerSellTokenAccount: offerSellTokenPda,
                offerBuyToken1Account: offerBuyToken1Pda,
                userSellTokenAccount,
                userBuyToken1Account: getAssociatedTokenAddressSync(buyToken1Mint, user.publicKey, true),
                offerTokenAuthority: offerAuthority,
                user: user.publicKey,
            })
            .instruction();

        await createAndSendTransaction(provider, user, [createUserBuyToken1AccountInstruction, takeOfferInstruction]);

        const offerSellTokenInfo = await provider.connection.getTokenAccountBalance(offerSellTokenPda);
        const offerBuyToken1Info = await provider.connection.getTokenAccountBalance(offerBuyToken1Pda);
        const userSellTokenInfo = await provider.connection.getTokenAccountBalance(userSellTokenAccount);
        const userBuyToken1Info = await provider.connection.getTokenAccountBalance(getAssociatedTokenAddressSync(buyToken1Mint, user.publicKey, true));
        expect(+offerSellTokenInfo.value.amount).toEqual(50e9);
        expect(+offerBuyToken1Info.value.amount).toEqual(75e9);
        expect(+userSellTokenInfo.value.amount).toEqual(50e9);
        expect(+userBuyToken1Info.value.amount).toEqual(25e9);
    });

    it("Fails to take offer with one buy token due to exceeding sell limit", async () => {
        const offerId = new anchor.BN(3);
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from("offer_authority"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from("offer"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const offerSellTokenPda = await getAssociatedTokenAddress(sellTokenMint, offerAuthority, true);
        const offerBuyToken1Pda = await getAssociatedTokenAddress(buyToken1Mint, offerAuthority, true);

        const currentTime = Date.now() / 1000;

        await program.methods
            .makeOfferOne(
                offerId,
                new anchor.BN(100e9), // buy token 1 amount
                new anchor.BN(50e9), // sell token start
                new anchor.BN(50e9), // sell token end
                new anchor.BN(currentTime), // offer start
                new anchor.BN(currentTime + 7200), // offer end
                new anchor.BN(3600), // offer interval
            )
            .accounts({ sellTokenMint, buyToken1Mint, state: statePda })
            .preInstructions([
                createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerSellTokenPda, offerAuthority, sellTokenMint),
                createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerBuyToken1Pda, offerAuthority, buyToken1Mint),
            ])
            .rpc();

        const user = new anchor.Wallet(Keypair.generate());
        await airdropLamports(provider, user.publicKey, anchor.web3.LAMPORTS_PER_SOL * 20);
        const userSellTokenAccount = await createATA(provider, user.payer, sellTokenMint, user.publicKey);
        await mintToAddress(provider, initialBoss.payer, sellTokenMint, userSellTokenAccount, initialBoss.publicKey, 100e9);

        const createUserBuyToken1AccountInstruction = createAssociatedTokenAccountInstruction(
            user.payer.publicKey,
            getAssociatedTokenAddressSync(buyToken1Mint, user.publicKey, true),
            user.publicKey,
            buyToken1Mint,
        );
        const takeOfferInstruction = await program.methods
            .takeOfferOne(new anchor.BN(75e9))
            .accountsPartial({
                offer: offerPda,
                offerSellTokenAccount: offerSellTokenPda,
                offerBuyToken1Account: offerBuyToken1Pda,
                userSellTokenAccount,
                userBuyToken1Account: getAssociatedTokenAddressSync(buyToken1Mint, user.publicKey, true),
                offerTokenAuthority: offerAuthority,
                user: user.publicKey,
                tokenProgram: TOKEN_PROGRAM_ID,
                systemProgram: anchor.web3.SystemProgram.programId,
            })
            .instruction();

        const tx = new VersionedTransaction(
            new TransactionMessage({
                payerKey: user.publicKey,
                recentBlockhash: (await provider.connection.getLatestBlockhash()).blockhash,
                instructions: [createUserBuyToken1AccountInstruction, takeOfferInstruction],
            }).compileToLegacyMessage(),
        );
        const versionedTransaction = await user.signTransaction(tx);
        const signedTransactionBytes = versionedTransaction.serialize();

        await expect(provider.connection.sendRawTransaction(signedTransactionBytes)).rejects.toThrow(RegExp(".*InsufficientOfferTokenOneBalance.*"));
    });

    it("Fails to take offer with one buy token due to invalid buy token mint", async () => {
        const offerId = new anchor.BN(5);
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from("offer_authority"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from("offer"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const offerSellTokenPda = await getAssociatedTokenAddress(sellTokenMint, offerAuthority, true);
        const offerBuyToken1Pda = await getAssociatedTokenAddress(buyToken1Mint, offerAuthority, true);

        const currentTime = Date.now() / 1000;

        await program.methods
            .makeOfferOne(
                offerId,
                new anchor.BN(100e9), // buy token 1 amount
                new anchor.BN(200e9), // sell token start
                new anchor.BN(300e9), // sell token end
                new anchor.BN(currentTime), // offer start
                new anchor.BN(currentTime + 7200), // offer end
                new anchor.BN(3600), // offer interval
            )
            .accounts({ sellTokenMint, buyToken1Mint, state: statePda })
            .preInstructions([
                createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerSellTokenPda, offerAuthority, sellTokenMint),
                createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerBuyToken1Pda, offerAuthority, buyToken1Mint),
            ])
            .rpc();

        const user = new anchor.Wallet(Keypair.generate());
        await airdropLamports(provider, user.publicKey, anchor.web3.LAMPORTS_PER_SOL * 20);
        const userSellTokenAccount = await createATA(provider, user.payer, sellTokenMint, user.publicKey);
        const userBuyToken1Account = await createATA(provider, user.payer, buyToken2Mint, user.publicKey); // Wrong mint
        await mintToAddress(provider, initialBoss.payer, sellTokenMint, userSellTokenAccount, initialBoss.publicKey, 100e9);

        const takeOfferInstruction = await program.methods
            .takeOfferOne(new anchor.BN(50e9))
            .accountsPartial({
                offer: offerPda,
                offerSellTokenAccount: offerSellTokenPda,
                offerBuyToken1Account: offerBuyToken1Pda,
                userSellTokenAccount,
                userBuyToken1Account, // Mismatched mint
                offerTokenAuthority: offerAuthority,
                user: user.publicKey,
            })
            .instruction();

        const tx = new VersionedTransaction(
            new TransactionMessage({
                payerKey: user.publicKey,
                recentBlockhash: (await provider.connection.getLatestBlockhash()).blockhash,
                instructions: [takeOfferInstruction],
            }).compileToLegacyMessage(),
        );
        const versionedTransaction = await user.signTransaction(tx);
        const signedTransactionBytes = versionedTransaction.serialize();

        await expect(provider.connection.sendRawTransaction(signedTransactionBytes)).rejects.toThrow(/Error Message: An associated constraint was violated./);
    });

    it("Closes an offer with two buy tokens", async () => {
        // Create an offer with two buy tokens
        const offerId = new anchor.BN(1000);
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from("offer_authority"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from("offer"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const offerSellTokenPda = await getAssociatedTokenAddress(sellTokenMint, offerAuthority, true);
        const offerBuyToken1Pda = await getAssociatedTokenAddress(buyToken1Mint, offerAuthority, true);
        const offerBuyToken2Pda = await getAssociatedTokenAddress(buyToken2Mint, offerAuthority, true);

        const offerSellTokenAccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerSellTokenPda, offerAuthority, sellTokenMint);
        const offerBuyToken1AccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerBuyToken1Pda, offerAuthority, buyToken1Mint);
        const offerBuyToken2AccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerBuyToken2Pda, offerAuthority, buyToken2Mint);

        const currentTime = Date.now() / 1000;

        await program.methods
            .makeOfferTwo(
                offerId,
                new anchor.BN(100e9), // buy token 1 amount
                new anchor.BN(200e9), // buy token 2 amount
                new anchor.BN(200e9), // sell token start
                new anchor.BN(400e9), // sell token end
                new anchor.BN(currentTime), // offer start
                new anchor.BN(currentTime + 7200), // offer end
                new anchor.BN(3600), // offer interval
            )
            .accounts({
                sellTokenMint: sellTokenMint,
                buyToken1Mint: buyToken1Mint,
                buyToken2Mint: buyToken2Mint,
                state: statePda,
            })
            .preInstructions([offerSellTokenAccountInstruction, offerBuyToken1AccountInstruction, offerBuyToken2AccountInstruction])
            .rpc();

        // Verify the offer was created correctly
        const offerAccount = await program.account.offer.fetch(offerPda);
        expect(offerAccount.offerId.eq(offerId)).toBe(true);
        expect(offerAccount.sellTokenMint.toBase58()).toEqual(sellTokenMint.toBase58());
        expect(offerAccount.buyToken1.mint.toBase58()).toEqual(buyToken1Mint.toBase58());
        expect(offerAccount.buyToken2.mint.toBase58()).toEqual(buyToken2Mint.toBase58());
        expect(offerAccount.sellTokenStartAmount.eq(new anchor.BN(200e9))).toBe(true);
        expect(offerAccount.sellTokenEndAmount.eq(new anchor.BN(400e9))).toBe(true);
        expect(offerAccount.buyToken1.amount.eq(new anchor.BN(100e9))).toBe(true);
        expect(offerAccount.buyToken2.amount.eq(new anchor.BN(200e9))).toBe(true);

        // Check token balances before closing
        const bossBuyToken1BalanceBefore = await provider.connection.getTokenAccountBalance(bossBuyTokenAccount1);
        const bossBuyToken2BalanceBefore = await provider.connection.getTokenAccountBalance(bossBuyTokenAccount2);
        const bossSellTokenBalanceBefore = await provider.connection.getTokenAccountBalance(bossSellTokenAccount);
        const offerBuyToken1BalanceBefore = await provider.connection.getTokenAccountBalance(offerBuyToken1Pda);
        const offerBuyToken2BalanceBefore = await provider.connection.getTokenAccountBalance(offerBuyToken2Pda);
        const offerSellTokenBalanceBefore = await provider.connection.getTokenAccountBalance(offerSellTokenPda);

        // Close the offer
        await program.methods
            .closeOfferTwo()
            .accounts({
                offer: offerPda,
                state: statePda,
            })
            .rpc();

        // Verify token balances after closing
        const bossBuyToken1BalanceAfter = await provider.connection.getTokenAccountBalance(bossBuyTokenAccount1);
        const bossBuyToken2BalanceAfter = await provider.connection.getTokenAccountBalance(bossBuyTokenAccount2);
        const bossSellTokenBalanceAfter = await provider.connection.getTokenAccountBalance(bossSellTokenAccount);

        // Boss should have received all tokens back
        expect(+bossBuyToken1BalanceAfter.value.amount - +bossBuyToken1BalanceBefore.value.amount).toEqual(+offerBuyToken1BalanceBefore.value.amount);
        expect(+bossBuyToken2BalanceAfter.value.amount - +bossBuyToken2BalanceBefore.value.amount).toEqual(+offerBuyToken2BalanceBefore.value.amount);
        expect(+bossSellTokenBalanceAfter.value.amount - +bossSellTokenBalanceBefore.value.amount).toEqual(+offerSellTokenBalanceBefore.value.amount);

        // Verify offer account is closed
        await expect(program.account.offer.fetch(offerPda)).rejects.toThrow();
    });

    it("Fails to close offer with one buy token using close_offer_two", async () => {
        // Create an offer with one buy token
        const offerId = new anchor.BN(1001);
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from("offer_authority"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from("offer"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const offerSellTokenPda = await getAssociatedTokenAddress(sellTokenMint, offerAuthority, true);
        const offerBuyToken1Pda = await getAssociatedTokenAddress(buyToken1Mint, offerAuthority, true);

        const offerSellTokenAccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerSellTokenPda, offerAuthority, sellTokenMint);
        const offerBuyToken1AccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerBuyToken1Pda, offerAuthority, buyToken1Mint);

        const currentTime = Date.now() / 1000;

        await program.methods
            .makeOfferOne(
                offerId,
                new anchor.BN(50e9), // buy token 1 amount
                new anchor.BN(200e9), // sell token start
                new anchor.BN(400e9), // sell token end
                new anchor.BN(currentTime), // offer start
                new anchor.BN(currentTime + 7200), // offer end
                new anchor.BN(3600), // offer interval
            )
            .accounts({
                sellTokenMint: sellTokenMint,
                buyToken1Mint: buyToken1Mint,
                state: statePda,
            })
            .preInstructions([offerSellTokenAccountInstruction, offerBuyToken1AccountInstruction])
            .rpc();

        // Try to close the offer using close_offer_two (should fail)
        const offerBuyToken2Pda = await getAssociatedTokenAddress(buyToken2Mint, offerAuthority, true);
        const offerBuyToken2AccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerBuyToken2Pda, offerAuthority, buyToken2Mint);
        // Create the token account for buy token 2
        await provider.sendAndConfirm(new anchor.web3.Transaction().add(offerBuyToken2AccountInstruction), [initialBoss.payer]);

        // Attempt to close with close_offer_two, which should fail
        await expect(
            program.methods
                .closeOfferTwo()
                .accounts({
                    offer: offerPda,
                    state: statePda,
                })
                .rpc(),
        ).rejects.toThrow();

        // Clean up - close the offer properly
        await program.methods
            .closeOfferOne()
            .accounts({
                offer: offerPda,
                state: statePda,
            })
            .rpc();
    });

    it("Fails to make offer with zero buy token amount in make_offer_two", async () => {
        const offerId = new anchor.BN(1002);
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from("offer_authority"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const offerSellTokenPda = await getAssociatedTokenAddress(sellTokenMint, offerAuthority, true);
        const offerBuyToken1Pda = await getAssociatedTokenAddress(buyToken1Mint, offerAuthority, true);
        const offerBuyToken2Pda = await getAssociatedTokenAddress(buyToken2Mint, offerAuthority, true);

        const offerSellTokenAccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerSellTokenPda, offerAuthority, sellTokenMint);
        const offerBuyToken1AccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerBuyToken1Pda, offerAuthority, buyToken1Mint);
        const offerBuyToken2AccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerBuyToken2Pda, offerAuthority, buyToken2Mint);

        const currentTime = Date.now() / 1000;

        // Try to make an offer with zero buy token 1 amount
        await expect(
            program.methods
                .makeOfferTwo(
                    offerId,
                    new anchor.BN(0), // buy token 1 amount
                    new anchor.BN(100e9), // buy token 2 amount
                    new anchor.BN(200e9), // sell token start
                    new anchor.BN(400e9), // sell token end
                    new anchor.BN(currentTime), // offer start
                    new anchor.BN(currentTime + 7200), // offer end
                    new anchor.BN(3600), // offer interval
                )
                .accounts({
                    sellTokenMint: sellTokenMint,
                    buyToken1Mint: buyToken1Mint,
                    buyToken2Mint: buyToken2Mint,
                    state: statePda,
                })
                .preInstructions([offerSellTokenAccountInstruction, offerBuyToken1AccountInstruction, offerBuyToken2AccountInstruction])
                .rpc(),
        ).rejects.toThrow(/Token transfer amount must be greater than zero/);

        // Try to make an offer with zero buy token 2 amount
        await expect(
            program.methods
                .makeOfferTwo(
                    offerId,
                    new anchor.BN(100e9), // buy token 1 amount
                    new anchor.BN(0), // buy token 2 amount
                    new anchor.BN(200e9), // sell token start
                    new anchor.BN(400e9), // sell token end
                    new anchor.BN(currentTime), // offer start
                    new anchor.BN(currentTime + 7200), // offer end
                    new anchor.BN(3600), // offer interval
                )
                .accounts({
                    sellTokenMint: sellTokenMint,
                    buyToken1Mint: buyToken1Mint,
                    buyToken2Mint: buyToken2Mint,
                    state: statePda,
                })
                .preInstructions([offerSellTokenAccountInstruction, offerBuyToken1AccountInstruction, offerBuyToken2AccountInstruction])
                .rpc(),
        ).rejects.toThrow(/Token transfer amount must be greater than zero/);

        // Try to make an offer with zero sell token amount
        await expect(
            program.methods
                .makeOfferTwo(
                    offerId,
                    new anchor.BN(100e9), // buy token 1 amount
                    new anchor.BN(100e9), // buy token 2 amount
                    new anchor.BN(0), // sell token start
                    new anchor.BN(0), // sell token end
                    new anchor.BN(currentTime), // offer start
                    new anchor.BN(currentTime + 7200), // offer end
                    new anchor.BN(3600), // offer interval
                )
                .accounts({
                    sellTokenMint: sellTokenMint,
                    buyToken1Mint: buyToken1Mint,
                    buyToken2Mint: buyToken2Mint,
                    state: statePda,
                })
                .preInstructions([offerSellTokenAccountInstruction, offerBuyToken1AccountInstruction, offerBuyToken2AccountInstruction])
                .rpc(),
        ).rejects.toThrow(/Token transfer amount must be greater than zero/);
    });

    it("Fails to make offer with zero amounts in make_offer_one", async () => {
        const offerId = new anchor.BN(1003);
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from("offer_authority"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const offerSellTokenPda = await getAssociatedTokenAddress(sellTokenMint, offerAuthority, true);
        const offerBuyToken1Pda = await getAssociatedTokenAddress(buyToken1Mint, offerAuthority, true);

        const offerSellTokenAccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerSellTokenPda, offerAuthority, sellTokenMint);
        const offerBuyToken1AccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerBuyToken1Pda, offerAuthority, buyToken1Mint);

        const currentTime = Date.now() / 1000;
        // Try to make an offer with zero buy token amount
        await expect(
            program.methods
                .makeOfferOne(
                    offerId,
                    new anchor.BN(0), // buy token 1 amount
                    new anchor.BN(200e9), // sell token start
                    new anchor.BN(400e9), // sell token end
                    new anchor.BN(currentTime), // offer start
                    new anchor.BN(currentTime + 7200), // offer end
                    new anchor.BN(3600), // offer interval
                )
                .accounts({
                    sellTokenMint: sellTokenMint,
                    buyToken1Mint: buyToken1Mint,
                    state: statePda,
                })
                .preInstructions([offerSellTokenAccountInstruction, offerBuyToken1AccountInstruction])
                .rpc(),
        ).rejects.toThrow(/Token transfer amount must be greater than zero/);

        // Try to make an offer with zero sell token amount
        await expect(
            program.methods
                .makeOfferOne(
                    offerId,
                    new anchor.BN(100e9), // buy token 1 amount
                    new anchor.BN(0), // sell token start
                    new anchor.BN(0), // sell token end
                    new anchor.BN(currentTime), // offer start
                    new anchor.BN(currentTime + 7200), // offer end
                    new anchor.BN(3600), // offer interval
                )
                .accounts({
                    sellTokenMint: sellTokenMint,
                    buyToken1Mint: buyToken1Mint,
                    state: statePda,
                })
                .preInstructions([offerSellTokenAccountInstruction, offerBuyToken1AccountInstruction])
                .rpc(),
        ).rejects.toThrow(/Token transfer amount must be greater than zero/);
    });

    it("Fails to take offer with two buy tokens due to invalid token mints", async () => {
        const offerId = new anchor.BN(1005);
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from("offer_authority"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from("offer"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const offerSellTokenPda = await getAssociatedTokenAddress(sellTokenMint, offerAuthority, true);
        const offerBuyToken1Pda = await getAssociatedTokenAddress(buyToken1Mint, offerAuthority, true);
        const offerBuyToken2Pda = await getAssociatedTokenAddress(buyToken2Mint, offerAuthority, true);

        // Create an offer with two buy tokens
        const offerSellTokenAccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerSellTokenPda, offerAuthority, sellTokenMint);
        const offerBuyToken1AccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerBuyToken1Pda, offerAuthority, buyToken1Mint);
        const offerBuyToken2AccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerBuyToken2Pda, offerAuthority, buyToken2Mint);

        const currentTime = Date.now() / 1000;

        await program.methods
            .makeOfferTwo(
                offerId,
                new anchor.BN(50e9), // buy token 1 amount
                new anchor.BN(50e9), // buy token 2 amount
                new anchor.BN(100e9), // sell token start
                new anchor.BN(200e9), // sell token end
                new anchor.BN(currentTime), // offer start
                new anchor.BN(currentTime + 7200), // offer end
                new anchor.BN(3600), // offer interval
            )
            .accounts({
                sellTokenMint: sellTokenMint,
                buyToken1Mint: buyToken1Mint,
                buyToken2Mint: buyToken2Mint,
                state: statePda,
            })
            .preInstructions([offerSellTokenAccountInstruction, offerBuyToken1AccountInstruction, offerBuyToken2AccountInstruction])
            .rpc();

        // Create a user with mismatched token accounts
        const user = new anchor.Wallet(Keypair.generate());
        await airdropLamports(provider, user.publicKey, anchor.web3.LAMPORTS_PER_SOL * 20);

        // Create user sell token account (correct)
        const userSellTokenAccount = await createATA(provider, user.payer, sellTokenMint, user.publicKey);
        await mintToAddress(provider, initialBoss.payer, sellTokenMint, userSellTokenAccount, initialBoss.publicKey, 50e9);

        // Create user buy token accounts with wrong mints (swapped)
        // Using buyToken2Mint for userBuyToken1Account and buyToken1Mint for userBuyToken2Account
        const userBuyToken1Account = await createATA(provider, user.payer, buyToken2Mint, user.publicKey); // Wrong mint
        const userBuyToken2Account = await createATA(provider, user.payer, buyToken1Mint, user.publicKey); // Wrong mint

        // Try to take the offer with mismatched token accounts
        const takeOfferInstruction = await program.methods
            .takeOfferTwo(new anchor.BN(50e9))
            .accountsPartial({
                offer: offerPda,
                offerSellTokenAccount: offerSellTokenPda,
                offerBuyToken1Account: offerBuyToken1Pda,
                offerBuyToken2Account: offerBuyToken2Pda,
                userSellTokenAccount,
                userBuyToken1Account, // Wrong mint
                userBuyToken2Account, // Wrong mint
                offerTokenAuthority: offerAuthority,
                user: user.publicKey,
                tokenProgram: TOKEN_PROGRAM_ID,
                systemProgram: anchor.web3.SystemProgram.programId,
            })
            .instruction();

        const tx = new VersionedTransaction(
            new TransactionMessage({
                payerKey: user.publicKey,
                recentBlockhash: (await provider.connection.getLatestBlockhash()).blockhash,
                instructions: [takeOfferInstruction],
            }).compileToLegacyMessage(),
        );
        const versionedTransaction = await user.signTransaction(tx);
        const signedTransactionBytes = versionedTransaction.serialize();

        await expect(provider.connection.sendRawTransaction(signedTransactionBytes)).rejects.toThrow(/Error Message: An associated constraint was violated/);

        // Clean up - close the offer
        await program.methods
            .closeOfferTwo()
            .accounts({
                offer: offerPda,
                state: statePda,
            })
            .rpc();
    });

    it("Fails to take offer with two buy tokens due to exceeding sell limit", async () => {
        const offerId = new anchor.BN(1004);
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from("offer_authority"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from("offer"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const offerSellTokenPda = await getAssociatedTokenAddress(sellTokenMint, offerAuthority, true);
        const offerBuyToken1Pda = await getAssociatedTokenAddress(buyToken1Mint, offerAuthority, true);
        const offerBuyToken2Pda = await getAssociatedTokenAddress(buyToken2Mint, offerAuthority, true);

        // Create an offer with two buy tokens
        const offerSellTokenAccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerSellTokenPda, offerAuthority, sellTokenMint);
        const offerBuyToken1AccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerBuyToken1Pda, offerAuthority, buyToken1Mint);
        const offerBuyToken2AccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerBuyToken2Pda, offerAuthority, buyToken2Mint);

        const currentTime = Date.now() / 1000;

        await program.methods
            .makeOfferTwo(
                offerId,
                new anchor.BN(50e9), // buy token 1 amount
                new anchor.BN(50e9), // buy token 2 amount
                new anchor.BN(100e9), // sell token start
                new anchor.BN(100e9), // sell token end
                new anchor.BN(currentTime), // offer start
                new anchor.BN(currentTime + 7200), // offer end
                new anchor.BN(3600), // offer interval
            )
            .accounts({
                sellTokenMint: sellTokenMint,
                buyToken1Mint: buyToken1Mint,
                buyToken2Mint: buyToken2Mint,
                state: statePda,
            })
            .preInstructions([offerSellTokenAccountInstruction, offerBuyToken1AccountInstruction, offerBuyToken2AccountInstruction])
            .rpc();

        // Create a user and try to take more than the sell limit
        const user = new anchor.Wallet(Keypair.generate());
        await airdropLamports(provider, user.publicKey, anchor.web3.LAMPORTS_PER_SOL * 20);
        const userSellTokenAccount = await createATA(provider, user.payer, sellTokenMint, user.publicKey);
        await mintToAddress(provider, initialBoss.payer, sellTokenMint, userSellTokenAccount, initialBoss.publicKey, 200e9);

        // Create user buy token accounts
        const createUserBuyToken1AccountInstruction = createAssociatedTokenAccountInstruction(
            user.payer.publicKey,
            getAssociatedTokenAddressSync(buyToken1Mint, user.publicKey, true),
            user.publicKey,
            buyToken1Mint,
        );
        const createUserBuyToken2AccountInstruction = createAssociatedTokenAccountInstruction(
            user.payer.publicKey,
            getAssociatedTokenAddressSync(buyToken2Mint, user.publicKey, true),
            user.publicKey,
            buyToken2Mint,
        );

        // Try to take more than the sell limit
        const takeOfferInstruction = await program.methods
            .takeOfferTwo(new anchor.BN(150e9)) // Exceeds the 100e9 limit
            .accountsPartial({
                offer: offerPda,
                offerSellTokenAccount: offerSellTokenPda,
                offerBuyToken1Account: offerBuyToken1Pda,
                offerBuyToken2Account: offerBuyToken2Pda,
                userSellTokenAccount,
                userBuyToken1Account: getAssociatedTokenAddressSync(buyToken1Mint, user.publicKey, true),
                userBuyToken2Account: getAssociatedTokenAddressSync(buyToken2Mint, user.publicKey, true),
                offerTokenAuthority: offerAuthority,
                user: user.publicKey,
            })
            .instruction();

        const tx = new VersionedTransaction(
            new TransactionMessage({
                payerKey: user.publicKey,
                recentBlockhash: (await provider.connection.getLatestBlockhash()).blockhash,
                instructions: [createUserBuyToken1AccountInstruction, createUserBuyToken2AccountInstruction, takeOfferInstruction],
            }).compileToLegacyMessage(),
        );
        const versionedTransaction = await user.signTransaction(tx);
        const signedTransactionBytes = versionedTransaction.serialize();

        await expect(provider.connection.sendRawTransaction(signedTransactionBytes)).rejects.toThrow(RegExp(".*InsufficientOfferTokenOneBalance.*"));

        // Clean up - close the offer
        await program.methods
            .closeOfferTwo()
            .accounts({
                offer: offerPda,
                state: statePda,
            })
            .rpc();
    });

    it("Fails to initialize the contract twice", async () => {
        // The contract is already initialized in the first test
        // Attempt to initialize it again should fail
        await expect(program.methods.initialize().accounts({ boss: initialBoss.publicKey }).rpc()).rejects.toThrow(/already in use/);
    });

    it("Fails to set boss from unauthorized account", async () => {
        // Create a new wallet that is not the current boss
        const unauthorizedWallet = new anchor.Wallet(Keypair.generate());
        await airdropLamports(provider, unauthorizedWallet.publicKey, anchor.web3.LAMPORTS_PER_SOL * 20);

        // Attempt to set a new boss from an unauthorized account
        const newBoss = Keypair.generate().publicKey;

        const setBossInstruction = await program.methods.setBoss(newBoss).accountsPartial({ state: statePda, boss: unauthorizedWallet.publicKey }).instruction();

        const tx = new VersionedTransaction(
            new TransactionMessage({
                payerKey: unauthorizedWallet.publicKey,
                recentBlockhash: (await provider.connection.getLatestBlockhash()).blockhash,
                instructions: [setBossInstruction],
            }).compileToLegacyMessage(),
        );
        const versionedTransaction = await unauthorizedWallet.signTransaction(tx);
        const signedTransactionBytes = versionedTransaction.serialize();

        await expect(provider.connection.sendRawTransaction(signedTransactionBytes)).rejects.toThrow(/Error Message: A has one constraint was violated/);
    });

    it("Handles very small token amounts correctly", async () => {
        // Create an offer with very small amounts to test edge cases in calculations
        const offerId = new anchor.BN(1006);
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from("offer_authority"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from("offer"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const offerSellTokenPda = await getAssociatedTokenAddress(sellTokenMint, offerAuthority, true);
        const offerBuyToken1Pda = await getAssociatedTokenAddress(buyToken1Mint, offerAuthority, true);

        // Create token accounts for the offer
        const offerSellTokenAccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerSellTokenPda, offerAuthority, sellTokenMint);
        const offerBuyToken1AccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerBuyToken1Pda, offerAuthority, buyToken1Mint);

        // Create an offer with very small amounts
        // 10 units of buy token for 100 units of sell token
        const buyTokenAmount = 10; // Very small amount
        const sellTokenAmount = 100; // Very small amount
        const currentTime = Date.now() / 1000;

        await program.methods
            .makeOfferOne(
                offerId,
                new anchor.BN(buyTokenAmount), // buy token 1 amount
                new anchor.BN(sellTokenAmount), // sell token start
                new anchor.BN(sellTokenAmount), // sell token end
                new anchor.BN(currentTime), // offer start
                new anchor.BN(currentTime + 7200), // offer end
                new anchor.BN(3600), // offer interval
            )
            .accounts({
                sellTokenMint: sellTokenMint,
                buyToken1Mint: buyToken1Mint,
                state: statePda,
            })
            .preInstructions([offerSellTokenAccountInstruction, offerBuyToken1AccountInstruction])
            .rpc();

        // Verify the offer was created correctly
        const offerAccount = await program.account.offer.fetch(offerPda);
        expect(offerAccount.offerId.eq(offerId)).toBe(true);
        expect(offerAccount.buyToken1.amount.toNumber()).toEqual(buyTokenAmount);
        expect(offerAccount.sellTokenStartAmount.toNumber()).toEqual(sellTokenAmount);
        expect(offerAccount.sellTokenEndAmount.toNumber()).toEqual(sellTokenAmount);

        // Create a user to take a small portion of the offer
        const user = new anchor.Wallet(Keypair.generate());
        await airdropLamports(provider, user.publicKey, anchor.web3.LAMPORTS_PER_SOL * 20);
        const userSellTokenAccount = await createATA(provider, user.payer, sellTokenMint, user.publicKey);
        await mintToAddress(provider, initialBoss.payer, sellTokenMint, userSellTokenAccount, initialBoss.publicKey, 50);

        // Create user buy token account
        const userBuyToken1Account = await createATA(provider, user.payer, buyToken1Mint, user.publicKey);

        // Take a very small portion of the offer (10 units of sell token)
        const takeAmount = 10; // Very small amount
        await program.methods
            .takeOfferOne(new anchor.BN(takeAmount))
            .accounts({
                offer: offerPda,
                user: user.publicKey,
            })
            .signers([user.payer])
            .rpc();

        // Verify token balances after taking the offer
        const offerSellTokenInfo = await provider.connection.getTokenAccountBalance(offerSellTokenPda);
        const offerBuyToken1Info = await provider.connection.getTokenAccountBalance(offerBuyToken1Pda);
        const userSellTokenInfo = await provider.connection.getTokenAccountBalance(userSellTokenAccount);
        const userBuyToken1Info = await provider.connection.getTokenAccountBalance(userBuyToken1Account);

        // Expected buy token amount for the user: (takeAmount * buyTokenAmount) / sellTokenAmount = (10 * 10) / 100 = 1
        const expectedBuyTokenAmount = Math.floor((takeAmount * buyTokenAmount) / sellTokenAmount);

        expect(+offerSellTokenInfo.value.amount).toEqual(takeAmount);
        expect(+offerBuyToken1Info.value.amount).toEqual(buyTokenAmount - expectedBuyTokenAmount);
        expect(+userSellTokenInfo.value.amount).toEqual(50 - takeAmount);
        expect(+userBuyToken1Info.value.amount).toEqual(expectedBuyTokenAmount);

        // Clean up - close the offer
        await program.methods
            .closeOfferOne()
            .accounts({
                offer: offerPda,
                state: statePda,
            })
            .rpc();
    });

    it("Takes an offer completely with two users and fails on third attempt", async () => {
        // Create a new offer
        const offerId = new anchor.BN(9999);
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from("offer_authority"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from("offer"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const offerSellTokenPda = await getAssociatedTokenAddress(sellTokenMint, offerAuthority, true);
        const offerBuyToken1Pda = await getAssociatedTokenAddress(buyToken1Mint, offerAuthority, true);

        // Create token accounts for the offer
        const offerSellTokenAccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerSellTokenPda, offerAuthority, sellTokenMint);
        const offerBuyToken1AccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerBuyToken1Pda, offerAuthority, buyToken1Mint);

        const currentTime = Date.now() / 1000;

        // Make an offer with 100e9 buy tokens for 100e9 sell tokens
        await program.methods
            .makeOfferOne(
                offerId,
                new anchor.BN(100e9), // buy token 1 amount
                new anchor.BN(100e9), // sell token start
                new anchor.BN(100e9), // sell token end
                new anchor.BN(currentTime), // offer start
                new anchor.BN(currentTime + 7200), // offer end
                new anchor.BN(3600), // offer interval
            )
            .accounts({ sellTokenMint, buyToken1Mint, state: statePda })
            .preInstructions([offerSellTokenAccountInstruction, offerBuyToken1AccountInstruction])
            .rpc();

        // Verify the offer was created correctly
        const offerAccount = await program.account.offer.fetch(offerPda);
        expect(offerAccount.offerId.eq(offerId)).toBe(true);
        expect(offerAccount.sellTokenMint.toBase58()).toEqual(sellTokenMint.toBase58());
        expect(offerAccount.buyToken1.mint.toBase58()).toEqual(buyToken1Mint.toBase58());
        expect(offerAccount.buyToken1.amount.eq(new anchor.BN(100e9))).toBe(true);
        expect(offerAccount.sellTokenStartAmount.eq(new anchor.BN(100e9))).toBe(true);
        expect(offerAccount.sellTokenEndAmount.eq(new anchor.BN(100e9))).toBe(true);

        // Create first user and have them take 60% of the offer
        const user1 = new anchor.Wallet(Keypair.generate());
        await airdropLamports(provider, user1.publicKey, anchor.web3.LAMPORTS_PER_SOL * 20);
        const user1SellTokenAccount = await createATA(provider, user1.payer, sellTokenMint, user1.publicKey);
        await mintToAddress(provider, initialBoss.payer, sellTokenMint, user1SellTokenAccount, initialBoss.publicKey, 100e9);

        const createUser1BuyToken1AccountInstruction = createAssociatedTokenAccountInstruction(
            user1.payer.publicKey,
            getAssociatedTokenAddressSync(buyToken1Mint, user1.publicKey, true),
            user1.publicKey,
            buyToken1Mint,
        );
        const takeOfferInstruction1 = await program.methods
            .takeOfferOne(new anchor.BN(60e9))
            .accountsPartial({
                offer: offerPda,
                offerSellTokenAccount: offerSellTokenPda,
                offerBuyToken1Account: offerBuyToken1Pda,
                userSellTokenAccount: user1SellTokenAccount,
                userBuyToken1Account: getAssociatedTokenAddressSync(buyToken1Mint, user1.publicKey, true),
                offerTokenAuthority: offerAuthority,
                user: user1.publicKey,
            })
            .instruction();

        await createAndSendTransaction(provider, user1, [createUser1BuyToken1AccountInstruction, takeOfferInstruction1]);

        // Verify token balances after first user takes 60% of the offer
        const offerSellTokenInfo1 = await provider.connection.getTokenAccountBalance(offerSellTokenPda);
        const offerBuyToken1Info1 = await provider.connection.getTokenAccountBalance(offerBuyToken1Pda);
        expect(+offerSellTokenInfo1.value.amount).toEqual(60e9);
        expect(+offerBuyToken1Info1.value.amount).toEqual(40e9);

        // Create second user and have them take the remaining 40% of the offer
        const user2 = new anchor.Wallet(Keypair.generate());
        await airdropLamports(provider, user2.publicKey, anchor.web3.LAMPORTS_PER_SOL * 20);
        const user2SellTokenAccount = await createATA(provider, user2.payer, sellTokenMint, user2.publicKey);
        await mintToAddress(provider, initialBoss.payer, sellTokenMint, user2SellTokenAccount, initialBoss.publicKey, 100e9);

        const createUser2BuyToken1AccountInstruction = createAssociatedTokenAccountInstruction(
            user2.payer.publicKey,
            getAssociatedTokenAddressSync(buyToken1Mint, user2.publicKey, true),
            user2.publicKey,
            buyToken1Mint,
        );
        const takeOfferInstruction2 = await program.methods
            .takeOfferOne(new anchor.BN(40e9))
            .accountsPartial({
                offer: offerPda,
                offerSellTokenAccount: offerSellTokenPda,
                offerBuyToken1Account: offerBuyToken1Pda,
                userSellTokenAccount: user2SellTokenAccount,
                userBuyToken1Account: getAssociatedTokenAddressSync(buyToken1Mint, user2.publicKey, true),
                offerTokenAuthority: offerAuthority,
                user: user2.publicKey,
            })
            .instruction();

        await createAndSendTransaction(provider, user2, [createUser2BuyToken1AccountInstruction, takeOfferInstruction2]);

        // Verify token balances after second user takes the remaining 40% of the offer
        const offerSellTokenInfo2 = await provider.connection.getTokenAccountBalance(offerSellTokenPda);
        const offerBuyToken1Info2 = await provider.connection.getTokenAccountBalance(offerBuyToken1Pda);
        expect(+offerSellTokenInfo2.value.amount).toEqual(100e9); // Offer is now fully consumed
        expect(+offerBuyToken1Info2.value.amount).toEqual(0); // All buy tokens have been distributed

        // Create third user and have them try to take from the fully consumed offer
        const user3 = new anchor.Wallet(Keypair.generate());
        await airdropLamports(provider, user3.publicKey, anchor.web3.LAMPORTS_PER_SOL * 20);
        const user3SellTokenAccount = await createATA(provider, user3.payer, sellTokenMint, user3.publicKey);
        await mintToAddress(provider, initialBoss.payer, sellTokenMint, user3SellTokenAccount, initialBoss.publicKey, 100e9);

        const createUser3BuyToken1AccountInstruction = createAssociatedTokenAccountInstruction(
            user3.payer.publicKey,
            getAssociatedTokenAddressSync(buyToken1Mint, user3.publicKey, true),
            user3.publicKey,
            buyToken1Mint,
        );
        const takeOfferInstruction3 = await program.methods
            .takeOfferOne(new anchor.BN(10e9))
            .accountsPartial({
                offer: offerPda,
                offerSellTokenAccount: offerSellTokenPda,
                offerBuyToken1Account: offerBuyToken1Pda,
                userSellTokenAccount: user3SellTokenAccount,
                userBuyToken1Account: getAssociatedTokenAddressSync(buyToken1Mint, user3.publicKey, true),
                offerTokenAuthority: offerAuthority,
                user: user3.publicKey,
            })
            .instruction();

        // This transaction should fail because the offer is fully consumed
        const tx = new VersionedTransaction(
            new TransactionMessage({
                payerKey: user3.publicKey,
                recentBlockhash: (await provider.connection.getLatestBlockhash()).blockhash,
                instructions: [createUser3BuyToken1AccountInstruction, takeOfferInstruction3],
            }).compileToLegacyMessage(),
        );
        const versionedTransaction = await user3.signTransaction(tx);
        const signedTransactionBytes = versionedTransaction.serialize();

        // Expect the transaction to fail with an error about insufficient funds
        await expect(provider.connection.sendRawTransaction(signedTransactionBytes)).rejects.toThrow();

        // Clean up - close the offer
        await program.methods
            .closeOfferOne()
            .accounts({
                offer: offerPda,
                state: statePda,
            })
            .rpc();
    }, 10000);

    it("Takes an offer with one buy token successfully via permissionless route", async () => {
        const offerId = new anchor.BN(5000);
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from("offer_authority"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from("offer"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const offerSellTokenPda = await getAssociatedTokenAddress(sellTokenMint, offerAuthority, true);
        const offerBuyToken1Pda = await getAssociatedTokenAddress(buyToken1Mint, offerAuthority, true);

        // Create token accounts for the offer
        const offerSellTokenAccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerSellTokenPda, offerAuthority, sellTokenMint);
        const offerBuyToken1AccountInstruction = createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerBuyToken1Pda, offerAuthority, buyToken1Mint);

        const currentTime = Date.now() / 1000;

        // Create an offer with fixed pricing (start = end)
        await program.methods
            .makeOfferOne(
                offerId,
                new anchor.BN(100e9), // buy token 1 amount
                new anchor.BN(200e9), // sell token start
                new anchor.BN(200e9), // sell token end
                new anchor.BN(currentTime), // offer start
                new anchor.BN(currentTime + 7200), // offer end
                new anchor.BN(3600), // offer interval
            )
            .accounts({ sellTokenMint, buyToken1Mint, state: statePda })
            .preInstructions([offerSellTokenAccountInstruction, offerBuyToken1AccountInstruction])
            .rpc();

        // Create a user to take the offer
        const user = new anchor.Wallet(Keypair.generate());
        await airdropLamports(provider, user.publicKey, anchor.web3.LAMPORTS_PER_SOL * 20);
        const userSellTokenAccount = await createATA(provider, user.payer, sellTokenMint, user.publicKey);
        const userBuyToken1Account = await createATA(provider, user.payer, buyToken1Mint, user.publicKey);
        await mintToAddress(provider, initialBoss.payer, sellTokenMint, userSellTokenAccount, initialBoss.publicKey, 100e9);

        // Derive intermediary authority PDA
        const [intermediaryAuthority] = PublicKey.findProgramAddressSync([Buffer.from("permissionless-1")], program.programId);
        const intermediaryBuyTokenAccount = await getAssociatedTokenAddress(buyToken1Mint, intermediaryAuthority, true);
        const intermediarySellTokenAccount = await getAssociatedTokenAddress(sellTokenMint, intermediaryAuthority, true);

        // Initialize permissionless account
        await program.methods
            .initializePermissionlessAccount("permissionless-1")
            .accounts({
                state: statePda,
            })
            .signers([initialBoss.payer])
            .rpc();

        // Pre-create intermediary token accounts (required after removing init_if_needed)
        await createIntermediaryAccountsIfNeeded(provider, user, buyToken1Mint, sellTokenMint, program.programId);

        // Record balances before transaction
        const userSellTokenBalanceBefore = await provider.connection.getTokenAccountBalance(userSellTokenAccount);
        const userBuyToken1BalanceBefore = await provider.connection.getTokenAccountBalance(userBuyToken1Account);
        const offerSellTokenBalanceBefore = await provider.connection.getTokenAccountBalance(offerSellTokenPda);
        const offerBuyToken1BalanceBefore = await provider.connection.getTokenAccountBalance(offerBuyToken1Pda);
        const userSolBalanceBefore = await provider.connection.getBalance(user.publicKey);

        // Take the offer using permissionless route
        await program.methods
            .takeOfferOnePermissionless(new anchor.BN(50e9))
            .accounts({
                offer: offerPda,
                buyToken1Mint,
                sellTokenMint,
                user: user.publicKey,
            })
            .signers([user.payer])
            .rpc();

        // Record balances after transaction
        const userSellTokenBalanceAfter = await provider.connection.getTokenAccountBalance(userSellTokenAccount);
        const userBuyToken1BalanceAfter = await provider.connection.getTokenAccountBalance(userBuyToken1Account);
        const offerSellTokenBalanceAfter = await provider.connection.getTokenAccountBalance(offerSellTokenPda);
        const offerBuyToken1BalanceAfter = await provider.connection.getTokenAccountBalance(offerBuyToken1Pda);
        const userSolBalanceAfter = await provider.connection.getBalance(user.publicKey);

        // Verify token transfers occurred correctly
        // Expected: User gave 50e9 sell tokens and should receive 25e9 buy tokens (50% of offer)
        expect(+userSellTokenBalanceAfter.value.amount).toEqual(+userSellTokenBalanceBefore.value.amount - 50e9);
        expect(+userBuyToken1BalanceAfter.value.amount).toEqual(+userBuyToken1BalanceBefore.value.amount + 25e9);
        expect(+offerSellTokenBalanceAfter.value.amount).toEqual(+offerSellTokenBalanceBefore.value.amount + 50e9);
        expect(+offerBuyToken1BalanceAfter.value.amount).toEqual(+offerBuyToken1BalanceBefore.value.amount - 25e9);

        // Verify intermediary account persists (no longer closed)
        // User should have paid for tx fees (and potentially account creation if first time)
        // Note: If accounts already exist from previous tests, only tx fees are paid
        expect(userSolBalanceAfter).toBeLessThanOrEqual(userSolBalanceBefore); // Account for tx fees and potential account creation

        // Verify intermediary account still exists and has zero balance
        const intermediaryAccountInfo = await provider.connection.getTokenAccountBalance(intermediaryBuyTokenAccount);
        expect(+intermediaryAccountInfo.value.amount).toEqual(0);

        // Clean up - close the offer
        await program.methods.closeOfferOne().accounts({ offer: offerPda, state: statePda }).rpc();
    });

    it("Verifies intermediary account is properly created and persists", async () => {
        const offerId = new anchor.BN(5001);
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from("offer_authority"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from("offer"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const offerSellTokenPda = await getAssociatedTokenAddress(sellTokenMint, offerAuthority, true);
        const offerBuyToken1Pda = await getAssociatedTokenAddress(buyToken1Mint, offerAuthority, true);

        const currentTime = Date.now() / 1000;

        // Create offer
        await program.methods
            .makeOfferOne(
                offerId,
                new anchor.BN(50e9), // buy token 1 amount
                new anchor.BN(100e9), // sell token start
                new anchor.BN(100e9), // sell token end
                new anchor.BN(currentTime), // offer start
                new anchor.BN(currentTime + 7200), // offer end
                new anchor.BN(3600), // offer interval
            )
            .accounts({ sellTokenMint, buyToken1Mint, state: statePda })
            .preInstructions([
                createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerSellTokenPda, offerAuthority, sellTokenMint),
                createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerBuyToken1Pda, offerAuthority, buyToken1Mint),
            ])
            .rpc();

        const user = new anchor.Wallet(Keypair.generate());
        await airdropLamports(provider, user.publicKey, anchor.web3.LAMPORTS_PER_SOL * 20);
        const userSellTokenAccount = await createATA(provider, user.payer, sellTokenMint, user.publicKey);
        const userBuyToken1Account = await createATA(provider, user.payer, buyToken1Mint, user.publicKey);
        await mintToAddress(provider, initialBoss.payer, sellTokenMint, userSellTokenAccount, initialBoss.publicKey, 50e9);

        // Derive intermediary account for verification
        const [intermediaryAuthority] = PublicKey.findProgramAddressSync([Buffer.from("permissionless-1")], program.programId);
        const intermediaryBuyTokenAccount = await getAssociatedTokenAddress(buyToken1Mint, intermediaryAuthority, true);
        const intermediarySellTokenAccount = await getAssociatedTokenAddress(sellTokenMint, intermediaryAuthority, true);

        // Pre-create intermediary token accounts (required after removing init_if_needed)
        await createIntermediaryAccountsIfNeeded(provider, user, buyToken1Mint, sellTokenMint, program.programId);

        // Create a partial transaction to observe intermediary account creation
        const takeOfferIx = await program.methods
            .takeOfferOnePermissionless(new anchor.BN(50e9))
            .accounts({
                offer: offerPda,
                buyToken1Mint,
                sellTokenMint,
                user: user.publicKey,
            })
            .instruction();

        // Execute the transaction
        await createAndSendTransaction(provider, user, [takeOfferIx]);

        // Verify intermediary account persists after transaction
        const intermediaryTokenAccountInfo = await provider.connection.getTokenAccountBalance(intermediaryBuyTokenAccount);
        expect(+intermediaryTokenAccountInfo.value.amount).toEqual(0);

        // Clean up
        await program.methods.closeOfferOne().accounts({ offer: offerPda, state: statePda }).rpc();
    });

    it("Fails to take offer via permissionless route with invalid buy token mint", async () => {
        const offerId = new anchor.BN(5002);
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from("offer_authority"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from("offer"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
        const offerSellTokenPda = await getAssociatedTokenAddress(sellTokenMint, offerAuthority, true);
        const offerBuyToken1Pda = await getAssociatedTokenAddress(buyToken1Mint, offerAuthority, true);

        const currentTime = Date.now() / 1000;

        // Create offer
        await program.methods
            .makeOfferOne(
                offerId,
                new anchor.BN(100e9), // buy token 1 amount
                new anchor.BN(200e9), // sell token start
                new anchor.BN(200e9), // sell token end
                new anchor.BN(currentTime), // offer start
                new anchor.BN(currentTime + 7200), // offer end
                new anchor.BN(3600), // offer interval
            )
            .accounts({ sellTokenMint, buyToken1Mint, state: statePda })
            .preInstructions([
                createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerSellTokenPda, offerAuthority, sellTokenMint),
                createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerBuyToken1Pda, offerAuthority, buyToken1Mint),
            ])
            .rpc();

        const user = new anchor.Wallet(Keypair.generate());
        await airdropLamports(provider, user.publicKey, anchor.web3.LAMPORTS_PER_SOL * 20);
        const userSellTokenAccount = await createATA(provider, user.payer, sellTokenMint, user.publicKey);
        const userBuyToken2Account = await createATA(provider, user.payer, buyToken2Mint, user.publicKey); // Create account for wrong mint
        await mintToAddress(provider, initialBoss.payer, sellTokenMint, userSellTokenAccount, initialBoss.publicKey, 100e9);

        // Derive intermediary authority
        const [intermediaryAuthority] = PublicKey.findProgramAddressSync([Buffer.from("permissionless-1")], program.programId);

        // Pre-create intermediary token accounts with CORRECT mints (not the wrong one)
        await createIntermediaryAccountsIfNeeded(provider, user, buyToken1Mint, sellTokenMint, program.programId);

        // Try to use wrong mint (buyToken2Mint instead of buyToken1Mint)
        // This will fail with AccountNotInitialized because the intermediary account for buyToken2Mint doesn't exist
        // (we created it with buyToken1Mint above)
        await expect(
            program.methods
                .takeOfferOnePermissionless(new anchor.BN(50e9))
                .accounts({
                    offer: offerPda,
                    buyToken1Mint: buyToken2Mint, // Wrong mint!
                    sellTokenMint,
                    user: user.publicKey,
                })
                .signers([user.payer])
                .rpc(),
        ).rejects.toThrow(/AccountNotInitialized|InvalidBuyTokenMint/);

        // Clean up
        await program.methods.closeOfferOne().accounts({ offer: offerPda, state: statePda }).rpc();
    });

    it("Compares permissionless route with regular route - same economic outcome", async () => {
        // Create two identical offers for comparison
        const regularOfferId = new anchor.BN(5004);
        const permissionlessOfferId = new anchor.BN(5005);

        // Setup for regular offer
        const [regularOfferAuthority] = PublicKey.findProgramAddressSync([Buffer.from("offer_authority"), regularOfferId.toArrayLike(Buffer, "le", 8)], program.programId);
        const [regularOfferPda] = PublicKey.findProgramAddressSync([Buffer.from("offer"), regularOfferId.toArrayLike(Buffer, "le", 8)], program.programId);
        const regularOfferSellTokenPda = await getAssociatedTokenAddress(sellTokenMint, regularOfferAuthority, true);
        const regularOfferBuyToken1Pda = await getAssociatedTokenAddress(buyToken1Mint, regularOfferAuthority, true);

        // Setup for permissionless offer
        const [permissionlessOfferAuthority] = PublicKey.findProgramAddressSync(
            [Buffer.from("offer_authority"), permissionlessOfferId.toArrayLike(Buffer, "le", 8)],
            program.programId,
        );
        const [permissionlessOfferPda] = PublicKey.findProgramAddressSync([Buffer.from("offer"), permissionlessOfferId.toArrayLike(Buffer, "le", 8)], program.programId);
        const permissionlessOfferSellTokenPda = await getAssociatedTokenAddress(sellTokenMint, permissionlessOfferAuthority, true);
        const permissionlessOfferBuyToken1Pda = await getAssociatedTokenAddress(buyToken1Mint, permissionlessOfferAuthority, true);

        const currentTime = Date.now() / 1000;

        // Create both offers with identical parameters
        const offerParams = {
            buyTokenAmount: new anchor.BN(100e9),
            sellTokenStart: new anchor.BN(200e9),
            sellTokenEnd: new anchor.BN(200e9),
            offerStart: new anchor.BN(currentTime),
            offerEnd: new anchor.BN(currentTime + 7200),
            priceInterval: new anchor.BN(3600),
        };

        await program.methods
            .makeOfferOne(
                regularOfferId,
                offerParams.buyTokenAmount,
                offerParams.sellTokenStart,
                offerParams.sellTokenEnd,
                offerParams.offerStart,
                offerParams.offerEnd,
                offerParams.priceInterval,
            )
            .accounts({ sellTokenMint, buyToken1Mint, state: statePda })
            .preInstructions([
                createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, regularOfferSellTokenPda, regularOfferAuthority, sellTokenMint),
                createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, regularOfferBuyToken1Pda, regularOfferAuthority, buyToken1Mint),
            ])
            .rpc();

        await program.methods
            .makeOfferOne(
                permissionlessOfferId,
                offerParams.buyTokenAmount,
                offerParams.sellTokenStart,
                offerParams.sellTokenEnd,
                offerParams.offerStart,
                offerParams.offerEnd,
                offerParams.priceInterval,
            )
            .accounts({ sellTokenMint, buyToken1Mint, state: statePda })
            .preInstructions([
                createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, permissionlessOfferSellTokenPda, permissionlessOfferAuthority, sellTokenMint),
                createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, permissionlessOfferBuyToken1Pda, permissionlessOfferAuthority, buyToken1Mint),
            ])
            .rpc();

        // Create two users for testing
        const regularUser = new anchor.Wallet(Keypair.generate());
        const permissionlessUser = new anchor.Wallet(Keypair.generate());

        await airdropLamports(provider, regularUser.publicKey, anchor.web3.LAMPORTS_PER_SOL * 20);
        await airdropLamports(provider, permissionlessUser.publicKey, anchor.web3.LAMPORTS_PER_SOL * 20);

        // Setup user accounts
        const regularUserSellTokenAccount = await createATA(provider, regularUser.payer, sellTokenMint, regularUser.publicKey);
        const regularUserBuyToken1Account = await createATA(provider, regularUser.payer, buyToken1Mint, regularUser.publicKey);
        const permissionlessUserSellTokenAccount = await createATA(provider, permissionlessUser.payer, sellTokenMint, permissionlessUser.publicKey);
        const permissionlessUserBuyToken1Account = await createATA(provider, permissionlessUser.payer, buyToken1Mint, permissionlessUser.publicKey);

        // Mint identical amounts to both users
        await mintToAddress(provider, initialBoss.payer, sellTokenMint, regularUserSellTokenAccount, initialBoss.publicKey, 100e9);
        await mintToAddress(provider, initialBoss.payer, sellTokenMint, permissionlessUserSellTokenAccount, initialBoss.publicKey, 100e9);

        const sellAmount = new anchor.BN(60e9);

        // Take regular offer
        await program.methods
            .takeOfferOne(sellAmount)
            .accounts({
                offer: regularOfferPda,
                user: regularUser.publicKey,
            })
            .signers([regularUser.payer])
            .rpc();

        // Pre-create intermediary token accounts for permissionless route
        await createIntermediaryAccountsIfNeeded(provider, permissionlessUser, buyToken1Mint, sellTokenMint, program.programId);

        // Take permissionless offer
        await program.methods
            .takeOfferOnePermissionless(sellAmount)
            .accounts({
                offer: permissionlessOfferPda,
                buyToken1Mint,
                sellTokenMint,
                user: permissionlessUser.publicKey,
            })
            .signers([permissionlessUser.payer])
            .rpc();

        // Compare final balances - should be identical
        const regularUserSellBalance = await provider.connection.getTokenAccountBalance(regularUserSellTokenAccount);
        const regularUserBuyBalance = await provider.connection.getTokenAccountBalance(regularUserBuyToken1Account);
        const permissionlessUserSellBalance = await provider.connection.getTokenAccountBalance(permissionlessUserSellTokenAccount);
        const permissionlessUserBuyBalance = await provider.connection.getTokenAccountBalance(permissionlessUserBuyToken1Account);

        expect(regularUserSellBalance.value.amount).toEqual(permissionlessUserSellBalance.value.amount);
        expect(regularUserBuyBalance.value.amount).toEqual(permissionlessUserBuyBalance.value.amount);

        // Both users should have received 30e9 buy tokens for 60e9 sell tokens
        expect(+regularUserBuyBalance.value.amount).toEqual(30e9);
        expect(+permissionlessUserBuyBalance.value.amount).toEqual(30e9);
        expect(+regularUserSellBalance.value.amount).toEqual(40e9); // 100e9 - 60e9
        expect(+permissionlessUserSellBalance.value.amount).toEqual(40e9);

        // Clean up
        await program.methods.closeOfferOne().accounts({ offer: regularOfferPda, state: statePda }).rpc();
        await program.methods.closeOfferOne().accounts({ offer: permissionlessOfferPda, state: statePda }).rpc();
    });

    it("Handles multiple permissionless transactions from same user with different offer IDs", async () => {
        const offerId1 = new anchor.BN(5006);
        const offerId2 = new anchor.BN(5007);

        // Create both offers
        for (const offerId of [offerId1, offerId2]) {
            const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from("offer_authority"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
            const offerSellTokenPda = await getAssociatedTokenAddress(sellTokenMint, offerAuthority, true);
            const offerBuyToken1Pda = await getAssociatedTokenAddress(buyToken1Mint, offerAuthority, true);

            const currentTime = Date.now() / 1000;

            await program.methods
                .makeOfferOne(
                    offerId,
                    new anchor.BN(50e9), // buy token amount
                    new anchor.BN(100e9), // sell token start
                    new anchor.BN(100e9), // sell token end
                    new anchor.BN(currentTime), // offer start
                    new anchor.BN(currentTime + 7200), // offer end
                    new anchor.BN(3600), // offer interval
                )
                .accounts({ sellTokenMint, buyToken1Mint, state: statePda })
                .preInstructions([
                    createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerSellTokenPda, offerAuthority, sellTokenMint),
                    createAssociatedTokenAccountInstruction(initialBoss.payer.publicKey, offerBuyToken1Pda, offerAuthority, buyToken1Mint),
                ])
                .rpc();
        }

        const user = new anchor.Wallet(Keypair.generate());
        await airdropLamports(provider, user.publicKey, anchor.web3.LAMPORTS_PER_SOL * 20);
        const userSellTokenAccount = await createATA(provider, user.payer, sellTokenMint, user.publicKey);
        const userBuyToken1Account = await createATA(provider, user.payer, buyToken1Mint, user.publicKey);
        await mintToAddress(provider, initialBoss.payer, sellTokenMint, userSellTokenAccount, initialBoss.publicKey, 200e9);

        // Pre-create intermediary token accounts (only need to do this once)
        await createIntermediaryAccountsIfNeeded(provider, user, buyToken1Mint, sellTokenMint, program.programId);

        // Take both offers using permissionless route
        for (const offerId of [offerId1, offerId2]) {
            const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from("offer"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);

            await program.methods
                .takeOfferOnePermissionless(new anchor.BN(50e9))
                .accounts({
                    offer: offerPda,
                    buyToken1Mint,
                    sellTokenMint,
                    user: user.publicKey,
                })
                .signers([user.payer])
                .rpc();
        }

        // Verify user received tokens from both offers
        const userBuyBalance = await provider.connection.getTokenAccountBalance(userBuyToken1Account);
        const userSellBalance = await provider.connection.getTokenAccountBalance(userSellTokenAccount);

        expect(+userBuyBalance.value.amount).toEqual(50e9); // 25e9 from each offer
        expect(+userSellBalance.value.amount).toEqual(100e9); // 200e9 - 100e9 spent

        // Clean up both offers
        for (const offerId of [offerId1, offerId2]) {
            const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from("offer"), offerId.toArrayLike(Buffer, "le", 8)], program.programId);
            await program.methods.closeOfferOne().accounts({ offer: offerPda, state: statePda }).rpc();
        }
    });

    it("Initializes permissionless account and verifies name", async () => {
        const accountName = "permissionless-1";
        const [permissionlessAccountPda] = PublicKey.findProgramAddressSync([Buffer.from(accountName)], program.programId);

        // Fetch and verify the account
        const permissionlessAccount = await program.account.permissionlessAccount.fetch(permissionlessAccountPda);
        expect(permissionlessAccount.name).toEqual(accountName);
    });

    it("Fails to initialize permissionless account from non-boss account", async () => {
        const unauthorizedUser = new anchor.Wallet(Keypair.generate());
        await airdropLamports(provider, unauthorizedUser.publicKey, anchor.web3.LAMPORTS_PER_SOL * 5);

        const accountName = "unauthorized-test";

        // Try to initialize from non-boss account - should fail
        await expect(
            program.methods
                .initializePermissionlessAccount(accountName)
                .accountsPartial({
                    state: statePda,
                    boss: unauthorizedUser.publicKey, // Wrong boss
                })
                .signers([unauthorizedUser.payer])
                .rpc(),
        ).rejects.toThrow();
    });

    it("Fails to initialize permissionless account with boss signature but wrong boss account", async () => {
        const wrongBoss = Keypair.generate().publicKey;
        const accountName = "wrong-boss-test";

        // Try to initialize with boss signature but wrong boss account - should fail
        await expect(
            program.methods
                .initializePermissionlessAccount(accountName)
                .accountsPartial({
                    state: statePda,
                    boss: wrongBoss, // Wrong boss account but signed by correct boss
                })
                .signers([initialBoss.payer])
                .rpc(),
        ).rejects.toThrow();
    });
});
