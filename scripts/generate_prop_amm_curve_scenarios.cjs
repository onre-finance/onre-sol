const fs = require("fs");
const path = require("path");
const { spawnSync } = require("child_process");

const repoRoot = process.cwd();
const outDir = path.join(repoRoot, "target", "prop_amm_curve_scenarios");

const scenarios = [
  { linearWeightBps: 100, baseExponent: 2 },
  { linearWeightBps: 100, baseExponent: 3 },
  { linearWeightBps: 100, baseExponent: 5 },
  { linearWeightBps: 500, baseExponent: 2 },
  { linearWeightBps: 500, baseExponent: 3 },
  { linearWeightBps: 500, baseExponent: 5 },
  { linearWeightBps: 2000, baseExponent: 2 },
  { linearWeightBps: 2000, baseExponent: 3 },
  { linearWeightBps: 2000, baseExponent: 5 },
  { linearWeightBps: 4000, baseExponent: 2 },
  { linearWeightBps: 4000, baseExponent: 3 },
  { linearWeightBps: 4000, baseExponent: 5 },
];

const config = {
  liquidity: 10_000_000,
  hardWallReserve: 10_000_000,
  points: 201,
  redemptionFeeBps: 500,
};

fs.mkdirSync(outDir, { recursive: true });

for (const scenario of scenarios) {
  const slug = `lw${String(scenario.linearWeightBps).padStart(4, "0")}_exp${scenario.baseExponent}`;
  const csvPath = path.join(outDir, `${slug}.csv`);
  const svgPath = path.join(outDir, `${slug}.svg`);

  const run = spawnSync(
    "cargo",
    [
      "run",
      "-p",
      "onreapp",
      "--example",
      "hard_wall_curve",
      "--",
      "--out",
      csvPath,
      "--liquidity",
      String(config.liquidity),
      "--hard-wall-reserve",
      String(config.hardWallReserve),
      "--points",
      String(config.points),
      "--linear-weight-bps",
      String(scenario.linearWeightBps),
      "--base-exponent",
      String(scenario.baseExponent),
      "--redemption-fee-bps",
      String(config.redemptionFeeBps),
    ],
    {
      cwd: repoRoot,
      stdio: "inherit",
    },
  );

  if (run.status !== 0) {
    process.exit(run.status ?? 1);
  }

  renderSvg(csvPath, svgPath, scenario);
  console.log(`wrote ${path.relative(repoRoot, csvPath)}`);
  console.log(`wrote ${path.relative(repoRoot, svgPath)}`);
}

