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

use std::collections::BTreeMap;

use anyhow::{Context, bail};
use nautilus_model::{
    data::{Bar, BarType},
    instruments::InstrumentAny,
};
use prost::Message as _;

use crate::{
    common::Market,
    connection::{Connection, RET_OK},
    generated::{
        qot_common::{self, KLine},
        qot_get_security_snapshot, qot_get_static_info, qot_get_sub_info, qot_request_history_kl,
        qot_request_history_kl_quota, qot_sub,
    },
    mappers::{
        bars::{Adjustment, BarSession, Interval, build_bars},
        instrument::instrument_from,
    },
    subscription::Subscription,
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

/// The protocol identifier of the historical K-line request.
pub const PROTO_ID_REQUEST_HISTORY_KL: u32 = 3103;

/// The protocol identifier of the historical allowance read.
pub const PROTO_ID_REQUEST_HISTORY_KL_QUOTA: u32 = 3104;

/// The most pages one series is assembled from before the loop is treated as a defect.
///
/// The venue bounds a series itself, so this only exists so that a continuation key the venue keeps
/// returning cannot spin forever.
const MAX_HISTORY_PAGES: u32 = 64;

/// One security the historical allowance has already been spent on in the current period.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowanceSpend {
    /// The security, in the gateway's own form.
    pub code: String,
    /// When the unit was spent, as the gateway reports it.
    pub request_time: String,
}

/// The gateway's own tally of the historical allowance.
///
/// The unit is a security, not a request. The gateway counts how many securities have been
/// downloaded in the current period, and a later request for one of them is free until the period
/// rolls over. That is what makes pagination cheap and the first load of a new security expensive,
/// and it is why the allowance is checked once per series rather than once per page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryAllowance {
    /// Securities already spent on in the current period.
    pub used: i32,
    /// Securities still available.
    pub remaining: i32,
    /// Which securities were spent on, as the gateway reports them.
    pub spent: Vec<AllowanceSpend>,
}

impl HistoryAllowance {
    /// Returns whether a security has already been spent on in the current period.
    #[must_use]
    pub fn covers(&self, market: Market, code: &str) -> bool {
        let wanted = describe(&security(market, code));

        self.spent.iter().any(|spend| spend.code == wanted)
    }
}

/// Reads the gateway's own tally of the historical allowance.
///
/// The adapter also keeps its own estimate of what it has spent, but this is the authority: the
/// allowance is shared with every other tool the operator runs against the same gateway.
///
/// # Errors
///
/// Returns an error if the request fails, the gateway refuses it, the answer cannot be decoded, or
/// it carries no payload.
pub async fn request_history_allowance(
    connection: &Connection,
) -> anyhow::Result<HistoryAllowance> {
    let request = qot_request_history_kl_quota::Request {
        c2s: qot_request_history_kl_quota::C2s {
            b_get_detail: Some(true),
            header: None,
        },
    };

    let message = connection
        .request(PROTO_ID_REQUEST_HISTORY_KL_QUOTA, &request.encode_to_vec())
        .await?;

    let response = qot_request_history_kl_quota::Response::decode(message.body.as_slice())
        .context("cannot decode a historical allowance response")?;
    check_outcome(response.ret_type, response.ret_msg, "historical allowance")?;

    let s2c = response
        .s2c
        .context("the historical allowance response carried no payload")?;

    Ok(HistoryAllowance {
        used: s2c.used_quota,
        remaining: s2c.remain_quota,
        spent: s2c
            .detail_list
            .into_iter()
            .map(|detail| AllowanceSpend {
                code: describe(&detail.security),
                request_time: detail.request_time,
            })
            .collect(),
    })
}

/// A historical bar request.
#[derive(Debug, Clone)]
pub struct HistoryRequest {
    /// The market the security trades on.
    pub market: Market,
    /// The security's code, without its market prefix.
    pub code: String,
    /// The bar interval.
    pub interval: Interval,
    /// The price adjustment.
    pub adjustment: Adjustment,
    /// The first date to include, as the gateway writes a date.
    pub begin: String,
    /// The last date to include, as the gateway writes a date.
    pub end: String,
    /// The session the request covers.
    pub session: BarSession,
    /// The largest number of bars to ask for in one page, when the caller sets one.
    pub page_size: Option<i32>,
}

