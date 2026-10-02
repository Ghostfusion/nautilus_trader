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

//! The EODHD REST API client.

use std::{collections::HashMap, fmt::Debug, io::Read};

use flate2::read::GzDecoder;
use nautilus_network::http::{
    HttpClient, HttpRedirectPolicy, HttpResponse, Method, create_standard_nautilus_headers,
};
use serde::de::DeserializeOwned;

use super::{
    error::{Error, Result},
    models::{
        EodhdBar, EodhdBulkBar, EodhdDelayedQuote, EodhdDelayedQuoteResponse, EodhdErrorResponse,
        EodhdIntradayBar, EodhdSymbol,
    },
};
use crate::common::{
    Credential, EODHD_API_KEY, EODHD_HTTP_BASE_URL, EODHD_HTTP_TIMEOUT_SECS, EODHD_REST_QUOTA,
    EODHD_REST_RATE_KEY,
};

const API_TOKEN: &str = "api_token";
const FORMAT_PARAM: &str = "fmt";
const FORMAT_JSON: &str = "json";
const SUCCESS_STATUS: u16 = 200;
const CONTENT_ENCODING: &str = "content-encoding";
const GZIP: &str = "gzip";
const MAX_DECOMPRESSED_BYTES: u64 = 64 * 1024 * 1024;
const BODY_PREVIEW_BYTES: usize = 120;

/// An EODHD REST API client.
///
/// The API token is passed as the `api_token` query parameter, which is how EODHD authenticates
/// requests; there is no authorization header. See <https://eodhd.com/financial-apis/>.
#[derive(Clone)]
pub struct EodhdHttpClient {
    base_url: String,
    credential: Credential,
    client: HttpClient,
}

impl Debug for EodhdHttpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(EodhdHttpClient))
            .field("base_url", &self.base_url)
            .field("credential", &self.credential)
            .finish()
    }
}

impl EodhdHttpClient {
    /// Creates a new [`EodhdHttpClient`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if no API token is provided by argument or the `EODHD_API_KEY`
    /// environment variable, or if the HTTP client cannot be built.
    pub fn new(
        api_key: Option<&str>,
        base_url: Option<&str>,
        timeout_secs: Option<u64>,
        proxy_url: Option<String>,
    ) -> anyhow::Result<Self> {
        let credential =
            Credential::resolve(api_key.map(ToString::to_string)).ok_or(Error::MissingApiKey {
                env_var: EODHD_API_KEY,
            })?;

        let base_url =
            base_url.map_or_else(|| EODHD_HTTP_BASE_URL.to_string(), ToString::to_string);

        let mut headers: HashMap<String, String> =
            create_standard_nautilus_headers().into_iter().collect();
        headers.insert("Accept-Encoding".to_string(), GZIP.to_string());

        let keyed_quotas = vec![(EODHD_REST_RATE_KEY.to_string(), *EODHD_REST_QUOTA)];
        let client = HttpClient::builder()
            .redirect_policy(HttpRedirectPolicy::Reject)
            .headers(headers)
            .keyed_quotas(keyed_quotas)
            .default_quota(*EODHD_REST_QUOTA)
            .maybe_timeout_secs(Some(timeout_secs.unwrap_or(EODHD_HTTP_TIMEOUT_SECS)))
            .maybe_proxy_url(proxy_url)
            .header_keys(vec![CONTENT_ENCODING.to_string()])
            .build()?;

        Ok(Self {
            base_url,
            credential,
            client,
        })
    }

    /// Returns a masked version of the API token for logging purposes.
    #[must_use]
    pub fn api_key_masked(&self) -> String {
        self.credential.api_key_masked()
    }

