use onreapp::instructions::prop_amm::{
    apply_hard_wall_liquidity_factor_at_time, dynamic_wall_liquidity_at_time,
    preview_effective_sell_volume, record_prop_amm_buy, record_prop_amm_sell, PropAmmState,
    DEFAULT_CADENCE_SENSITIVITY_SCALED, DEFAULT_CADENCE_THRESHOLD, DEFAULT_CURVE_EXPONENT_SCALED,
    DEFAULT_CURVE_PEG_HAIRCUT_BPS, DEFAULT_EPOCH_DURATION_SECONDS,
    DEFAULT_MIN_CADENCE_EXPONENT_SCALED, DEFAULT_MIN_LIQUIDATION_HAIRCUT_BPS,
    DEFAULT_WALL_SENSITIVITY_SCALED,
};
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::PathBuf;

const STABLE_DECIMALS: u32 = 6;
const ONYC_DECIMALS: u32 = 9;
const DEFAULT_POOL_TARGET_BPS: u16 = 1_500;

#[derive(Clone, Copy)]
enum Action {
    Sell(u64),
    Buy(u64),
    Wait(i64),
}

#[derive(Clone)]
struct Scenario {
    name: String,
    actions: Vec<Action>,
}

struct Row {
    scenario: String,
    step: usize,
    now: i64,
    action: &'static str,
    amount: u64,
    vault_before: u64,
    pressure_before: u64,
    effective_volume: u64,
    wall: u64,
    output: u64,
    vault_after: u64,
    pressure_after: u64,
}

struct Config {
    source: String,
    out_dir: PathBuf,
    initial_vault: u64,
    hard_wall_reserve: u64,
    pool_target_bps: u16,
    min_liquidation_haircut_bps: u16,
    curve_peg_haircut_bps: u16,
    curve_exponent_scaled: u32,
    wall_sensitivity_scaled: u32,
    epoch_duration_seconds: i64,
    seconds_between_actions: i64,
    custom_actions: Option<Vec<Action>>,
    scenarios: Option<Vec<Scenario>>,
    comparisons: Vec<Comparison>,
    split_sweeps: Vec<SplitSweep>,
}

struct Comparison {
    name: String,
    total_sell: u64,
    chunk_count: u64,
    wait_seconds: Vec<i64>,
}

struct ComparisonRow {
    comparison: String,
    wait_seconds: i64,
    one_big_output: u64,
    split_output: u64,
    one_big_haircut_bps: f64,
    split_haircut_bps: f64,
    split_advantage: i128,
}

struct SplitSweep {
    name: String,
    total_sell: u64,
    max_chunks: u64,
    wait_seconds: i64,
}

struct SplitSweepRow {
    sweep: String,
    chunks: u64,
    one_big_output: u64,
    split_output: u64,
    one_big_haircut_bps: f64,
    split_haircut_bps: f64,
    split_advantage: i128,
}

#[derive(Deserialize)]
struct ConfigFile {
    config: Option<FileConfig>,
    scenarios: Option<Vec<FileScenario>>,
    comparisons: Option<Vec<FileComparison>>,
    split_sweeps: Option<Vec<FileSplitSweep>>,
}

#[derive(Deserialize)]
struct FileConfig {
    out_dir: Option<String>,
    initial_vault: Option<String>,
    hard_wall_reserve: Option<String>,
    pool_target_bps: Option<u16>,
    min_liquidation_haircut_bps: Option<u16>,
    curve_peg_haircut_bps: Option<u16>,
    curve_exponent_scaled: Option<u32>,
    wall_sensitivity_scaled: Option<u32>,
    epoch_duration_seconds: Option<i64>,
    seconds_between_actions: Option<i64>,
}

#[derive(Deserialize)]
struct FileScenario {
    name: String,
    actions: Vec<String>,
}

#[derive(Deserialize)]
struct FileComparison {
    name: String,
    total_sell: String,
    chunk_count: u64,
    wait_seconds: Vec<i64>,
}

#[derive(Deserialize)]
struct FileSplitSweep {
    name: String,
    total_sell: String,
    max_chunks: u64,
    wait_seconds: i64,
}