function renderSvg(csvPath, svgPath, scenario) {
  const csv = fs.readFileSync(csvPath, "utf8").trim().split(/\r?\n/);
  const rows = csv.slice(1).map((line) => {
    const [
      ,
      utilPct,
      actualLiquidity,
      hardWallReserve,
      reserveFullnessPct,
      rawOut,
      netOutAfterFee,
      dampenedOut,
      liquidityFactorBps,
      effectivePricePct,
    ] = line.split(",");

    return {
      x: Number(utilPct),
      yRed: Number(effectivePricePct),
      yBlue: Number(liquidityFactorBps) / 100,
      yGray: (Number(rawOut) / Number(hardWallReserve)) * 100,
      yGreen: (Number(dampenedOut) / Number(hardWallReserve)) * 100,
      reserveFullnessPct: Number(reserveFullnessPct),
      actualLiquidity: Number(actualLiquidity),
      hardWallReserve: Number(hardWallReserve),
      netOutAfterFee: Number(netOutAfterFee),
    };
  });

  const width = 1100;
  const height = 680;
  const left = 80;
  const right = 40;
  const top = 45;
  const bottom = 75;
  const plotWidth = width - left - right;
  const plotHeight = height - top - bottom;
  const xMin = 0;
  const xMax = 99;
  const yMin = 0;
  const yMax = 100;

  const sx = (x) => left + ((x - xMin) / (xMax - xMin)) * plotWidth;
  const sy = (y) =>
    top + plotHeight - ((Math.max(yMin, Math.min(yMax, y)) - yMin) / (yMax - yMin)) * plotHeight;
  const pathFor = (key) =>
    rows
      .map((row, index) => `${index === 0 ? "M" : "L"}${sx(row.x).toFixed(2)} ${sy(row[key]).toFixed(2)}`)
      .join(" ");

  const fullness = rows[0]?.reserveFullnessPct.toFixed(2) ?? "?";
  const subtitle =
    `X: raw sell size as % of hard-wall reserve. Fullness ${fullness}%. ` +
    `linear_weight_bps=${scenario.linearWeightBps}, base_exponent=${scenario.baseExponent}.`;

  let svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">`;
  svg += `<style>.bg{fill:#fbfdff}.grid{stroke:#d8e0e8;stroke-width:1}.axis{stroke:#4a5568;stroke-width:1.5}.small{font:14px sans-serif;fill:#334155}.legend{font:15px sans-serif;fill:#1f2937}</style>`;
  svg += `<rect class="bg" width="${width}" height="${height}"/>`;
  svg += `<text x="80" y="30" class="legend">Prop AMM Hard-Wall Curve</text>`;
  svg += `<text x="80" y="50" class="small">${subtitle}</text>`;

  for (const tick of [0, 25, 50, 75, 100]) {
    svg += `<line class="grid" x1="${left}" y1="${sy(tick)}" x2="${width - right}" y2="${sy(tick)}"/>`;
    svg += `<text class="small" x="28" y="${sy(tick) + 4}">${tick}%</text>`;
  }

  for (const tick of [0, 25, 50, 75, 99]) {
    svg += `<line class="grid" x1="${sx(tick)}" y1="${top}" x2="${sx(tick)}" y2="${height - bottom}"/>`;
    svg += `<text class="small" x="${sx(tick) - 12}" y="${height - bottom + 27}">${tick}%</text>`;
  }

  svg += `<line class="axis" x1="${left}" y1="${height - bottom}" x2="${width - right}" y2="${height - bottom}"/>`;
  svg += `<line class="axis" x1="${left}" y1="${top}" x2="${left}" y2="${height - bottom}"/>`;
  svg += `<path d="${pathFor("yRed")}" fill="none" stroke="#d23f31" stroke-width="3"/>`;
  svg += `<path d="${pathFor("yBlue")}" fill="none" stroke="#2b8cbe" stroke-width="2" stroke-dasharray="7 5"/>`;
  svg += `<path d="${pathFor("yGray")}" fill="none" stroke="#8795a1" stroke-width="2"/>`;
  svg += `<path d="${pathFor("yGreen")}" fill="none" stroke="#2ca25f" stroke-width="2"/>`;
  svg += `<text class="small" x="420" y="666">raw sell size / hard-wall reserve</text>`;
  svg += `<text class="small" transform="translate(18 430) rotate(-90)">effective payout % / output scale</text>`;
  svg += `<rect x="690" y="58" width="362" height="114" rx="10" fill="#ffffff" stroke="#d8e0e8"/>`;
  svg += `<line x1="708" y1="86" x2="743" y2="86" stroke="#d23f31" stroke-width="3"/><text class="legend" x="753" y="91">effective payout %</text>`;
  svg += `<line x1="708" y1="108" x2="743" y2="108" stroke="#2b8cbe" stroke-width="2" stroke-dasharray="7 5"/><text class="legend" x="753" y="113">dampened / fee-adjusted raw</text>`;
  svg += `<line x1="708" y1="130" x2="743" y2="130" stroke="#8795a1" stroke-width="2"/><text class="legend" x="753" y="135">raw output</text>`;
  svg += `<line x1="708" y1="152" x2="743" y2="152" stroke="#2ca25f" stroke-width="2"/><text class="legend" x="753" y="157">actual payout</text>`;
  svg += `</svg>`;

  fs.writeFileSync(svgPath, svg);
}
