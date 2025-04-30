// getBoss.ts
import { getBossAccount, initProgram } from "./script-commons";

async function main() {
  try {
    const program = await initProgram();
    const BOSS = await getBossAccount(program);

    console.log(BOSS);
  } catch (error) {
    console.error("Failed to create set boss transaction:", error);
  }
}

await main();
