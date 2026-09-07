import { Buffer } from "buffer";
import bs58 from "bs58";
import { Connection, PublicKey, SystemProgram, Transaction, TransactionInstruction, TransactionMessage, VersionedTransaction } from "@solana/web3.js";
import { STATE_ACCOUNT_MIN_LENGTH, STATE_ACCOUNT_OFFSETS, decodeKnownAccount, decodeStateAccount, isDefaultPublicKey, sameStateInfo, u64Seed } from "./account-decoders";
import {
    ASSOCIATED_TOKEN_PROGRAM_ID,
    CONFIGURABLE_VAULT_ACCOUNT_SEEDS,
    CONFIGURABLE_VAULT_KIND_SEEDS,
    DEFAULT_PROGRAM_ID,
    DEFAULT_RPC_PATH,
    idl,
    instructionByName,
    MAINNET_MINTS,
    MAINNET_RPC_URL,
    OFFER_TOKEN_IN_KEY,
    OFFER_TOKEN_OUT_KEY,
    PDA_SEEDS,
    PLACEHOLDER_BLOCKHASH,
    REDEMPTION_OFFER_TOKEN_OUT_KEY,
    REDEMPTION_REQUEST_ID_KEY,
    SYSVAR_INSTRUCTIONS_PUBKEY,
    TOKEN_CHOICES,
    TOKEN_PROGRAM_ID,
} from "./constants";
import { compactAddress, displayName, errorMessage, escapeHtml, jsonReplacer, normalizeName } from "./format";
import { decodeReturnData, defaultArgValue, encodeType, enumTypeDef, isDefinedType, parseBoolean, typeLabel } from "./idl-codec";
import type { AppState, DecodedAccount, FlatAccount, IdlAccount, IdlArg, IdlInstruction, IdlPda, IdlSeed, SolanaWallet } from "./types";
import "./styles.css";

globalThis.Buffer = Buffer;

// Runtime state and bootstrapping.
const state: AppState = {
    rpcUrl: initialRpcUrl(),
    customRpcUrl: initialCustomRpcUrl(),
    programId: initialProgramId(),
    connection: new Connection(initialRpcUrl(), "confirmed"),
    selectedInstructionName: idl.instructions[0]?.name ?? "",
    search: "",
    accountValues: {},
    accountAuto: {},
    argValues: {},
    derivationValues: {},
    instructionListScrollTop: 0,
    decodedAccounts: {},
    accountExistence: {},
    output: "",
};

const accountFetchesInFlight = new Set<string>();
let instructionListRestoreTimers: number[] = [];
let pendingInstructionListScrollTop: number | undefined;
let isRestoringInstructionListScroll = false;
let surfpoolEnvironment: SurfpoolEnvironment | undefined;

interface SurfpoolEnvironment {
    rpcUrl?: string;
    studioUrl?: string;
    upgradeAuthority?: string;
    disclaimer?: string;
}

const app = document.querySelector<HTMLDivElement>("#app");
if (!app) {
    throw new Error("App root not found");
}
const appRoot = app;

function initialRpcUrl(): string {
    const envRpcUrl = import.meta.env.VITE_ONRE_RPC_URL;
    const storedCustomRpcUrl = initialCustomRpcUrl();
    if (storedCustomRpcUrl) return customRpcProxyUrl(storedCustomRpcUrl);

    const storedRpcUrl = localStorage.getItem("onre-ui-rpc-url");
    if (envRpcUrl && (!storedRpcUrl || storedRpcUrl === MAINNET_RPC_URL || storedRpcUrl === DEFAULT_RPC_PATH)) {
        return isExternalRpcUrl(envRpcUrl) ? customRpcProxyUrl(envRpcUrl) : envRpcUrl;
    }
    if (!storedRpcUrl || storedRpcUrl === MAINNET_RPC_URL || storedRpcUrl === DEFAULT_RPC_PATH) return defaultRpcUrl();
    if (isExternalRpcUrl(storedRpcUrl)) return customRpcProxyUrl(storedRpcUrl);
    return storedRpcUrl;
}

function initialCustomRpcUrl(): string | undefined {
    const storedCustomRpcUrl = localStorage.getItem("onre-ui-custom-rpc-url");
    if (storedCustomRpcUrl) return storedCustomRpcUrl;

    const storedRpcUrl = localStorage.getItem("onre-ui-rpc-url");
    if (storedRpcUrl && isExternalRpcUrl(storedRpcUrl)) return storedRpcUrl;
    return undefined;
}

function initialProgramId(): PublicKey {
    const storedProgramId = localStorage.getItem("onre-ui-program-id");
    if (!storedProgramId) return DEFAULT_PROGRAM_ID;
    try {
        return new PublicKey(storedProgramId);
    } catch {
        return DEFAULT_PROGRAM_ID;
    }
}

function defaultRpcUrl(): string {
    return `${window.location.origin}${DEFAULT_RPC_PATH}`;
}

function browserRpcUrl(rpcUrl: string): string {
    return rpcUrl.startsWith("/") ? `${window.location.origin}${rpcUrl}` : rpcUrl;
}

function surfpoolRpcUrl(): string {
    return browserRpcUrl(surfpoolEnvironment?.rpcUrl ?? DEFAULT_RPC_PATH);
}

function isSurfpoolRpcSelected(): boolean {
    return !state.customRpcUrl && state.rpcUrl === surfpoolRpcUrl();
}

function rpcInputValue(): string {
    return state.customRpcUrl ?? state.rpcUrl;
}

function customRpcProxyUrl(target: string): string {
    return `${window.location.origin}/custom-rpc?target=${encodeURIComponent(target)}`;
}

function isExternalRpcUrl(rpcUrl: string): boolean {
    return /^https?:\/\//i.test(rpcUrl);
}

function boot(): void {
    initializeSelectedInstruction();
    render();
    void loadSurfpoolEnvironment();
    void refreshStateDerivedAccounts();
}

function initializeSelectedInstruction(): void {
    const instruction = selectedInstruction();
    state.accountValues = {};
    state.accountAuto = {};
    state.argValues = {};

    for (const arg of instruction.args ?? []) {
        state.argValues[arg.name] = defaultArgValue(arg.type);
    }
    if (instruction.name === "create_redemption_request") {
        state.argValues.request_id = crypto.randomUUID().replaceAll("-", "");
    }

    for (const flat of flattenAccounts(instruction.accounts ?? [])) {
        const value = defaultAccountValue(flat.account, flat.fullName);
        state.accountValues[flat.fullName] = value;
        state.accountAuto[flat.fullName] = value.length > 0 || shouldAutoDeriveByDefault(flat.account);
    }

    deriveAccounts();
}

async function refreshStateDerivedAccounts(): Promise<void> {
    try {
        const statePda = PublicKey.findProgramAddressSync([Buffer.from("state")], state.programId)[0];
        const accountInfo = await state.connection.getAccountInfo(statePda, "confirmed");
        if (!accountInfo) return;
        const stateInfo = decodeStateAccount(accountInfo.data);
        if (!stateInfo || sameStateInfo(state.stateInfo, stateInfo)) return;
        state.stateInfo = stateInfo;
        deriveAccounts();
        render();
        void refreshDecodedDerivedAccounts();
    } catch (error) {
        console.warn(`State fetch warning: ${errorMessage(error)}`);
    }
}

function selectedInstruction(): IdlInstruction {
    const instruction = instructionByName.get(state.selectedInstructionName);
    if (!instruction) {
        throw new Error(`Instruction not found: ${state.selectedInstructionName}`);
    }
    return instruction;
}

// Rendering. This owns the HTML shape and delegates business rules to the sections below.
function render(): void {
    const instruction = selectedInstruction();
    appRoot.innerHTML = `
        <main class="shell">
            <header class="topbar">
                <div class="brand">
                    <span class="brand-mark">OnRe</span>
                    <span class="brand-subtitle">Operations</span>
                    <span class="program-pill monospace">${compactAddress(state.programId.toBase58())}</span>
                </div>
                <div class="walletbar">
                    <span class="status-dot ${state.walletPublicKey ? "online" : ""}"></span>
                    <span class="wallet-key monospace">${state.walletPublicKey?.toBase58() ?? "No wallet"}</span>
                    <button id="wallet-button" class="primary">${state.walletPublicKey ? "Disconnect" : "Connect Wallet"}</button>
                </div>
            </header>

            <section class="rpc-band">
                <div class="rpc-copy">
                    <span>RPC</span>
                    <small>${surfpoolEnvironment ? "Docker Surfpool detected" : "Custom RPCs are proxied by this UI"}</small>
                </div>
                <div class="rpc-mode">
                    <button id="use-surfpool-rpc" class="${isSurfpoolRpcSelected() ? "active" : ""}">Use Surfpool</button>
                    <input id="rpc-url" value="${escapeHtml(rpcInputValue())}" placeholder="Paste custom RPC URL" />
                    <button id="apply-rpc" class="${isSurfpoolRpcSelected() ? "" : "active"}">Use Custom</button>
                </div>
                <label for="program-id">Program ID</label>
                <input id="program-id" value="${escapeHtml(state.programId.toBase58())}" />
                <button id="apply-program">Apply Program</button>
                <button id="ping-rpc">Ping</button>
                <button id="refresh-accounts">Refresh Accounts</button>
            </section>

            ${renderCheatcodeBand()}

            <section class="workspace">
                <aside class="sidebar">
                    <div class="sidebar-head">
                        <span>Instructions</span>
                        <small>${idl.instructions.length}</small>
                    </div>
                    <input id="instruction-search" class="search" placeholder="Search instructions" value="${escapeHtml(state.search)}" />
                    <div class="instruction-list">
                        ${renderInstructionList()}
                    </div>
                </aside>

                <section class="panel">
                    <div class="panel-head">
                        <div>
                            <h1>${displayName(instruction.name)}</h1>
                            <div class="instruction-meta">
                                <span>${instruction.args?.length ?? 0} args</span>
                                <span>${flattenAccounts(instruction.accounts ?? []).length} accounts</span>
                                ${instruction.returns ? `<span>returns ${typeLabel(instruction.returns)}</span>` : ""}
                            </div>
                        </div>
                        <div class="actions">
                            <button id="build-tx">Build</button>
                            <button id="simulate-tx">Simulate</button>
                            <button id="send-tx" class="primary">Sign & Send</button>
                        </div>
                    </div>

                    <div class="form-grid">
                        <section class="section">
                            <div class="section-title">Arguments</div>
                            ${renderArgs(instruction)}
                        </section>
                        <section class="section">
                            <div class="section-title">Accounts</div>
                            ${renderAccounts(instruction)}
                        </section>
                    </div>
                </section>

                <aside class="output">
                    <div class="section-title">Output</div>
                    <pre id="output-text">${escapeHtml(state.output || "Ready.")}</pre>
                    <div class="output-actions">
                        <button id="copy-base58" ${state.lastTransaction ? "" : "disabled"}>Copy Base58</button>
                        <button id="clear-output">Clear</button>
                    </div>
                </aside>
            </section>
        </main>
    `;

    bindEvents();
    restoreInstructionListScroll();
    scrollOutputToBottom();
}

