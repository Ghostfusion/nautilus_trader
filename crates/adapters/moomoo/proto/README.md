# Vendored protobuf definitions

These files are the message schema for the moomoo OpenD gateway frame protocol. They are copied
verbatim from the gateway client, Chinese comments included, so that a later gateway version can be
diffed rather than rediscovered.

- Source package: `moomoo-api`
- Version: `10.10.7008`
- Source path: `moomoo/common/pb/` in the installed package

## Why they are vendored

The adapter builds without network access and without a `protoc` on the host. The schema is therefore
part of the crate rather than fetched or compiled at build time. The generated Rust is committed
alongside, and the crate's build script is inert unless asked to regenerate.

## What is vendored

Only the transitive import closure of the protocols the adapter uses, which is 23 files:

```
Common.proto
GetGlobalState.proto
GetUserInfo.proto
InitConnect.proto
KeepAlive.proto
Notify.proto
Qot_Common.proto
Qot_GetBasicQot.proto
Qot_GetCorporateActionsDividends.proto
Qot_GetCorporateActionsStockSplits.proto
Qot_GetKL.proto
Qot_GetOrderBook.proto
Qot_GetSecuritySnapshot.proto
Qot_GetStaticInfo.proto
Qot_GetSubInfo.proto
Qot_GetTicker.proto
Qot_RequestHistoryKL.proto
Qot_RequestHistoryKLQuota.proto
Qot_Sub.proto
Qot_UpdateBasicQot.proto
Qot_UpdateKL.proto
Qot_UpdateOrderBook.proto
Qot_UpdateTicker.proto
```

The full package ships 184 definitions. The 21 above are the exact import closure of the set the
adapter needs, and every import resolves inside them, so no `google/protobuf` well-known type has to
be vendored or compiled.

The set grows by endpoint, not by accident: `Qot_RequestHistoryKLQuota` was added when the adapter
needed to read the historical allowance, and its own two imports were already present. The same
holds for `Qot_GetCorporateActionsDividends` and `Qot_GetCorporateActionsStockSplits`, which the
adapter sends for corporate actions and which import only `Qot_Common`.

Regeneration is additive and deterministic: adding these two definitions rewrote `mod.rs` and wrote
their two files, and left the other twenty-two generated files byte-identical.

## Regenerating

The definitions are `proto2`, with `required` fields, which prost supports. Regeneration needs a
`protoc` on the path, and the gateway client ships one at `moomoo/common/pb/protoc.exe`. From the
repository root, with the path to that binary:

```bash
MOOMOO_PROTO_REBUILD=1 PROTOC=/path/to/protoc cargo build -p nautilus-moomoo
```

The build script writes the generated Rust into `src/generated/`, which is committed. Regeneration is
deliberate rather than automatic so that a stale generated file is caught in review instead of
appearing as a build failure on a host without `protoc`.

Comments are disabled in the generated output. The definitions document their fields in Chinese, and
the generated Rust is committed, so the comments stay in the `.proto` files, which remain the
reference for field semantics.
