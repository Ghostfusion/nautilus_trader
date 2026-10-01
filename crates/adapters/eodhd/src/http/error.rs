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

//! Errors for the EODHD REST API client.

/// Result type for EODHD HTTP operations.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors returned by the EODHD REST API client.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An HTTP request failed at the transport level.
    #[error("HTTP request failed: {0}")]
    Request(String),

    /// The EODHD API returned an error response.
    ///
    /// EODHD reports most failures as an HTTP 200 with a JSON error body, so `status` is often
    /// 200 even for a rejected request. The `message` carries the provider's own explanation.
    #[error("EODHD API error [status {status}]: {message}")]
    ApiError {
        /// The HTTP status code of the response.
        status: u16,
        /// The provider's error message.
        message: String,
    },

    /// No API token was provided and the environment variable is unset.
    #[error("EODHD API token is not set; pass `api_key` or set the {env_var} environment variable")]
    MissingApiKey {
        /// The name of the environment variable consulted for the token.
        env_var: &'static str,
    },

    /// The response body was valid JSON but not the expected shape.
    #[error("Failed to parse response into an EODHD type: {0}")]
    ResponseParse(String),

    /// A timestamp in the response could not be parsed.
    #[error("Failed to parse timestamp '{value}': {reason}")]
    TimestampParse {
        /// The offending value.
        value: String,
        /// Why parsing failed.
        reason: String,
    },
}