    /// Returns the end-of-day bars for `ticker` between `from` and `to` inclusive.
    ///
    /// `period` selects the aggregation: `d` for daily, `w` for weekly, `m` for monthly. The
    /// `ticker` carries the exchange suffix, for example `AAPL.US`.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails, the token is rejected, or the body does not parse.
    pub async fn eod_bars(
        &self,
        ticker: &str,
        from: Option<&str>,
        to: Option<&str>,
        period: &str,
    ) -> Result<Vec<EodhdBar>> {
        let mut params = self.base_params();
        params.insert("period".to_string(), vec![period.to_string()]);

        if let Some(from) = from {
            params.insert("from".to_string(), vec![from.to_string()]);
        }

        if let Some(to) = to {
            params.insert("to".to_string(), vec![to.to_string()]);
        }

        let url = format!("{}/eod/{ticker}", self.base_url);
        let body = self.request(&url, &params).await?;
        parse_json_list::<EodhdBar>(&body, &url)
    }

    /// Returns the intraday bars for `ticker` between `from` and `to` inclusive.
    ///
    /// `from` and `to` are epoch seconds. `interval` is one of `1m`, `5m`, or `1h`; EODHD rejects
    /// any other value with an HTTP 422.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails, the token is rejected, or the body does not parse.
    pub async fn intraday_bars(
        &self,
        ticker: &str,
        from: i64,
        to: i64,
        interval: &str,
    ) -> Result<Vec<EodhdIntradayBar>> {
        let mut params = self.base_params();
        params.insert("interval".to_string(), vec![interval.to_string()]);
        params.insert("from".to_string(), vec![from.to_string()]);
        params.insert("to".to_string(), vec![to.to_string()]);

        let url = format!("{}/intraday/{ticker}", self.base_url);
        let body = self.request(&url, &params).await?;

        parse_json_list::<EodhdIntradayBar>(&body, &url)
    }

    /// Returns the last day of bars for every symbol listed on `exchange`.
    ///
    /// One request covers a whole exchange, which is what keeps a large daily universe current
    /// on one request per poll. The endpoint offers no filter for a subset of symbols, so the
    /// full list is always returned and the caller selects the rows it wants.
    ///
    /// `date` selects a trading day in `YYYY-MM-DD` format; without it the endpoint returns its
    /// most recent day, which during a session is the forming bar for the current day.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails, the token is not entitled to the endpoint, or the
    /// body does not parse.
    pub async fn bulk_last_day(
        &self,
        exchange: &str,
        date: Option<&str>,
    ) -> Result<Vec<EodhdBulkBar>> {
        let mut params = self.base_params();

        if let Some(date) = date {
            params.insert("date".to_string(), vec![date.to_string()]);
        }

        let url = format!("{}/eod-bulk-last-day/{exchange}", self.base_url);
        let body = self.request(&url, &params).await?;

        parse_json_list::<EodhdBulkBar>(&body, &url)
    }

    /// Returns the delayed quote snapshot for `ticker`.
    ///
    /// EODHD describes the instrument alongside the quote and returns the row keyed by ticker, so
    /// the first row of the response is taken.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails, the token is not entitled to the endpoint, or the
    /// response carries no quote for `ticker`.
    pub async fn delayed_quote(&self, ticker: &str) -> Result<EodhdDelayedQuote> {
        let mut params = self.base_params();
        params.insert("s".to_string(), vec![ticker.to_string()]);

        let url = format!("{}/us-quote-delayed", self.base_url);
        let body = self.request(&url, &params).await?;

        parse_delayed_quote(&body, &url, ticker)
    }

    /// Returns the symbol list for `exchange`, where `exchange` is an EODHD exchange code.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails, the token is rejected, or the body does not parse.
    pub async fn exchange_symbols(&self, exchange: &str) -> Result<Vec<EodhdSymbol>> {
        let params = self.base_params();
        let url = format!("{}/exchange-symbol-list/{exchange}", self.base_url);
        let body = self.request(&url, &params).await?;
        parse_json_list::<EodhdSymbol>(&body, &url)
    }

    fn base_params(&self) -> HashMap<String, Vec<String>> {
        let mut params = HashMap::new();
        params.insert(
            API_TOKEN.to_string(),
            vec![self.credential.api_key().to_string()],
        );
        params.insert(FORMAT_PARAM.to_string(), vec![FORMAT_JSON.to_string()]);
        params
    }

