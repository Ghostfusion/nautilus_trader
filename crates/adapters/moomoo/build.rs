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

//! Regenerates the Rust types for the vendored gateway schema.
//!
//! The generated output is committed under `src/generated/`, so an ordinary build never runs this.
//! Generating at build time would need a `protoc` on the host, which this repository does not
//! require anywhere else. Regeneration is therefore explicit: set `MOOMOO_PROTO_REBUILD` and point
//! `PROTOC` at a compiler, then build the crate.
//!
//! The vendored definitions are `proto2` and carry `required` fields, which prost supports.

fn main() {
    println!("cargo::rustc-check-cfg=cfg(docsrs)");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=MOOMOO_PROTO_REBUILD");
    println!("cargo:rerun-if-env-changed=PROTOC");

    if std::env::var("MOOMOO_PROTO_REBUILD").is_err() {
        return;
    }

    regenerate();
}

/// Compiles every vendored definition into `src/generated/`.
///
/// Panics if the proto directory is missing or if `protoc` rejects the schema, because a
/// regeneration that silently produced nothing would leave stale committed output in place.
fn regenerate() {
    use std::path::PathBuf;

    let manifest_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set for build scripts"),
    );
    let proto_dir = manifest_dir.join("proto");
    let out_dir = manifest_dir.join("src").join("generated");

    println!("cargo:rerun-if-changed={}", proto_dir.display());

    let mut protos: Vec<PathBuf> = std::fs::read_dir(&proto_dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", proto_dir.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path: &PathBuf| path.extension().is_some_and(|ext| ext == "proto"))
        .collect();
    protos.sort();

    assert!(
        !protos.is_empty(),
        "no .proto files found in {}",
        proto_dir.display()
    );

    std::fs::create_dir_all(&out_dir)
        .unwrap_or_else(|e| panic!("cannot create {}: {e}", out_dir.display()));

    let mut config = prost_build::Config::new();
    config.out_dir(&out_dir);
    config.include_file("mod.rs");
    // The vendored definitions document their fields in Chinese. The generated Rust is committed,
    // and this repository's source conventions are English-only, so the comments stay in the
    // `.proto` files, which remain the reference for field semantics.
    config.disable_comments(["."]);

    config
        .compile_protos(&protos, &[proto_dir])
        .expect("protobuf compilation failed");
}