function renderCheatcodeBand(): string {
    const boss = state.stateInfo?.boss.toBase58() ?? "";
    const worker = state.stateInfo?.worker.toBase58() ?? "";
    const fundTarget = state.walletPublicKey?.toBase58() ?? boss;
    const disclaimer = surfpoolEnvironment?.disclaimer ?? "Surfpool cheatcodes only work against a local Surfpool RPC. They are not available on the production mainnet program.";
    return `
        <section class="cheatcode-band">
            <div class="cheatcode-copy">
                <strong>Surfpool cheatcodes</strong>
                <span>${escapeHtml(disclaimer)}</span>
                ${surfpoolEnvironment?.upgradeAuthority ? `<small>Upgrade wallet <code>${escapeHtml(compactAddress(surfpoolEnvironment.upgradeAuthority))}</code></small>` : ""}
            </div>
            <div class="cheatcode-fields">
                <label>
                    <span>Boss</span>
                    <input id="cheat-boss" class="monospace" value="${escapeHtml(boss)}" placeholder="Boss address" />
                </label>
                <label>
                    <span>Worker</span>
                    <input id="cheat-worker" class="monospace" value="${escapeHtml(worker)}" placeholder="Worker address" />
                </label>
                <button id="apply-state-cheatcodes" class="primary">Set State</button>
                <label>
                    <span>SOL target</span>
                    <input id="cheat-fund-address" class="monospace" value="${escapeHtml(fundTarget)}" placeholder="Address to fund on Surfpool" />
                </label>
                <label class="short-field">
                    <span>Lamports</span>
                    <input id="cheat-fund-lamports" inputmode="numeric" value="10000000000000" />
                </label>
                <button id="fund-sol-cheatcode">Fund SOL</button>
            </div>
        </section>
    `;
}

function renderInstructionList(): string {
    const needle = state.search.trim().toLowerCase();
    return idl.instructions
        .filter((instruction) => instruction.name.toLowerCase().includes(needle))
        .map((instruction) => {
            const active = instruction.name === state.selectedInstructionName ? "active" : "";
            return `<button class="instruction ${active}" data-instruction="${instruction.name}">${displayName(instruction.name)}</button>`;
        })
        .join("");
}

function renderArgs(instruction: IdlInstruction): string {
    const args = instruction.args ?? [];
    if (!args.length) {
        return `<div class="empty">No arguments</div>`;
    }

    return args
        .map((arg) => {
            const value = state.argValues[arg.name] ?? defaultArgValue(arg.type);
            return `
                <label class="field">
                    <span>${arg.name}<small>${typeLabel(arg.type)}</small></span>
                    ${renderArgInput(arg, value)}
                </label>
            `;
        })
        .join("");
}

function renderArgInput(arg: IdlArg, value: string): string {
    if (arg.type === "bool") {
        const checked = parseBoolean(value) ? "checked" : "";
        return `<input type="checkbox" data-arg="${arg.name}" ${checked} />`;
    }

    const enumType = enumTypeDef(arg.type);
    if (enumType?.type.kind === "enum") {
        return `
            <div class="segmented" role="group" aria-label="${arg.name}">
                ${enumType.type.variants
                    .map((variant) => {
                        const active = normalizeName(variant.name) === normalizeName(value) ? "active" : "";
                        return `<button type="button" class="${active}" data-arg-choice="${arg.name}" data-arg-value="${variant.name}">${displayName(variant.name)}</button>`;
                    })
                    .join("")}
            </div>
        `;
    }

    const complex = typeof arg.type !== "string" || arg.type === "pubkey" || arg.type === "string";
    const inputMode = isIntegerArgType(arg.type) ? "numeric" : "text";
    return `
        <div class="input-stack">
            <input data-arg="${arg.name}" inputmode="${inputMode}" class="${complex ? "monospace" : ""}" value="${escapeHtml(value)}" />
            ${arg.type === "pubkey" && state.walletPublicKey ? `<button type="button" data-arg-wallet="${arg.name}">Use Wallet</button>` : ""}
        </div>
    `;
}

function isIntegerArgType(type: IdlArg["type"]): boolean {
    return typeof type === "string" && ["i64", "u8", "u16", "u32", "u64"].includes(type);
}

function renderAccounts(instruction: IdlInstruction): string {
    const accounts = flattenAccounts(instruction.accounts ?? []);
    if (!accounts.length) {
        return `<div class="empty">No accounts</div>`;
    }

    return accounts
        .map((flat) => {
            const account = flat.account;
            const value = state.accountValues[flat.fullName] ?? "";
            const flags = [account.signer ? "signer" : "", account.writable ? "writable" : "", account.pda ? "pda" : "", account.address ? "fixed" : ""].filter(Boolean).join(" ");
            const group = flat.group ? `<small>${flat.group}</small>` : "";
            return `
                <label class="field account-field">
                    <span>${flat.fullName}${group}<small>${flags || "readonly"}</small></span>
                    ${renderAccountControl(flat, value)}
                </label>
            `;
        })
        .join("");
}

function renderAccountControl(flat: FlatAccount, value: string): string {
    const lowerName = flat.account.name.toLowerCase();
    if (lowerName === "offer") {
        return renderOfferControl(flat, value);
    }
    if (lowerName === "redemption_offer") {
        return renderRedemptionOfferControl(flat, value);
    }
    if (lowerName === "redemption_request") {
        return renderRedemptionRequestControl(flat, value);
    }

    if (isTokenMintAccount(flat.account.name)) {
        const selected = tokenLabel(value);
        return `
            <div class="token-picker">
                ${TOKEN_CHOICES.map((token) => {
                    const active = selected === token.label ? "active" : "";
                    return `<button type="button" class="${active}" data-account-token="${flat.fullName}" data-token-value="${token.value.toBase58()}">${token.label}</button>`;
                }).join("")}
                <button type="button" class="${selected ? "" : "active"}" data-account-custom="${flat.fullName}">Custom</button>
            </div>
            ${
                selected
                    ? `<input type="hidden" data-account="${flat.fullName}" value="${escapeHtml(value)}" />`
                    : `<input data-account="${flat.fullName}" class="monospace custom-account" value="${escapeHtml(value)}" placeholder="Custom mint address" />`
            }
        `;
    }

    if (isManagedAccount(flat, value)) {
        return `
            <div class="derived-row">
                <code>${escapeHtml(compactAddress(value))}</code>
                <button type="button" data-copy-value="${escapeHtml(value)}">Copy</button>
                ${flat.account.signer ? `<button type="button" data-account-wallet="${flat.fullName}" ${state.walletPublicKey ? "" : "disabled"}>Wallet</button>` : ""}
                ${flat.account.pda ? `<button type="button" data-account-derive="${flat.fullName}">Rederive</button>` : ""}
            </div>
            <input type="hidden" data-account="${flat.fullName}" value="${escapeHtml(value)}" />
        `;
    }

    return `
        <div class="account-row">
            <input data-account="${flat.fullName}" class="monospace" value="${escapeHtml(value)}" placeholder="Paste address" />
            <button type="button" data-account-wallet="${flat.fullName}" ${state.walletPublicKey ? "" : "disabled"}>Wallet</button>
            <button type="button" data-account-derive="${flat.fullName}" ${flat.account.pda ? "" : "disabled"}>Derive</button>
        </div>
    `;
}

function renderOfferControl(flat: FlatAccount, value: string): string {
    const usesInstructionMints = Boolean(accountValueByName("token_in_mint") && accountValueByName("token_out_mint"));
    return `
        ${renderDerivedRow(flat, value)}
        ${usesInstructionMints ? "" : renderOfferMintPickers()}
        <input type="hidden" data-account="${flat.fullName}" value="${escapeHtml(value)}" />
    `;
}

