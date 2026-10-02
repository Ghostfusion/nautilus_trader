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

//! The OpenD gateway frame format.
//!
//! Every message on the wire is a fixed 44-byte header followed by a body. The header names the
//! protocol identifier that selects the message type, a serial number that correlates a response
//! with the request that caused it, the body length, and a SHA-1 digest of the body. All integers
//! are little-endian and the header is packed, so the body always begins at a fixed offset.
//!
//! This module is the one part of the adapter with an exact specification, so it depends on nothing
//! else in the crate and is tested against its own byte layout.
//!
//! # Reading a stream
//!
//! [`decode_frame`] returns `Ok(None)` when the buffer does not yet hold a whole frame, which is the
//! ordinary case for a socket read that splits a frame across two reads. The caller keeps the buffer
//! and calls again when more bytes arrive, and removes [`Frame::total_len`] bytes once a frame is
//! returned.
//!
//! # What a failure means
//!
//! The three errors are not equivalent, and the difference decides what the connection does with
//! them.
//!
//! - [`CodecError::DigestMismatch`] says one frame is corrupt. The length was plausible and the
//!   header parsed, so the next frame starts where this one ends; the frame can be discarded and
//!   the stream read on.
//! - [`CodecError::InvalidMagic`] says the bytes are not a frame at all. The reader has lost the
//!   frame boundary, and no later byte can be trusted to be a header, so the connection must be
//!   closed rather than resynchronised by guessing.
//! - [`CodecError::BodyTooLarge`] is a header asking for more than [`MAX_BODY_LEN`], which is either
//!   corruption or hostility, and is treated like the lost boundary above.
//!
//! # Digest scope
//!
//! The digest covers the body as the protocol understands it, which on the encrypted remote path is
//! the plaintext, computed before encryption and verified after decryption. The loopback path this
//! adapter supports has no encryption, so the digest covers exactly the bytes that follow the
//! header.

use sha1::{Digest, Sha1};
use thiserror::Error;

/// The length of the fixed frame header in bytes.
pub const HEADER_LEN: usize = 44;

/// The two bytes that open every frame.
pub const MAGIC: [u8; 2] = *b"FT";

/// The protocol format identifier for a Protobuf body.
pub const PROTO_FMT_PROTOBUF: u8 = 0;

/// The protocol format identifier for a JSON body, which this adapter does not use.
pub const PROTO_FMT_JSON: u8 = 1;

/// The protocol version sent on every frame.
pub const PROTO_VER: u8 = 0;

/// The largest body the decoder will accept.
///
/// The length field is a 32-bit count, so without a bound a corrupt or hostile header could ask the
/// reader to buffer four gigabytes before it could decide the frame was invalid. The limit is
/// generous against real traffic: a page of a thousand bars is on the order of a hundred kilobytes.
pub const MAX_BODY_LEN: u32 = 64 * 1024 * 1024;

/// A parsed frame header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHeader {
    /// The protocol identifier, which selects the message type.
    pub proto_id: u32,
    /// The body encoding; [`PROTO_FMT_PROTOBUF`] for every frame this adapter sends.
    pub proto_fmt_type: u8,
    /// The protocol version, [`PROTO_VER`].
    pub proto_ver: u8,
    /// The serial number correlating a response with the request that caused it.
    pub serial_no: u32,
    /// The body length in bytes.
    pub body_len: u32,
    /// The SHA-1 digest of the body.
    pub sha1: [u8; 20],
    /// Eight reserved bytes, sent as zero and not interpreted.
    pub reserve: [u8; 8],
}

/// A frame borrowed from the buffer it was decoded from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame<'a> {
    /// The decoded header.
    pub header: FrameHeader,
    /// The body, still encoded in the format the header names.
    pub body: &'a [u8],
}

impl Frame<'_> {
    /// Returns the number of bytes this frame occupies in the stream.
    ///
    /// This is how much the caller removes from the front of its buffer once the frame is handled.
    #[must_use]
    pub fn total_len(&self) -> usize {
        HEADER_LEN + self.body.len()
    }
}