    async fn request(&self, url: &str, params: &HashMap<String, Vec<String>>) -> Result<Vec<u8>> {
        let keys = Some(vec![EODHD_REST_RATE_KEY.to_string()]);
        // The API token travels as a query parameter, so transport errors must not echo the URL.
        let response = self
            .client
            .request_with_url_redacted(
                Method::GET,
                url.to_string(),
                Some(params),
                None,
                None,
                None,
                keys,
            )
            .await
            .map_err(|e| Error::Request(e.to_string()))?;

        let status = response.status.as_u16();
        let body = decode_body(&response)?;

        if !response.status.is_success() {
            let message = serde_json::from_slice::<EodhdErrorResponse>(&body)
                .ok()
                .and_then(|e| e.message)
                .unwrap_or_else(|| String::from_utf8_lossy(&body).to_string());

            return Err(Error::ApiError { status, message });
        }

        Ok(body)
    }
}

/// Returns the decompressed response body, honouring the `Content-Encoding` header.
fn decode_body(response: &HttpResponse) -> Result<Vec<u8>> {
    match response.headers.get(CONTENT_ENCODING) {
        None => Ok(response.body.to_vec()),
        Some(encoding) if encoding.eq_ignore_ascii_case(GZIP) => decompress_gzip(&response.body),
        Some(encoding) => Err(Error::ResponseParse(format!(
            "Unsupported response content encoding '{encoding}'"
        ))),
    }
}

fn decompress_gzip(body: &[u8]) -> Result<Vec<u8>> {
    let mut reader = GzDecoder::new(body).take(MAX_DECOMPRESSED_BYTES + 1);
    let mut decompressed = Vec::new();

    reader
        .read_to_end(&mut decompressed)
        .map_err(|e| Error::ResponseParse(format!("Failed to decompress response body: {e}")))?;

    if reader.limit() == 0 {
        return Err(Error::ResponseParse(format!(
            "Decompressed response exceeded {MAX_DECOMPRESSED_BYTES} bytes"
        )));
    }

    Ok(decompressed)
}

/// Returns a lossy preview of `body`, for diagnosing an unexpected payload.
fn body_preview(body: &[u8]) -> String {
    let end = body.len().min(BODY_PREVIEW_BYTES);

    String::from_utf8_lossy(&body[..end]).to_string()
}

/// Parses a delayed quote response, taking the row the provider keyed by ticker.
fn parse_delayed_quote(body: &[u8], url: &str, ticker: &str) -> Result<EodhdDelayedQuote> {
    let response: EodhdDelayedQuoteResponse = serde_json::from_slice(body).map_err(|e| {
        Error::ResponseParse(format!(
            "{url}: {e} | body starts with {:?}",
            body_preview(body)
        ))
    })?;

    response
        .data
        .into_values()
        .next()
        .ok_or_else(|| Error::ApiError {
            status: SUCCESS_STATUS,
            message: format!("no delayed quote returned for {ticker}"),
        })
}