function renderOfferMintPickers(): string {
    return `
        <div class="offer-mint-grid">
            ${renderOfferTokenPicker(OFFER_TOKEN_IN_KEY, "Token in", MAINNET_MINTS.usdc)}
            ${renderOfferTokenPicker(OFFER_TOKEN_OUT_KEY, "Token out", onycMint())}
        </div>
    `;
}

function renderOfferTokenPicker(key: string, label: string, fallback: PublicKey): string {
    const value = state.derivationValues[key] ?? fallback.toBase58();
    const selected = tokenLabel(value);
    return `
        <div class="picker-stack">
            <small>${label}</small>
            <div class="token-picker">
                ${TOKEN_CHOICES.map((token) => {
                    const active = selected === token.label ? "active" : "";
                    return `<button type="button" class="${active}" data-offer-mint-key="${key}" data-offer-mint-value="${token.value.toBase58()}">${token.label}</button>`;
                }).join("")}
                <button type="button" class="${selected ? "" : "active"}" data-offer-mint-custom="${key}">Custom</button>
            </div>
            ${selected ? "" : `<input data-offer-mint-custom-input="${key}" class="monospace custom-account" value="${escapeHtml(value)}" placeholder="${label} mint address" />`}
        </div>
    `;
}

function renderRedemptionOfferControl(flat: FlatAccount, value: string): string {
    const usesInstructionTokenOut = Boolean(accountValueByName("token_out_mint"));
    return `
        ${renderDerivedRow(flat, value)}
        ${usesInstructionTokenOut ? "" : renderRedemptionOfferTokenOutPicker()}
        <input type="hidden" data-account="${flat.fullName}" value="${escapeHtml(value)}" />
    `;
}

function renderRedemptionRequestControl(flat: FlatAccount, value: string): string {
    if (selectedInstruction().name === "create_redemption_request") {
        return `
            ${renderDerivedRow(flat, value)}
            <input type="hidden" data-account="${flat.fullName}" value="${escapeHtml(value)}" />
        `;
    }

    return `
        ${renderDerivedRow(flat, value)}
        <div class="input-stack">
            <input data-redemption-request-id="${flat.fullName}" value="${escapeHtml(state.derivationValues[REDEMPTION_REQUEST_ID_KEY] ?? "")}" placeholder="32-byte UUID without hyphens" maxlength="32" />
        </div>
        <input type="hidden" data-account="${flat.fullName}" value="${escapeHtml(value)}" />
    `;
}

function renderDerivedRow(flat: FlatAccount, value: string): string {
    const existence = accountExistenceLabel(value);
    return `
        <div class="derived-row">
            <code>${escapeHtml(value ? compactAddress(value) : "Not derived")}</code>
            ${existence}
            <button type="button" data-copy-value="${escapeHtml(value)}" ${value ? "" : "disabled"}>Copy</button>
            ${flat.account.signer ? `<button type="button" data-account-wallet="${flat.fullName}" ${state.walletPublicKey ? "" : "disabled"}>Wallet</button>` : ""}
            <button type="button" data-account-derive="${flat.fullName}">Rederive</button>
        </div>
    `;
}

function accountExistenceLabel(value: string): string {
    const status = state.accountExistence[value];
    if (!status) return "";
    return `<span class="account-status ${status}">${status}</span>`;
}

function renderRedemptionOfferTokenOutPicker(): string {
    const value = state.derivationValues[REDEMPTION_OFFER_TOKEN_OUT_KEY] ?? MAINNET_MINTS.usdc.toBase58();
    const selected = tokenLabel(value);
    return `
        <div class="token-picker">
            ${TOKEN_CHOICES.filter((token) => !token.value.equals(onycMint()))
                .map((token) => {
                    const active = selected === token.label ? "active" : "";
                    return `<button type="button" class="${active}" data-redemption-offer-token="${token.value.toBase58()}">${token.label}</button>`;
                })
                .join("")}
            <button type="button" class="${selected ? "" : "active"}" data-redemption-offer-custom>Custom</button>
        </div>
        ${selected ? "" : `<input data-redemption-offer-custom-input class="monospace custom-account" value="${escapeHtml(value)}" placeholder="Token out mint address" />`}
    `;
}

// DOM events. Keep event handlers thin: update state, call helpers, then render.
function bindEvents(): void {
    document.querySelector("#wallet-button")?.addEventListener("click", () => void toggleWallet());
    document.querySelector("#use-surfpool-rpc")?.addEventListener("click", useSurfpoolRpc);
    document.querySelector("#apply-rpc")?.addEventListener("click", applyRpcUrl);
    document.querySelector("#apply-program")?.addEventListener("click", applyProgramId);
    document.querySelector("#ping-rpc")?.addEventListener("click", () => void pingRpc());
    document.querySelector("#refresh-accounts")?.addEventListener("click", () => void refreshAccountsFromChain());
    document.querySelector("#apply-state-cheatcodes")?.addEventListener("click", () => void applySurfpoolStateCheatcodes());
    document.querySelector("#fund-sol-cheatcode")?.addEventListener("click", () => void fundSolWithSurfpoolCheatcode());
    document.querySelector("#build-tx")?.addEventListener("click", () => void buildAndReport());
    document.querySelector("#simulate-tx")?.addEventListener("click", () => void simulate());
    document.querySelector("#send-tx")?.addEventListener("click", () => void send());
    document.querySelector("#copy-base58")?.addEventListener("click", () => void copyBase58());
    document.querySelector("#clear-output")?.addEventListener("click", () => {
        state.output = "";
        render();
    });

    document.querySelector<HTMLInputElement>("#instruction-search")?.addEventListener("input", (event) => {
        state.search = (event.target as HTMLInputElement).value;
        render();
    });

    document.querySelector<HTMLDivElement>(".instruction-list")?.addEventListener("scroll", (event) => {
        if (isRestoringInstructionListScroll) return;
        state.instructionListScrollTop = (event.currentTarget as HTMLDivElement).scrollTop;
    });

    for (const button of document.querySelectorAll<HTMLButtonElement>("[data-instruction]")) {
        button.addEventListener("click", () => {
            const instructionList = button.closest<HTMLDivElement>(".instruction-list");
            if (instructionList) {
                state.instructionListScrollTop = instructionList.scrollTop;
                pendingInstructionListScrollTop = instructionList.scrollTop;
            }
            state.selectedInstructionName = button.dataset.instruction ?? state.selectedInstructionName;
            initializeSelectedInstruction();
            render();
            void refreshAccountsForSelectedInstruction();
        });
    }

    document.querySelector<HTMLInputElement>("#rpc-url")?.addEventListener("keydown", (event) => {
        if (event.key === "Enter") {
            applyRpcUrl();
        }
    });

    document.querySelector<HTMLInputElement>("#program-id")?.addEventListener("keydown", (event) => {
        if (event.key === "Enter") {
            applyProgramId();
        }
    });

    for (const input of document.querySelectorAll<HTMLInputElement>("[data-arg]")) {
        input.addEventListener("input", () => {
            const name = input.dataset.arg!;
            state.argValues[name] = input.type === "checkbox" ? String(input.checked) : input.value;
            deriveAccounts();
            updateAccountInputs();
            void refreshDecodedDerivedAccounts();
        });
    }

    for (const button of document.querySelectorAll<HTMLButtonElement>("[data-arg-choice]")) {
        button.addEventListener("click", () => {
            const name = button.dataset.argChoice!;
            state.argValues[name] = button.dataset.argValue ?? "";
            deriveAccounts();
            render();
            void refreshDecodedDerivedAccounts();
        });
    }

    for (const button of document.querySelectorAll<HTMLButtonElement>("[data-arg-wallet]")) {
        button.addEventListener("click", () => {
            const name = button.dataset.argWallet!;
            if (!state.walletPublicKey) return;
            state.argValues[name] = state.walletPublicKey.toBase58();
            deriveAccounts();
            render();
            void refreshDecodedDerivedAccounts();
        });
    }

    for (const input of document.querySelectorAll<HTMLInputElement>("[data-account]")) {
        input.addEventListener("input", () => {
            const name = input.dataset.account!;
            state.accountValues[name] = input.value.trim();
            state.accountAuto[name] = false;
            deriveAccounts();
            updateAccountInputs();
            void refreshDecodedDerivedAccounts();
        });
    }

    for (const button of document.querySelectorAll<HTMLButtonElement>("[data-account-token]")) {
        button.addEventListener("click", () => {
            const name = button.dataset.accountToken!;
            state.accountValues[name] = button.dataset.tokenValue ?? "";
            state.accountAuto[name] = false;
            deriveAccounts();
            render();
            void refreshDecodedDerivedAccounts();
        });
    }

    for (const button of document.querySelectorAll<HTMLButtonElement>("[data-account-custom]")) {
        button.addEventListener("click", () => {
            const name = button.dataset.accountCustom!;
            state.accountValues[name] = "";
            state.accountAuto[name] = false;
            deriveAccounts();
            render();
            void refreshDecodedDerivedAccounts();
        });
    }

    for (const button of document.querySelectorAll<HTMLButtonElement>("[data-offer-mint-value]")) {
        button.addEventListener("click", () => {
            const key = button.dataset.offerMintKey!;
            state.derivationValues[key] = button.dataset.offerMintValue ?? "";
            markOfferDependentAccountsAuto();
            deriveAccounts();
            render();
            void refreshDecodedDerivedAccounts();
        });
    }

    for (const button of document.querySelectorAll<HTMLButtonElement>("[data-offer-mint-custom]")) {
        button.addEventListener("click", () => {
            const key = button.dataset.offerMintCustom!;
            state.derivationValues[key] = "";
            markOfferDependentAccountsAuto();
            deriveAccounts();
            render();
        });
    }

    for (const input of document.querySelectorAll<HTMLInputElement>("[data-offer-mint-custom-input]")) {
        input.addEventListener("input", () => {
            const key = input.dataset.offerMintCustomInput!;
            state.derivationValues[key] = input.value.trim();
            markOfferDependentAccountsAuto();
            deriveAccounts();
            updateAccountInputs();
            void refreshDecodedDerivedAccounts();
        });
        input.addEventListener("change", () => render());
    }

    for (const button of document.querySelectorAll<HTMLButtonElement>("[data-redemption-offer-token]")) {
        button.addEventListener("click", () => {
            state.derivationValues[REDEMPTION_OFFER_TOKEN_OUT_KEY] = button.dataset.redemptionOfferToken ?? "";
            markAccountAuto("redemption_offer");
            markAccountAuto("redemption_request");
            deriveAccounts();
            render();
            void refreshDecodedDerivedAccounts();
        });
    }

    document.querySelector<HTMLButtonElement>("[data-redemption-offer-custom]")?.addEventListener("click", () => {
        state.derivationValues[REDEMPTION_OFFER_TOKEN_OUT_KEY] = "";
        markAccountAuto("redemption_offer");
        markAccountAuto("redemption_request");
        deriveAccounts();
        render();
    });

    document.querySelector<HTMLInputElement>("[data-redemption-offer-custom-input]")?.addEventListener("input", (event) => {
        state.derivationValues[REDEMPTION_OFFER_TOKEN_OUT_KEY] = (event.target as HTMLInputElement).value.trim();
        markAccountAuto("redemption_offer");
        markAccountAuto("redemption_request");
        deriveAccounts();
        updateAccountInputs();
        void refreshDecodedDerivedAccounts();
    });

    for (const input of document.querySelectorAll<HTMLInputElement>("[data-redemption-request-id]")) {
        input.addEventListener("input", () => {
            state.derivationValues[REDEMPTION_REQUEST_ID_KEY] = input.value;
            markAccountAuto("redemption_request");
            deriveAccounts();
            updateAccountInputs();
        });
        input.addEventListener("change", () => render());
    }

    for (const button of document.querySelectorAll<HTMLButtonElement>("[data-account-wallet]")) {
        button.addEventListener("click", () => {
            const name = button.dataset.accountWallet!;
            if (!state.walletPublicKey) return;
            state.accountValues[name] = state.walletPublicKey.toBase58();
            state.accountAuto[name] = false;
            updateAccountInputs();
            void refreshDecodedDerivedAccounts();
        });
    }

    for (const button of document.querySelectorAll<HTMLButtonElement>("[data-account-derive]")) {
        button.addEventListener("click", () => {
            const name = button.dataset.accountDerive!;
            state.accountAuto[name] = true;
            deriveAccounts();
            updateAccountInputs();
            void refreshDecodedDerivedAccounts();
        });
    }

    for (const button of document.querySelectorAll<HTMLButtonElement>("[data-copy-value]")) {
        button.addEventListener("click", () => {
            void navigator.clipboard.writeText(button.dataset.copyValue ?? "");
        });
    }
}