/// The ways a frame can fail to satisfy the format.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CodecError {
    /// The frame does not begin with the magic bytes, so the frame boundary is lost.
    #[error("invalid frame magic, expected FT")]
    InvalidMagic,
    /// The header declares a body larger than [`MAX_BODY_LEN`].
    #[error("frame body length of {0} bytes exceeds the maximum of 67108864")]
    BodyTooLarge(u64),
    /// The body does not hash to the digest in the header.
    #[error("frame body digest does not match the header")]
    DigestMismatch,
}

/// Encodes a frame around `body`.
///
/// The digest covers the body as given, so the caller passes exactly the bytes that should appear
/// after the header.
///
/// # Errors
///
/// Returns [`CodecError::BodyTooLarge`] if the body does not fit the protocol's 32-bit length field.
pub fn encode_frame(proto_id: u32, serial_no: u32, body: &[u8]) -> Result<Vec<u8>, CodecError> {
    let body_len =
        u32::try_from(body.len()).map_err(|_| CodecError::BodyTooLarge(body.len() as u64))?;
    let digest: [u8; 20] = Sha1::digest(body).into();

    let mut frame = Vec::with_capacity(HEADER_LEN + body.len());
    frame.extend_from_slice(&MAGIC);
    frame.extend_from_slice(&proto_id.to_le_bytes());
    frame.push(PROTO_FMT_PROTOBUF);
    frame.push(PROTO_VER);
    frame.extend_from_slice(&serial_no.to_le_bytes());
    frame.extend_from_slice(&body_len.to_le_bytes());
    frame.extend_from_slice(&digest);
    frame.extend_from_slice(&[0_u8; 8]);
    frame.extend_from_slice(body);

    Ok(frame)
}

/// Decodes the frame at the front of `buf`.
///
/// Returns `Ok(None)` when `buf` does not yet hold a whole frame, which is not an error: a socket
/// read may deliver a frame in pieces.
///
/// # Errors
///
/// Returns [`CodecError::InvalidMagic`] if the leading bytes are not a frame header,
/// [`CodecError::BodyTooLarge`] if the header asks for a body beyond [`MAX_BODY_LEN`], or
/// [`CodecError::DigestMismatch`] if the body does not hash to the digest the header carries. The
/// first two mean the reader has lost the frame boundary; the third means one frame is corrupt.
pub fn decode_frame(buf: &[u8]) -> Result<Option<Frame<'_>>, CodecError> {
    if buf.len() < HEADER_LEN {
        return Ok(None);
    }

    if buf[0] != MAGIC[0] || buf[1] != MAGIC[1] {
        return Err(CodecError::InvalidMagic);
    }

    let proto_id = read_u32(buf, 2);
    let proto_fmt_type = buf[6];
    let proto_ver = buf[7];
    let serial_no = read_u32(buf, 8);
    let body_len = read_u32(buf, 12);

    let mut sha1 = [0_u8; 20];
    sha1.copy_from_slice(&buf[16..36]);

    let mut reserve = [0_u8; 8];
    reserve.copy_from_slice(&buf[36..44]);

    if body_len > MAX_BODY_LEN {
        return Err(CodecError::BodyTooLarge(u64::from(body_len)));
    }

    let total_len = HEADER_LEN + body_len as usize;
    if buf.len() < total_len {
        return Ok(None);
    }

    let body = &buf[HEADER_LEN..total_len];
    let digest: [u8; 20] = Sha1::digest(body).into();
    if digest != sha1 {
        return Err(CodecError::DigestMismatch);
    }

    Ok(Some(Frame {
        header: FrameHeader {
            proto_id,
            proto_fmt_type,
            proto_ver,
            serial_no,
            body_len,
            sha1,
            reserve,
        },
        body,
    }))
}

