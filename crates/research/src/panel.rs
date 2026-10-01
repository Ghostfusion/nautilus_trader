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

//! A point-in-time panel.
//!
//! A panel has one row per instrument and timestamp, carrying feature columns, a label column, and
//! the membership that applies at the row's timestamp. Two structural rules keep it honest, and a
//! violation is a typed error rather than a silent value:
//!
//! - A row's membership is resolved from the stored membership series at the row's timestamp, not
//!   from current membership. A row that claims membership the series does not record at that
//!   instant is rejected.
//! - A feature may not read a value observed after its own row timestamp. Each feature value
//!   carries the instant of the latest input it reads, and a feature whose aperture reaches past
//!   the row timestamp is rejected as lookahead.
//!
//! Labels are the target of prediction and may legitimately read forward, so they are not subject
//! to the feature aperture rule.

use std::collections::BTreeMap;

use nautilus_core::UnixNanos;
use nautilus_model::identifiers::InstrumentId;
use thiserror::Error;

use crate::membership::MembershipSeries;

/// A feature value together with the instant of the latest input it reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FeatureValue {
    /// The feature value.
    pub value: f64,
    /// The timestamp of the latest input the value reads.
    pub as_of: UnixNanos,
}

impl FeatureValue {
    /// Creates a new [`FeatureValue`].
    #[must_use]
    pub const fn new(value: f64, as_of: UnixNanos) -> Self {
        Self { value, as_of }
    }
}

/// One row of a point-in-time panel.
#[derive(Clone, Debug, PartialEq)]
pub struct PanelRow {
    /// The instrument this row describes.
    pub instrument_id: InstrumentId,
    /// The row timestamp, at which the features are observed.
    pub ts_event: UnixNanos,
    /// The feature columns, keyed by canonical feature name.
    pub features: BTreeMap<String, FeatureValue>,
    /// The optional label value, which may read forward.
    pub label: Option<f64>,
    /// The membership that applies at `ts_event`.
    pub member: bool,
}

impl PanelRow {
    /// Creates an empty panel row for an instrument at a timestamp.
    #[must_use]
    pub fn new(instrument_id: InstrumentId, ts_event: UnixNanos) -> Self {
        Self {
            instrument_id,
            ts_event,
            features: BTreeMap::new(),
            label: None,
            member: false,
        }
    }

    /// Sets a feature column, returning the row.
    #[must_use]
    pub fn with_feature(mut self, name: impl Into<String>, value: f64, as_of: UnixNanos) -> Self {
        self.features
            .insert(name.into(), FeatureValue::new(value, as_of));
        self
    }

    /// Sets the label, returning the row.
    #[must_use]
    pub fn with_label(mut self, label: f64) -> Self {
        self.label = Some(label);
        self
    }

    /// Sets the membership that applies at `ts_event`, returning the row.
    #[must_use]
    pub fn with_member(mut self, member: bool) -> Self {
        self.member = member;
        self
    }
}

/// A structural violation of the point-in-time panel contract.
#[derive(Clone, Debug, Error, PartialEq)]
pub enum PanelError {
    /// A feature reads a value observed after the row timestamp.
    #[error(
        "feature `{feature}` for {instrument_id} at {ts_event} reads a value observed at {as_of}, \
         which is after the row timestamp"
    )]
    Lookahead {
        /// The instrument of the offending row.
        instrument_id: InstrumentId,
        /// The row timestamp.
        ts_event: UnixNanos,
        /// The offending feature name.
        feature: String,
        /// The timestamp of the value the feature reads.
        as_of: UnixNanos,
    },
    /// A row claims membership the stored series does not record at the row timestamp.
    #[error(
        "membership for {instrument_id} at {ts_event} in universe `{universe}` is {claimed}, \
         but the stored series records {actual}"
    )]
    MembershipMismatch {
        /// The universe identity.
        universe: String,
        /// The instrument of the offending row.
        instrument_id: InstrumentId,
        /// The row timestamp.
        ts_event: UnixNanos,
        /// The membership the row claims.
        claimed: bool,
        /// The membership the stored series records.
        actual: bool,
    },
}

/// A point-in-time panel.
///
/// Construct with [`Panel::new`], which runs [`Panel::check`] so an invalid panel cannot be built
/// through the public API.
#[derive(Clone, Debug, PartialEq)]
pub struct Panel {
    rows: Vec<PanelRow>,
    membership: MembershipSeries,
}

impl Panel {
    /// Creates a panel from rows and resolves every row's membership from `membership`.
    ///
    /// # Errors
    ///
    /// Returns a [`PanelError`] if a feature reads past its row timestamp or a row's membership
    /// disagrees with the stored series.
    pub fn new(membership: &MembershipSeries, rows: Vec<PanelRow>) -> Result<Self, PanelError> {
        let panel = Self {
            rows,
            membership: membership.clone(),
        };
        panel.check()?;
        Ok(panel)
    }

    /// Checks both structural rules: membership is point-in-time and no feature reads the future.
    ///
    /// # Errors
    ///
    /// Returns a [`PanelError`] on the first violation found.
    pub fn check(&self) -> Result<(), PanelError> {
        for row in &self.rows {
            for (feature, value) in &row.features {
                if value.as_of > row.ts_event {
                    return Err(PanelError::Lookahead {
                        instrument_id: row.instrument_id,
                        ts_event: row.ts_event,
                        feature: feature.clone(),
                        as_of: value.as_of,
                    });
                }
            }
        }

        for row in &self.rows {
            let actual = self
                .membership
                .is_member_at(row.instrument_id, row.ts_event);
            if actual != row.member {
                return Err(PanelError::MembershipMismatch {
                    universe: self.membership.universe().to_string(),
                    instrument_id: row.instrument_id,
                    ts_event: row.ts_event,
                    claimed: row.member,
                    actual,
                });
            }
        }

        Ok(())
    }

    /// Returns the panel rows.
    #[must_use]
    pub fn rows(&self) -> &[PanelRow] {
        &self.rows
    }

    /// Returns the universe identity of the panel's membership.
    #[must_use]
    pub fn universe(&self) -> &str {
        self.membership.universe()
    }

    /// Returns the number of rows.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Returns whether the panel has no rows.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}