async function loadSurfpoolEnvironment(): Promise<void> {
    try {
        const response = await fetch("/surfpool-env.json", { cache: "no-store" });
        if (!response.ok) return;
        surfpoolEnvironment = (await response.json()) as SurfpoolEnvironment;
        if (surfpoolEnvironment.rpcUrl && !state.customRpcUrl) {
            const rpcUrl = browserRpcUrl(surfpoolEnvironment.rpcUrl);
            setRpcEndpoint(rpcUrl);
            localStorage.setItem("onre-ui-rpc-url", rpcUrl);
            localStorage.removeItem("onre-ui-custom-rpc-url");
            void refreshStateDerivedAccounts();
        }
        render();
    } catch {
        // This file exists only in the Docker Surfpool environment.
    }
}

function updateAccountInputs(): void {
    for (const input of document.querySelectorAll<HTMLInputElement>("[data-account]")) {
        const name = input.dataset.account!;
        input.value = state.accountValues[name] ?? "";
    }
}

// Wallet, RPC, and chain refresh actions.
async function toggleWallet(): Promise<void> {
    if (state.walletPublicKey) {
        await state.wallet?.disconnect?.();
        state.wallet = undefined;
        state.walletPublicKey = undefined;
        appendOutput("Wallet disconnected.");
        initializeSelectedInstruction();
        render();
        void refreshStateDerivedAccounts();
        return;
    }

    const wallet = findWallet();
    if (!wallet) {
        appendOutput("No injected Solana wallet found.");
        render();
        return;
    }

    const result = await wallet.connect();
    state.wallet = wallet;
    state.walletPublicKey = result?.publicKey ?? wallet.publicKey;
    if (!state.walletPublicKey) {
        appendOutput("Wallet connected without a public key.");
    } else {
        appendOutput(`Wallet connected: ${state.walletPublicKey.toBase58()}`);
    }
    initializeSelectedInstruction();
    render();
    void refreshStateDerivedAccounts();
}

function findWallet(): SolanaWallet | undefined {
    const win = window as Window & {
        solana?: SolanaWallet;
        solflare?: SolanaWallet;
    };
    if (win.solana?.isPhantom || win.solana?.signTransaction) return win.solana;
    if (win.solflare?.signTransaction) return win.solflare;
    return win.solana ?? win.solflare;
}

function applyRpcUrl(): void {
    const input = document.querySelector<HTMLInputElement>("#rpc-url");
    const rpcUrl = input?.value.trim() || defaultRpcUrl();
    setCustomRpcUrl(rpcUrl);
    const programChanged = applyProgramIdFromInput();
    appendOutput(`Custom RPC set: ${rpcUrl}${programChanged ? `\nProgram: ${state.programId.toBase58()}` : ""}`);
    render();
    void refreshStateDerivedAccounts();
}

function useSurfpoolRpc(): void {
    const rpcUrl = surfpoolRpcUrl();
    state.customRpcUrl = undefined;
    setRpcEndpoint(rpcUrl);
    localStorage.setItem("onre-ui-rpc-url", rpcUrl);
    localStorage.removeItem("onre-ui-custom-rpc-url");
    const programChanged = applyProgramIdFromInput();
    appendOutput(`Surfpool RPC set: ${rpcUrl}${programChanged ? `\nProgram: ${state.programId.toBase58()}` : ""}`);
    render();
    void refreshStateDerivedAccounts();
}

function setCustomRpcUrl(rpcUrl: string): void {
    state.customRpcUrl = rpcUrl;
    const endpoint = isExternalRpcUrl(rpcUrl) ? customRpcProxyUrl(rpcUrl) : rpcUrl;
    setRpcEndpoint(endpoint);
    localStorage.setItem("onre-ui-rpc-url", endpoint);
    localStorage.setItem("onre-ui-custom-rpc-url", rpcUrl);
}

function setRpcEndpoint(rpcUrl: string): void {
    state.rpcUrl = rpcUrl;
    state.connection = new Connection(rpcUrl, "confirmed");
    state.stateInfo = undefined;
    state.decodedAccounts = {};
    state.accountExistence = {};
    accountFetchesInFlight.clear();
}

function applyProgramId(): void {
    if (!applyProgramIdFromInput()) return;
    appendOutput(`Program set: ${state.programId.toBase58()}`);
    render();
    void refreshStateDerivedAccounts();
}

function applyProgramIdFromInput(): boolean {
    const programInput = document.querySelector<HTMLInputElement>("#program-id");
    const rawProgramId = programInput?.value.trim() || DEFAULT_PROGRAM_ID.toBase58();
    let programId: PublicKey;

    try {
        programId = new PublicKey(rawProgramId);
    } catch {
        appendOutput(`Invalid program ID: ${rawProgramId}`);
        render();
        return false;
    }

    if (state.programId.equals(programId)) return false;
    state.programId = programId;
    localStorage.setItem("onre-ui-program-id", programId.toBase58());
    state.stateInfo = undefined;
    state.decodedAccounts = {};
    state.accountExistence = {};
    accountFetchesInFlight.clear();
    initializeSelectedInstruction();
    return true;
}

async function pingRpc(): Promise<void> {
    try {
        const slot = await state.connection.getSlot("confirmed");
        appendOutput(`RPC ok. Confirmed slot: ${slot}`);
    } catch (error) {
        appendOutput(`RPC error: ${errorMessage(error)}`);
    }
    render();
}

async function refreshAccountsFromChain(): Promise<void> {
    await refreshStateDerivedAccounts();
    await refreshDecodedDerivedAccounts();
    appendOutput("Account resolver refreshed from RPC.");
    render();
}

async function applySurfpoolStateCheatcodes(): Promise<void> {
    try {
        const bossValue = document.querySelector<HTMLInputElement>("#cheat-boss")?.value.trim();
        const workerValue = document.querySelector<HTMLInputElement>("#cheat-worker")?.value.trim();
        const boss = bossValue ? new PublicKey(bossValue) : undefined;
        const worker = workerValue ? new PublicKey(workerValue) : undefined;
        if (!boss && !worker) {
            throw new Error("Enter a boss or worker address.");
        }

        await patchStateAccountAuthorities({ boss, worker });
        state.stateInfo = undefined;
        state.decodedAccounts = {};
        state.accountExistence = {};
        accountFetchesInFlight.clear();
        await refreshStateDerivedAccounts();
        appendOutput(
            ["Surfpool state cheatcode applied.", boss ? `Boss: ${boss.toBase58()}` : undefined, worker ? `Worker: ${worker.toBase58()}` : undefined].filter(Boolean).join("\n"),
        );
    } catch (error) {
        appendOutput(`Surfpool cheatcode error: ${errorMessage(error)}`);
    }
    render();
}

