#!/usr/bin/env npx tsx

// Parse network argument early before any imports that use config
// This must happen before the dynamic import below
const args = process.argv.slice(2);
const networkArgIndex = args.findIndex(arg => arg === "-n" || arg === "--network");
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
        registerInitCommands
    } = await import("./commands/index.js");

    // Create the main program
    const program = new Command();

    program
        .name("npm run cli --")
        .description("CLI tool for managing OnRE tokenized (re)insurance pool")
        .version("1.0.0")
        .option("-n, --network <network>", "Network to use (mainnet-prod, mainnet-test, mainnet-dev, devnet-test, devnet-dev)")
        .option("--json", "Output in JSON format")
        .option("--dry-run", "Generate transaction without execution prompt")
        .option("--no-interactive", "Disable interactive prompts (require all params as flags)");

    // Register command groups
    const stateCmd = program
        .command("state")
        .description("Manage program state (boss, admins, approvers, kill switch)");
    registerStateCommands(stateCmd);

    const marketCmd = program
        .command("market")
        .description("Query market information (NAV, APY, TVL, supply)");
    registerMarketCommands(marketCmd);

    const offerCmd = program
        .command("offer")
        .description("Manage token offers (create, fetch, update, close)");
    registerOfferCommands(offerCmd);

    const vaultCmd = program
        .command("vault")
        .description("Vault operations (deposit, withdraw)");
    registerVaultCommands(vaultCmd);

    const mintAuthorityCmd = program
        .command("mint-authority")
        .description("Mint authority operations (transfer to/from program)");
    registerMintAuthorityCommands(mintAuthorityCmd);

    const redemptionCmd = program
        .command("redemption")
        .description("Redemption operations (create redemption offers)");
    registerRedemptionCommands(redemptionCmd);

    const initCmd = program
        .command("init")
        .description("Initialize program and authorities");
    registerInitCommands(initCmd);

    // Add help examples
    program.addHelpText("after", `
${chalk.bold("Examples:")}
  ${chalk.gray("# Get program state")}
  $ npm run cli -- state get

  ${chalk.gray("# Get NAV with JSON output")}
  $ npm run cli -- market nav --json

  ${chalk.gray("# Create an offer on testnet")}
  $ npm run cli -- -n mainnet-test offer make

  ${chalk.gray("# Add a pricing vector")}
  $ npm run cli -- offer add-vector --token-in usdc --token-out onyc

  ${chalk.gray("# Fetch offer details")}
  $ npm run cli -- offer fetch -i usdc -o onyc

${chalk.bold("Networks:")}
  mainnet-prod   Production mainnet with real tokens (default)
  mainnet-test   Mainnet with test tokens
  mainnet-dev    Mainnet development
  devnet-test    Devnet with test tokens
  devnet-dev     Devnet development
`);

    // Parse and execute
    program.parse(process.argv);

    // Show help if no command provided
    if (!process.argv.slice(2).length) {
        program.outputHelp();
    }
}

main().catch(console.error);
