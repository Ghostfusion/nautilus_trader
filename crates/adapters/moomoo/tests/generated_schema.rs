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

//! Conformance tests for the generated gateway schema.
//!
//! Every expected byte string was produced by `protoc` from the same vendored definitions, with
//!
//! ```text
//! protoc --encode=<Message> -I proto proto/<File>.proto < <text form>
//! ```
//!
//! and the text form recorded beside the constant that holds the result. Comparing against `protoc`
//! rather than against prost itself is the point: a round trip through one library would hide an
//! encoding difference, and an encoding difference here corrupts the wire protocol silently.
//!
//! The definitions are `proto2`, so the cases that matter are the proto2 ones: `required` fields are
//! encoded even at their default value, `optional` fields are omitted when unset, and repeated
//! scalars are unpacked unless the field opts in.

use nautilus_moomoo::generated::{init_connect, qot_common, qot_sub};
use prost::Message;
use rstest::rstest;

/// The handshake request, from the text form:
///
/// ```text
/// c2s {
///   clientVer: 100
///   clientID: "abc"
///   recvNotify: true
/// }
/// ```
const INIT_CONNECT_REQUEST: &[u8] = &[
    0x0a, 0x09, 0x08, 0x64, 0x12, 0x03, 0x61, 0x62, 0x63, 0x18, 0x01,
];

/// A subscription request, from the text form:
///
/// ```text
/// c2s {
///   securityList { market: 11 code: "AAPL" }
///   subTypeList: 1
///   subTypeList: 4
///   isSubOrUnSub: true
///   isRegOrUnRegPush: true
/// }
/// ```
///
/// `QotMarket_US_Security` is 11 and `SubType_Basic` and `SubType_Ticker` are 1 and 4.
const QOT_SUB_REQUEST: &[u8] = &[
    0x0a, 0x12, 0x0a, 0x08, 0x08, 0x0b, 0x12, 0x04, 0x41, 0x41, 0x50, 0x4c, 0x10, 0x01, 0x10, 0x04,
    0x18, 0x01, 0x20, 0x01,
];

/// A subscription whose only required field is `false`, from the text form `c2s { isSubOrUnSub:
/// false }`. The field is still on the wire, because proto2 `required` is presence, not value.
const QOT_SUB_REQUIRED_FALSE: &[u8] = &[0x0a, 0x02, 0x18, 0x00];

/// A handshake whose required fields are at their default value, from the text form:
///
/// ```text
/// c2s {
///   clientVer: 0
///   clientID: ""
/// }
/// ```
const INIT_CONNECT_DEFAULTS: &[u8] = &[0x0a, 0x04, 0x08, 0x00, 0x12, 0x00];

#[rstest]
fn test_init_connect_request_matches_protoc() {
    let request = init_connect::Request {
        c2s: init_connect::C2s {
            client_ver: 100,
            client_id: "abc".to_string(),
            recv_notify: Some(true),
            ..Default::default()
        },
    };

    assert_eq!(request.encode_to_vec(), INIT_CONNECT_REQUEST);
    assert_eq!(
        init_connect::Request::decode(INIT_CONNECT_REQUEST).unwrap(),
        request
    );
}

#[rstest]
fn test_qot_sub_request_matches_protoc() {
    let request = qot_sub::Request {
        c2s: qot_sub::C2s {
            security_list: vec![qot_common::Security {
                market: 11,
                code: "AAPL".to_string(),
            }],
            sub_type_list: vec![1, 4],
            is_sub_or_un_sub: true,
            is_reg_or_un_reg_push: Some(true),
            ..Default::default()
        },
    };

    assert_eq!(request.encode_to_vec(), QOT_SUB_REQUEST);
    assert_eq!(qot_sub::Request::decode(QOT_SUB_REQUEST).unwrap(), request);
}

/// A proto3 assumption would drop this field, and a proto3 decoder would then read the message as
/// malformed, because proto2 requires the field to be present regardless of its value.
#[rstest]
fn test_required_field_is_encoded_at_its_default_value() {
    let request = qot_sub::Request {
        c2s: qot_sub::C2s {
            is_sub_or_un_sub: false,
            ..Default::default()
        },
    };

    assert_eq!(request.encode_to_vec(), QOT_SUB_REQUIRED_FALSE);
    assert_eq!(
        qot_sub::Request::decode(QOT_SUB_REQUIRED_FALSE).unwrap(),
        request
    );
}

/// The same rule for a required integer and a required string, both empty.
#[rstest]
fn test_required_scalars_are_encoded_when_empty() {
    let request = init_connect::Request {
        c2s: init_connect::C2s {
            client_ver: 0,
            client_id: String::new(),
            ..Default::default()
        },
    };

    assert_eq!(request.encode_to_vec(), INIT_CONNECT_DEFAULTS);
}

/// The repeated `subTypeList` is unpacked, so each element carries its own tag rather than sharing
/// one length-delimited group. The two tag bytes are at offsets 12 and 14 of the request.
#[rstest]
fn test_proto2_repeated_scalars_are_unpacked() {
    assert_eq!(QOT_SUB_REQUEST[12], 0x10);
    assert_eq!(QOT_SUB_REQUEST[13], 0x01);
    assert_eq!(QOT_SUB_REQUEST[14], 0x10);
    assert_eq!(QOT_SUB_REQUEST[15], 0x04);
}