async function patchStateAccountAuthorities(params: { boss?: PublicKey; worker?: PublicKey }): Promise<void> {
    const statePda = findPda(["state"]);
    const account = await state.connection.getAccountInfo(statePda, "confirmed");
    if (!account) {
        throw new Error(`State account not found at ${statePda.toBase58()}`);
    }

    const data = Buffer.from(account.data);
    if (data.length < STATE_ACCOUNT_MIN_LENGTH) {
        throw new Error(`State account is too short to patch (${data.length} bytes).`);
    }
    if (params.boss) data.set(params.boss.toBuffer(), STATE_ACCOUNT_OFFSETS.boss);
    if (params.worker) data.set(params.worker.toBuffer(), STATE_ACCOUNT_OFFSETS.worker);

    await surfnetRpc("surfnet_setAccount", [
        statePda.toBase58(),
        {
            lamports: account.lamports,
            owner: account.owner.toBase58(),
            executable: account.executable,
            data: data.toString("hex"),
        },
    ]);
}

async function fundSolWithSurfpoolCheatcode(): Promise<void> {
    try {
        const targetValue = document.querySelector<HTMLInputElement>("#cheat-fund-address")?.value.trim();
        const lamportsValue = document.querySelector<HTMLInputElement>("#cheat-fund-lamports")?.value.trim() ?? "";
        if (!targetValue) throw new Error("Enter an address to fund.");
        const target = new PublicKey(targetValue);
        const lamports = Number(lamportsValue);
        if (!Number.isSafeInteger(lamports) || lamports <= 0) {
            throw new Error("Lamports must be a positive safe integer.");
        }
        await setSurfpoolLamports(target, lamports);
        appendOutput(`Surfpool SOL funded: ${target.toBase58()} = ${lamports} lamports`);
    } catch (error) {
        appendOutput(`Surfpool fund error: ${errorMessage(error)}`);
    }
    render();
}

async function setSurfpoolLamports(publicKey: PublicKey, lamports: number): Promise<void> {
    const account = await state.connection.getAccountInfo(publicKey, "confirmed");
    await surfnetRpc("surfnet_setAccount", [
        publicKey.toBase58(),
        {
            lamports,
            owner: (account?.owner ?? SystemProgram.programId).toBase58(),
            executable: account?.executable ?? false,
            data: account ? Buffer.from(account.data).toString("hex") : "",
        },
    ]);
}

async function surfnetRpc<T>(method: string, params: unknown[]): Promise<T> {
    const response = await fetch(state.rpcUrl, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ jsonrpc: "2.0", id: "onre-ui-cheatcode", method, params }),
    });
    const body = (await response.json()) as { result?: T; error?: unknown };
    if (body.error) {
        throw new Error(`${method} failed. Is the RPC URL a running Surfpool instance? ${JSON.stringify(body.error)}`);
    }
    return body.result as T;
}

async function refreshAccountsForSelectedInstruction(): Promise<void> {
    if (!state.stateInfo) {
        await refreshStateDerivedAccounts();
    }
    await refreshDecodedDerivedAccounts();
}

// Transaction lifecycle: build, simulate, send, and base58 export.
async function buildAndReport(): Promise<Transaction | undefined> {
    try {
        const { tx, offlineBlockhashReason } = await buildTransaction({ allowOfflineBlockhash: true });
        state.lastTransaction = tx;
        const base58 = serializeBase58(tx);
        appendOutput(
            [
                `Built ${selectedInstruction().name}`,
                `Fee payer: ${tx.feePayer?.toBase58()}`,
                `Recent blockhash: ${tx.recentBlockhash}`,
                offlineBlockhashReason ? `Blockhash note: ${offlineBlockhashReason}` : undefined,
                `Instructions: ${tx.instructions.length}`,
                `Base58: ${base58}`,
            ]
                .filter(Boolean)
                .join("\n"),
        );
        render();
        return tx;
    } catch (error) {
        appendOutput(`Build error: ${errorMessage(error)}`);
        render();
        return undefined;
    }
}

async function simulate(): Promise<void> {
    try {
        const { tx } = await buildTransaction({ allowOfflineBlockhash: false });
        state.lastTransaction = tx;
        const versionedTx = new VersionedTransaction(
            new TransactionMessage({
                payerKey: tx.feePayer!,
                recentBlockhash: tx.recentBlockhash!,
                instructions: tx.instructions,
            }).compileToV0Message(),
        );
        const result = await state.connection.simulateTransaction(versionedTx, {
            sigVerify: false,
            replaceRecentBlockhash: true,
        });
        const returnData = result.value.returnData;
        const decodedReturn = returnData ? decodeReturnData(selectedInstruction(), returnData.data[0]) : undefined;
        appendOutput(
            JSON.stringify(
                {
                    err: result.value.err,
                    unitsConsumed: result.value.unitsConsumed,
                    returnData: returnData
                        ? {
                              programId: returnData.programId,
                              rawBase64: returnData.data[0],
                              decoded: decodedReturn,
                          }
                        : null,
                    logs: result.value.logs,
                },
                jsonReplacer,
                2,
            ),
        );
    } catch (error) {
        appendOutput(`Simulation error: ${errorMessage(error)}`);
    }
    render();
}

async function send(): Promise<void> {
    if (!state.walletPublicKey || !state.wallet?.signTransaction) {
        appendOutput("Connect a wallet with signTransaction support first.");
        render();
        return;
    }

    try {
        const { tx } = await buildTransaction({ allowOfflineBlockhash: false });
        const signed = await state.wallet.signTransaction(tx);
        const signature = await state.connection.sendRawTransaction(signed.serialize(), {
            skipPreflight: false,
        });
        await state.connection.confirmTransaction(signature, "confirmed");
        state.lastTransaction = tx;
        state.lastSignature = signature;
        appendOutput(`Confirmed: ${signature}\nhttps://solscan.io/tx/${signature}`);
    } catch (error) {
        appendOutput(`Send error: ${errorMessage(error)}`);
    }
    render();
}

async function copyBase58(): Promise<void> {
    if (!state.lastTransaction) return;
    const base58 = serializeBase58(state.lastTransaction);
    await navigator.clipboard.writeText(base58);
    appendOutput("Base58 copied.");
    render();
}

async function buildTransaction(options: { allowOfflineBlockhash: boolean }): Promise<{ tx: Transaction; offlineBlockhashReason?: string }> {
    const instruction = buildInstruction();
    const tx = new Transaction();
    tx.add(instruction);
    tx.feePayer = transactionFeePayer(instruction);
    const blockhash = await getBlockhash(options.allowOfflineBlockhash);
    tx.recentBlockhash = blockhash.blockhash;
    return { tx, offlineBlockhashReason: blockhash.offlineReason };
}

async function getBlockhash(allowOfflineBlockhash: boolean): Promise<{ blockhash: string; offlineReason?: string }> {
    try {
        return { blockhash: (await state.connection.getLatestBlockhash("confirmed")).blockhash };
    } catch (error) {
        if (!allowOfflineBlockhash) throw error;
        return {
            blockhash: PLACEHOLDER_BLOCKHASH,
            offlineReason: `RPC blockhash fetch failed, using placeholder for inspection/export only: ${errorMessage(error)}`,
        };
    }
}

function buildInstruction(): TransactionInstruction {
    const instruction = selectedInstruction();
    const accounts = flattenAccounts(instruction.accounts ?? []);
    if (accounts.some((flat) => flat.account.name === "redemption_request")) {
        requireExactRedemptionRequestId(instruction.name === "create_redemption_request"
            ? state.argValues.request_id
            : state.derivationValues[REDEMPTION_REQUEST_ID_KEY]);
    }
    const keys = accounts.map((flat) => {
        const value = state.accountValues[flat.fullName]?.trim();
        if (!value) {
            throw new Error(`Missing account: ${flat.fullName}`);
        }
        return {
            pubkey: new PublicKey(value),
            isSigner: Boolean(flat.account.signer),
            isWritable: Boolean(flat.account.writable),
        };
    });

    const data = Buffer.concat([Buffer.from(instruction.discriminator), ...((instruction.args ?? []).map((arg) => encodeType(arg.type, parseArgValue(arg))) ?? [])]);

    return new TransactionInstruction({
        programId: state.programId,
        keys,
        data,
    });
}

function parseArgValue(arg: IdlArg): unknown {
    const raw = state.argValues[arg.name] ?? "";
    if (arg.type === "bool") return parseBoolean(raw);
    if (typeof arg.type === "string" && arg.type !== "pubkey" && arg.type !== "string") return raw.trim();
    if (arg.type === "pubkey" || arg.type === "string") return raw.trim();
    return raw.trim() ? JSON.parse(raw) : null;
}

function firstSigner(instruction: TransactionInstruction): PublicKey | undefined {
    return instruction.keys.find((key) => key.isSigner)?.pubkey;
}

function transactionFeePayer(instruction: TransactionInstruction): PublicKey {
    const feePayer = state.walletPublicKey ?? firstSigner(instruction) ?? state.stateInfo?.boss ?? publicKeyFromAccountValue("boss");
    if (!feePayer) {
        throw new Error("Missing fee payer. Connect a wallet or refresh accounts so state.boss can be used.");
    }
    return feePayer;
}

