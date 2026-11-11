#!/usr/bin/env npx tsx

// Parse network argument early before any imports that use config
// This must happen before the dynamic import below
const args = process.argv.slice(2);
const networkArgIndex = args.findIndex((arg) => arg === "-n" || arg === "--network");
if (networkArgIndex !== -1 && args[networkArgIndex + 1]) {
    process.env.NETWORK = args[networkArgIndex + 1];
}

// Dynamic import to ensure NETWORK env is set before config loads
async function main() {
    const { Command } = await import("commander");
    const chalk = (await import("chalk")).default;

    // Import command registrations (after network is set)
    const {
        registerStateCommands,
        registerMarketCommands,
        registerOfferCommands,
        registerVaultCommands,
        registerMintAuthorityCommands,
        registerRedemptionCommands,
        registerInitCommands,
        registerProgramCommands,
        registerBufferCommands,
        registerPropAmmCommands,
    } = await import("./commands/index.js");

    // Create the main program
    const program = new Command();

    program
        .name("pnpm cli")
        .description("CLI tool for managing OnRE tokenized (re)insurance pool")
        .version("1.0.0")
        .option("-n, --network <network>", "Network to use (mainnet-prod, mainnet-test, mainnet-dev, devnet-test, devnet-dev)")
        .option("-w, --wallet <path>", "Wallet keypair path or ~/.config/solana/<name>.json")
        .option("--json", "Output in JSON format")
        .option("--dry-run", "Generate transaction without execution prompt")
        .option("--no-interactive", "Disable interactive prompts (require all params as flags)")
        .option("--yes", "Sign and send locally without confirmation prompts");

    // Register command groups
    const stateCmd = program.command("state").description("Manage program state (boss, admins, approvers, kill switch)");
    registerStateCommands(stateCmd);

    const marketCmd = program.command("market").description("Read or refresh cached market stats");
    registerMarketCommands(marketCmd);

    const offerCmd = program.command("offer").description("Manage token offers (create, fetch, update, close)");
    registerOfferCommands(offerCmd);

    const vaultCmd = program.command("vault").description("Vault operations (deposit, withdraw)");
    registerVaultCommands(vaultCmd);

    const mintAuthorityCmd = program.command("mint-authority").description("Mint authority operations (transfer to/from program)");
    registerMintAuthorityCommands(mintAuthorityCmd);

    const redemptionCmd = program.command("redemption").description("Redemption operations (create redemption offers)");
    registerRedemptionCommands(redemptionCmd);

    const initCmd = program.command("init").description("Initialize program and authorities");
    registerInitCommands(initCmd);

    const bufferCmd = program.command("buffer").description("BUFFER pool operations");
    registerBufferCommands(bufferCmd);

    const propAmmCmd = program.command("prop-amm").description("Prop AMM configuration, quotes, and swaps");
    registerPropAmmCommands(propAmmCmd);

    const programCmd = program.command("program").description("Program management (extend data account)");
    registerProgramCommands(programCmd);

    // Add help examples
    program.addHelpText(
        "after",
        `
${chalk.bold("Examples:")}
  ${chalk.gray("# Get program state")}
  $ pnpm cli state get

  ${chalk.gray("# Get NAV with JSON output")}
  $ pnpm cli market fetch --json

  ${chalk.gray("# Create an offer on testnet")}
  $ pnpm cli -n mainnet-test offer make

  ${chalk.gray("# Add a pricing vector")}
  $ pnpm cli offer add-vector --token-in usdc --token-out onyc

  ${chalk.gray("# Fetch offer details")}
  $ pnpm cli offer fetch -i usdc -o onyc

${chalk.bold("Networks:")}
  mainnet-prod   Production mainnet with real tokens (default)
  mainnet-test   Mainnet with test tokens
  mainnet-dev    Mainnet development
  devnet-test    Devnet with test tokens
  devnet-dev     Devnet development
`,
    );

    // Parse and execute
    program.parse(process.argv);

    // Show help if no command provided
    if (!process.argv.slice(2).length) {
        program.outputHelp();
    }
}

main().catch(console.error);
