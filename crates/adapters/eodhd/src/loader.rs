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

//! Historical data loader for EODHD end-of-day market data.

use std::fmt::Debug;

use jiff::{civil::Date, tz::Offset};
use nautilus_core::UnixNanos;
use nautilus_model::{
    data::{Bar, BarSpecification, BarType},
    enums::{AggregationSource, BarAggregation, PriceType},
    identifiers::InstrumentId,
    instruments::InstrumentAny,
    types::{Currency, Price, Quantity},
};

use crate::{
    http::{EodhdBar, EodhdHttpClient},
    providers::equity_from_ticker,
};

/// The default bar price precision applied when none is supplied.
pub const EODHD_DEFAULT_PRICE_PRECISION: u8 = 2;

/// The size precision applied to EODHD share volumes.
const VOLUME_PRECISION: u8 = 0;

/// Parses an EODHD `period` code into a bar step and aggregation.
///
/// # Errors
///
/// Returns an error if `period` is not one of `d`, `w`, or `m`.
pub fn parse_period(period: &str) -> anyhow::Result<(usize, BarAggregation)> {
    match period.to_ascii_lowercase().as_str() {
        "" | "d" | "day" | "daily" => Ok((1, BarAggregation::Day)),
        "w" | "week" | "weekly" => Ok((1, BarAggregation::Week)),
        "m" | "month" | "monthly" => Ok((1, BarAggregation::Month)),
        other => {
            anyhow::bail!("Unsupported EODHD period '{other}': expected one of 'd', 'w', or 'm'")
        }
    }
}

/// Returns the UTC millisecond timestamp for an EODHD `YYYY-MM-DD` date row.
///
/// # Errors
///
/// Returns an error if the date cannot be parsed.
pub fn date_to_millis(value: &str) -> anyhow::Result<i64> {
    let date = Date::strptime("%Y-%m-%d", value).map_err(|e| anyhow::anyhow!("{e}"))?;
    let timestamp = Offset::UTC
        .to_timestamp(date.at(0, 0, 0, 0))
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    Ok(timestamp.as_millisecond())
}

/// Converts EODHD end-of-day rows into Nautilus [`Bar`] objects.
///
/// The rows are converted in the order supplied, which for `/eod` is ascending by date.
///
/// # Errors
///
/// Returns an error if a row date is unparseable, or if a row violates an OHLC relationship, in
/// which case [`Bar::new_checked`] reports the offending field rather than filling a bar with
/// values the provider never published.
pub fn build_bars(
    rows: &[EodhdBar],
    instrument_id: InstrumentId,
    period: &str,
    price_precision: u8,
) -> anyhow::Result<Vec<Bar>> {
    let (step, aggregation) = parse_period(period)?;
    let spec = BarSpecification::new(step, aggregation, PriceType::Last);
    let bar_type = BarType::new(instrument_id, spec, AggregationSource::External);

    let mut bars = Vec::with_capacity(rows.len());

    for row in rows {
        let millis = date_to_millis(&row.date)?;
        let ts_event = UnixNanos::from_millis(millis.unsigned_abs());

        let open = Price::new_checked(row.open, price_precision)?;
        let high = Price::new_checked(row.high, price_precision)?;
        let low = Price::new_checked(row.low, price_precision)?;
        let close = Price::new_checked(row.close, price_precision)?;
        let volume = Quantity::new_checked(row.volume.unwrap_or(0.0), VOLUME_PRECISION)?;

        bars.push(Bar::new_checked(
            bar_type, open, high, low, close, volume, ts_event, ts_event,
        )?);
    }

    Ok(bars)
}