fn main() {
    let config = parse_args();
    fs::create_dir_all(&config.out_dir).expect("create output directory");

    let scenarios = scenarios(&config);
    let mut all_rows = Vec::new();
    let mut summaries = Vec::new();
    let mut comparison_rows = Vec::new();
    let mut split_sweep_rows = Vec::new();

    for scenario in scenarios {
        let rows = run_scenario(&config, &scenario);
        summaries.push(summary_for(&scenario.name, &rows));
        all_rows.extend(rows);
    }
    for comparison in &config.comparisons {
        comparison_rows.extend(run_comparison(&config, comparison));
    }
    for split_sweep in &config.split_sweeps {
        let rows = run_split_sweep(&config, split_sweep);
        write_split_sweep_svg(&config, split_sweep, &rows);
        split_sweep_rows.extend(rows);
    }

    let csv_path = config.out_dir.join("business_scenarios.csv");
    let comparison_csv_path = config.out_dir.join("business_comparisons.csv");
    let split_sweep_csv_path = config.out_dir.join("business_split_sweeps.csv");
    let md_path = config.out_dir.join("business_scenarios.md");
    fs::write(&csv_path, render_csv(&all_rows)).expect("write csv");
    fs::write(
        &comparison_csv_path,
        render_comparison_csv(&comparison_rows),
    )
    .expect("write comparison csv");
    fs::write(
        &split_sweep_csv_path,
        render_split_sweep_csv(&split_sweep_rows),
    )
    .expect("write split sweep csv");
    fs::write(
        &md_path,
        render_markdown(&config, &summaries, &comparison_rows, &split_sweep_rows),
    )
    .expect("write markdown");

    println!("wrote {}", csv_path.display());
    println!("wrote {}", comparison_csv_path.display());
    println!("wrote {}", split_sweep_csv_path.display());
    println!("wrote {}", md_path.display());
}

fn scenarios(config: &Config) -> Vec<Scenario> {
    if let Some(actions) = &config.custom_actions {
        return vec![Scenario {
            name: "custom".to_string(),
            actions: actions.clone(),
        }];
    }
    if let Some(scenarios) = &config.scenarios {
        return scenarios.clone();
    }

    let sell_250 = stable("250");
    let sell_1000 = stable("1000");
    let sell_3000 = stable("3000");

    vec![
        Scenario {
            name: "one_3000_sell".to_string(),
            actions: vec![Action::Sell(sell_3000)],
        },
        Scenario {
            name: "three_1000_sells".to_string(),
            actions: vec![
                Action::Sell(sell_1000),
                Action::Sell(sell_1000),
                Action::Sell(sell_1000),
            ],
        },
        Scenario {
            name: "twelve_250_sells".to_string(),
            actions: vec![Action::Sell(sell_250); 12],
        },
        Scenario {
            name: "four_250_then_one_3000_sell".to_string(),
            actions: vec![
                Action::Sell(sell_250),
                Action::Sell(sell_250),
                Action::Sell(sell_250),
                Action::Sell(sell_250),
                Action::Sell(sell_3000),
            ],
        },
        Scenario {
            name: "sell_1000_buy_1000_sell_1000_sell_1000".to_string(),
            actions: vec![
                Action::Sell(sell_1000),
                Action::Buy(sell_1000),
                Action::Sell(sell_1000),
                Action::Sell(sell_1000),
            ],
        },
        Scenario {
            name: "sell_3000_buy_3000_sell_3000".to_string(),
            actions: vec![
                Action::Sell(sell_3000),
                Action::Buy(sell_3000),
                Action::Sell(sell_3000),
            ],
        },
        Scenario {
            name: "one_full_vault_sell".to_string(),
            actions: vec![Action::Sell(config.initial_vault)],
        },
        Scenario {
            name: "epoch_decay_after_3000_sell".to_string(),
            actions: vec![
                Action::Sell(sell_3000),
                Action::Wait(DEFAULT_EPOCH_DURATION_SECONDS / 2),
                Action::Sell(sell_1000),
                Action::Wait(DEFAULT_EPOCH_DURATION_SECONDS),
                Action::Sell(sell_1000),
            ],
        },
    ]
}