function serializeBase58(tx: Transaction): string {
    return bs58.encode(
        tx.serialize({
            requireAllSignatures: false,
            verifySignatures: false,
        }),
    );
}

// Account derivation. This mirrors the Rust seeds, account constraints, and known mainnet defaults.
function deriveAccounts(): void {
    const instruction = selectedInstruction();
    for (const flat of flattenAccounts(instruction.accounts ?? [])) {
        if (!state.accountAuto[flat.fullName]) continue;
        const value = deriveAccountValue(flat.account, flat.fullName);
        if (value) {
            state.accountValues[flat.fullName] = value;
        } else if (flat.account.pda) {
            state.accountValues[flat.fullName] = "";
        }
    }
}

function deriveAccountValue(account: IdlAccount, fullName: string): string {
    const lowerName = account.name.toLowerCase();
    if (lowerName === "boss" && state.stateInfo) return state.stateInfo.boss.toBase58();
    if (lowerName === "boss" && state.accountValues[fullName]) return state.accountValues[fullName];
    if (lowerName === "new_boss" && state.stateInfo && !isDefaultPublicKey(state.stateInfo.proposedBoss)) return state.stateInfo.proposedBoss.toBase58();
    if (lowerName === "worker" && state.stateInfo) return state.stateInfo.worker.toBase58();
    if (lowerName === "main_offer" && state.stateInfo) return state.stateInfo.mainOffer.toBase58();
    if (lowerName === "redeemer") {
        const redemptionRequest = decodedAccountByName("redemption_request", "redemption_request");
        if (redemptionRequest?.kind === "redemption_request") return redemptionRequest.value.redeemer.toBase58();
    }
    if (lowerName === "destination") {
        const vault = decodedAccountByName("configurable_vault", "configurable_vault");
        if (vault?.kind === "configurable_vault") return vault.value.withdrawalDestination.toBase58();
    }
    if (account.signer && state.walletPublicKey) return state.walletPublicKey.toBase58();
    if (lowerName === "system_program") return SystemProgram.programId.toBase58();
    if (lowerName === "token_program" || lowerName === "token_in_program" || lowerName === "token_out_program") return TOKEN_PROGRAM_ID.toBase58();
    if (lowerName === "associated_token_program") return ASSOCIATED_TOKEN_PROGRAM_ID.toBase58();
    if (lowerName === "instructions_sysvar") return SYSVAR_INSTRUCTIONS_PUBKEY.toBase58();
    if (lowerName === "program") return state.programId.toBase58();
    if (lowerName === "onyc_mint") return onycMint().toBase58();
    if (lowerName === "token_in_mint") return defaultTokenInMint().toBase58();
    if (lowerName === "token_out_mint") return defaultTokenOutMint().toBase58();
    if (lowerName === "asset_mint") return MAINNET_MINTS.usdc.toBase58();
    if (lowerName === "mint" || lowerName === "token_mint") return defaultGenericMint().toBase58();
    if (lowerName === "usdc_mint") return MAINNET_MINTS.usdc.toBase58();
    if (lowerName === "usdt_mint") return MAINNET_MINTS.usdt.toBase58();
    if (lowerName === "usdg_mint") return MAINNET_MINTS.usdg.toBase58();

    const fixedPda = deriveFixedPda(lowerName);
    if (fixedPda) return fixedPda.toBase58();

    if (lowerName === "offer") {
        const offer = deriveOfferPda();
        if (offer) return offer.toBase58();
        const redemptionOffer = decodedAccountByName("redemption_offer", "redemption_offer");
        if (redemptionOffer?.kind === "redemption_offer") return redemptionOffer.value.offer.toBase58();
    }

    if (lowerName === "redemption_offer") {
        const redemptionOffer = deriveRedemptionOfferPda();
        if (redemptionOffer) return redemptionOffer.toBase58();
    }

    if (lowerName === "redemption_request") {
        const redemptionRequest = deriveRedemptionRequestPda();
        if (redemptionRequest) return redemptionRequest.toBase58();
    }

    if (lowerName === "prop_amm_pair_state") {
        const offer = publicKeyFromAccountValue("offer");
        if (offer) return findPda(["prop_amm_pair", offer]).toBase58();
    }

    if (isTokenAccountName(lowerName)) {
        const owner = inferTokenAccountOwner(lowerName);
        const mint = inferTokenAccountMint(lowerName);
        if (owner && mint) return getAssociatedTokenAddress(mint, owner).toBase58();
    }

    if (account.address) return account.address;

    if (account.pda) {
        const derived = tryDerivePda(account.pda);
        if (derived) return derived.toBase58();
    }

    return state.accountValues[fullName] ?? "";
}

function defaultAccountValue(account: IdlAccount, fullName: string): string {
    return deriveAccountValue(account, fullName);
}

function markAccountAuto(accountName: string): void {
    for (const flat of flattenAccounts(selectedInstruction().accounts ?? [])) {
        if (flat.account.name === accountName) {
            state.accountAuto[flat.fullName] = true;
        }
    }
}

function markOfferDependentAccountsAuto(): void {
    markAccountAuto("offer");
    markAccountAuto("prop_amm_pair_state");
}

function shouldAutoDeriveByDefault(account: IdlAccount): boolean {
    const lowerName = account.name.toLowerCase();
    return (
        Boolean(account.pda || account.address) ||
        lowerName === "offer" ||
        lowerName === "boss" ||
        lowerName === "new_boss" ||
        lowerName === "worker" ||
        lowerName === "main_offer" ||
        lowerName === "redeemer" ||
        lowerName === "destination" ||
        lowerName === "redemption_offer" ||
        lowerName === "redemption_request" ||
        Boolean(PDA_SEEDS[lowerName] || CONFIGURABLE_VAULT_ACCOUNT_SEEDS[lowerName]) ||
        isTokenAccountName(lowerName)
    );
}

function isManagedAccount(flat: FlatAccount, value: string): boolean {
    if (!value) return false;
    if (isTokenMintAccount(flat.account.name)) return false;

    const lowerName = flat.account.name.toLowerCase();
    if (flat.account.address) return true;
    if (flat.account.pda && state.accountAuto[flat.fullName]) return true;
    if (deriveFixedPda(lowerName) || isTokenAccountName(lowerName)) return true;
    return (
        lowerName === "boss" ||
        lowerName === "worker" ||
        lowerName === "main_offer" ||
        lowerName.startsWith("boss_") ||
        ["system_program", "token_program", "token_in_program", "token_out_program", "associated_token_program", "instructions_sysvar", "program"].includes(lowerName)
    );
}

function inferTokenAccountMint(lowerName: string): PublicKey | undefined {
    if (lowerName.includes("onyc")) return onycMint();
    if (lowerName.includes("usdc")) return MAINNET_MINTS.usdc;
    if (lowerName.includes("usdt")) return MAINNET_MINTS.usdt;
    if (lowerName.includes("usdg")) return MAINNET_MINTS.usdg;
    if (lowerName.includes("token_in")) return publicKeyFromAccountValue("token_in_mint");
    if (lowerName.includes("token_out")) return publicKeyFromAccountValue("token_out_mint");
    if (lowerName.includes("token_account")) return publicKeyFromAccountValue("token_mint") ?? publicKeyFromAccountValue("mint") ?? publicKeyFromAccountValue("token_in_mint");
    if (lowerName.includes("vault_token_account"))
        return publicKeyFromAccountValue("token_mint") ?? publicKeyFromAccountValue("mint") ?? publicKeyFromAccountValue("token_in_mint");
    if (lowerName.includes("boss_token_account")) return publicKeyFromAccountValue("token_mint") ?? publicKeyFromAccountValue("mint") ?? publicKeyFromAccountValue("token_in_mint");
    if (lowerName.includes("destination_token_account")) return publicKeyFromAccountValue("mint") ?? publicKeyFromAccountValue("token_mint");
    return undefined;
}

function inferTokenAccountOwner(lowerName: string): PublicKey | undefined {
    if (lowerName.includes("permissionless_offer_fee")) return deriveConfigurableVaultPda("permissionless_offer_fee");
    if (lowerName.includes("offer_fee")) return deriveConfigurableVaultPda("offer_fee");
    if (lowerName.includes("redemption_fee")) return deriveConfigurableVaultPda("redemption_fee");
    if (lowerName.includes("management_fee")) return deriveConfigurableVaultPda("management_fee");
    if (lowerName.includes("performance_fee")) return deriveConfigurableVaultPda("performance_fee");
    if (lowerName.includes("prop_amm_buy_fee")) return deriveConfigurableVaultPda("prop_amm_buy_fee");
    if (lowerName.includes("prop_amm_sell_fee")) return deriveConfigurableVaultPda("prop_amm_sell_fee");
    if (lowerName.includes("offer_proceeds")) return deriveConfigurableVaultPda("offer_proceeds");
    if (lowerName.includes("prop_amm_proceeds")) return deriveConfigurableVaultPda("prop_amm_proceeds");
    if (lowerName.includes("reserve_vault")) return deriveFixedPda("reserve_vault_authority");
    if (lowerName.includes("redemption_vault") || (selectedInstruction().name.includes("redemption") && lowerName === "vault_token_account"))
        return deriveFixedPda("redemption_vault_authority");
    if (lowerName.startsWith("offer_vault")) return deriveFixedPda("offer_vault_authority");
    if (lowerName.includes("permissionless")) return deriveFixedPda("permissionless_authority");
    if (lowerName.includes("vault_token")) return genericVaultTokenAuthority();
    if (lowerName.includes("boss")) return state.stateInfo?.boss;
    if (lowerName.includes("redeemer")) return publicKeyFromAccountValue("redeemer") ?? state.walletPublicKey;
    if (lowerName.includes("depositor")) return publicKeyFromAccountValue("depositor") ?? state.walletPublicKey;
    if (lowerName.includes("user")) return publicKeyFromAccountValue("user") ?? state.walletPublicKey;
    if (lowerName.includes("destination")) return publicKeyFromAccountValue("destination");
    return undefined;
}

