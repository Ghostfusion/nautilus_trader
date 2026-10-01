// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2026 Nautech Systems Pty Ltd. All rights reserved.
//  https://nautechsystems.io
//
//  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
//  You may not use this file except in compliance with the License.
//  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
// -------------------------------------------------------------------------------------------------

//! Benchmarks the membership-history backfill cost.
//!
//! Two measurements, both over a synthetic but realistically shaped universe:
//!
//! 1. Producing the stored membership history for the whole range: a [`MembershipRule`] evaluates
//!    into one entry/exit spell per instrument over a multi-year range, and
//!    [`MembershipSeries::from_rule`] stamps and stores the resulting intervals.
//! 2. Building a point-in-time panel over that history at a stated frequency: membership is
//!    resolved per timestamp with [`MembershipSeries::members_at`] and the resulting rows are
//!    validated by [`Panel::new`], so the per-timestamp resolution cost is measured rather than
//!    hidden.
//!
//! Both measurements run over several instrument counts so the scaling in instruments is observed
//! rather than assumed. The fixture is deterministic and entirely in-memory; no catalog round trip
//! is measured.

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use nautilus_core::UnixNanos;
use nautilus_model::identifiers::InstrumentId;
use nautilus_research::{MembershipRule, MembershipSeries, MembershipSpell, Panel, PanelRow};

/// Instrument counts measured; 500 is the headline shape.
const INSTRUMENT_COUNTS: [usize; 3] = [250, 500, 1000];
/// Length of the history in days: 10 years including leap days from 2016-01-01.
const HISTORY_DAYS: u64 = 3653;
/// Nanoseconds in one day.
const DAY_NS: u64 = 86_400_000_000_000;
/// Start of the history: 2016-01-01T00:00:00Z.
const START_NS: u64 = 1_451_606_400_000_000_000;

/// A deterministic synthetic rule producing one entry/exit spell per instrument.
///
/// Entry days are spread across the whole range; roughly half the instruments exit inside the
/// range (the rest stay open to the end), so membership is genuinely time-varying rather than a
/// constant universe.
struct SyntheticMembership {
    universe: String,
    source: String,
    spells: Vec<MembershipSpell>,
}

impl SyntheticMembership {
    fn new(instruments: usize) -> Self {
        let spells = (0..instruments)
            .map(|index| {
                let index = index as u64;
                // Stagger entries across the whole range, with a small deterministic offset.
                let entry_day = index * HISTORY_DAYS / instruments as u64 + (index * 37) % 30;
                let lifespan_days = 900 + (index % 8) * 300;
                let exit_day = entry_day + lifespan_days;
                let entered_at = UnixNanos::from(START_NS + entry_day * DAY_NS);
                let exited_at = (exit_day < HISTORY_DAYS)
                    .then(|| UnixNanos::from(START_NS + exit_day * DAY_NS));

                MembershipSpell {
                    instrument_id: InstrumentId::from(format!("INST{index:04}.XSIM").as_str()),
                    entered_at,
                    exited_at,
                }
            })
            .collect();

        Self {
            universe: "synthetic/universe".to_string(),
            source: "synthetic/rule/v1".to_string(),
            spells,
        }
    }

    /// The panel timestamps: one per day across the full range.
    fn schedule() -> Vec<UnixNanos> {
        (0..HISTORY_DAYS)
            .map(|day| UnixNanos::from(START_NS + day * DAY_NS))
            .collect()
    }
}

impl MembershipRule for SyntheticMembership {
    fn universe(&self) -> &str {
        &self.universe
    }

    fn source(&self) -> &str {
        &self.source
    }

    fn evaluate(&self) -> Vec<MembershipSpell> {
        self.spells.clone()
    }
}

/// A deterministic feature value that does not depend on the instrument.
fn feature_value(day: usize) -> f64 {
    day as f64 * 0.25 + 1.0
}

fn bench_backfill_history(c: &mut Criterion) {
    let mut group = c.benchmark_group("membership_backfill/history");

    for instruments in INSTRUMENT_COUNTS {
        let rule = SyntheticMembership::new(instruments);
        group.throughput(Throughput::Elements(instruments as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{instruments}_instruments_10y")),
            &rule,
            |b, rule| {
                b.iter(|| black_box(MembershipSeries::from_rule(black_box(rule))));
            },
        );
    }

    group.finish();
}

fn bench_panel_over_history(c: &mut Criterion) {
    let schedule = SyntheticMembership::schedule();

    let mut group = c.benchmark_group("membership_backfill/panel");
    group.sample_size(10);
    group.throughput(Throughput::Elements(schedule.len() as u64));

    for instruments in INSTRUMENT_COUNTS {
        let rule = SyntheticMembership::new(instruments);
        let series = MembershipSeries::from_rule(&rule);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{instruments}_instruments_daily_10y")),
            &series,
            |b, series| {
                b.iter(|| {
                    let mut rows = Vec::new();
                    for (day, ts) in schedule.iter().enumerate() {
                        for instrument_id in series.members_at(*ts) {
                            rows.push(
                                PanelRow::new(instrument_id, *ts)
                                    .with_member(true)
                                    .with_feature("close", feature_value(day), *ts),
                            );
                        }
                    }
                    black_box(Panel::new(black_box(series), rows))
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_backfill_history, bench_panel_over_history);
criterion_main!(benches);