fn run_scenario(config: &Config, scenario: &Scenario) -> Vec<Row> {
    let mut state = PropAmmState {
        pool_target_bps: config.pool_target_bps,
        min_liquidation_haircut_bps: config.min_liquidation_haircut_bps,
        curve_peg_haircut_bps: config.curve_peg_haircut_bps,
        curve_exponent_scaled: config.curve_exponent_scaled,
        min_cadence_exponent_scaled: DEFAULT_MIN_CADENCE_EXPONENT_SCALED,
        cadence_threshold: DEFAULT_CADENCE_THRESHOLD,
        cadence_sensitivity_scaled: DEFAULT_CADENCE_SENSITIVITY_SCALED,
        epoch_duration_seconds: config.epoch_duration_seconds,
        wall_sensitivity_scaled: config.wall_sensitivity_scaled,
        curr_sell_value_stable: 0,
        curr_buy_value_stable: 0,
        prev_net_sell_value_stable: 0,
        curr_sell_trade_count: 0,
        epoch_start: 0,
        bump: 0,
    };
    let mut vault = config.initial_vault;
    let mut now = 1_i64;
    let mut rows = Vec::new();

    for action in &scenario.actions {
        match *action {
            Action::Wait(seconds) => {
                now = now.saturating_add(seconds);
            }
            Action::Buy(amount) => {
                let pressure_before = preview_effective_sell_volume(&state, 0, now).unwrap();
                record_prop_amm_buy(&mut state, amount, now).unwrap();
                vault = vault.saturating_add(amount);
                let pressure_after = preview_effective_sell_volume(&state, 0, now).unwrap();
                rows.push(Row {
                    scenario: scenario.name.clone(),
                    step: rows.len() + 1,
                    now,
                    action: "buy_onyc",
                    amount,
                    vault_before: vault.saturating_sub(amount),
                    pressure_before,
                    effective_volume: pressure_after,
                    wall: dynamic_wall_liquidity_at_time(
                        0,
                        vault,
                        config.hard_wall_reserve,
                        &state,
                        now,
                    )
                    .unwrap(),
                    output: 0,
                    vault_after: vault,
                    pressure_after,
                });
                now = now.saturating_add(config.seconds_between_actions);
            }
            Action::Sell(raw_value) => {
                let vault_before = vault;
                let pressure_before = preview_effective_sell_volume(&state, 0, now).unwrap();
                let effective_volume =
                    preview_effective_sell_volume(&state, raw_value, now).unwrap();
                let wall = dynamic_wall_liquidity_at_time(
                    raw_value,
                    vault,
                    config.hard_wall_reserve,
                    &state,
                    now,
                )
                .unwrap();
                let output = apply_hard_wall_liquidity_factor_at_time(
                    raw_value,
                    vault,
                    config.hard_wall_reserve,
                    &state,
                    now,
                )
                .expect("sell quote failed");
                vault = vault.saturating_sub(output);
                record_prop_amm_sell(&mut state, raw_value, now).unwrap();
                let pressure_after = preview_effective_sell_volume(&state, 0, now).unwrap();
                rows.push(Row {
                    scenario: scenario.name.clone(),
                    step: rows.len() + 1,
                    now,
                    action: "sell_onyc",
                    amount: raw_value,
                    vault_before,
                    pressure_before,
                    effective_volume,
                    wall,
                    output,
                    vault_after: vault,
                    pressure_after,
                });
                now = now.saturating_add(config.seconds_between_actions);
            }
        }
    }

    rows
}

fn render_csv(rows: &[Row]) -> String {
    let mut out = String::from(
        "scenario,step,now,action,amount_stable,amount_onyc_equivalent,vault_before_stable,pressure_before_stable,effective_volume_stable,wall_stable,output_stable,output_pct,vault_after_stable,pressure_after_stable\n",
    );
    for row in rows {
        let output_pct = if row.amount == 0 || row.action == "buy_onyc" {
            0.0
        } else {
            row.output as f64 * 100.0 / row.amount as f64
        };
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{:.6},{},{}\n",
            row.scenario,
            row.step,
            row.now,
            row.action,
            row.amount,
            row.amount * 10_u64.pow(ONYC_DECIMALS - STABLE_DECIMALS),
            row.vault_before,
            row.pressure_before,
            row.effective_volume,
            row.wall,
            row.output,
            output_pct,
            row.vault_after,
            row.pressure_after,
        ));
    }
    out
}

fn render_comparison_csv(rows: &[ComparisonRow]) -> String {
    let mut out = String::from(
        "comparison,wait_seconds,one_big_output_stable,split_output_stable,one_big_haircut_bps,split_haircut_bps,split_advantage_stable\n",
    );
    for row in rows {
        out.push_str(&format!(
            "{},{},{},{},{:.6},{:.6},{}\n",
            row.comparison,
            row.wait_seconds,
            row.one_big_output,
            row.split_output,
            row.one_big_haircut_bps,
            row.split_haircut_bps,
            row.split_advantage,
        ));
    }
    out
}

fn render_split_sweep_csv(rows: &[SplitSweepRow]) -> String {
    let mut out = String::from(
        "sweep,chunks,one_big_output_stable,split_output_stable,one_big_haircut_bps,split_haircut_bps,split_advantage_stable\n",
    );
    for row in rows {
        out.push_str(&format!(
            "{},{},{},{},{:.6},{:.6},{}\n",
            row.sweep,
            row.chunks,
            row.one_big_output,
            row.split_output,
            row.one_big_haircut_bps,
            row.split_haircut_bps,
            row.split_advantage,
        ));
    }
    out
}

