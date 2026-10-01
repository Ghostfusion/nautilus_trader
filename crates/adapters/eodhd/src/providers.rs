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

//! Instrument provider for EODHD equities.

use std::{collections::HashMap, fmt::Debug};

use async_trait::async_trait;
use nautilus_common::providers::{InstrumentProvider, InstrumentStore};
use nautilus_core::time::get_atomic_clock_realtime;
use nautilus_model::{
    identifiers::{InstrumentId, Symbol, Venue},
    instruments::{Equity, InstrumentAny},
    types::{Currency, Price},
};
use ustr::Ustr;

use crate::{
    common::EODHD,
    http::{EodhdHttpClient, EodhdSymbol},
};

/// The EODHD instrument types treated as equities by [`EodhdInstrumentProvider`].
pub const EODHD_EQUITY_SYMBOL_TYPES: [&str; 2] = ["Common Stock", "ETF"];

/// Builds an [`InstrumentAny`] equity from an EODHD ticker.
///
/// The ticker carries its exchange suffix, for example `AAPL.US`. The suffix becomes the venue,
/// which makes an [`InstrumentId`] round-trip back to the EODHD ticker with no lookup table.
///
/// # Errors
///
/// Returns an error if the ticker has no exchange suffix, the venue or symbol is invalid, or the
/// price increment cannot be represented.
pub fn equity_from_ticker(
    ticker: &str,
    currency: Currency,
    price_precision: u8,
    isin: Option<Ustr>,
) -> anyhow::Result<InstrumentAny> {
    let (code, exchange) = ticker.rsplit_once('.').ok_or_else(|| {
        anyhow::anyhow!("Invalid EODHD ticker '{ticker}': expected '<code>.<exchange>'")
    })?;

    if code.is_empty() || exchange.is_empty() {
        anyhow::bail!("Invalid EODHD ticker '{ticker}': empty code or exchange");
    }

    let instrument_id = InstrumentId::new(Symbol::from(code), Venue::new_checked(exchange)?);
    let increment = 10f64.powi(-i32::from(price_precision));
    let timestamp = get_atomic_clock_realtime().get_time_ns();

    let equity = Equity::builder()
        .instrument_id(instrument_id)
        .raw_symbol(Symbol::from(ticker))
        .maybe_isin(isin)
        .currency(currency)
        .price_precision(price_precision)
        .price_increment(Price::new(increment, price_precision))
        .ts_event(timestamp)
        .ts_init(timestamp)
        .build()?;

    Ok(InstrumentAny::from(equity))
}

/// Returns whether a symbol list row is an equity.
#[must_use]
pub fn is_equity(row: &EodhdSymbol) -> bool {
    row.symbol_type.as_deref().is_none_or(|symbol_type| {
        EODHD_EQUITY_SYMBOL_TYPES
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(symbol_type))
    })
}

/// An instrument provider for EODHD equities.
///
/// This adapter has no live data client, so the provider is not driven by a client lifecycle.
/// Instrument definitions are fetched on demand and cached in the provider's [`InstrumentStore`].
#[derive(Debug)]
pub struct EodhdInstrumentProvider {
    store: InstrumentStore,
    client: EodhdHttpClient,
    currency: Currency,
    price_precision: u8,
}

impl EodhdInstrumentProvider {
    /// Creates a new [`EodhdInstrumentProvider`] instance.
    #[must_use]
    pub fn new(client: EodhdHttpClient, currency: Currency, price_precision: u8) -> Self {
        Self {
            store: InstrumentStore::new(),
            client,
            currency,
            price_precision,
        }
    }

    /// Returns a reference to the HTTP client.
    #[must_use]
    pub fn client(&self) -> &EodhdHttpClient {
        &self.client
    }

    /// Returns the default currency applied to fetched instruments.
    #[must_use]
    pub const fn currency(&self) -> Currency {
        self.currency
    }

    /// Returns the price precision applied to fetched instruments.
    #[must_use]
    pub const fn price_precision(&self) -> u8 {
        self.price_precision
    }

    /// Returns the equity for the given EODHD `ticker`.
    ///
    /// # Errors
    ///
    /// Returns an error if the ticker cannot be parsed into an equity.
    pub fn instrument_from_ticker(&self, ticker: &str) -> anyhow::Result<InstrumentAny> {
        equity_from_ticker(ticker, self.currency, self.price_precision, None)
    }