/// Loads a bar series, paginating until the venue stops returning a continuation key.
///
/// The allowance is checked before the first page rather than discovered part way through, because
/// the unit is spent on the security: a series for a security already paid for in this period costs
/// nothing, and a series for a new one needs a unit to be available.
///
/// # Errors
///
/// Returns an error if the allowance cannot be read, if it is exhausted and the security is not
/// already covered, if a page is refused, if the venue keeps paginating past the page bound, or if
/// the rows cannot be mapped.
pub async fn request_history_bars(
    connection: &Connection,
    request: &HistoryRequest,
    bar_type: BarType,
    price_precision: u8,
) -> anyhow::Result<Vec<Bar>> {
    let allowance = request_history_allowance(connection).await?;

    if allowance.remaining <= 0 && !allowance.covers(request.market, &request.code) {
        bail!(
            "the historical allowance is exhausted at {} securities in this period, and {} is not \
             one of them",
            allowance.used,
            describe(&security(request.market, &request.code))
        );
    }

    let mut next_key: Option<Vec<u8>> = None;
    let mut rows: Vec<KLine> = Vec::new();
    let mut pages = 0_u32;

    loop {
        if pages >= MAX_HISTORY_PAGES {
            bail!("the gateway kept returning a continuation key past {MAX_HISTORY_PAGES} pages");
        }
        pages += 1;

        let body = qot_request_history_kl::Request {
            c2s: qot_request_history_kl::C2s {
                rehab_type: request.adjustment.rehab_type(),
                kl_type: request.interval.kl_type(),
                security: security(request.market, &request.code),
                begin_time: request.begin.clone(),
                end_time: request.end.clone(),
                max_ack_kl_num: request.page_size,
                need_kl_fields_flag: None,
                next_req_key: next_key.take(),
                extended_time: Some(request.session.extended_time()),
                session: Some(request.session.session()),
                header: None,
            },
        };

        let message = connection
            .request(PROTO_ID_REQUEST_HISTORY_KL, &body.encode_to_vec())
            .await?;

        let response = qot_request_history_kl::Response::decode(message.body.as_slice())
            .context("cannot decode a historical bar response")?;
        check_outcome(response.ret_type, response.ret_msg, "historical bars")?;

        let s2c = response
            .s2c
            .context("the historical bar response carried no payload")?;

        rows.extend(s2c.kl_list);

        match s2c.next_req_key.filter(|key| !key.is_empty()) {
            Some(key) => next_key = Some(key),
            None => {
                return build_bars(
                    &rows,
                    bar_type,
                    request.interval,
                    request.market,
                    price_precision,
                );
            }
        }
    }
}

/// The protocol identifier of the subscribe request.
pub const PROTO_ID_SUB: u32 = 3001;

/// The protocol identifier of the subscription information request.
pub const PROTO_ID_GET_SUB_INFO: u32 = 3003;

/// The venue's own tally of the subscription allowance.
///
/// The allowance is shared with every other tool pointed at the same gateway, and the venue reports
/// the option allowance separately, so both counts are kept rather than folded into one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubscriptionAllowance {
    /// Subscriptions already in use.
    pub used: i32,
    /// Subscriptions still available.
    pub remaining: i32,
    /// Option subscriptions already in use, where the venue reports them separately.
    pub option_used: Option<i32>,
    /// Option subscriptions still available, where the venue reports them separately.
    pub option_remaining: Option<i32>,
}

impl SubscriptionAllowance {
    /// Returns the total quota the used and remaining counts describe.
    #[must_use]
    pub fn quota(self) -> usize {
        usize::try_from(self.used.max(0) + self.remaining.max(0)).unwrap_or_default()
    }
}

/// Reads the venue's own tally of the subscription allowance.
///
/// # Errors
///
/// Returns an error if the request fails, the venue refuses it, the answer cannot be decoded, or it
/// carries no payload.
pub async fn request_subscription_allowance(
    connection: &Connection,
) -> anyhow::Result<SubscriptionAllowance> {
    let request = qot_get_sub_info::Request {
        c2s: qot_get_sub_info::C2s {
            // The adapter accounts for its own subscriptions; this asks for this connection's.
            is_req_all_conn: Some(false),
            header: None,
        },
    };

    let message = connection
        .request(PROTO_ID_GET_SUB_INFO, &request.encode_to_vec())
        .await?;

    let response = qot_get_sub_info::Response::decode(message.body.as_slice())
        .context("cannot decode a subscription allowance response")?;
    check_outcome(
        response.ret_type,
        response.ret_msg,
        "subscription allowance",
    )?;

    let s2c = response
        .s2c
        .context("the subscription allowance response carried no payload")?;

    Ok(SubscriptionAllowance {
        used: s2c.total_used_quota,
        remaining: s2c.remain_quota,
        option_used: s2c.option_used_quota,
        option_remaining: s2c.option_remain_quota,
    })
}

/// Subscribes or releases a set of subscriptions.
///
/// The venue's request takes a list of securities and a list of data types and applies the product
/// of the two, so a batch carrying different types for different securities is split into one
/// request per type. Sending the product instead would subscribe securities to types nobody asked
/// for, and each of those spends allowance that is scarce.
///
/// # Errors
///
/// Returns an error if a request fails, or if the venue refuses it, in which case the venue's own
/// message is the one surfaced.
pub async fn set_subscriptions(
    connection: &Connection,
    subscriptions: &[Subscription],
    subscribe: bool,
    session: BarSession,
) -> anyhow::Result<()> {
    let mut by_type: BTreeMap<i32, Vec<qot_common::Security>> = BTreeMap::new();

    for subscription in subscriptions {
        by_type
            .entry(subscription.kind.sub_type())
            .or_default()
            .push(security(subscription.market, &subscription.code));
    }

    for (sub_type, securities) in by_type {
        let request = qot_sub::Request {
            c2s: qot_sub::C2s {
                security_list: securities,
                sub_type_list: vec![sub_type],
                is_sub_or_un_sub: subscribe,
                // The push registration travels with the subscription: a subscription with no
                // registration would be held and paid for while delivering nothing.
                is_reg_or_un_reg_push: Some(subscribe),
                reg_push_rehab_type_list: Vec::new(),
                is_first_push: None,
                is_unsub_all: None,
                is_sub_order_book_detail: None,
                extended_time: Some(session.extended_time()),
                session: Some(session.session()),
                header: None,
            },
        };

        let message = connection
            .request(PROTO_ID_SUB, &request.encode_to_vec())
            .await?;

        let response = qot_sub::Response::decode(message.body.as_slice())
            .context("cannot decode a subscribe response")?;

        check_outcome(
            response.ret_type,
            response.ret_msg,
            if subscribe { "subscribe" } else { "release" },
        )?;
    }

    Ok(())
}
