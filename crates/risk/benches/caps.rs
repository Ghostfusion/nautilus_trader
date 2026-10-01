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

//! Benchmarks for count cap evaluation on the send path.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use nautilus_common::cache::Cache;
use nautilus_core::{DurationNanos, UUID4, UnixNanos};
use nautilus_model::{
    enums::{OrderSide, OrderType},
    events::{OrderAccepted, OrderEventAny},
    identifiers::{AccountId, ClientOrderId, InstrumentId, StrategyId, VenueOrderId},
    instruments::{InstrumentAny, stubs::audusd_sim},
    orders::{Order, OrderTestBuilder},
    risk::{RiskCapMetric, RiskCapScope},
    types::{Price, Quantity},
};
use nautilus_risk::engine::cap::{RiskCap, RiskCounterKey, RiskCounters, RiskSubject, evaluate};

const ACTIVE_ORDERS: usize = 64;
const WARM_OCCURRENCES: usize = 1_000;

fn window() -> DurationNanos {
    DurationNanos::from_secs(86_400)
}

/// The VeighNa daily defaults, dimensioned as caps over a rolling day.
///
/// Provenance: `vnpy_riskmanager/daily_limit_rule.py` defaults (20,000 orders, 10,000 cancels and
/// 10,000 trades in total, 2,000 orders per instrument) and `active_order_rule.py`
/// (`active_order_limit` default 50). The limits are benchmark inputs, not defaults of the
/// configuration.
fn caps(active_order_limit: u32) -> Vec<RiskCap> {
    vec![
        RiskCap::new(
            RiskCapMetric::Active,
            RiskCapScope::Global,
            active_order_limit,
            None,
        ),
        RiskCap::new(
            RiskCapMetric::Submit,
            RiskCapScope::Global,
            20_000,
            Some(window()),
        ),
        RiskCap::new(
            RiskCapMetric::Submit,
            RiskCapScope::Instrument,
            2_000,
            Some(window()),
        ),
        RiskCap::new(
            RiskCapMetric::Cancel,
            RiskCapScope::Global,
            10_000,
            Some(window()),
        ),
        RiskCap::new(
            RiskCapMetric::Fill,
            RiskCapScope::Global,
            10_000,
            Some(window()),
        ),
    ]
}

/// A cache holding the instrument and `count` open orders.
fn seeded_cache(count: usize) -> Cache {
    let mut cache = Cache::default();
    cache
        .add_instrument(InstrumentAny::CurrencyPair(audusd_sim()))
        .unwrap();

    for i in 0..count {
        let order = OrderTestBuilder::new(OrderType::Limit)
            .instrument_id(InstrumentId::from("AUD/USD.SIM"))
            .client_order_id(ClientOrderId::from(format!("O-{i:05}")))
            .side(OrderSide::Buy)
            .quantity(Quantity::from("100"))
            .price(Price::from("1.00000"))
            .build();
        cache.add_order(order.clone(), None, None, false).unwrap();

        let accepted = OrderAccepted::new(
            order.trader_id(),
            order.strategy_id(),
            order.instrument_id(),
            order.client_order_id(),
            VenueOrderId::from(format!("V-{i:05}")),
            AccountId::from("SIM-001"),
            UUID4::new(),
            UnixNanos::default(),
            UnixNanos::default(),
            false,
        );
        cache
            .update_order(&OrderEventAny::Accepted(accepted))
            .unwrap();
    }

    cache
}

fn seeded_counters(caps: &[RiskCap], subject: &RiskSubject) -> RiskCounters {
    let mut counters = RiskCounters::default();

    for cap in caps {
        let Some(subject_key) = subject.key(cap.scope) else {
            continue;
        };

        if cap.window.is_none() {
            continue;
        }

        let key = RiskCounterKey::new(cap, subject_key, None);
        for _ in 0..WARM_OCCURRENCES {
            counters.record(key.clone(), UnixNanos::default());
        }
    }

    counters
}

fn bench_caps_evaluate_send_path(c: &mut Criterion) {
    let cache = seeded_cache(ACTIVE_ORDERS);
    let subject = RiskSubject::new(
        StrategyId::from("S-001"),
        InstrumentId::from("AUD/USD.SIM"),
        Some(AccountId::from("SIM-001")),
    );

    let allow_caps = caps(u32::MAX);
    let mut allow_counters = seeded_counters(&allow_caps, &subject);

    c.bench_function("risk_caps_evaluate_allow", |b_bench| {
        b_bench.iter(|| {
            black_box(evaluate(
                black_box(&allow_caps),
                &mut allow_counters,
                black_box(&cache),
                RiskCapMetric::Submit,
                black_box(&subject),
                None,
                UnixNanos::default(),
            ))
        });
    });

    let refuse_caps = caps(1);
    let mut refuse_counters = seeded_counters(&refuse_caps, &subject);

    c.bench_function("risk_caps_evaluate_refuse", |b_bench| {
        b_bench.iter(|| {
            black_box(evaluate(
                black_box(&refuse_caps),
                &mut refuse_counters,
                black_box(&cache),
                RiskCapMetric::Submit,
                black_box(&subject),
                None,
                UnixNanos::default(),
            ))
        });
    });
}

criterion_group!(benches, bench_caps_evaluate_send_path);
criterion_main!(benches);
