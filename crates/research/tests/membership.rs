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

use nautilus_core::UnixNanos;
use nautilus_model::identifiers::InstrumentId;
use nautilus_persistence::backend::parquet::catalog::ParquetDataCatalog;
use nautilus_research::{MembershipInterval, MembershipSeries, Panel, PanelError, PanelRow};
use tempfile::TempDir;

fn instrument(value: &str) -> InstrumentId {
    InstrumentId::from(value)
}

fn interval(
    universe: &str,
    source: &str,
    value: &str,
    entered: u64,
    exited: Option<u64>,
) -> MembershipInterval {
    MembershipInterval::entry(
        universe.to_string(),
        source.to_string(),
        instrument(value),
        UnixNanos::from(entered),
        exited.map(UnixNanos::from),
    )
}

fn series_with_later_joiner() -> MembershipSeries {
    let mut series = MembershipSeries::new("research", "rule/v1");
    series.push(interval("research", "rule/v1", "A.X", 100, None));
    series.push(interval("research", "rule/v1", "B.X", 300, None));
    series
}

#[test]
fn panel_for_past_timestamp_excludes_a_later_joiner() {
    let series = series_with_later_joiner();
    let ts = UnixNanos::from(150);

    assert!(!series.members_at(ts).contains(&instrument("B.X")));
    assert!(!series.is_member_at(instrument("B.X"), ts));

    let rows = vec![
        PanelRow::new(instrument("A.X"), ts).with_member(true),
        PanelRow::new(instrument("B.X"), ts).with_member(false),
    ];
    let panel = Panel::new(&series, rows).unwrap();

    assert!(panel.rows()[0].member);
    assert!(!panel.rows()[1].member);
}

#[test]
fn panel_claiming_current_membership_for_a_past_timestamp_is_rejected() {
    let series = series_with_later_joiner();
    let ts = UnixNanos::from(150);

    let rows = vec![PanelRow::new(instrument("B.X"), ts).with_member(true)];
    let error = Panel::new(&series, rows).unwrap_err();

    match error {
        PanelError::MembershipMismatch {
            universe,
            instrument_id,
            claimed,
            actual,
            ..
        } => {
            assert_eq!(universe, "research");
            assert_eq!(instrument_id, instrument("B.X"));
            assert!(claimed);
            assert!(!actual);
        }
        other => panic!("expected MembershipMismatch, was {other:?}"),
    }
}

#[test]
fn membership_series_round_trips_through_the_catalog() {
    let series = series_with_later_joiner();

    let directory = TempDir::new().unwrap();
    let catalog = ParquetDataCatalog::new(directory.path(), None, None, None, None);
    series.write_to_catalog(&catalog).unwrap();

    let mut catalog = catalog;
    let restored =
        MembershipSeries::read_from_catalog(&mut catalog, "research", "rule/v1").unwrap();

    assert_eq!(restored, series);
    assert_eq!(
        restored.members_at(UnixNanos::from(150)),
        vec![instrument("A.X")]
    );
    assert_eq!(
        restored.members_at(UnixNanos::from(350)),
        vec![instrument("A.X"), instrument("B.X")],
    );
}
