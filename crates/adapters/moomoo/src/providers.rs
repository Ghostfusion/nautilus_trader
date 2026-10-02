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

//! Reading instrument definitions from the gateway.
//!
//! Loading one instrument costs two requests, and both are made because neither carries enough on
//! its own: the static record has the identity and the definition, and the snapshot has the price
//! spread that decides the tick size. Both are plain requests rather than subscriptions, so they do
//! not touch the subscription allowance.

use anyhow::{Context, bail};
use nautilus_model::instruments::InstrumentAny;
use prost::Message as _;

use crate::{
    common::Market,
    connection::{Connection, RET_OK},
    generated::{qot_common, qot_get_security_snapshot, qot_get_static_info},
    mappers::instrument::instrument_from,
};

/// The protocol identifier of the static information request.
pub const PROTO_ID_GET_STATIC_INFO: u32 = 3202;

/// The protocol identifier of the security snapshot request.
pub const PROTO_ID_GET_SECURITY_SNAPSHOT: u32 = 3203;

/// Builds the gateway's own record for a security.
#[must_use]
pub fn security(market: Market, code: &str) -> qot_common::Security {
    qot_common::Security {
        market: market.qot_market(),
        code: code.to_string(),
    }
}

/// Renders a gateway security for an error message.
fn describe(security: &qot_common::Security) -> String {
    Market::from_qot_market(security.market).map_or_else(
        || format!("QotMarket {} code {}", security.market, security.code),
        |market| format!("{}.{}", market.code(), security.code),
    )
}

/// Fails with the gateway's own message when a response is not a success.
///
/// The message matters more than the code: it is what tells an operator that a quote card is
/// missing, which "insufficient permission" does not.
fn check_outcome(ret_type: i32, ret_msg: Option<String>, what: &str) -> anyhow::Result<()> {
    if ret_type == RET_OK {
        return Ok(());
    }

    bail!(
        "the gateway refused the {what} request with retType {ret_type}: {}",
        ret_msg.unwrap_or_default()
    )
}

/// Requests the static record for one security.
///
/// # Errors
///
/// Returns an error if the request fails, the gateway refuses it, the answer cannot be decoded, or
/// it carries no record for the security.
pub async fn request_static_info(
    connection: &Connection,
    security: qot_common::Security,
) -> anyhow::Result<qot_common::SecurityStaticInfo> {
    let request = qot_get_static_info::Request {
        c2s: qot_get_static_info::C2s {
            market: None,
            sec_type: None,
            security_list: vec![security.clone()],
            header: None,
        },
    };

    let message = connection
        .request(PROTO_ID_GET_STATIC_INFO, &request.encode_to_vec())
        .await?;

    let response = qot_get_static_info::Response::decode(message.body.as_slice())
        .context("cannot decode a static information response")?;
    check_outcome(response.ret_type, response.ret_msg, "static information")?;

    response
        .s2c
        .and_then(|s2c| s2c.static_info_list.into_iter().next())
        .with_context(|| format!("no static record returned for {}", describe(&security)))
}

/// Requests the snapshot for one security.
///
/// # Errors
///
/// Returns an error if the request fails, the gateway refuses it, the answer cannot be decoded, or
/// it carries no snapshot for the security.
pub async fn request_snapshot(
    connection: &Connection,
    security: qot_common::Security,
) -> anyhow::Result<qot_get_security_snapshot::SnapshotBasicData> {
    let request = qot_get_security_snapshot::Request {
        c2s: qot_get_security_snapshot::C2s {
            security_list: vec![security.clone()],
            header: None,
        },
    };

    let message = connection
        .request(PROTO_ID_GET_SECURITY_SNAPSHOT, &request.encode_to_vec())
        .await?;

    let response = qot_get_security_snapshot::Response::decode(message.body.as_slice())
        .context("cannot decode a snapshot response")?;
    check_outcome(response.ret_type, response.ret_msg, "snapshot")?;

    response
        .s2c
        .and_then(|s2c| s2c.snapshot_list.into_iter().next())
        .map(|snapshot| snapshot.basic)
        .with_context(|| format!("no snapshot returned for {}", describe(&security)))
}

/// Loads one instrument definition from the gateway.
///
/// # Errors
///
/// Returns an error if either request fails or the records cannot be mapped, which includes a
/// security the adapter does not model.
pub async fn load_instrument(
    connection: &Connection,
    market: Market,
    code: &str,
) -> anyhow::Result<InstrumentAny> {
    let security = security(market, code);
    let static_info = request_static_info(connection, security.clone()).await?;
    let snapshot = request_snapshot(connection, security).await?;

    instrument_from(&static_info, Some(&snapshot))
}