fn read_u32(buf: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([buf[at], buf[at + 1], buf[at + 2], buf[at + 3]])
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    /// The published SHA-1 digest of `abc`, used to pin the digest and its position in the header.
    const ABC_DIGEST: [u8; 20] = [
        0xa9, 0x99, 0x3e, 0x36, 0x47, 0x06, 0x81, 0x6a, 0xba, 0x3e, 0x25, 0x71, 0x78, 0x50, 0xc2,
        0x6c, 0x9c, 0xd0, 0xd8, 0x9d,
    ];

    /// The published SHA-1 digest of the empty string.
    const EMPTY_DIGEST: [u8; 20] = [
        0xda, 0x39, 0xa3, 0xee, 0x5e, 0x6b, 0x4b, 0x0d, 0x32, 0x55, 0xbf, 0xef, 0x95, 0x60, 0x18,
        0x90, 0xaf, 0xd8, 0x07, 0x09,
    ];

    #[rstest]
    fn test_encoded_frame_layout() {
        let frame = encode_frame(1001, 1, b"abc").unwrap();

        assert_eq!(frame.len(), HEADER_LEN + 3);
        assert_eq!(&frame[0..2], b"FT");
        assert_eq!(&frame[2..6], &1001_u32.to_le_bytes());
        assert_eq!(frame[6], PROTO_FMT_PROTOBUF);
        assert_eq!(frame[7], PROTO_VER);
        assert_eq!(&frame[8..12], &1_u32.to_le_bytes());
        assert_eq!(&frame[12..16], &3_u32.to_le_bytes());
        assert_eq!(&frame[16..36], &ABC_DIGEST);
        assert_eq!(&frame[36..44], &[0_u8; 8]);
        assert_eq!(&frame[44..], b"abc");
    }

    #[rstest]
    fn test_empty_body_uses_the_empty_digest() {
        let frame = encode_frame(1002, 7, b"").unwrap();

        assert_eq!(frame.len(), HEADER_LEN);
        assert_eq!(&frame[12..16], &0_u32.to_le_bytes());
        assert_eq!(&frame[16..36], &EMPTY_DIGEST);
    }

    #[rstest]
    #[case::empty(b"")]
    #[case::single(b"\x00")]
    #[case::text(b"the quick brown fox")]
    fn test_round_trip(#[case] body: &[u8]) {
        let encoded = encode_frame(3001, 42, body).unwrap();
        let frame = decode_frame(&encoded).unwrap().unwrap();

        assert_eq!(frame.header.proto_id, 3001);
        assert_eq!(frame.header.serial_no, 42);
        assert_eq!(frame.header.body_len, body.len() as u32);
        assert_eq!(frame.body, body);
        assert_eq!(frame.total_len(), encoded.len());
    }

    #[rstest]
    fn test_trailing_bytes_are_not_consumed() {
        let mut buf = encode_frame(3001, 1, b"one").unwrap();
        buf.extend_from_slice(b"next frame begins here");

        let frame = decode_frame(&buf).unwrap().unwrap();

        assert_eq!(frame.body, b"one");
        assert_eq!(frame.total_len(), HEADER_LEN + 3);
    }

    /// Every prefix of a well formed frame is incomplete rather than invalid, so a split socket
    /// read is not mistaken for corruption.
    #[rstest]
    fn test_short_buffer_is_incomplete() {
        let encoded = encode_frame(3001, 1, b"payload").unwrap();

        for len in 0..encoded.len() {
            assert_eq!(
                decode_frame(&encoded[..len]),
                Ok(None),
                "prefix of {len} bytes should be incomplete"
            );
        }
    }

    #[rstest]
    #[case::empty(b"")]
    #[case::partial_magic(b"F")]
    #[case::wrong_magic(b"XT")]
    #[case::swapped_magic(b"TF")]
    fn test_invalid_magic(#[case] prefix: &[u8]) {
        let mut buf = vec![0_u8; HEADER_LEN];
        buf[..prefix.len()].copy_from_slice(prefix);

        assert_eq!(decode_frame(&buf), Err(CodecError::InvalidMagic));
    }

    #[rstest]
    fn test_corrupt_body_is_a_digest_mismatch() {
        let mut encoded = encode_frame(3001, 1, b"payload").unwrap();
        let last = encoded.len() - 1;
        encoded[last] ^= 0xff;

        assert_eq!(decode_frame(&encoded), Err(CodecError::DigestMismatch));
    }

    #[rstest]
    fn test_corrupt_digest_is_a_digest_mismatch() {
        let mut encoded = encode_frame(3001, 1, b"payload").unwrap();
        encoded[16] ^= 0xff;

        assert_eq!(decode_frame(&encoded), Err(CodecError::DigestMismatch));
    }

    /// A declared length beyond the bound is rejected before any buffering decision, so it cannot
    /// be used to make the reader wait for four gigabytes that will never arrive.
    #[rstest]
    fn test_body_length_beyond_the_bound() {
        let mut encoded = encode_frame(3001, 1, b"payload").unwrap();
        encoded[12..16].copy_from_slice(&(MAX_BODY_LEN + 1).to_le_bytes());

        assert_eq!(
            decode_frame(&encoded),
            Err(CodecError::BodyTooLarge(u64::from(MAX_BODY_LEN) + 1))
        );
    }

    /// A declared length larger than the buffer is incomplete, not invalid: the rest may still be
    /// in flight.
    #[rstest]
    fn test_declared_length_larger_than_the_buffer_is_incomplete() {
        let mut encoded = encode_frame(3001, 1, b"payload").unwrap();
        encoded[12..16].copy_from_slice(&1024_u32.to_le_bytes());

        assert_eq!(decode_frame(&encoded), Ok(None));
    }

    /// Decodes a hex string into bytes, so a captured frame can be written as it appears on the
    /// wire rather than as a list of byte literals.
    fn hex(text: &str) -> Vec<u8> {
        assert!(
            text.len().is_multiple_of(2),
            "hex input must have an even length"
        );
        let bytes = text.as_bytes();
        (0..text.len() / 2)
            .map(|i| {
                let pair = std::str::from_utf8(&bytes[i * 2..i * 2 + 2]).unwrap();
                u8::from_str_radix(pair, 16).unwrap()
            })
            .collect()
    }

    /// The frames the gateway client's own `_joint_head` produced for the same inputs. This
    /// compares the implementation against the vendor's framing rather than against the reading of
    /// it that produced this module, and the two digests it uses are the published SHA-1 vectors
    /// for `abc` and for the empty string.
    #[rstest]
    #[case::init_connect(
        1001,
        1,
        b"abc",
        "4654e903000000000100000003000000a9993e364706816aba3e25717850c26c9cd0d89d0000000000000000616263"
    )]
    #[case::subscribe(
        3001,
        42,
        b"abc",
        "4654b90b000000002a00000003000000a9993e364706816aba3e25717850c26c9cd0d89d0000000000000000616263"
    )]
    #[case::empty_body(
        1004,
        0,
        b"",
        "4654ec03000000000000000000000000da39a3ee5e6b4b0d3255bfef95601890afd807090000000000000000"
    )]
    #[case::static_info(
        3202,
        7,
        b"payload",
        "4654820c000000000700000007000000f07e5a815613c5abeddc4b682247a4c42d8a95df00000000000000007061796c6f6164"
    )]
    fn test_matches_the_vendor_framing(
        #[case] proto_id: u32,
        #[case] serial_no: u32,
        #[case] body: &[u8],
        #[case] expected: &str,
    ) {
        let encoded = encode_frame(proto_id, serial_no, body).unwrap();

        assert_eq!(encoded, hex(expected));

        let frame = decode_frame(&encoded).unwrap().unwrap();
        assert_eq!(frame.header.proto_id, proto_id);
        assert_eq!(frame.header.serial_no, serial_no);
        assert_eq!(frame.header.proto_fmt_type, PROTO_FMT_PROTOBUF);
        assert_eq!(frame.header.proto_ver, PROTO_VER);
        assert_eq!(frame.header.reserve, [0_u8; 8]);
        assert_eq!(frame.body, body);
    }
}
