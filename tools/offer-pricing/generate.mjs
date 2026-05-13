#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const INT_SCALE = 1_000_000_000_000_000_000n;
const PRICE_SCALE = 1_000_000_000n;
const APR_SCALE = 1_000_000n;
const SECONDS_IN_DAY = 86_400n;

function parseArgs(argv) {
  const options = {
    apr: 36_500n,
    basePrice: PRICE_SCALE,
    priceFixDuration: 3_600n,
    days: 10n,
    startDate: "2026-05-02",
    out: "output.html",
  };

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    const next = argv[i + 1];

    if (arg === "--apr") {
      options.apr = BigInt(next);
      i += 1;
    } else if (arg === "--base-price") {
      options.basePrice = BigInt(next);
      i += 1;
    } else if (arg === "--price-fix-duration") {
      options.priceFixDuration = BigInt(next);
      i += 1;
    } else if (arg === "--days") {
      options.days = BigInt(next);
      i += 1;
    } else if (arg === "--start-date") {
      options.startDate = next;
      i += 1;
    } else if (arg === "--out") {
      options.out = next;
      i += 1;
    } else if (arg === "--help" || arg === "-h") {
      printHelp();
      process.exit(0);
    }
  }

  if (options.priceFixDuration <= 0n) {
    throw new Error("--price-fix-duration must be greater than 0");
  }
  if (options.days <= 0n) {
    throw new Error("--days must be greater than 0");
  }

  return options;
}

function printHelp() {
  console.log(`Usage:
  node tools/offer-pricing/generate.mjs [options]

Options:
  --apr <u64>                  APR scaled like the program, 10_000 = 1%, 1_000_000 = 100%
  --base-price <u64>           Base price scaled by 1e9
  --price-fix-duration <sec>   Step duration in seconds
  --days <n>                   Number of days to render
  --start-date <YYYY-MM-DD>    Calendar date for x-axis/table labels
  --out <path>                 Output HTML path

Example:
  node tools/offer-pricing/generate.mjs --apr 36500 --price-fix-duration 3600 --days 14
`);
}

function divRound(numerator, denominator) {
  return (numerator + denominator / 2n) / denominator;
}

function mulDivRound(a, b, denominator) {
  return divRound(a * b, denominator);
}

function powFixed(base, exponent, scale) {
  let acc = scale;
  let value = base;
  let exp = exponent;

  while (exp > 0n) {
    if ((exp & 1n) === 1n) {
      acc = mulDivRound(acc, value, scale);
    }
    exp >>= 1n;
    if (exp > 0n) {
      value = mulDivRound(value, value, scale);
    }
  }

  return acc;
}

function calculateVectorPrice(apr, basePrice, elapsedTime) {
  if (apr === 0n || elapsedTime === 0n) {
    return basePrice;
  }

  const dailyIncrement = divRound(INT_SCALE * apr, APR_SCALE * 365n);
  const dailyFactor = INT_SCALE + dailyIncrement;
  const fullDays = elapsedTime / SECONDS_IN_DAY;
  const remainingSeconds = elapsedTime % SECONDS_IN_DAY;

  const fullDayFactor = powFixed(dailyFactor, fullDays, INT_SCALE);
  const fullDayPrice = mulDivRound(basePrice, fullDayFactor, INT_SCALE);

  if (remainingSeconds === 0n) {
    return fullDayPrice;
  }

  const nextDayPrice = mulDivRound(fullDayPrice, dailyFactor, INT_SCALE);
  const dailyDelta = nextDayPrice - fullDayPrice;
  const partialDayDelta = mulDivRound(dailyDelta, remainingSeconds, SECONDS_IN_DAY);
  return fullDayPrice + partialDayDelta;
}

function calculateStepPrice(apr, basePrice, baseTime, priceFixDuration, time) {
  if (baseTime > time) {
    throw new Error("baseTime must be <= time");
  }

  const elapsedSinceStart = time - baseTime;
  const currentStep = elapsedSinceStart / priceFixDuration;
  const stepEndTime = (currentStep + 1n) * priceFixDuration;
  return calculateVectorPrice(apr, basePrice, stepEndTime);
}