fn render_markdown(
    config: &Config,
    summaries: &[String],
    comparison_rows: &[ComparisonRow],
    split_sweep_rows: &[SplitSweepRow],
) -> String {
    let mut out = String::new();
    out.push_str("# Prop AMM Business Scenarios\n\n");
    out.push_str("Generated by the Rust example `prop_amm_business_scenarios`, using the same Rust Prop AMM quote/tracker methods as the program code.\n\n");
    out.push_str(&format!(
        "- Config source: `{}`\n- Initial vault: {}\n- Hard wall reserve: {}\n- Pool target bps: {}\n- Wall sensitivity scaled: {} ({:.2})\n- Min liquidation haircut bps: {}\n- Curve peg haircut bps: {}\n- Curve exponent scaled: {} ({:.2})\n- Epoch duration seconds: {}\n- Seconds between actions: {}\n\n",
        config.source,
        format_stable(config.initial_vault),
        format_stable(config.hard_wall_reserve),
        config.pool_target_bps,
        config.wall_sensitivity_scaled,
        config.wall_sensitivity_scaled as f64 / 10_000.0,
        config.min_liquidation_haircut_bps,
        config.curve_peg_haircut_bps,
        config.curve_exponent_scaled,
        config.curve_exponent_scaled as f64 / 10_000.0,
        config.epoch_duration_seconds,
        config.seconds_between_actions,
    ));
    for summary in summaries {
        out.push_str(summary);
        out.push('\n');
    }
    if !comparison_rows.is_empty() {
        out.push_str("## Split Comparison\n\n");
        out.push_str("This compares one large sell against split sells using the same Rust quote path. `Split advantage` is how much more stablecoin the split path receives versus one big sell.\n\n");
        let mut current = "";
        for row in comparison_rows {
            if current != row.comparison {
                current = &row.comparison;
                out.push_str(&format!("### {}\n\n", row.comparison));
                out.push_str("| Wait Between Split Sells | One Big Output | Split Output | One Big Haircut | Split Haircut | Split Advantage |\n");
                out.push_str("| ---: | ---: | ---: | ---: | ---: | ---: |\n");
            }
            out.push_str(&format!(
                "| {}s | {} | {} | {:.6} bps | {:.6} bps | {} |\n",
                row.wait_seconds,
                format_stable(row.one_big_output),
                format_stable(row.split_output),
                row.one_big_haircut_bps,
                row.split_haircut_bps,
                format_signed_stable(row.split_advantage),
            ));
        }
        out.push('\n');
        out.push_str("If split advantage is positive even at `0s`, then waiting cannot make the split path worse; waiting only lets pressure decay, so the same-haircut time does not exist under the current endpoint formula.\n\n");
    }
    if !split_sweep_rows.is_empty() {
        out.push_str("## Split Sweep Curves\n\n");
        out.push_str("Each SVG plots two curves over `n`: the one-big-sell haircut and the total haircut from splitting the same sell into `n` chunks.\n\n");
        let mut current = "";
        for row in split_sweep_rows {
            if current != row.sweep {
                current = &row.sweep;
                out.push_str(&format!("### {}\n\n", row.sweep));
                out.push_str(&format!(
                    "SVG: `{}`\n\n",
                    config
                        .out_dir
                        .join(format!("{}_split_sweep.svg", row.sweep))
                        .display()
                ));
                out.push_str("| n sells | One Big Haircut | Split Haircut | Split Advantage |\n");
                out.push_str("| ---: | ---: | ---: | ---: |\n");
            }
            if row.chunks == 1 || row.chunks % 10 == 0 {
                out.push_str(&format!(
                    "| {} | {:.6} bps | {:.6} bps | {} |\n",
                    row.chunks,
                    row.one_big_haircut_bps,
                    row.split_haircut_bps,
                    format_signed_stable(row.split_advantage),
                ));
            }
        }
        out.push('\n');
    }
    out
}

