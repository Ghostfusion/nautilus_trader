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

//! EODHD adapter constants and API credential handling.

use std::{fmt::Debug, num::NonZeroU32, sync::LazyLock};

use nautilus_core::{
    env::get_or_env_var_opt,
    string::secret::{REDACTED, mask_api_key},
};
use nautilus_model::identifiers::{ClientId, Venue};
use nautilus_network::ratelimiter::quota::Quota;
use zeroize::ZeroizeOnDrop;

/// The EODHD adapter identifier string.
pub const EODHD: &str = "EODHD";

/// Static venue instance.
pub static EODHD_VENUE: LazyLock<Venue> = LazyLock::new(|| Venue::new(EODHD));

/// Static client ID instance.
pub static EODHD_CLIENT_ID: LazyLock<ClientId> = LazyLock::new(|| ClientId::new(EODHD));

/// Environment variable name for the EODHD API token.
pub const EODHD_API_KEY: &str = "EODHD_API_KEY";

/// Default base URL for the EODHD REST API.
///
/// The endpoints are documented at <https://eodhd.com/financial-apis/>.
pub const EODHD_HTTP_BASE_URL: &str = "https://eodhd.com/api";

/// Default request timeout in seconds.
pub const EODHD_HTTP_TIMEOUT_SECS: u64 = 30;

/// The default EODHD exchange code used for instrument discovery.
///
/// EODHD addresses every United States listing with the `US` code, so the listing venue a symbol
/// list reports for a row (`NYSE`, `NASDAQ`, `PINK`, `NMFQS`) is not a valid ticker suffix.
pub const EODHD_DEFAULT_EXCHANGE: &str = "US";

/// The default price precision applied to instruments and bars.
pub const EODHD_DEFAULT_PRICE_PRECISION: u8 = 2;

/// Rate limit key for EODHD REST API requests.
pub const EODHD_REST_RATE_KEY: &str = "eodhd_rest";

/// Default rate limit for the EODHD REST API (10 requests per second).
pub static EODHD_REST_QUOTA: LazyLock<Quota> = LazyLock::new(|| {
    Quota::per_second(NonZeroU32::new(10).expect("non-zero")).expect("valid quota")
});

/// An EODHD API credential.
///
/// The token is zeroized on drop and redacted from `Debug` output.
#[derive(Clone, ZeroizeOnDrop)]
pub struct Credential {
    api_key: Box<[u8]>,
}

impl Debug for Credential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(Credential))
            .field("api_key", &REDACTED)
            .finish()
    }
}

impl Credential {
    /// Creates a new [`Credential`] instance from the `api_key`.
    #[must_use]
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into().into_bytes().into_boxed_slice(),
        }
    }

    /// Returns the API token associated with this credential.
    ///
    /// # Panics
    ///
    /// This method should never panic as the API token is always valid UTF-8,
    /// having been created from a `String`.
    #[must_use]
    pub fn api_key(&self) -> &str {
        std::str::from_utf8(&self.api_key).expect("API token is valid UTF-8")
    }

    /// Returns a masked version of the API token for logging purposes.
    #[must_use]
    pub fn api_key_masked(&self) -> String {
        mask_api_key(self.api_key())
    }

    /// Resolves a credential from the provided value or the `EODHD_API_KEY` environment variable.
    #[must_use]
    pub fn resolve(api_key: Option<String>) -> Option<Self> {
        get_or_env_var_opt(api_key, EODHD_API_KEY).map(Self::new)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_credential_round_trip() {
        let credential = Credential::new("secret-token");

        assert_eq!(credential.api_key(), "secret-token");
    }

    #[rstest]
    fn test_credential_debug_is_redacted() {
        let credential = Credential::new("secret-token");
        let debug = format!("{credential:?}");

        assert!(!debug.contains("secret-token"));
        assert!(debug.contains("Credential"));
    }

    #[rstest]
    fn test_credential_masked_hides_middle() {
        let credential = Credential::new("abcdefghijklmnop");

        assert!(!credential.api_key_masked().contains("efghijklmn"));
    }

    #[rstest]
    fn test_credential_resolve_prefers_the_explicit_value() {
        let credential = Credential::resolve(Some("explicit-token".to_string())).unwrap();

        assert_eq!(credential.api_key(), "explicit-token");
    }
}
