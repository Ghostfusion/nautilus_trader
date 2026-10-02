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

//! Historical data loader for EODHD end-of-day and intraday market data.

use std::fmt::Debug;

use nautilus_core::time::get_atomic_clock_realtime;
use nautilus_model::{
    data::{Bar, CorporateAction},
    identifiers::InstrumentId,
    instruments::InstrumentAny,
    types::Currency,
};

use crate::{
    bars::{
        bar_type_for, build_eod_bars, build_intraday_bars, date_end_secs, date_start_secs,
        parse_interval,
    },
    common::EODHD_DEFAULT_PRICE_PRECISION,
    corporate_actions::{action_from_dividend, action_from_split},
    http::EodhdHttpClient,
    providers::{equity_from_ticker, fetch_exchange_equities},
};

/// A historical data loader for EODHD end-of-day and intraday market data.
///
/// Converts EODHD rows into Nautilus [`Bar`] and [`InstrumentAny`] objects. This adapter has no
/// dependency on a running node: the loader is driven directly, from Rust through its methods and
/// from Python through the bindings it exposes. For streaming into a running node, see the data
/// client in [`crate::data`].
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.adapters.eodhd", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.adapters.eodhd")
)]
#[derive(Debug, Clone)]
pub struct EodhdDataLoader {
    client: EodhdHttpClient,
    currency: Currency,
    price_precision: u8,
}

impl EodhdDataLoader {
    /// Creates a new [`EodhdDataLoader`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if no API token is provided by argument or the `EODHD_API_KEY`
    /// environment variable, or if the HTTP client cannot be built.
    pub fn new(
        api_key: Option<&str>,
        base_url: Option<&str>,
        currency: Option<Currency>,
        price_precision: Option<u8>,
        timeout_secs: Option<u64>,
        proxy_url: Option<String>,
    ) -> anyhow::Result<Self> {
        let client = EodhdHttpClient::new(api_key, base_url, timeout_secs, proxy_url)?;

        Ok(Self {
            client,
            currency: currency.unwrap_or_else(Currency::USD),
            price_precision: price_precision.unwrap_or(EODHD_DEFAULT_PRICE_PRECISION),
        })
    }

    /// Returns a reference to the HTTP client.
    #[must_use]
    pub fn client(&self) -> &EodhdHttpClient {
        &self.client
    }

    /// Returns the price precision applied to loaded bars.
    #[must_use]
    pub const fn price_precision(&self) -> u8 {
        self.price_precision
    }

    /// Returns the equity instrument for an EODHD `ticker`.
    ///
    /// No network request is made: the ticker carries its own exchange suffix.
    ///
    /// # Errors
    ///
    /// Returns an error if the ticker cannot be parsed into an equity.
    pub fn instrument(&self, ticker: &str) -> anyhow::Result<InstrumentAny> {
        equity_from_ticker(ticker, self.currency, self.price_precision, None)
    }

    /// Returns the equity instruments listed on `exchange`.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or a row cannot be mapped.
    pub async fn instruments(&self, exchange: &str) -> anyhow::Result<Vec<InstrumentAny>> {
        fetch_exchange_equities(&self.client, exchange, self.currency, self.price_precision).await
    }

    /// Returns the historical bars for `instrument_id` between `start` and `end` inclusive.
    ///
    /// `start` and `end` are `YYYY-MM-DD` dates, and both are inclusive. `period` selects the
    /// interval: `d`, `w`, or `m` are served by the end-of-day endpoint, and `1m`, `5m`, or `1h`
    /// by the intraday endpoint. The EODHD ticker is derived from the instrument ID, so the venue
    /// carries the exchange.
    ///
    /// # Errors
    ///
    /// Returns an error if the period is unsupported, the request fails, or a row cannot be
    /// converted into a bar.
    pub async fn bars(
        &self,
        instrument_id: InstrumentId,
        start: &str,
        end: &str,
        period: &str,
    ) -> anyhow::Result<Vec<Bar>> {
        let interval = parse_interval(period)?;
        let bar_type = bar_type_for(instrument_id, interval);
        let ticker = instrument_id.to_string();

        if interval.is_intraday() {
            let from = date_start_secs(start)?;
            let to = date_end_secs(end)?;
            let rows = self
                .client
                .intraday_bars(&ticker, from, to, interval.code())
                .await?;

            return build_intraday_bars(&rows, bar_type, self.price_precision);
        }

        let rows = self
            .client
            .eod_bars(&ticker, Some(start), Some(end), interval.code())
            .await?;

        build_eod_bars(&rows, bar_type, self.price_precision)
    }

    /// Returns the corporate actions EODHD reports for `instrument_id`.
    ///
    /// `start` and `end` are `YYYY-MM-DD` dates bounding the action date, and both are inclusive.
    /// The dividends and splits are returned together, ordered by the time they take effect.
    ///
    /// A dividend carries the as-reported cash amount per share, and a split carries the new shares
    /// per old share, both as an exact decimal.
    ///
    /// # Errors
    ///
    /// Returns an error if a request fails or a row cannot be converted.
    pub async fn corporate_actions(
        &self,
        instrument_id: InstrumentId,
        start: &str,
        end: &str,
    ) -> anyhow::Result<Vec<CorporateAction>> {
        let ticker = instrument_id.to_string();
        let ts_event = get_atomic_clock_realtime().get_time_ns();
        let mut actions = Vec::new();

        for row in self
            .client
            .dividends(&ticker, Some(start), Some(end))
            .await?
        {
            actions.push(action_from_dividend(instrument_id, &row, ts_event)?);
        }

        for row in self.client.splits(&ticker, Some(start), Some(end)).await? {
            actions.push(action_from_split(instrument_id, &row, ts_event)?);
        }

        actions.sort_by_key(|action| action.effective_ns);

        Ok(actions)
    }
}

#[cfg(test)]
mod tests {
    use nautilus_model::instruments::Instrument;
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_loader_rejects_a_ticker_without_an_exchange() {
        let loader =
            EodhdDataLoader::new(Some("test-token"), None, None, None, None, None).unwrap();

        assert!(loader.instrument("AAPL").is_err());
    }

    #[rstest]
    fn test_loader_defaults_to_precision_two_and_maps_the_venue() {
        let loader =
            EodhdDataLoader::new(Some("test-token"), None, None, None, None, None).unwrap();

        let instrument = loader.instrument("AAPL.US").unwrap();

        assert_eq!(loader.price_precision(), 2);
        assert_eq!(instrument.id().to_string(), "AAPL.US");
    }

    #[rstest]
    fn test_loader_honours_an_explicit_precision() {
        let loader =
            EodhdDataLoader::new(Some("test-token"), None, None, Some(4), None, None).unwrap();

        assert_eq!(loader.price_precision(), 4);
    }
}
