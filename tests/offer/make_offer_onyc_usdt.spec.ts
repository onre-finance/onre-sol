import { PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

/**
 * Tests for the ONyc → USDT offer created on 2026-04-16.
 *
 * Production parameters:
 *   token_in:          ONyc  (9 decimals, SPL Token)
 *   token_out:         USDT  (6 decimals, SPL Token)
 *   fee:               0 bps
 *   needs_approval:    true
 *   permissionless:    true
 *
 * Vector:
 *   base_time:         1773878400  (Thu, 19 Mar 2026 00:00:00 GMT)
 *   base_price:        1085708975  (scale=9, ≈ 1.085708975)
 *   apr:               97593       (scale=6, ≈ 9.7593% APR)
 *   price_fix_duration: 86400      (1 day in seconds)
 */

describe("ONyc → USDT offer", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    // Simulate ONyc (9 decimals) and USDT (6 decimals) — both standard SPL Token
    let onycMint: PublicKey;
    let usdtMint: PublicKey;

    // Production vector values
    const BASE_TIME = 1773878400;
    const BASE_PRICE = 1085708975;
    const APR = 97593;
    const PRICE_FIX_DURATION = 86400;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper);

        onycMint = testHelper.createMint(9); // ONyc: 9 decimals
        usdtMint = testHelper.createMint(6); // USDT: 6 decimals

        await program.initialize({ onycMint });
    });

    test("Should create ONyc → USDT offer with production parameters", async () => {
        await program.makeOffer({
            tokenInMint: onycMint,
            tokenOutMint: usdtMint,
            feeBasisPoints: 0,
            withApproval: true,
            allowPermissionless: true,
        });

        const offer = await program.getOffer(onycMint, usdtMint);

        expect(offer.tokenInMint.toString()).toBe(onycMint.toString());
        expect(offer.tokenOutMint.toString()).toBe(usdtMint.toString());
        expect(offer.feeBasisPoints).toBe(0);
        expect(offer.needsApproval).toBe(1);
        expect(offer.allowPermissionless).toBe(1);
    });

    test("Should initialize vault token account for ONyc on offer creation", async () => {
        await program.makeOffer({
            tokenInMint: onycMint,
            tokenOutMint: usdtMint,
            feeBasisPoints: 0,
            withApproval: true,
            allowPermissionless: true,
        });

        await expect(
            testHelper.getAccount(
                getAssociatedTokenAddressSync(onycMint, program.pdas.offerVaultAuthorityPda, true)
            )
        ).resolves.toBeDefined();
    });

    test("Should add production vector to ONyc → USDT offer", async () => {
        await program.makeOffer({
            tokenInMint: onycMint,
            tokenOutMint: usdtMint,
            feeBasisPoints: 0,
            withApproval: true,
            allowPermissionless: true,
        });

        await program.addOfferVector({
            tokenInMint: onycMint,
            tokenOutMint: usdtMint,
            baseTime: BASE_TIME,
            basePrice: BASE_PRICE,
            apr: APR,
            priceFixDuration: PRICE_FIX_DURATION,
        });

        const offer = await program.getOffer(onycMint, usdtMint);
        const vector = offer.vectors[0];

        // base_time is in the past relative to bankrun's default clock,
        // so start_time will be set to current time — only verify stored fields
        expect(vector.baseTime.toNumber()).toBe(BASE_TIME);
        expect(vector.basePrice.toNumber()).toBe(BASE_PRICE);
        expect(vector.apr.toNumber()).toBe(APR);
        expect(vector.priceFixDuration.toNumber()).toBe(PRICE_FIX_DURATION);
    });

    test("Should reject a second identical ONyc → USDT offer", async () => {
        await program.makeOffer({
            tokenInMint: onycMint,
            tokenOutMint: usdtMint,
            feeBasisPoints: 0,
            withApproval: true,
            allowPermissionless: true,
        });

        await expect(
            program.makeOffer({
                tokenInMint: onycMint,
                tokenOutMint: usdtMint,
                feeBasisPoints: 0,
                withApproval: true,
                allowPermissionless: true,
            })
        ).rejects.toThrow();
    });
});