fn run_comparison(config: &Config, comparison: &Comparison) -> Vec<ComparisonRow> {
    let one_big_rows = run_scenario(
        config,
        &Scenario {
            name: format!("{}_one_big", comparison.name),
            actions: vec![Action::Sell(comparison.total_sell)],
        },
    );
    let one_big_output = total_output(&one_big_rows);
    let one_big_haircut_bps = haircut_bps(comparison.total_sell, one_big_output);
    let chunk = comparison
        .total_sell
        .checked_div(comparison.chunk_count)
        .expect("chunk count must be nonzero");

    comparison
        .wait_seconds
        .iter()
        .map(|wait_seconds| {
            let mut actions = Vec::new();
            for index in 0..comparison.chunk_count {
                actions.push(Action::Sell(chunk));
                if index + 1 != comparison.chunk_count {
                    actions.push(Action::Wait(
                        wait_seconds.saturating_sub(config.seconds_between_actions),
                    ));
                }
            }
            let split_rows = run_scenario(
                config,
                &Scenario {
                    name: format!("{}_split", comparison.name),
                    actions,
                },
            );
            let split_output = total_output(&split_rows);
            ComparisonRow {
                comparison: comparison.name.clone(),
                wait_seconds: *wait_seconds,
                one_big_output,
                split_output,
                one_big_haircut_bps,
                split_haircut_bps: haircut_bps(comparison.total_sell, split_output),
                split_advantage: split_output as i128 - one_big_output as i128,
            }
        })
        .collect()
}

fn total_output(rows: &[Row]) -> u64 {
    rows.iter().map(|row| row.output).sum()
}

fn run_split_sweep(config: &Config, sweep: &SplitSweep) -> Vec<SplitSweepRow> {
    let one_big_rows = run_scenario(
        config,
        &Scenario {
            name: format!("{}_one_big", sweep.name),
            actions: vec![Action::Sell(sweep.total_sell)],
        },
    );
    let one_big_output = total_output(&one_big_rows);
    let one_big_haircut_bps = haircut_bps(sweep.total_sell, one_big_output);

    (1..=sweep.max_chunks)
        .map(|chunks| {
            let base_chunk = sweep.total_sell / chunks;
            let remainder = sweep.total_sell % chunks;
            let mut actions = Vec::new();
            for index in 0..chunks {
                let raw = base_chunk + if index == chunks - 1 { remainder } else { 0 };
                actions.push(Action::Sell(raw));
                if index + 1 != chunks {
                    actions.push(Action::Wait(
                        sweep
                            .wait_seconds
                            .saturating_sub(config.seconds_between_actions),
                    ));
                }
            }
            let split_rows = run_scenario(
                config,
                &Scenario {
                    name: format!("{}_{}x", sweep.name, chunks),
                    actions,
                },
            );
            let split_output = total_output(&split_rows);
            SplitSweepRow {
                sweep: sweep.name.clone(),
                chunks,
                one_big_output,
                split_output,
                one_big_haircut_bps,
                split_haircut_bps: haircut_bps(sweep.total_sell, split_output),
                split_advantage: split_output as i128 - one_big_output as i128,
            }
        })
        .collect()
}