function genericVaultTokenAuthority(): PublicKey | undefined {
    const instructionName = selectedInstruction().name;
    if (instructionName.includes("redemption")) return deriveFixedPda("redemption_vault_authority");
    return deriveFixedPda("offer_vault_authority");
}

function defaultTokenInMint(): PublicKey {
    const decoded = decodedAccountByName("redemption_offer", "redemption_offer");
    if (decoded?.kind === "redemption_offer") return decoded.value.tokenInMint;
    const offer = decodedAccountByName("offer", "offer");
    if (offer?.kind === "offer") return offer.value.tokenInMint;
    return instructionDefaultsToRedemptionDirection() ? onycMint() : MAINNET_MINTS.usdc;
}

function defaultTokenOutMint(): PublicKey {
    const decoded = decodedAccountByName("redemption_offer", "redemption_offer");
    if (decoded?.kind === "redemption_offer") return decoded.value.tokenOutMint;
    const offer = decodedAccountByName("offer", "offer");
    if (offer?.kind === "offer") return offer.value.tokenOutMint;
    return instructionDefaultsToRedemptionDirection() ? MAINNET_MINTS.usdc : onycMint();
}

function defaultGenericMint(): PublicKey {
    return publicKeyFromAccountValue("token_mint") ?? publicKeyFromAccountValue("mint") ?? publicKeyFromAccountValue("token_in_mint") ?? onycMint();
}

function onycMint(): PublicKey {
    return state.stateInfo?.onycMint ?? MAINNET_MINTS.onyc;
}

function instructionDefaultsToRedemptionDirection(): boolean {
    return ["make_redemption_offer", "create_redemption_request", "cancel_redemption_request", "fulfill_redemption_request", "open_swap_sell", "quote_swap_sell"].includes(
        selectedInstruction().name,
    );
}

function deriveFixedPda(lowerName: string): PublicKey | undefined {
    const configurableSeed = CONFIGURABLE_VAULT_ACCOUNT_SEEDS[lowerName];
    if (configurableSeed) return deriveConfigurableVaultPda(configurableSeed);
    const seed = PDA_SEEDS[lowerName];
    return seed ? findPda([seed]) : undefined;
}

function deriveConfigurableVaultPda(kindSeed: string): PublicKey {
    return findPda(["configurable_vault", kindSeed]);
}

function deriveOfferPda(): PublicKey | undefined {
    const redemptionOffer = decodedAccountByName("redemption_offer", "redemption_offer");
    if (!offerHasInstructionMints() && redemptionOffer?.kind === "redemption_offer") return redemptionOffer.value.offer;

    const [tokenInMint, tokenOutMint] = offerSeedMints();
    if (tokenInMint && tokenOutMint) return findPda(["offer", tokenInMint, tokenOutMint]);
    if (state.stateInfo?.mainOffer && selectedInstruction().name !== "set_main_offer") return state.stateInfo.mainOffer;
    return undefined;
}

function deriveRedemptionOfferPda(): PublicKey | undefined {
    const [tokenInMint, tokenOutMint] = redemptionOfferSeedMints();
    return tokenInMint && tokenOutMint ? findPda(["redemption_offer", tokenInMint, tokenOutMint]) : undefined;
}

function deriveRedemptionRequestPda(): PublicKey | undefined {
    const request = decodedAccountByName("redemption_request", "redemption_request");
    if (request?.kind === "redemption_request") {
        return findPda(["redemption_request", request.value.offer, request.value.redeemer, Buffer.from(request.value.requestId, "utf8")]);
    }

    const redemptionOfferAddress = publicKeyFromAccountValue("redemption_offer") ?? deriveRedemptionOfferPda();
    const requestId = selectedInstruction().name === "create_redemption_request"
        ? state.argValues.request_id?.trim()
        : state.derivationValues[REDEMPTION_REQUEST_ID_KEY]?.trim();
    const redeemer = publicKeyFromAccountValue("redeemer");
    return redemptionOfferAddress && requestId && redeemer && isExactRedemptionRequestId(requestId)
        ? findPda(["redemption_request", redemptionOfferAddress, redeemer, Buffer.from(requestId, "utf8")])
        : undefined;
}

function isExactRedemptionRequestId(requestId: string): boolean {
    return Buffer.byteLength(requestId, "utf8") === 32;
}

function requireExactRedemptionRequestId(requestId: string | undefined): void {
    if ((selectedInstruction().name === "create_redemption_request" || state.accountAuto.redemption_request)
        && !isExactRedemptionRequestId(requestId?.trim() ?? "")) {
        throw new Error("Redemption request ID must be exactly 32 UTF-8 bytes (UUID without hyphens).");
    }
}

function offerSeedMints(): [PublicKey | undefined, PublicKey | undefined] {
    const instructionName = selectedInstruction().name;
    const tokenInMint = publicKeyFromAccountValue("token_in_mint");
    const tokenOutMint = publicKeyFromAccountValue("token_out_mint");
    if (!tokenInMint && !tokenOutMint) return offerDerivationMints();
    if (["make_redemption_offer", "fulfill_redemption_request", "open_swap_sell", "quote_swap_sell"].includes(instructionName)) {
        return [tokenOutMint ?? MAINNET_MINTS.usdc, tokenInMint ?? onycMint()];
    }
    return [tokenInMint, tokenOutMint];
}

function offerHasInstructionMints(): boolean {
    return Boolean(publicKeyFromAccountValue("token_in_mint") || publicKeyFromAccountValue("token_out_mint"));
}

function offerDerivationMints(): [PublicKey | undefined, PublicKey | undefined] {
    return [derivationMintValue(OFFER_TOKEN_IN_KEY, MAINNET_MINTS.usdc), derivationMintValue(OFFER_TOKEN_OUT_KEY, onycMint())];
}

function derivationMintValue(key: string, fallback: PublicKey): PublicKey | undefined {
    const value = state.derivationValues[key];
    if (value === "") return undefined;
    if (!value) return fallback;
    try {
        return new PublicKey(value);
    } catch {
        return undefined;
    }
}

function redemptionOfferSeedMints(): [PublicKey | undefined, PublicKey | undefined] {
    const instructionName = selectedInstruction().name;
    const tokenInMint = publicKeyFromAccountValue("token_in_mint");
    const tokenOutMint = publicKeyFromAccountValue("token_out_mint");
    if (["open_swap_buy", "take_offer_v2", "take_offer_permissionless_v2"].includes(instructionName)) {
        return [tokenOutMint ?? onycMint(), tokenInMint ?? redemptionOfferTokenOutMint()];
    }
    return [tokenInMint ?? onycMint(), tokenOutMint ?? redemptionOfferTokenOutMint()];
}

function redemptionOfferTokenOutMint(): PublicKey | undefined {
    const explicit = publicKeyFromAccountValue("token_out_mint") ?? publicKeyFromAccountValue("asset_mint");
    if (explicit && !explicit.equals(onycMint())) return explicit;

    const value = state.derivationValues[REDEMPTION_OFFER_TOKEN_OUT_KEY];
    if (value === "") return undefined;
    if (!value) return MAINNET_MINTS.usdc;
    try {
        return new PublicKey(value);
    } catch {
        return undefined;
    }
}

function isTokenAccountName(lowerName: string): boolean {
    return lowerName.endsWith("_account") || lowerName.endsWith("_token_account");
}

function publicKeyFromAccountValue(name: string): PublicKey | undefined {
    const value = accountValueByName(name);
    if (!value) return undefined;
    try {
        return new PublicKey(value);
    } catch {
        return undefined;
    }
}

function getAssociatedTokenAddress(mint: PublicKey, owner: PublicKey): PublicKey {
    return PublicKey.findProgramAddressSync([owner.toBuffer(), TOKEN_PROGRAM_ID.toBuffer(), mint.toBuffer()], ASSOCIATED_TOKEN_PROGRAM_ID)[0];
}

function findPda(seeds: Array<string | PublicKey | Uint8Array | Buffer>): PublicKey {
    return PublicKey.findProgramAddressSync(
        seeds.map((seed) => {
            if (typeof seed === "string") return Buffer.from(seed);
            if (seed instanceof PublicKey) return seed.toBuffer();
            return seed;
        }),
        state.programId,
    )[0];
}

function isTokenMintAccount(name: string): boolean {
    const lowerName = name.toLowerCase();
    return lowerName === "mint" || lowerName === "asset_mint" || lowerName === "onyc_mint" || lowerName.endsWith("_mint");
}

function tokenLabel(value: string): string | undefined {
    return TOKEN_CHOICES.find((token) => token.value.toBase58() === value)?.label;
}

// IDL PDA seed resolution.
function tryDerivePda(pda: IdlPda): PublicKey | undefined {
    try {
        const seeds = pda.seeds.map(resolveSeed);
        if (seeds.some((seed) => !seed)) return undefined;
        const programId = pda.program ? seedToProgramId(pda.program) : state.programId;
        if (!programId) return undefined;
        return PublicKey.findProgramAddressSync(seeds as Uint8Array[], programId)[0];
    } catch {
        return undefined;
    }
}