function formatPrice(value) {
  const whole = value / PRICE_SCALE;
  const fraction = (value % PRICE_SCALE).toString().padStart(9, "0");
  return `${whole}.${fraction}`;
}

function toNumberPrice(value) {
  return Number(value) / Number(PRICE_SCALE);
}

function buildRows(options) {
  const totalSeconds = options.days * SECONDS_IN_DAY - 1n;
  const sampleStep = options.priceFixDuration;
  const rows = [];

  for (let t = 0n; t < totalSeconds; t += sampleStep) {
    const stepPrice = calculateStepPrice(
      options.apr,
      options.basePrice,
      0n,
      options.priceFixDuration,
      t,
    );
    const exactPrice = calculateVectorPrice(options.apr, options.basePrice, t);
    rows.push({ t, stepPrice, exactPrice });
  }
  if (rows.length === 0 || rows[rows.length - 1].t !== totalSeconds) {
    const stepPrice = calculateStepPrice(
      options.apr,
      options.basePrice,
      0n,
      options.priceFixDuration,
      totalSeconds,
    );
    const exactPrice = calculateVectorPrice(options.apr, options.basePrice, totalSeconds);
    rows.push({ t: totalSeconds, stepPrice, exactPrice });
  }

  return rows;
}

function scalePoints(rows, key, width, height, padding) {
  const xs = rows.map((row) => Number(row.t));
  const ys = rows.map((row) => toNumberPrice(row[key]));
  const minX = Math.min(...xs);
  const maxX = Math.max(...xs);
  const minY = Math.min(...ys);
  const maxY = Math.max(...ys);
  const yPad = Math.max((maxY - minY) * 0.08, 0.00001);
  const yMin = minY - yPad;
  const yMax = maxY + yPad;

  return rows
    .map((row) => {
      const xRatio = (Number(row.t) - minX) / Math.max(maxX - minX, 1);
      const yRatio = (toNumberPrice(row[key]) - yMin) / Math.max(yMax - yMin, 0.000000001);
      const x = padding + xRatio * (width - padding * 2);
      const y = height - padding - yRatio * (height - padding * 2);
      return `${x.toFixed(2)},${y.toFixed(2)}`;
    })
    .join(" ");
}

