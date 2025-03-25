import * as anchor from '@coral-xyz/anchor';
import {Program} from '@coral-xyz/anchor';
import {Keypair, PublicKey, TransactionMessage, VersionedTransaction} from '@solana/web3.js';
import {OnreApp} from '../target/types/onre_app';
import bs58 from 'bs58';
import {
    createAssociatedTokenAccount,
    createAssociatedTokenAccountInstruction,
    createMint,
    getAssociatedTokenAddress,
    getAssociatedTokenAddressSync,
    mintTo,
    TOKEN_PROGRAM_ID,
} from '@solana/spl-token';


async function airdropLamports(provider, publicKey, amount) {
    const signature = await provider.connection.requestAirdrop(publicKey, amount);
    await provider.connection.confirmTransaction({signature, ...(await provider.connection.getLatestBlockhash())});
    return signature;
}

async function mintToAddress(provider, payer, mint, destination, authority, amount) {
    await mintTo(provider.connection, payer, mint, destination, authority, amount);
}

async function createATA(provider, payer, mint, owner) {
    return await createAssociatedTokenAccount(provider.connection, payer, mint, owner);
}

async function createAndSendTransaction(provider, payer, instructions) {
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

describe('onreapp', () => {
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
    let offerSellTokenPda: PublicKey;
    let offerBuyToken1Pda: PublicKey;
    let offerBuyToken2Pda: PublicKey;
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

        await mintToAddress(provider, initialBoss.payer, sellTokenMint, bossSellTokenAccount, initialBoss.publicKey, 1000e9);
        await mintToAddress(provider, initialBoss.payer, buyToken1Mint, bossBuyTokenAccount1, initialBoss.publicKey, 1000e9);
        await mintToAddress(provider, initialBoss.payer, buyToken2Mint, bossBuyTokenAccount2, initialBoss.publicKey, 1000e9);

        [offerPda] = PublicKey.findProgramAddressSync([Buffer.from('offer'), offerId.toArrayLike(Buffer, 'le', 8)], program.programId);
        [statePda] = PublicKey.findProgramAddressSync([Buffer.from('state')], program.programId);
        [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from('offer_authority'), offerId.toArrayLike(Buffer, 'le', 8)], program.programId);
        offerSellTokenPda = await getAssociatedTokenAddress(sellTokenMint, offerAuthority, true);
        offerBuyToken1Pda = await getAssociatedTokenAddress(buyToken1Mint, offerAuthority, true);
        offerBuyToken2Pda = await getAssociatedTokenAddress(buyToken2Mint, offerAuthority, true);
    });

    it('Initialize onre with right boss account', async () => {
        await program.methods
            .initialize()
            .accounts({boss: initialBoss.publicKey})
            .rpc();
        const base58 = bs58.encode(Buffer.from([175, 175, 109, 31, 13, 152, 155, 237]));
        console.log(base58); // Outputs: 8oAk2RHn8C4
        const currentBoss = await program.account.state.fetch(statePda);
        expect(currentBoss.boss).toEqual(initialBoss.publicKey);
    });

    it('Set boss account sets a new boss account', async () => {
        const newBoss = new anchor.Wallet(Keypair.generate());
        await program.methods
            .setBoss(newBoss.publicKey)
            .accounts({state: statePda})
            .rpc();
        const currentBoss = await program.account.state.fetch(statePda);
        expect(currentBoss.boss).toEqual(newBoss.publicKey);

        await airdropLamports(provider, newBoss.publicKey, anchor.web3.LAMPORTS_PER_SOL * 200);
        const setBossInstruction = await program.methods
            .setBoss(initialBoss.publicKey)
            .accountsPartial({state: statePda, boss: newBoss.publicKey})
            .instruction();
        await createAndSendTransaction(provider, newBoss, [setBossInstruction]);

        const finalBoss = await program.account.state.fetch(statePda);
        expect(finalBoss.boss).toEqual(initialBoss.publicKey);
    });
    it('Makes an offer', async () => {
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
            .makeOfferOne(offerId, new anchor.BN(500e9), new anchor.BN(500e9))
            .accounts({sellTokenMint, buyToken1Mint, state: statePda})
            .preInstructions([buyToken1AccountInstruction, offerSellTokenAccountInstruction])
            .rpc();

        const offerAccount = await program.account.offer.fetch(offerPda);
        expect(offerAccount.offerId.eq(offerId)).toBe(true);
        expect(offerAccount.sellTokenMint.toBase58()).toEqual(sellTokenMint.toBase58());
        expect(offerAccount.buyTokenMint1.toBase58()).toEqual(buyToken1Mint.toBase58());
        expect(offerAccount.sellTokenTotalAmount.eq(new anchor.BN(500e9))).toBe(true);
        expect(offerAccount.buyToken1TotalAmount.eq(new anchor.BN(500e9))).toBe(true);

        const bossBuyTokenAccountInfo = await provider.connection.getTokenAccountBalance(bossBuyTokenAccount1);
        expect(+bossBuyTokenAccountInfo.value.amount).toEqual(500e9);

        const offerBuyTokenAccountInfo = await provider.connection.getTokenAccountBalance(offerBuyToken1Pda);
        expect(+offerBuyTokenAccountInfo.value.amount).toEqual(500e9);
    });

    it('Make offer fails on boss account with non boss signature', async () => {
        const newBoss = new anchor.Wallet(Keypair.generate());
        await airdropLamports(provider, newBoss.publicKey, anchor.web3.LAMPORTS_PER_SOL * 20);
        await expect(
            program.methods
                .makeOfferOne(offerId, new anchor.BN(500e9), new anchor.BN(500e9))
                .accountsPartial({
                    bossBuyToken1Account: bossBuyTokenAccount1,
                    sellTokenMint,
                    buyToken1Mint,
                    state: statePda
                })
                .signers([newBoss.payer])
                .rpc(),
        ).rejects.toThrow();
    });

    it('Make offer fails on non boss account with boss signature', async () => {
        const newBoss = new anchor.Wallet(Keypair.generate());
        await expect(
            program.methods
                .makeOfferOne(offerId, new anchor.BN(500e9), new anchor.BN(500e9))
                .accountsPartial({
                    bossBuyToken1Account: bossBuyTokenAccount1,
                    sellTokenMint,
                    buyToken1Mint,
                    state: statePda,
                    boss: newBoss.publicKey
                })
                .signers([initialBoss.payer])
                .rpc(),
        ).rejects.toThrow();
    });

    it('Make offer fails on non boss account with non boss signature', async () => {
        const newBoss = new anchor.Wallet(Keypair.generate());
        await expect(
            program.methods
                .makeOfferOne(offerId, new anchor.BN(500e9), new anchor.BN(500e9))
                .accountsPartial({
                    bossBuyToken1Account: bossBuyTokenAccount1,
                    sellTokenMint,
                    buyToken1Mint,
                    state: statePda,
                    boss: newBoss.publicKey
                })
                .signers([newBoss.payer])
                .rpc(),
        ).rejects.toThrow();
    });

    it('Replace an offer', async () => {
        const newOfferId = new anchor.BN(123123124);
        const [newOfferAuthorityPda] = PublicKey.findProgramAddressSync([Buffer.from('offer_authority'), newOfferId.toArrayLike(Buffer, 'le', 8)], program.programId);
        const newOfferSellTokenPda = await getAssociatedTokenAddress(sellTokenMint, newOfferAuthorityPda, true);
        const newOfferBuyTokenPda = await getAssociatedTokenAddress(buyToken1Mint, newOfferAuthorityPda, true);

        const closeInstruction = await program.methods.closeOfferOne().accounts({
            offer: offerPda,
            state: statePda
        }).instruction();
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
            .makeOfferOne(newOfferId, new anchor.BN(500e9), new anchor.BN(500e9))
            .accounts({sellTokenMint, buyToken1Mint, state: statePda})
            .instruction();

        await createAndSendTransaction(provider, initialBoss, [closeInstruction, offerSellTokenAccountInstruction, buyToken1AccountInstruction, makeOfferInstruction]);

        const [newOfferPda] = PublicKey.findProgramAddressSync([Buffer.from('offer'), newOfferId.toArrayLike(Buffer, 'le', 8)], program.programId);
        const offerAccount = await program.account.offer.fetch(newOfferPda);
        expect(offerAccount.offerId.eq(newOfferId)).toBe(true);
        expect(offerAccount.sellTokenMint.toBase58()).toEqual(sellTokenMint.toBase58());
        expect(offerAccount.buyTokenMint1.toBase58()).toEqual(buyToken1Mint.toBase58());
        expect(offerAccount.sellTokenTotalAmount.eq(new anchor.BN(500e9))).toBe(true);
        expect(offerAccount.buyToken1TotalAmount.eq(new anchor.BN(500e9))).toBe(true);

        const bossSellTokenAccountInfo = await provider.connection.getTokenAccountBalance(bossSellTokenAccount);
        const offerSellTokenAccountInfo = await provider.connection.getTokenAccountBalance(newOfferSellTokenPda);
        const offerBuyToken1AccountInfo = await provider.connection.getTokenAccountBalance(newOfferBuyTokenPda);
        expect(+bossSellTokenAccountInfo.value.amount).toEqual(1000e9);
        expect(+offerSellTokenAccountInfo.value.amount).toEqual(0);
        expect(+offerBuyToken1AccountInfo.value.amount).toEqual(500e9);
    });

    it('Create and take offer', async () => {
        const offerId = new anchor.BN(1);
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from('offer_authority'), offerId.toArrayLike(Buffer, 'le', 8)], program.programId);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from('offer'), offerId.toArrayLike(Buffer, 'le', 8)], program.programId);
        const offerSellTokenPda = await getAssociatedTokenAddress(sellTokenMint, offerAuthority, true);
        const offerBuyToken1Pda = await getAssociatedTokenAddress(buyToken1Mint, offerAuthority, true);
        const offerBuyToken2Pda = await getAssociatedTokenAddress(buyToken2Mint, offerAuthority, true);

        const offerSellTokenAccountInstruction = createAssociatedTokenAccountInstruction(
            initialBoss.payer.publicKey,
            offerSellTokenPda,
            offerAuthority,
            sellTokenMint,
        );
        const offerBuyToken1AccountInstruction = createAssociatedTokenAccountInstruction(
            initialBoss.payer.publicKey,
            offerBuyToken1Pda,
            offerAuthority,
            buyToken1Mint,
        );
        const offerBuyToken2AccountInstruction = createAssociatedTokenAccountInstruction(
            initialBoss.payer.publicKey,
            offerBuyToken2Pda,
            offerAuthority,
            buyToken2Mint,
        );

        await program.methods
            .makeOfferTwo(offerId, new anchor.BN(100e9), new anchor.BN(20e9), new anchor.BN(240e9))
            .accounts({sellTokenMint, buyToken1Mint, buyToken2Mint, state: statePda})
            .preInstructions([offerSellTokenAccountInstruction, offerBuyToken1AccountInstruction, offerBuyToken2AccountInstruction])
            .rpc();

        const user1 = new anchor.Wallet(Keypair.generate());
        await airdropLamports(provider, user1.publicKey, anchor.web3.LAMPORTS_PER_SOL * 20);
        const user1SellTokenAccount = await createATA(provider, user1.payer, sellTokenMint, user1.publicKey);
        await mintToAddress(provider, initialBoss.payer, sellTokenMint, user1SellTokenAccount, initialBoss.publicKey, 1000e9);

        const takeOfferInstruction = await program.methods
            .takeOfferTwo(new anchor.BN(120e9))
            .accountsPartial({userSellTokenAccount: user1SellTokenAccount, offer: offerPda, user: user1.publicKey})
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
            .accountsPartial({userSellTokenAccount: user2SellTokenAccount, offer: offerPda, user: user2.publicKey})
            .instruction();

        await createAndSendTransaction(provider, user2, [createUser2BuyToken1AccountInstruction, createUser2BuyToken2AccountInstruction, takeOfferInstruction2]);

        const offerBuyToken1Info2 = await provider.connection.getTokenAccountBalance(offerBuyToken1Pda);
        const offerBuyToken2Info2 = await provider.connection.getTokenAccountBalance(offerBuyToken2Pda);
        const offerSellTokenInfo2 = await provider.connection.getTokenAccountBalance(offerSellTokenPda);
        expect(+offerSellTokenInfo2.value.amount).toEqual(144e9);
        expect(+offerBuyToken1Info2.value.amount).toEqual(40e9);
        expect(+offerBuyToken2Info2.value.amount).toEqual(8e9);
    });

    it('Takes an offer with one buy token successfully', async () => {
        const offerId = new anchor.BN(2);
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from('offer_authority'), offerId.toArrayLike(Buffer, 'le', 8)], program.programId);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from('offer'), offerId.toArrayLike(Buffer, 'le', 8)], program.programId);
        const offerSellTokenPda = await getAssociatedTokenAddress(sellTokenMint, offerAuthority, true);
        const offerBuyToken1Pda = await getAssociatedTokenAddress(buyToken1Mint, offerAuthority, true);

        const offerSellTokenAccountInstruction = createAssociatedTokenAccountInstruction(
            initialBoss.payer.publicKey,
            offerSellTokenPda,
            offerAuthority,
            sellTokenMint,
        );
        const offerBuyToken1AccountInstruction = createAssociatedTokenAccountInstruction(
            initialBoss.payer.publicKey,
            offerBuyToken1Pda,
            offerAuthority,
            buyToken1Mint,
        );

        await program.methods
            .makeOfferOne(offerId, new anchor.BN(100e9), new anchor.BN(200e9))
            .accounts({sellTokenMint, buyToken1Mint, state: statePda})
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

    it('Fails to take offer with one buy token due to exceeding sell limit', async () => {
        const offerId = new anchor.BN(3);
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from('offer_authority'), offerId.toArrayLike(Buffer, 'le', 8)], program.programId);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from('offer'), offerId.toArrayLike(Buffer, 'le', 8)], program.programId);
        const offerSellTokenPda = await getAssociatedTokenAddress(sellTokenMint, offerAuthority, true);
        const offerBuyToken1Pda = await getAssociatedTokenAddress(buyToken1Mint, offerAuthority, true);

        await program.methods
            .makeOfferOne(offerId, new anchor.BN(100e9), new anchor.BN(50e9))
            .accounts({sellTokenMint, buyToken1Mint, state: statePda})
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

        await expect(provider.connection.sendRawTransaction(signedTransactionBytes)).rejects.toThrow(/The offer would exceed its total sell token limit/);
    });

    it('Fails to take offer with one buy token due to invalid buy token mint', async () => {
        const offerId = new anchor.BN(5);
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from('offer_authority'), offerId.toArrayLike(Buffer, 'le', 8)], program.programId);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from('offer'), offerId.toArrayLike(Buffer, 'le', 8)], program.programId);
        const offerSellTokenPda = await getAssociatedTokenAddress(sellTokenMint, offerAuthority, true);
        const offerBuyToken1Pda = await getAssociatedTokenAddress(buyToken1Mint, offerAuthority, true);

        await program.methods
            .makeOfferOne(offerId, new anchor.BN(100e9), new anchor.BN(200e9))
            .accounts({sellTokenMint, buyToken1Mint, state: statePda})
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
});
