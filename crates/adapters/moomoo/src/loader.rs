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

//! Loading a market's instrument universe.
//!
//! One instrument is two requests, and a market is one request and then as many snapshot requests as
//! it takes to cover the market four hundred securities at a time. The two paths make different
//! promises, and the difference is measured rather than assumed.
//!
//! A snapshot is the only record carrying the spread the price precision is derived from, so it is
//! what makes an instrument's tick authoritative. For one instrument the venue always has one. For a
//! market it does not: a snapshot batch is refused as a whole when one of its members cannot be
//! priced, and the United States list carries over-the-counter codes throughout, so read live every
//! batch across the whole list was refused, each naming a different code, while every Hong Kong
//! batch was served.
//!
//! A refused batch therefore falls back to static information for its members, which is a weaker
//! instrument and not a wrong one: the market's own fallback precision is used, and the log says how
//! many instruments that happened to.

use std::{collections::HashMap, ops::Range};

use nautilus_model::instruments::InstrumentAny;

use crate::{
    common::Market,
    connection::Connection,
    generated::{
        qot_common::{Security, SecurityStaticInfo},
        qot_get_security_snapshot::SnapshotBasicData,
    },
    mappers::instrument::{instrument_from, is_modelled},
    providers::{self, MAX_SNAPSHOT_BATCH},
};

/// What loading one market produced.
#[derive(Debug, Default)]
pub struct MarketLoad {
    /// The instruments the market's list resolved to.
    pub instruments: Vec<InstrumentAny>,
    /// How many of them were priced from the venue's own spread.
    pub priced: usize,
    /// The codes the venue listed that could not be built into an instrument.
    pub skipped: Vec<String>,
}

impl MarketLoad {
    /// Returns how many instruments carry the market's fallback precision.
    #[must_use]
    pub fn unpriced(&self) -> usize {
        self.instruments.len() - self.priced
    }
}

/// Returns the index ranges a count is split into, none larger than `size`.
///
/// The last range is short rather than padded, because a range that named a security past the end
/// would be a request for something that does not exist.
fn batches(total: usize, size: usize) -> Vec<Range<usize>> {
    let size = size.max(1);

    (0..total)
        .step_by(size)
        .map(|start| start..(start + size).min(total))
        .collect()
}

/// Builds one market's instruments from its static records and the snapshots that were served.
///
/// The two are paired by the security each names rather than by their position in the answers: a
/// refused batch is one the venue sent nothing for, so the lists stop lining up as soon as a batch
/// is refused, and pairing by position would give every later instrument the wrong spread.
///
/// A record whose security type this adapter does not model is not an instrument it can build, and
/// one that fails validation is left out rather than approximated. Both are reported by code so
/// that an operator can see what the venue listed and the adapter did not take.
///
/// Returns the instruments, how many carry the venue's own spread, and the codes left out.
#[must_use]
pub fn build_instruments(
    statics: &[SecurityStaticInfo],
    snapshots: &[SnapshotBasicData],
    market: Market,
) -> (Vec<InstrumentAny>, usize, Vec<String>) {
    let served: HashMap<&str, &SnapshotBasicData> = snapshots
        .iter()
        .map(|snapshot| (snapshot.security.code.as_str(), snapshot))
        .collect();

    let mut instruments = Vec::new();
    let mut priced = 0;
    let mut skipped = Vec::new();

    for static_info in statics {
        if !is_modelled(static_info.basic.sec_type) {
            continue;
        }

        let code = static_info.basic.security.code.as_str();
        let snapshot = served.get(code).copied();

        match instrument_from(static_info, snapshot) {
            Ok(instrument) => {
                if snapshot.is_some() {
                    priced += 1;
                }

                instruments.push(instrument);
            }
            Err(e) => {
                log::debug!("leaving out {}.{code}: {e}", market.code());
                skipped.push(code.to_string());
            }
        }
    }

    (instruments, priced, skipped)
}

/// Loads the instruments of one market.
///
/// When `snapshots` is set, the modelled securities are snapshotted in batches the venue accepts and
/// a batch it refuses falls back to static information. This is the one place where a refusal is
/// expected rather than reported as a failure: the venue's answer for a batch is its answer for the
/// batch, and there is nothing to retry.
///
/// # Errors
///
/// Returns an error if the market's static list cannot be read, which is a failure of the request
/// rather than an answer about its contents.
pub async fn load_market(
    connection: &Connection,
    market: Market,
    snapshots: bool,
) -> anyhow::Result<MarketLoad> {
    let statics = providers::request_market_static_info(connection, market).await?;
    let securities: Vec<Security> = statics
        .iter()
        .filter(|record| is_modelled(record.basic.sec_type))
        .map(|record| record.basic.security.clone())
        .collect();

    let mut served: Vec<SnapshotBasicData> = Vec::new();
    let mut refused_batches = 0;

    if snapshots && !securities.is_empty() {
        for range in batches(securities.len(), MAX_SNAPSHOT_BATCH) {
            match providers::request_snapshots(connection, &securities[range.clone()]).await {
                Ok(records) => served.extend(records),
                Err(e) => {
                    refused_batches += 1;
                    log::debug!(
                        "the venue refused a snapshot batch of {} {} securities: {e}",
                        range.len(),
                        market.code()
                    );
                }
            }
        }
    }

    let (instruments, priced, skipped) = build_instruments(&statics, &served, market);

    if snapshots && refused_batches > 0 {
        log::info!(
            "{} of {} {} securities were priced from the venue's own spread, and {} carry the \
             market's fallback precision because the venue refused {refused_batches} snapshot \
             batch(es)",
            priced,
            instruments.len(),
            market.code(),
            instruments.len() - priced
        );
    }

    Ok(MarketLoad {
        instruments,
        priced,
        skipped,
    })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(0, 400, 0)]
    #[case(1, 400, 1)]
    #[case(400, 400, 1)]
    #[case(401, 400, 2)]
    #[case(1000, 400, 3)]
    fn test_batches_cover_every_index_exactly_once(
        #[case] total: usize,
        #[case] size: usize,
        #[case] expected: usize,
    ) {
        let ranges = batches(total, size);

        assert_eq!(ranges.len(), expected);

        let covered: Vec<usize> = ranges.iter().flat_map(|range| range.clone()).collect();

        assert_eq!(covered, (0..total).collect::<Vec<_>>());
    }

    #[rstest]
    fn test_batches_never_exceed_the_size() {
        for range in batches(1000, 400) {
            assert!(range.len() <= 400);
        }
    }

    #[rstest]
    fn test_a_zero_size_still_terminates() {
        // A size of zero would step forever, so it is raised to one rather than trusted.
        assert_eq!(batches(3, 0).len(), 3);
    }
}