    /// Builds an equity from a symbol list `row`.
    ///
    /// # Errors
    ///
    /// Returns an error if the row has no exchange and no default can be applied, or if the
    /// resulting instrument fails validation.
    pub fn instrument_from_row(&self, row: &EodhdSymbol) -> anyhow::Result<InstrumentAny> {
        let exchange = row.exchange.as_deref().unwrap_or(EODHD);
        let ticker = format!("{}.{}", row.code, exchange);
        let currency = row
            .currency
            .as_deref()
            .map_or(self.currency, Currency::from);
        let isin = row.isin.as_deref().map(Ustr::from);

        equity_from_ticker(&ticker, currency, self.price_precision, isin)
    }

    /// Fetches and maps the equity instruments listed on `exchange`.
    ///
    /// Rows whose type is not an equity are skipped.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or the response cannot be parsed.
    pub async fn fetch_instruments(&self, exchange: &str) -> anyhow::Result<Vec<InstrumentAny>> {
        let rows = self.client.exchange_symbols(exchange).await?;
        let mut instruments = Vec::with_capacity(rows.len());

        for row in rows.iter().filter(|row| is_equity(row)) {
            instruments.push(self.instrument_from_row(row)?);
        }

        Ok(instruments)
    }

    /// Fetches the equity instruments listed on `exchange` and caches them.
    ///
    /// # Errors
    ///
    /// Returns an error if fetching fails. The store is left unchanged on error.
    pub async fn initialize(&mut self, exchange: &str) -> anyhow::Result<usize> {
        let instruments = self.fetch_instruments(exchange).await?;
        let count = instruments.len();

        self.store.add_bulk(instruments);
        self.store.set_initialized();

        Ok(count)
    }
}

#[async_trait(?Send)]
impl InstrumentProvider for EodhdInstrumentProvider {
    fn store(&self) -> &InstrumentStore {
        &self.store
    }

    fn store_mut(&mut self) -> &mut InstrumentStore {
        &mut self.store
    }

    async fn load_all(&mut self, filters: Option<&HashMap<String, String>>) -> anyhow::Result<()> {
        let exchange = filters
            .and_then(|filters| filters.get("exchange"))
            .map_or_else(|| EODHD.to_string(), ToString::to_string);

        self.initialize(&exchange).await?;

        Ok(())
    }

    async fn load(
        &mut self,
        instrument_id: &InstrumentId,
        _filters: Option<&HashMap<String, String>>,
    ) -> anyhow::Result<()> {
        if self.store.contains(instrument_id) {
            return Ok(());
        }

        let ticker = instrument_id.to_string();
        let instrument = self.instrument_from_ticker(&ticker)?;

        self.store.add(instrument);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use nautilus_model::instruments::Instrument;
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_equity_from_ticker_maps_suffix_to_venue() {
        let instrument = equity_from_ticker("AAPL.US", Currency::USD(), 2, None).unwrap();

        assert_eq!(instrument.id().to_string(), "AAPL.US");
        assert_eq!(instrument.raw_symbol().to_string(), "AAPL.US");
    }

    #[rstest]
    fn test_equity_from_ticker_round_trips_through_instrument_id() {
        let ticker = "VOD.LSE";

        let instrument = equity_from_ticker(ticker, Currency::GBP(), 2, None).unwrap();

        assert_eq!(instrument.id().to_string(), ticker);
    }

    #[rstest]
    fn test_equity_from_ticker_rejects_a_ticker_without_an_exchange() {
        let result = equity_from_ticker("AAPL", Currency::USD(), 2, None);

        assert!(result.is_err());
    }

    #[rstest]
    fn test_is_equity_filters_non_equity_rows() {
        let row = |symbol_type: Option<&str>| EodhdSymbol {
            code: "X".to_string(),
            name: None,
            country: None,
            exchange: Some("US".to_string()),
            currency: None,
            symbol_type: symbol_type.map(ToString::to_string),
            isin: None,
        };

        assert!(is_equity(&row(Some("Common Stock"))));
        assert!(is_equity(&row(Some("ETF"))));
        assert!(is_equity(&row(Some("common stock"))));
        assert!(is_equity(&row(None)));
        assert!(!is_equity(&row(Some("Mutual Fund"))));
    }
}
