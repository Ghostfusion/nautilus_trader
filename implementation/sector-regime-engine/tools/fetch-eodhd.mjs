#!/usr/bin/env bun
/**
 * Optional real-data path: pull daily adjusted closes for the eleven sector ETFs from EODHD and
 * write them as a CSV the application can load.
 *
 * Usage:   bun run fetch              defaults to 8 years
 *          bun run fetch -- 12        12 years
 *
 * The key is read from EODHD_API_KEY in the repository .env, or from the environment. That is the
 * same variable the Rust adapter uses, so nothing new has to be configured. The key is never printed.
 *
 * On adjusted close: EODHD's adjusted_close is split and dividend adjusted. That is close enough to
 * a total-return series for measurement work, but it is not a licensed total-return index, and the
 * difference matters when comparing against published fund performance.
 */

import { readFileSync } from 'node:fs';
import { join, normalize } from 'node:path';

// Matches EODHD_HTTP_BASE_URL in crates/adapters/eodhd/src/common.rs
const BASE_URL = 'https://eodhd.com/api';
const SYMBOLS = ['XLC', 'XLY', 'XLP', 'XLE', 'XLF', 'XLV', 'XLI', 'XLB', 'XLRE', 'XLK', 'XLU'];
const REQUEST_PAUSE_MS = 120; // EODHD allows 10 requests per second

function readKey() {
  if (process.env.EODHD_API_KEY) return process.env.EODHD_API_KEY.replace(/[\r\n"]/g, '');
  const envPath = normalize(join(import.meta.dir, '..', '..', '..', '.env'));
  try {
    const text = readFileSync(envPath, 'utf8');
    for (const line of text.split(/\r?\n/)) {
      if (line.startsWith('EODHD_API_KEY=')) {
        return line.slice('EODHD_API_KEY='.length).trim().replace(/^["']|["']$/g, '');
      }
    }
  } catch {
    // fall through to the error below
  }
  throw new Error(`no EODHD_API_KEY in the environment or in ${envPath}`);
}

/** Split a date range into one-year windows, so no single response is truncated by a row limit. */
function yearWindows(fromIso, toIso) {
  const windows = [];
  let start = new Date(`${fromIso}T00:00:00Z`);
  const end = new Date(`${toIso}T00:00:00Z`);
  while (start < end) {
    const next = new Date(start);
    next.setUTCFullYear(next.getUTCFullYear() + 1);
    const stop = next < end ? next : end;
    windows.push([start.toISOString().slice(0, 10), stop.toISOString().slice(0, 10)]);
    start = stop;
  }
  return windows;
}

async function fetchWindow(symbol, key, from, to) {
  const url =
    `${BASE_URL}/eod/${symbol}.US?api_token=${encodeURIComponent(key)}` +
    `&fmt=json&period=d&order=a&from=${from}&to=${to}`;
  const response = await fetch(url);
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  const rows = await response.json();
  return Array.isArray(rows) ? rows : [];
}

async function fetchSymbol(symbol, key, fromIso, toIso) {
  const byDate = new Map();
  for (const [from, to] of yearWindows(fromIso, toIso)) {
    const rows = await fetchWindow(symbol, key, from, to);
    for (const row of rows) {
      const close = row.adjusted_close ?? row.close;
      if (Number.isFinite(close)) byDate.set(row.date, close);
    }
    await new Promise((resolve) => setTimeout(resolve, REQUEST_PAUSE_MS));
  }
  if (byDate.size === 0) throw new Error('no rows returned');
  return byDate;
}

const years = Number(process.argv[2] ?? 8);
const to = new Date().toISOString().slice(0, 10);
const from = new Date(Date.now() - years * 365.25 * 24 * 3600 * 1000).toISOString().slice(0, 10);
const key = readKey();
console.log(`fetching ${SYMBOLS.length} symbols from ${from} to ${to}`);

const series = new Map();
const failures = [];
for (const symbol of SYMBOLS) {
  try {
    const byDate = await fetchSymbol(symbol, key, from, to);
    series.set(symbol, byDate);
    console.log(`  ${symbol}: ${byDate.size} rows`);
  } catch (error) {
    failures.push(`${symbol}: ${error.message}`);
    console.log(`  ${symbol}: FAILED (${error.message})`);
  }
}

if (series.size < 3) {
  throw new Error(`only ${series.size} symbols succeeded; nothing worth writing\n${failures.join('\n')}`);
}
if (failures.length > 0) {
  console.log(`\nwarning: ${failures.length} symbol(s) failed and are excluded:`);
  for (const f of failures) console.log(`  ${f}`);
}

// Intersect dates so every column is complete. Funds with later inception dates shorten the sample.
let dates = null;
for (const byDate of series.values()) {
  const keys = [...byDate.keys()];
  dates = dates === null ? new Set(keys) : new Set(keys.filter((d) => dates.has(d)));
}
const ordered = [...dates].sort();
if (ordered.length < 260) throw new Error(`only ${ordered.length} common dates; not enough to analyse`);

const symbols = [...series.keys()];
const lines = [`date,${symbols.join(',')}`];
for (const date of ordered) {
  lines.push([date, ...symbols.map((s) => series.get(s).get(date).toFixed(4))].join(','));
}

const outPath = normalize(join(import.meta.dir, '..', 'data', 'sector-prices.csv'));
await Bun.write(outPath, `${lines.join('\n')}\n`);
console.log(
  `\nwrote ${ordered.length} rows (${ordered[0]} to ${ordered[ordered.length - 1]}) to ${outPath}`,
);
console.log('Load it in the app with the CSV source option, mode "prices".');