function resolveSeed(seed: IdlSeed): Uint8Array | undefined {
    if (seed.kind === "const") return Uint8Array.from(seed.value);
    if (seed.kind === "account") {
        if (seed.path.includes(".")) return resolveAccountFieldSeed(seed.path);
        const value = accountValueByName(seed.path);
        return value ? new PublicKey(value).toBuffer() : undefined;
    }
    if (seed.kind === "arg") {
        const arg = selectedInstruction().args?.find((candidate) => candidate.name === seed.path);
        if (!arg) return undefined;
        const configurableVaultSeed = configurableVaultKindSeed(arg, state.argValues[arg.name]);
        if (configurableVaultSeed) return Buffer.from(configurableVaultSeed);
        return encodeType(arg.type, parseArgValue(arg));
    }
    return undefined;
}

function resolveAccountFieldSeed(path: string): Uint8Array | undefined {
    const [accountName, fieldName] = path.split(".");
    if (!accountName || !fieldName) return undefined;

    if (accountName === "state" && state.stateInfo) {
        return encodeDecodedField(
            {
                boss: state.stateInfo.boss,
                proposed_boss: state.stateInfo.proposedBoss,
                onyc_mint: state.stateInfo.onycMint,
                worker: state.stateInfo.worker,
                main_offer: state.stateInfo.mainOffer,
            },
            fieldName,
        );
    }

    const decoded = decodedAccountByName(accountName, accountName);
    if (!decoded) return undefined;
    if (decoded.kind === "offer") {
        return encodeDecodedField(
            {
                token_in_mint: decoded.value.tokenInMint,
                token_out_mint: decoded.value.tokenOutMint,
            },
            fieldName,
        );
    }
    if (decoded.kind === "redemption_offer") {
        return encodeDecodedField(
            {
                offer: decoded.value.offer,
                token_in_mint: decoded.value.tokenInMint,
                token_out_mint: decoded.value.tokenOutMint,
            },
            fieldName,
        );
    }
    if (decoded.kind === "redemption_request") {
        return encodeDecodedField(
            {
                offer: decoded.value.offer,
                request_id: decoded.value.requestId,
                redeemer: decoded.value.redeemer,
            },
            fieldName,
        );
    }
    if (decoded.kind === "configurable_vault") {
        return encodeDecodedField(
            {
                kind: BigInt(decoded.value.kind),
                withdrawal_destination: decoded.value.withdrawalDestination,
            },
            fieldName,
        );
    }
    return undefined;
}

function encodeDecodedField(fields: Record<string, PublicKey | bigint | string>, fieldName: string): Uint8Array | undefined {
    const value = fields[fieldName];
    if (value instanceof PublicKey) return value.toBuffer();
    if (typeof value === "bigint") return u64Seed(value);
    if (typeof value === "string") return Buffer.from(value, "utf8");
    return undefined;
}

function configurableVaultKindSeed(arg: IdlArg, value: string): string | undefined {
    if (!isDefinedType(arg.type, "ConfigurableVaultKind")) return undefined;
    return CONFIGURABLE_VAULT_KIND_SEEDS[value];
}

function seedToProgramId(seed: IdlSeed): PublicKey | undefined {
    if (seed.kind === "const") return new PublicKey(Uint8Array.from(seed.value));
    const resolved = resolveSeed(seed);
    return resolved ? new PublicKey(resolved) : undefined;
}

function accountValueByName(name: string): string | undefined {
    const direct = state.accountValues[name];
    if (direct) return direct;

    const suffix = `.${name}`;
    for (const [key, value] of Object.entries(state.accountValues)) {
        if (key.endsWith(suffix) || key === name) return value;
    }
    return undefined;
}

// Account list flattening and on-chain account decoding refresh.
function flattenAccounts(accounts: IdlAccount[], group?: string): FlatAccount[] {
    const flat: FlatAccount[] = [];
    for (const account of accounts) {
        if (account.accounts) {
            flat.push(...flattenAccounts(account.accounts, account.name));
        } else {
            flat.push({
                account,
                fullName: group ? `${group}.${account.name}` : account.name,
                group,
            });
        }
    }
    return flat;
}

async function refreshDecodedDerivedAccounts(): Promise<void> {
    const targets = decodedFetchTargets();
    const existenceTargets = currentAccountPublicKeys();
    const existenceFetched = await Promise.all(
        existenceTargets.map(async (publicKey) => {
            const key = publicKey.toBase58();
            if (state.accountExistence[key] || accountFetchesInFlight.has(`exists:${key}`)) return false;
            accountFetchesInFlight.add(`exists:${key}`);
            try {
                const accountInfo = await state.connection.getAccountInfo(publicKey, "confirmed");
                state.accountExistence[key] = accountInfo ? "exists" : "missing";
                return true;
            } catch (error) {
                console.warn(`Account existence warning ${key}: ${errorMessage(error)}`);
                return false;
            } finally {
                accountFetchesInFlight.delete(`exists:${key}`);
            }
        }),
    );

    const decodedFetched = await Promise.all(
        targets.map(async (target) => {
            const key = target.publicKey.toBase58();
            if (state.decodedAccounts[key] || accountFetchesInFlight.has(`decode:${key}`)) return false;
            accountFetchesInFlight.add(`decode:${key}`);
            try {
                const accountInfo = await state.connection.getAccountInfo(target.publicKey, "confirmed");
                if (!accountInfo) return false;
                const decoded = decodeKnownAccount(target.kind, accountInfo.data);
                if (!decoded) return false;
                state.decodedAccounts[key] = decoded;
                return true;
            } catch (error) {
                console.warn(`Account fetch warning ${key}: ${errorMessage(error)}`);
                return false;
            } finally {
                accountFetchesInFlight.delete(`decode:${key}`);
            }
        }),
    );

    if (existenceFetched.some(Boolean) || decodedFetched.some(Boolean)) {
        deriveAccounts();
        render();
    }
}

function currentAccountPublicKeys(): PublicKey[] {
    const keys = new Map<string, PublicKey>();
    for (const value of Object.values(state.accountValues)) {
        if (!value) continue;
        try {
            const publicKey = new PublicKey(value);
            keys.set(publicKey.toBase58(), publicKey);
        } catch {
            // Ignore partial custom address input.
        }
    }
    return [...keys.values()];
}

function decodedFetchTargets(): Array<{ publicKey: PublicKey; kind: DecodedAccount["kind"] }> {
    const targets = new Map<string, { publicKey: PublicKey; kind: DecodedAccount["kind"] }>();
    const add = (publicKey: PublicKey | undefined, kind: DecodedAccount["kind"]) => {
        if (!publicKey) return;
        targets.set(`${kind}:${publicKey.toBase58()}`, { publicKey, kind });
    };

    add(state.stateInfo?.mainOffer, "offer");

    for (const flat of flattenAccounts(selectedInstruction().accounts ?? [])) {
        const lowerName = flat.account.name.toLowerCase();
        const publicKey = publicKeyFromAccountValue(flat.fullName) ?? publicKeyFromAccountValue(flat.account.name);
        if (lowerName === "offer" || lowerName === "main_offer") add(publicKey, "offer");
        if (lowerName === "redemption_offer") add(publicKey, "redemption_offer");
        if (lowerName === "redemption_request") add(publicKey, "redemption_request");
        if (lowerName === "configurable_vault") add(publicKey, "configurable_vault");
    }

    return [...targets.values()];
}

function decodedAccountByName(name: string, fallbackName?: string): DecodedAccount | undefined {
    const publicKey = publicKeyFromAccountValue(name) ?? (fallbackName ? publicKeyFromAccountValue(fallbackName) : undefined);
    return decodedAccountByPublicKey(publicKey);
}

function decodedAccountByPublicKey(publicKey: PublicKey | undefined): DecodedAccount | undefined {
    return publicKey ? state.decodedAccounts[publicKey.toBase58()] : undefined;
}

// Page output and scroll behavior.
function appendOutput(message: string): void {
    const stamp = new Date().toLocaleTimeString();
    state.output = state.output ? `${state.output}\n\n[${stamp}] ${message}` : `[${stamp}] ${message}`;
}

function saveInstructionListScroll(): void {
    const list = document.querySelector<HTMLDivElement>(".instruction-list");
    if (list) {
        state.instructionListScrollTop = list.scrollTop;
    }
}

function restoreInstructionListScroll(): void {
    for (const timer of instructionListRestoreTimers) {
        globalThis.clearTimeout(timer);
    }
    instructionListRestoreTimers = [];

    const scrollTop = pendingInstructionListScrollTop ?? state.instructionListScrollTop;
    const restore = () => {
        const list = document.querySelector<HTMLDivElement>(".instruction-list");
        if (list) {
            isRestoringInstructionListScroll = true;
            list.scrollTop = scrollTop;
            window.setTimeout(() => {
                isRestoringInstructionListScroll = false;
                state.instructionListScrollTop = list.scrollTop;
            }, 0);
        }
    };
    requestAnimationFrame(restore);
    instructionListRestoreTimers.push(window.setTimeout(restore, 0), window.setTimeout(restore, 50), window.setTimeout(restore, 150));
    pendingInstructionListScrollTop = undefined;
}

function scrollOutputToBottom(): void {
    requestAnimationFrame(() => {
        const output = document.querySelector<HTMLPreElement>("#output-text");
        if (output) {
            output.scrollTop = output.scrollHeight;
        }
    });
}

boot();