fn write_split_sweep_svg(config: &Config, sweep: &SplitSweep, rows: &[SplitSweepRow]) {
    if rows.is_empty() {
        return;
    }
    let width = 1100.0;
    let height = 620.0;
    let left = 78.0;
    let right = 38.0;
    let top = 58.0;
    let bottom = 72.0;
    let plot_width = width - left - right;
    let plot_height = height - top - bottom;
    let x_min = 1.0;
    let x_max = sweep.max_chunks.max(1) as f64;
    let y_min = rows
        .iter()
        .flat_map(|row| [row.one_big_haircut_bps, row.split_haircut_bps])
        .fold(f64::INFINITY, f64::min);
    let y_max = rows
        .iter()
        .flat_map(|row| [row.one_big_haircut_bps, row.split_haircut_bps])
        .fold(f64::NEG_INFINITY, f64::max);
    let y_pad = ((y_max - y_min) * 0.15).max(0.000001);
    let y_min = y_min - y_pad;
    let y_max = y_max + y_pad;
    let sx = |x: f64| left + ((x - x_min) / (x_max - x_min).max(1.0)) * plot_width;
    let sy = |y: f64| top + plot_height - ((y - y_min) / (y_max - y_min)) * plot_height;
    let path = |selector: fn(&SplitSweepRow) -> f64| {
        rows.iter()
            .enumerate()
            .map(|(index, row)| {
                format!(
                    "{}{:.2} {:.2}",
                    if index == 0 { "M" } else { "L" },
                    sx(row.chunks as f64),
                    sy(selector(row))
                )
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#
    );
    svg.push_str(r#"<style>.bg{fill:#fbfdff}.grid{stroke:#d8e0e8;stroke-width:1}.axis{stroke:#475569;stroke-width:1.5}.label{font:14px sans-serif;fill:#334155}.title{font:18px sans-serif;fill:#0f172a}.legend{font:15px sans-serif;fill:#1f2937}</style>"#);
    svg.push_str(&format!(
        r#"<rect class="bg" width="{width}" height="{height}"/>"#
    ));
    svg.push_str(&format!(
        r#"<text x="{left}" y="32" class="title">Split Sweep: {}</text>"#,
        sweep.name
    ));
    svg.push_str(&format!(
        r#"<text x="{left}" y="52" class="label">total={}, wait={}s, vault={}</text>"#,
        format_stable(sweep.total_sell),
        sweep.wait_seconds,
        format_stable(config.initial_vault)
    ));
    for tick in 0..=4 {
        let y = y_min + (y_max - y_min) * tick as f64 / 4.0;
        svg.push_str(&format!(
            r#"<line class="grid" x1="{left}" y1="{:.2}" x2="{:.2}" y2="{:.2}"/><text class="label" x="18" y="{:.2}">{:.4}</text>"#,
            sy(y),
            width - right,
            sy(y),
            sy(y) + 4.0,
            y
        ));
    }
    for x in [1_u64, 10, 20, 30, 40, 50] {
        if x <= sweep.max_chunks {
            svg.push_str(&format!(
                r#"<line class="grid" x1="{:.2}" y1="{top}" x2="{:.2}" y2="{:.2}"/><text class="label" x="{:.2}" y="{:.2}">{x}</text>"#,
                sx(x as f64),
                sx(x as f64),
                height - bottom,
                sx(x as f64) - 8.0,
                height - bottom + 28.0
            ));
        }
    }
    svg.push_str(&format!(
        r#"<line class="axis" x1="{left}" y1="{:.2}" x2="{:.2}" y2="{:.2}"/><line class="axis" x1="{left}" y1="{top}" x2="{left}" y2="{:.2}"/>"#,
        height - bottom,
        width - right,
        height - bottom,
        height - bottom
    ));
    svg.push_str(&format!(
        r##"<path d="{}" fill="none" stroke="#d23f31" stroke-width="3"/>"##,
        path(|row| row.one_big_haircut_bps)
    ));
    svg.push_str(&format!(
        r##"<path d="{}" fill="none" stroke="#2563eb" stroke-width="3"/>"##,
        path(|row| row.split_haircut_bps)
    ));
    svg.push_str(r##"<line x1="770" y1="86" x2="812" y2="86" stroke="#d23f31" stroke-width="3"/><text class="legend" x="824" y="91">one big sell haircut</text>"##);
    svg.push_str(r##"<line x1="770" y1="112" x2="812" y2="112" stroke="#2563eb" stroke-width="3"/><text class="legend" x="824" y="117">n split sells haircut</text>"##);
    svg.push_str(&format!(
        r#"<text class="label" x="500" y="{:.2}">number of split sells (n)</text>"#,
        height - 18.0
    ));
    svg.push_str(
        r#"<text class="label" transform="translate(18 390) rotate(-90)">haircut bps</text>"#,
    );
    svg.push_str("</svg>");
    fs::write(
        config
            .out_dir
            .join(format!("{}_split_sweep.svg", sweep.name)),
        svg,
    )
    .expect("write split sweep svg");
}

fn haircut_bps(raw: u64, output: u64) -> f64 {
    if raw == 0 {
        return 0.0;
    }
    (raw.saturating_sub(output) as f64) * 10_000.0 / raw as f64
}

fn summary_for(name: &str, rows: &[Row]) -> String {
    let total_raw: u64 = rows
        .iter()
        .filter(|row| row.action == "sell_onyc")
        .map(|row| row.amount)
        .sum();
    let total_output: u64 = rows.iter().map(|row| row.output).sum();
    let final_vault = rows.last().map(|row| row.vault_after).unwrap_or(0);
    let final_pressure = rows.last().map(|row| row.pressure_after).unwrap_or(0);
    let payout_pct = if total_raw == 0 {
        0.0
    } else {
        total_output as f64 * 100.0 / total_raw as f64
    };

    let mut out = format!(
        "## {}\n\nTotal ONYC sold at raw stable value {} -> user receives {} ({:.2}%). Final vault: {}. Final pressure: {}.\n\n",
        name,
        format_stable(total_raw),
        format_stable(total_output),
        payout_pct,
        format_stable(final_vault),
        format_stable(final_pressure),
    );
    out.push_str(
        "| Step | Action | Amount | Wall | Output | Output % | Vault After | Pressure After |\n",
    );
    out.push_str("| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |\n");
    for row in rows {
        let output_pct = if row.action == "sell_onyc" && row.amount > 0 {
            row.output as f64 * 100.0 / row.amount as f64
        } else {
            0.0
        };
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {:.2}% | {} | {} |\n",
            row.step,
            row.action,
            format_stable(row.amount),
            format_stable(row.wall),
            format_stable(row.output),
            output_pct,
            format_stable(row.vault_after),
            format_stable(row.pressure_after),
        ));
    }
    out
}

fn parse_args() -> Config {
    let mut args = env::args().skip(1).peekable();
    let mut config_path = PathBuf::from("configs/prop_amm_business_scenarios.toml");
    let mut remaining_args = Vec::new();

    while let Some(arg) = args.next() {
        if arg == "--config" {
            config_path = PathBuf::from(args.next().expect("missing --config"));
        } else {
            remaining_args.push(arg);
        }
    }

    let mut config = Config {
        source: if config_path.exists() {
            config_path.display().to_string()
        } else {
            "built-in defaults".to_string()
        },
        out_dir: PathBuf::from("target/prop_amm_business_scenarios"),
        initial_vault: stable("10000"),
        hard_wall_reserve: stable("10000"),
        pool_target_bps: DEFAULT_POOL_TARGET_BPS,
        min_liquidation_haircut_bps: DEFAULT_MIN_LIQUIDATION_HAIRCUT_BPS,
        curve_peg_haircut_bps: DEFAULT_CURVE_PEG_HAIRCUT_BPS,
        curve_exponent_scaled: DEFAULT_CURVE_EXPONENT_SCALED,
        wall_sensitivity_scaled: DEFAULT_WALL_SENSITIVITY_SCALED,
        epoch_duration_seconds: DEFAULT_EPOCH_DURATION_SECONDS,
        seconds_between_actions: 1,
        custom_actions: None,
        scenarios: None,
        comparisons: Vec::new(),
        split_sweeps: Vec::new(),
    };

    if config_path.exists() {
        apply_config_file(&mut config, &config_path);
    }

    let mut args = remaining_args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out-dir" => config.out_dir = PathBuf::from(args.next().expect("missing --out-dir")),
            "--vault" => config.initial_vault = stable(&args.next().expect("missing --vault")),
            "--reserve" => {
                config.hard_wall_reserve = stable(&args.next().expect("missing --reserve"))
            }
            "--pool-target-bps" => {
                config.pool_target_bps = args
                    .next()
                    .expect("missing --pool-target-bps")
                    .parse()
                    .expect("invalid --pool-target-bps")
            }
            "--min-liquidation-haircut-bps" => {
                config.min_liquidation_haircut_bps = args
                    .next()
                    .expect("missing --min-liquidation-haircut-bps")
                    .parse()
                    .expect("invalid --min-liquidation-haircut-bps")
            }
            "--curve-peg-haircut-bps" => {
                config.curve_peg_haircut_bps = args
                    .next()
                    .expect("missing --curve-peg-haircut-bps")
                    .parse()
                    .expect("invalid --curve-peg-haircut-bps")
            }
            "--curve-exponent-scaled" => {
                config.curve_exponent_scaled = args
                    .next()
                    .expect("missing --curve-exponent-scaled")
                    .parse()
                    .expect("invalid --curve-exponent-scaled")
            }
            "--wall-sensitivity-scaled" => {
                config.wall_sensitivity_scaled = args
                    .next()
                    .expect("missing --wall-sensitivity-scaled")
                    .parse()
                    .expect("invalid --wall-sensitivity-scaled")
            }
            "--epoch-duration-seconds" => {
                config.epoch_duration_seconds = args
                    .next()
                    .expect("missing --epoch-duration-seconds")
                    .parse()
                    .expect("invalid --epoch-duration-seconds")
            }
            "--seconds-between-actions" => {
                config.seconds_between_actions = args
                    .next()
                    .expect("missing --seconds-between-actions")
                    .parse()
                    .expect("invalid --seconds-between-actions")
            }
            "--custom" => {
                config.custom_actions =
                    Some(parse_action_script(&args.next().expect("missing --custom")));
                config.scenarios = None;
            }
            "--help" => {
                println!(
                    "Usage: cargo run -p onreapp --example prop_amm_business_scenarios -- --config configs/prop_amm_business_scenarios.toml"
                );
                std::process::exit(0);
            }
            other => panic!("unknown argument {other}"),
        }
    }

    config
}

fn apply_config_file(config: &mut Config, path: &PathBuf) {
    let contents = fs::read_to_string(path).expect("read config file");
    let file: ConfigFile = toml::from_str(&contents).expect("parse config file");

    if let Some(file_config) = file.config {
        if let Some(out_dir) = file_config.out_dir {
            config.out_dir = PathBuf::from(out_dir);
        }
        if let Some(initial_vault) = file_config.initial_vault {
            config.initial_vault = stable(&initial_vault);
        }
        if let Some(hard_wall_reserve) = file_config.hard_wall_reserve {
            config.hard_wall_reserve = stable(&hard_wall_reserve);
        }
        if let Some(pool_target_bps) = file_config.pool_target_bps {
            config.pool_target_bps = pool_target_bps;
        }
        if let Some(min_liquidation_haircut_bps) = file_config.min_liquidation_haircut_bps {
            config.min_liquidation_haircut_bps = min_liquidation_haircut_bps;
        }
        if let Some(curve_peg_haircut_bps) = file_config.curve_peg_haircut_bps {
            config.curve_peg_haircut_bps = curve_peg_haircut_bps;
        }
        if let Some(curve_exponent_scaled) = file_config.curve_exponent_scaled {
            config.curve_exponent_scaled = curve_exponent_scaled;
        }
        if let Some(wall_sensitivity_scaled) = file_config.wall_sensitivity_scaled {
            config.wall_sensitivity_scaled = wall_sensitivity_scaled;
        }
        if let Some(epoch_duration_seconds) = file_config.epoch_duration_seconds {
            config.epoch_duration_seconds = epoch_duration_seconds;
        }
        if let Some(seconds_between_actions) = file_config.seconds_between_actions {
            config.seconds_between_actions = seconds_between_actions;
        }
    }

    config.scenarios = file.scenarios.map(|scenarios| {
        scenarios
            .into_iter()
            .map(|scenario| Scenario {
                name: scenario.name,
                actions: parse_actions(&scenario.actions),
            })
            .collect()
    });
    config.comparisons = file
        .comparisons
        .unwrap_or_default()
        .into_iter()
        .map(|comparison| Comparison {
            name: comparison.name,
            total_sell: stable(&comparison.total_sell),
            chunk_count: comparison.chunk_count,
            wait_seconds: comparison.wait_seconds,
        })
        .collect();
    config.split_sweeps = file
        .split_sweeps
        .unwrap_or_default()
        .into_iter()
        .map(|sweep| SplitSweep {
            name: sweep.name,
            total_sell: stable(&sweep.total_sell),
            max_chunks: sweep.max_chunks,
            wait_seconds: sweep.wait_seconds,
        })
        .collect();
}

fn parse_actions(actions: &[String]) -> Vec<Action> {
    actions.iter().map(|action| parse_action(action)).collect()
}

fn parse_action_script(script: &str) -> Vec<Action> {
    script
        .split(',')
        .filter(|part| !part.trim().is_empty())
        .map(parse_action)
        .collect()
}

fn parse_action(action: impl AsRef<str>) -> Action {
    let action = action.as_ref().trim();
    let (kind, value) = action.split_once(':').expect("actions must use kind:value");
    match kind {
        "sell" => Action::Sell(stable(value)),
        "buy" => Action::Buy(stable(value)),
        "wait" => Action::Wait(value.parse().expect("invalid wait seconds")),
        _ => panic!("action kind must be sell, buy, or wait"),
    }
}

fn stable(input: &str) -> u64 {
    let (whole, fraction) = input.split_once('.').unwrap_or((input, ""));
    let whole = whole.parse::<u64>().expect("invalid stable amount");
    let mut fraction = fraction.to_string();
    assert!(
        fraction.len() <= STABLE_DECIMALS as usize,
        "stable amount has too many decimals"
    );
    while fraction.len() < STABLE_DECIMALS as usize {
        fraction.push('0');
    }
    let fraction = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<u64>().expect("invalid stable decimals")
    };
    whole
        .checked_mul(10_u64.pow(STABLE_DECIMALS))
        .and_then(|value| value.checked_add(fraction))
        .expect("stable amount overflow")
}

fn format_stable(value: u64) -> String {
    let scale = 10_u64.pow(STABLE_DECIMALS);
    let whole = value / scale;
    let cents = (value % scale) / 10_000;
    format!("${whole}.{cents:02}")
}

fn format_signed_stable(value: i128) -> String {
    if value < 0 {
        format!("-{}", format_stable(value.unsigned_abs() as u64))
    } else {
        format_stable(value as u64)
    }
}
