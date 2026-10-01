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
use nautilus_research::{MembershipInterval, MembershipSeries, Panel, PanelError, PanelRow};

fn instrument(value: &str) -> InstrumentId {
    InstrumentId::from(value)
}

fn series() -> MembershipSeries {
    let mut series = MembershipSeries::new("research", "rule/v1");
    series.push(MembershipInterval::entry(
        "research".to_string(),
        "rule/v1".to_string(),
        instrument("A.X"),
        UnixNanos::from(100),
        None,
    ));
    series
}

#[test]
fn leaky_feature_reading_its_own_future_is_rejected() {
    let row = PanelRow::new(instrument("A.X"), UnixNanos::from(100))
        .with_feature("future_return", 0.5, UnixNanos::from(101))
        .with_member(true);

    let error = Panel::new(&series(), vec![row]).unwrap_err();

    match error {
        PanelError::Lookahead {
            feature,
            instrument_id,
            ts_event,
            as_of,
        } => {
            assert_eq!(feature, "future_return");
            assert_eq!(instrument_id, instrument("A.X"));
            assert_eq!(ts_event, UnixNanos::from(100));
            assert_eq!(as_of, UnixNanos::from(101));
        }
        other => panic!("expected Lookahead, was {other:?}"),
    }
}

#[test]
fn feature_observed_at_the_row_timestamp_is_accepted() {
    let row = PanelRow::new(instrument("A.X"), UnixNanos::from(100))
        .with_feature("mid", 42.0, UnixNanos::from(100))
        .with_label(1.5)
        .with_member(true);

    let panel = Panel::new(&series(), vec![row]).unwrap();

    panel.check().unwrap();
    assert_eq!(panel.len(), 1);
}