/// A historical data loader for EODHD end-of-day market data.
///
/// Converts EODHD rows into Nautilus [`Bar`] and [`InstrumentAny`] objects. This adapter has no
/// live data client, so the loader is driven directly: from Rust through its methods, and from
/// Python through the bindings it exposes.
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
        let rows = self.client.exchange_symbols(exchange).await?;
        let mut instruments = Vec::with_capacity(rows.len());

        for row in rows.iter().filter(|row| crate::providers::is_equity(row)) {
            let exchange = row.exchange.as_deref().unwrap_or(exchange);
            let ticker = format!("{}.{}", row.code, exchange);
            let currency = row
                .currency
                .as_deref()
                .map_or(self.currency, Currency::from);

            instruments.push(equity_from_ticker(
                &ticker,
                currency,
                self.price_precision,
                None,
            )?);
        }

        Ok(instruments)
    }

    /// Returns the historical bars for `instrument_id` between `start` and `end` inclusive.
    ///
    /// `start` and `end` are `YYYY-MM-DD` dates. `period` selects the aggregation: `d`, `w`, or
    /// `m`. The EODHD ticker is derived from the instrument ID, so the venue carries the exchange.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or a row cannot be converted into a bar.
    pub async fn bars(
        &self,
        instrument_id: InstrumentId,
        start: &str,
        end: &str,
        period: &str,
    ) -> anyhow::Result<Vec<Bar>> {
        let ticker = instrument_id.to_string();
        let rows = self
            .client
            .eod_bars(&ticker, Some(start), Some(end), period)
            .await?;

        build_bars(&rows, instrument_id, period, self.price_precision)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn row(date: &str, close: f64) -> EodhdBar {
        EodhdBar {
            date: date.to_string(),
            open: close - 1.0,
            high: close + 1.0,
            low: close - 2.0,
            close,
            adjusted_close: None,
            volume: Some(1_000.0),
        }
    }

    #[rstest]
    fn test_parse_period_accepts_the_documented_codes() {
        assert_eq!(parse_period("d").unwrap(), (1, BarAggregation::Day));
        assert_eq!(parse_period("w").unwrap(), (1, BarAggregation::Week));
        assert_eq!(parse_period("m").unwrap(), (1, BarAggregation::Month));
        assert_eq!(parse_period("").unwrap(), (1, BarAggregation::Day));
    }

    #[rstest]
    fn test_parse_period_rejects_an_unknown_code() {
        assert!(parse_period("x").is_err());
    }

    #[rstest]
    fn test_date_to_millis_is_utc_midnight() {
        assert_eq!(date_to_millis("1970-01-01").unwrap(), 0);
        assert_eq!(date_to_millis("1970-01-02").unwrap(), 86_400_000);
        assert_eq!(date_to_millis("2024-01-02").unwrap(), 1_704_153_600_000);
    }

    #[rstest]
    fn test_date_to_millis_rejects_a_malformed_date() {
        assert!(date_to_millis("02/01/2024").is_err());
    }

    #[rstest]
    fn test_build_bars_preserves_the_supplied_order() {
        let instrument_id = InstrumentId::from("AAPL.US");
        let rows = vec![row("2024-01-02", 185.64), row("2024-01-03", 184.25)];

        let bars = build_bars(&rows, instrument_id, "d", 2).unwrap();

        assert_eq!(bars.len(), 2);
        assert_eq!(bars[0].bar_type.to_string(), "AAPL.US-1-DAY-LAST-EXTERNAL");
        assert!(bars[0].ts_event < bars[1].ts_event);
        assert_eq!(bars[0].close, Price::new(185.64, 2));
        assert_eq!(bars[0].volume, Quantity::new(1_000.0, 0));
    }

    #[rstest]
    fn test_build_bars_maps_the_period_to_the_aggregation() {
        let instrument_id = InstrumentId::from("AAPL.US");
        let rows = vec![row("2024-01-02", 185.64)];

        let weekly = build_bars(&rows, instrument_id, "w", 2).unwrap();
        let monthly = build_bars(&rows, instrument_id, "m", 2).unwrap();

        assert_eq!(
            weekly[0].bar_type.to_string(),
            "AAPL.US-1-WEEK-LAST-EXTERNAL"
        );
        assert_eq!(
            monthly[0].bar_type.to_string(),
            "AAPL.US-1-MONTH-LAST-EXTERNAL"
        );
    }

    #[rstest]
    fn test_build_bars_rejects_a_row_with_an_impossible_high() {
        let instrument_id = InstrumentId::from("AAPL.US");
        let mut invalid = row("2024-01-02", 185.64);
        invalid.high = invalid.close - 5.0;

        assert!(build_bars(&[invalid], instrument_id, "d", 2).is_err());
    }

    #[rstest]
    fn test_build_bars_treats_a_missing_volume_as_zero() {
        let instrument_id = InstrumentId::from("AAPL.US");
        let mut no_volume = row("2024-01-02", 185.64);
        no_volume.volume = None;

        let bars = build_bars(&[no_volume], instrument_id, "d", 2).unwrap();

        assert_eq!(bars[0].volume, Quantity::new(0.0, 0));
    }
}