function renderHtml(options, rows) {
  const width = 1100;
  const height = 520;
  const padding = 54;
  const leftPadding = 120;
  const rightPadding = 54;

  return `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Offer Pricing Model</title>
    <style>
      body { margin: 0; font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; color: #15171c; background: #f6f7f9; }
      main { max-width: 1180px; margin: 0 auto; padding: 28px; }
      h1 { margin: 0 0 6px; font-size: 26px; }
      p { margin: 0; color: #596170; }
      .metrics { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 12px; margin: 22px 0; }
      .metric, .chart, .table, .controls { background: #fff; border: 1px solid #d9dde5; border-radius: 8px; }
      .metric { padding: 14px; }
      .metric span { display: block; color: #596170; font-size: 12px; margin-bottom: 6px; }
      .metric strong { font-size: 18px; font-variant-numeric: tabular-nums; }
      .controls { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 18px; margin: 22px 0; padding: 16px; }
      .control { display: grid; gap: 7px; min-width: 0; }
      .control label { display: flex; justify-content: space-between; gap: 12px; color: #596170; font-size: 12px; }
      .control output { color: #15171c; font-variant-numeric: tabular-nums; white-space: nowrap; }
      input[type="range"] { width: 100%; accent-color: #1f6feb; }
      input[type="number"] { width: 100%; border: 1px solid #cfd5df; border-radius: 6px; padding: 7px 9px; color: #15171c; font: inherit; font-variant-numeric: tabular-nums; }
      .chart { padding: 18px; overflow: auto; }
      svg { display: block; min-width: 820px; width: 100%; height: auto; }
      .axis { stroke: #c4cad4; stroke-width: 1; }
      .axis-label { fill: #596170; font-size: 13px; font-weight: 600; }
      .tick-label { fill: #717987; font-size: 11px; font-variant-numeric: tabular-nums; }
      .grid { stroke: #e9ecf1; stroke-width: 1; }
      .step { fill: none; stroke: #1f6feb; stroke-width: 2.5; }
      .smooth { fill: none; stroke: #17803d; stroke-width: 2; stroke-dasharray: 7 5; }
      .legend { display: flex; gap: 18px; margin-top: 12px; color: #596170; font-size: 13px; }
      .swatch { display: inline-block; width: 22px; height: 3px; margin-right: 7px; vertical-align: middle; background: #1f6feb; }
      .swatch.smooth { background: #17803d; border-top: 2px dashed #17803d; height: 0; }
      .table { margin-top: 18px; overflow: auto; max-height: 460px; }
      table { width: 100%; border-collapse: collapse; font-variant-numeric: tabular-nums; }
      th, td { padding: 9px 12px; border-bottom: 1px solid #edf0f4; text-align: right; }
      th:first-child, td:first-child { text-align: left; }
      th { color: #596170; font-size: 12px; background: #fafbfc; position: sticky; top: 0; }
      code { background: #edf0f4; padding: 2px 5px; border-radius: 4px; }
      input[type="date"] { width: 100%; border: 1px solid #cfd5df; border-radius: 6px; padding: 7px 9px; color: #15171c; font: inherit; }
      @media (max-width: 980px) { .controls { grid-template-columns: 1fr 1fr; } }
      @media (max-width: 800px) { main { padding: 18px; } .metrics, .controls { grid-template-columns: 1fr 1fr; } }
    </style>
  </head>
  <body>
    <main>
      <h1>Offer Pricing Model</h1>
      <p>Daily compounding with linear remainder interpolation, sampled at each price fix duration.</p>

      <div class="controls">
        <div class="control">
          <label for="apr"><span>APR</span><output id="apr-output"></output></label>
          <input id="apr" type="range" min="0" max="1000000" step="1000" value="${options.apr.toString()}" />
          <input id="apr-input" type="number" min="0" step="1" value="${options.apr.toString()}" />
        </div>
        <div class="control">
          <label for="base-price"><span>Base Price</span><output id="base-price-output"></output></label>
          <input id="base-price" type="range" min="500000000" max="2000000000" step="1000000" value="${options.basePrice.toString()}" />
          <input id="base-price-input" type="number" min="1" step="1" value="${options.basePrice.toString()}" />
        </div>
        <div class="control">
          <label for="fix-duration"><span>Fix Duration</span><output id="fix-duration-output"></output></label>
          <input id="fix-duration" type="range" min="300" max="86400" step="300" value="${options.priceFixDuration.toString()}" />
          <input id="fix-duration-input" type="number" min="1" step="1" value="${options.priceFixDuration.toString()}" />
        </div>
        <div class="control">
          <label for="days"><span>Rendered Days</span><output id="days-output"></output></label>
          <input id="days" type="range" min="1" max="3650" step="1" value="${options.days.toString()}" />
          <input id="days-input" type="number" min="1" step="1" value="${options.days.toString()}" />
        </div>
        <div class="control">
          <label for="start-date"><span>Starting Date</span><output id="start-date-output"></output></label>
          <input id="start-date" type="date" value="${options.startDate}" />
        </div>
      </div>

      <div class="metrics">
        <div class="metric"><span>APR Scale</span><strong id="metric-apr"></strong></div>
        <div class="metric"><span>Base Price</span><strong id="metric-base-price"></strong></div>
        <div class="metric"><span>Fix Duration</span><strong id="metric-fix-duration"></strong></div>
        <div class="metric"><span>Rendered Period</span><strong id="metric-days"></strong></div>
      </div>

      <div class="chart">
        <svg viewBox="0 0 ${width} ${height}" role="img" aria-label="Offer pricing chart">
          <g id="grid-lines"></g>
          <g id="tick-labels"></g>
          <line class="axis" x1="${leftPadding}" y1="${height - padding}" x2="${width - rightPadding}" y2="${height - padding}" />
          <line class="axis" x1="${leftPadding}" y1="${padding}" x2="${leftPadding}" y2="${height - padding}" />
          <text class="axis-label" x="${width / 2}" y="${height - 10}" text-anchor="middle">Elapsed time (days)</text>
          <text class="axis-label" transform="translate(18 ${height / 2}) rotate(-90)" text-anchor="middle">Price (scaled to decimal)</text>
          <polyline id="smooth-line" class="smooth" points="" />
          <path id="step-line" class="step" d=""></path>
        </svg>
        <div class="legend">
          <span><i class="swatch"></i>On-chain step price</span>
          <span><i class="swatch smooth"></i>Exact-time price before step snapping</span>
        </div>
      </div>

      <div class="metrics">
        <div class="metric"><span>Initial Step Price</span><strong id="metric-initial-step"></strong></div>
        <div class="metric"><span>Final Step Price</span><strong id="metric-final-step"></strong></div>
        <div class="metric"><span>Initial Exact-Time Price</span><strong id="metric-initial-exact"></strong></div>
        <div class="metric"><span>Final Exact-Time Price</span><strong id="metric-final-exact"></strong></div>
      </div>

      <div class="table">
        <table>
          <thead><tr><th>End Date</th><th>On-Chain Step Price</th><th>Exact-Time Price</th></tr></thead>
          <tbody id="daily-price-rows"></tbody>
        </table>
      </div>

    </main>
    <script>
      const INT_SCALE = 1000000000000000000n;
      const PRICE_SCALE = 1000000000n;
      const APR_SCALE = 1000000n;
      const SECONDS_IN_DAY = 86400n;
      const width = ${width};
      const height = ${height};
      const padding = ${padding};
      const leftPadding = ${leftPadding};
      const rightPadding = ${rightPadding};

      const controls = {
        apr: document.getElementById("apr"),
        basePrice: document.getElementById("base-price"),
        priceFixDuration: document.getElementById("fix-duration"),
        days: document.getElementById("days"),
      };
      const inputs = {
        apr: document.getElementById("apr-input"),
        basePrice: document.getElementById("base-price-input"),
        priceFixDuration: document.getElementById("fix-duration-input"),
        days: document.getElementById("days-input"),
      };
      const startDateInput = document.getElementById("start-date");

      function divRound(numerator, denominator) {
        return (numerator + denominator / 2n) / denominator;
      }

      function mulDivRound(a, b, denominator) {
        return divRound(a * b, denominator);
      }

      function powFixed(base, exponent, scale) {
        let acc = scale;
        let value = base;
        let exp = exponent;
        while (exp > 0n) {
          if ((exp & 1n) === 1n) acc = mulDivRound(acc, value, scale);
          exp >>= 1n;
          if (exp > 0n) value = mulDivRound(value, value, scale);
        }
        return acc;
      }

      function calculateVectorPrice(apr, basePrice, elapsedTime) {
        if (apr === 0n || elapsedTime === 0n) return basePrice;
        const dailyIncrement = divRound(INT_SCALE * apr, APR_SCALE * 365n);
        const dailyFactor = INT_SCALE + dailyIncrement;
        const fullDays = elapsedTime / SECONDS_IN_DAY;
        const remainingSeconds = elapsedTime % SECONDS_IN_DAY;
        const fullDayFactor = powFixed(dailyFactor, fullDays, INT_SCALE);
        const fullDayPrice = mulDivRound(basePrice, fullDayFactor, INT_SCALE);
        if (remainingSeconds === 0n) return fullDayPrice;
        const nextDayPrice = mulDivRound(fullDayPrice, dailyFactor, INT_SCALE);
        const dailyDelta = nextDayPrice - fullDayPrice;
        const partialDayDelta = mulDivRound(dailyDelta, remainingSeconds, SECONDS_IN_DAY);
        return fullDayPrice + partialDayDelta;
      }

      function calculateStepPrice(apr, basePrice, baseTime, priceFixDuration, time) {
        if (baseTime > time) throw new Error("baseTime must be <= time");
        const elapsedSinceStart = time - baseTime;
        const currentStep = elapsedSinceStart / priceFixDuration;
        const stepEndTime = (currentStep + 1n) * priceFixDuration;
        return calculateVectorPrice(apr, basePrice, stepEndTime);
      }

      function formatPrice(value) {
        const whole = value / PRICE_SCALE;
        const fraction = (value % PRICE_SCALE).toString().padStart(9, "0");
        return whole.toString() + "." + fraction;
      }

      function toNumberPrice(value) {
        return Number(value) / Number(PRICE_SCALE);
      }

      function formatApr(apr) {
        return (Number(apr) / 10000).toFixed(3) + "%";
      }

      function formatDuration(seconds) {
        if (seconds % 86400 === 0) return (seconds / 86400) + "d";
        if (seconds % 3600 === 0) return (seconds / 3600) + "h";
        if (seconds % 60 === 0) return (seconds / 60) + "m";
        return seconds + "s";
      }

      function parseStartDate(value) {
        const fallback = "2026-05-02";
        return new Date((value || fallback) + "T00:00:00Z");
      }

      function formatDate(startDate, elapsedSeconds) {
        return new Date(startDate.getTime() + Number(elapsedSeconds) * 1000).toISOString().slice(0, 10);
      }

      function buildRows(options) {
        const totalSeconds = options.days * SECONDS_IN_DAY - 1n;
        const rows = [];
        for (let t = 0n; t < totalSeconds; t += options.priceFixDuration) {
          rows.push({
            t,
            stepPrice: calculateStepPrice(options.apr, options.basePrice, 0n, options.priceFixDuration, t),
            exactPrice: calculateVectorPrice(options.apr, options.basePrice, t),
          });
        }
        if (rows.length === 0 || rows[rows.length - 1].t !== totalSeconds) {
          rows.push({
            t: totalSeconds,
            stepPrice: calculateStepPrice(options.apr, options.basePrice, 0n, options.priceFixDuration, totalSeconds),
            exactPrice: calculateVectorPrice(options.apr, options.basePrice, totalSeconds),
          });
        }
        return rows;
      }

      function getScale(rows) {
        const xs = rows.map((row) => Number(row.t));
        const ys = rows.flatMap((row) => [toNumberPrice(row.stepPrice), toNumberPrice(row.exactPrice)]);
        const minX = Math.min(...xs);
        const maxX = Math.max(...xs);
        const minY = Math.min(...ys);
        const maxY = Math.max(...ys);
        const yPad = Math.max((maxY - minY) * 0.08, 0.00001);
        return { minX, maxX, yMin: minY - yPad, yMax: maxY + yPad };
      }

      function scalePoints(rows, key, scale) {
        return rows
          .map((row) => {
            const xRatio = (Number(row.t) - scale.minX) / Math.max(scale.maxX - scale.minX, 1);
            const yRatio = (toNumberPrice(row[key]) - scale.yMin) / Math.max(scale.yMax - scale.yMin, 0.000000001);
            const x = leftPadding + xRatio * (width - leftPadding - rightPadding);
            const y = height - padding - yRatio * (height - padding * 2);
            return x.toFixed(2) + "," + y.toFixed(2);
          })
          .join(" ");
      }

      function scalePoint(row, key, scale) {
        const xRatio = (Number(row.t) - scale.minX) / Math.max(scale.maxX - scale.minX, 1);
        const yRatio = (toNumberPrice(row[key]) - scale.yMin) / Math.max(scale.yMax - scale.yMin, 0.000000001);
        const x = leftPadding + xRatio * (width - leftPadding - rightPadding);
        const y = height - padding - yRatio * (height - padding * 2);
        return { x, y };
      }

      function stepPath(rows, scale) {
        if (rows.length === 0) return "";
        const first = scalePoint(rows[0], "stepPrice", scale);
        const commands = ["M " + first.x.toFixed(2) + " " + first.y.toFixed(2)];
        for (let i = 1; i < rows.length; i += 1) {
          const prev = scalePoint(rows[i - 1], "stepPrice", scale);
          const next = scalePoint(rows[i], "stepPrice", scale);
          commands.push("H " + next.x.toFixed(2));
          commands.push("V " + next.y.toFixed(2));
        }
        return commands.join(" ");
      }

      function renderGrid(scale) {
        const grid = [];
        const labels = [];
        for (let i = 0; i <= 5; i += 1) {
          const y = padding + (i * (height - padding * 2)) / 5;
          const value = scale.yMax - ((scale.yMax - scale.yMin) * i) / 5;
          grid.push('<line class="grid" x1="' + leftPadding + '" y1="' + y + '" x2="' + (width - rightPadding) + '" y2="' + y + '" />');
          labels.push('<text class="tick-label" x="' + (leftPadding - 10) + '" y="' + (y + 4) + '" text-anchor="end">' + value.toFixed(9) + '</text>');
        }
        for (let i = 0; i <= 7; i += 1) {
          const x = leftPadding + (i * (width - leftPadding - rightPadding)) / 7;
          const seconds = scale.minX + ((scale.maxX - scale.minX) * i) / 7;
          grid.push('<line class="grid" x1="' + x + '" y1="' + padding + '" x2="' + x + '" y2="' + (height - padding) + '" />');
          labels.push('<text class="tick-label" x="' + x + '" y="' + (height - padding + 18) + '" text-anchor="middle">' + (seconds / 86400).toFixed(0) + '</text>');
        }
        document.getElementById("grid-lines").innerHTML = grid.join("");
        document.getElementById("tick-labels").innerHTML = labels.join("");
      }

      function render() {
        const options = {
          apr: BigInt(inputs.apr.value || "0"),
          basePrice: BigInt(inputs.basePrice.value || "1"),
          priceFixDuration: BigInt(inputs.priceFixDuration.value || "1"),
          days: BigInt(inputs.days.value || "1"),
        };
        const startDate = parseStartDate(startDateInput.value);
        const rows = buildRows(options);
        const scale = getScale(rows);
        const first = rows[0];
        const last = rows[rows.length - 1];

        renderGrid(scale);
        const stepLine = document.getElementById("step-line");
        stepLine.outerHTML = '<path id="step-line" class="step" d="' + stepPath(rows, scale) + '"></path>';
        document.getElementById("smooth-line").setAttribute("points", scalePoints(rows, "exactPrice", scale));

        document.getElementById("apr-output").textContent = formatApr(options.apr) + " (" + options.apr.toString() + ")";
        document.getElementById("base-price-output").textContent = formatPrice(options.basePrice);
        document.getElementById("fix-duration-output").textContent = formatDuration(Number(options.priceFixDuration));
        document.getElementById("days-output").textContent = options.days.toString() + "d";
        document.getElementById("start-date-output").textContent = startDate.toISOString().slice(0, 10);

        document.getElementById("metric-apr").textContent = options.apr.toString();
        document.getElementById("metric-base-price").textContent = formatPrice(options.basePrice);
        document.getElementById("metric-fix-duration").textContent = options.priceFixDuration.toString() + "s";
        document.getElementById("metric-days").textContent = options.days.toString() + " days";
        document.getElementById("metric-initial-step").textContent = formatPrice(first.stepPrice);
        document.getElementById("metric-final-step").textContent = formatPrice(last.stepPrice);
        document.getElementById("metric-initial-exact").textContent = formatPrice(first.exactPrice);
        document.getElementById("metric-final-exact").textContent = formatPrice(last.exactPrice);

        const dailyRows = [];
        for (let day = 1n; day <= options.days; day += 1n) {
          const elapsed = day * SECONDS_IN_DAY - 1n;
          dailyRows.push(
            "<tr><td>" +
              formatDate(startDate, elapsed) +
              "</td><td>" +
              formatPrice(calculateStepPrice(options.apr, options.basePrice, 0n, options.priceFixDuration, elapsed)) +
              "</td><td>" +
              formatPrice(calculateVectorPrice(options.apr, options.basePrice, elapsed)) +
              "</td></tr>",
          );
        }
        document.getElementById("daily-price-rows").innerHTML = dailyRows.join("");
      }

      function syncFromRange(key) {
        inputs[key].value = controls[key].value;
        render();
      }

      function syncFromInput(key) {
        const parsed = Number(inputs[key].value);
        if (Number.isFinite(parsed)) {
          controls[key].value = String(parsed);
        }
        render();
      }

      Object.keys(controls).forEach((key) => {
        controls[key].addEventListener("input", () => syncFromRange(key));
        inputs[key].addEventListener("input", () => syncFromInput(key));
      });
      startDateInput.addEventListener("input", render);
      render();
    </script>
  </body>
</html>
`;
}

const options = parseArgs(process.argv.slice(2));
const rows = buildRows(options);
const html = renderHtml(options, rows);
const cwd = process.cwd();
const outPath = path.resolve(cwd, options.out);
fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, html);

const relativePath = path.relative(cwd, outPath);
const self = fileURLToPath(import.meta.url);
console.log(`Generated ${relativePath}`);
console.log(`Source: ${path.relative(cwd, self)}`);