/// Parses a JSON array body, surfacing the provider's own error envelope when present.
///
/// EODHD answers most failures with an HTTP 200 and an error object rather than an error status,
/// so the envelope is checked before the payload is deserialized.
fn parse_json_list<T>(body: &[u8], url: &str) -> Result<Vec<T>>
where
    T: DeserializeOwned,
{
    let value: serde_json::Value = serde_json::from_slice(body).map_err(|e| {
        Error::ResponseParse(format!(
            "{url}: {e} | body starts with {:?}",
            body_preview(body)
        ))
    })?;
    let fallback = value.to_string();

    if let Some(object) = value.as_object()
        && object.contains_key("message")
    {
        let message = object
            .get("message")
            .and_then(serde_json::Value::as_str)
            .map_or(fallback, ToString::to_string);

        return Err(Error::ApiError {
            status: SUCCESS_STATUS,
            message,
        });
    }

    serde_json::from_value(value).map_err(|e| Error::ResponseParse(format!("{url}: {e}")))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_parse_json_list_accepts_a_row_array() {
        let body = br#"[
            {"date":"2024-01-02","open":187.15,"high":188.44,"low":183.89,"close":185.64,
             "adjusted_close":183.404,"volume":82488700}
        ]"#;

        let bars = parse_json_list::<EodhdBar>(body, "test").unwrap();

        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].date, "2024-01-02");
        assert_eq!(bars[0].close, 185.64);
        assert_eq!(bars[0].volume, Some(82_488_700.0));
    }

    #[rstest]
    fn test_parse_json_list_accepts_a_row_without_volume() {
        let body = br#"[{"date":"2024-01-02","open":1.0,"high":2.0,"low":0.5,"close":1.5}]"#;

        let bars = parse_json_list::<EodhdBar>(body, "test").unwrap();

        assert_eq!(bars[0].volume, None);
        assert_eq!(bars[0].adjusted_close, None);
    }

    #[rstest]
    fn test_parse_json_list_surfaces_the_error_envelope() {
        let body = br#"{"code":404,"message":"Not found"}"#;

        let result = parse_json_list::<EodhdBar>(body, "test");

        assert!(matches!(
            result,
            Err(Error::ApiError { message, .. }) if message == "Not found"
        ));
    }

    #[rstest]
    fn test_parse_delayed_quote_reads_the_quote_fields() {
        let body = br#"{
            "meta": {"count": 1},
            "data": {
                "AAPL.US": {
                    "symbol": "AAPL.US",
                    "exchange": "XNAS",
                    "name": "Apple",
                    "open": 330,
                    "high": 332.4816,
                    "low": 325.81,
                    "bidPrice": 330.46,
                    "askPrice": 330.57,
                    "askSize": 2,
                    "bidSize": 12,
                    "bidTime": 1790886551000,
                    "askTime": 1790886551000,
                    "lastTradePrice": 330.55,
                    "previousClosePrice": 330.32,
                    "currency": "USD",
                    "timestamp": 1790900940
                }
            },
            "links": {"next": null}
        }"#;

        let quote = parse_delayed_quote(body, "test", "AAPL.US").unwrap();

        assert_eq!(quote.symbol, "AAPL.US");
        assert_eq!(quote.bid_price, 330.46);
        assert_eq!(quote.ask_price, 330.57);
        assert_eq!(quote.bid_size, 12.0);
        assert_eq!(quote.ask_size, 2.0);
        assert_eq!(quote.ts_event().as_u64(), 1_790_886_551_000_000_000);
    }

    #[rstest]
    fn test_parse_delayed_quote_falls_back_to_the_snapshot_time() {
        let body = br#"{"data": {"AAPL.US": {"symbol": "AAPL.US", "bidPrice": 1.0,
            "askPrice": 1.1, "bidSize": 1, "askSize": 1, "timestamp": 1790900940}}}"#;

        let quote = parse_delayed_quote(body, "test", "AAPL.US").unwrap();

        assert_eq!(quote.ts_event().as_u64(), 1_790_900_940_000_000_000);
    }

    #[rstest]
    fn test_parse_delayed_quote_rejects_an_empty_payload() {
        let result = parse_delayed_quote(br#"{"data": {}}"#, "test", "AAPL.US");

        assert!(matches!(
            result,
            Err(Error::ApiError { message, .. }) if message.contains("AAPL.US")
        ));
    }

    #[rstest]
    fn test_parse_json_list_does_not_treat_a_code_only_object_as_an_error() {
        let body = br#"{"code":"AAPL.US","close":330.32}"#;

        // A `code`-only object is not the error envelope, so it fails as a shape mismatch
        // rather than being reported as a provider error.
        let result = parse_json_list::<EodhdBar>(body, "test");

        assert!(matches!(result, Err(Error::ResponseParse(_))));
    }
}
