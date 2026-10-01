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

//! Python bindings for the EODHD historical data loader.

use nautilus_core::python::{to_pyruntime_err, to_pyvalue_err};
use nautilus_model::{
    identifiers::InstrumentId, python::instruments::instrument_any_to_pyobject, types::Currency,
};
use pyo3::{IntoPyObjectExt, prelude::*};

use crate::loader::EodhdDataLoader;

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl EodhdDataLoader {
    /// A historical data loader for EODHD end-of-day market data.
    ///
    /// The API token is read from the `api_key` argument, falling back to the
    /// `EODHD_API_KEY` environment variable.
    ///
    /// # Errors
    ///
    /// Returns an error if no API token is available or the HTTP client cannot be built.
    #[new]
    #[pyo3(signature = (api_key=None, base_url=None, currency=None, price_precision=None, timeout_secs=None, proxy_url=None))]
    fn py_new(
        api_key: Option<&str>,
        base_url: Option<&str>,
        currency: Option<&str>,
        price_precision: Option<u8>,
        timeout_secs: Option<u64>,
        proxy_url: Option<String>,
    ) -> PyResult<Self> {
        Self::new(
            api_key,
            base_url,
            currency.map(Currency::from),
            price_precision,
            timeout_secs,
            proxy_url,
        )
        .map_err(to_pyvalue_err)
    }

    /// Returns a masked version of the API token, for logging.
    #[must_use]
    #[pyo3(name = "api_key_masked")]
    fn py_api_key_masked(&self) -> String {
        self.client().api_key_masked()
    }

    /// Returns the equity instrument for an EODHD `ticker`.
    ///
    /// The ticker carries its exchange suffix, for example `AAPL.US`, which becomes the venue.
    /// No network request is made.
    ///
    /// # Errors
    ///
    /// Returns an error if the ticker cannot be parsed into an equity.
    #[pyo3(name = "instrument")]
    fn py_instrument(&self, py: Python<'_>, ticker: &str) -> PyResult<Py<PyAny>> {
        let instrument = self.instrument(ticker).map_err(to_pyvalue_err)?;

        instrument_any_to_pyobject(py, instrument)
    }

    /// Returns the equity instruments listed on `exchange`, an EODHD exchange code.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or a row cannot be mapped.
    #[pyo3(name = "instruments")]
    #[pyo3(signature = (exchange))]
    fn py_instruments<'py>(&self, py: Python<'py>, exchange: &str) -> PyResult<Bound<'py, PyAny>> {
        let this = self.clone();
        let exchange = exchange.to_string();

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let instruments = this
                .instruments(&exchange)
                .await
                .map_err(to_pyruntime_err)?;

            Python::attach(|py| {
                let mut objects = Vec::with_capacity(instruments.len());

                for instrument in instruments {
                    objects.push(instrument_any_to_pyobject(py, instrument)?);
                }

                objects.into_py_any(py)
            })
        })
    }

    /// Returns the historical bars for `instrument_id` between `start` and `end` inclusive.
    ///
    /// `start` and `end` are `YYYY-MM-DD` dates. `period` selects the aggregation: `d`, `w`, or
    /// `m`, defaulting to `d`. The EODHD ticker is derived from the instrument ID, so the venue
    /// carries the exchange.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or a row cannot be converted into a bar.
    #[pyo3(name = "bars")]
    #[pyo3(signature = (instrument_id, start, end, period=None))]
    fn py_bars<'py>(
        &self,
        py: Python<'py>,
        instrument_id: InstrumentId,
        start: &str,
        end: &str,
        period: Option<&str>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let this = self.clone();
        let start = start.to_string();
        let end = end.to_string();
        let period = period.unwrap_or("d").to_string();

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let bars = this
                .bars(instrument_id, &start, &end, &period)
                .await
                .map_err(to_pyruntime_err)?;

            Python::attach(|py| bars.into_py_any(py))
        })
    }
}
