# Catalog data contract

The on-disk contract for the auxiliary data the catalog stores: the directory layout, the Arrow
schema emitted for a corporate action batch, and the metadata carried with it. This readme sits
next to the backend-neutral catalog APIs in this module, following the convention of
`crates/adapters/binance/src/spot/sbe/README.md`, which colocated a readme with the code it
documents. The prose form of this contract is in `docs/concepts/data/catalog.md`.

## Catalog layout

- Root: the `ParquetDataCatalog` base path, a local path or an object-store URI.
- Directory: `data/{path_prefix}/{identifier}/`, where the identifier is made URI-safe.
- File: `{start_timestamp}_{end_timestamp}.parquet`, ISO 8601 with `:` and `.` replaced by `-`.

Built-in path prefixes come from the `for_each_data_type!` table in
`crates/model/src/data/mod.rs`. `parquet_data_path_prefix` in `crates/persistence/src/catalog/types.rs`
maps a `NautilusDataType` to its prefix, and `CatalogStore::make_path` in
`crates/persistence/src/backend/parquet/catalog/store.rs` joins `data`, the prefix, and the
URI-safe identifier.

A corporate action uses the prefix `corporate_actions` and is partitioned by instrument ID:

```text
data/
  corporate_actions/
    AAPL.XNYS/
      2024-01-01T00-00-00-000000000Z_2024-12-31T23-59-59-999999999Z.parquet
```

Write with `ParquetDataCatalog.write_corporate_actions(...)`; read with
`ParquetDataCatalog.query(data_type=NautilusDataType.CorporateAction, ...)`, or the Rust
`CatalogReader::corporate_actions`. Period consolidation accepts the directory as
`corporate_actions` (`crates/persistence/src/backend/parquet/consolidation.rs`).

## Arrow schema

The field specs are in `crates/serialization/src/arrow/corporate_action.rs`; the Arrow type of each
encoding is resolved by `crates/serialization/src/arrow/json.rs`.

| Field          | Arrow type                     | Nullable | Content                              |
| -------------- | ------------------------------ | -------- | ------------------------------------ |
| `action`       | `Dictionary(Int8, Utf8)`       | No       | The kind name, SCREAMING_SNAKE_CASE. |
| `value`        | `Utf8`                         | No       | The decimal as a string.             |
| `new_symbol`   | `Utf8`                         | Yes      | The new symbol, when present.        |
| `effective_ns` | `Timestamp(Nanosecond, "UTC")` | No       | Effect instant.                      |
| `ts_event`     | `Timestamp(Nanosecond, "UTC")` | No       | Event instant.                       |
| `ts_init`      | `Timestamp(Nanosecond, "UTC")` | No       | Init instant.                        |
| `identifier`   | `Utf8`                         | Yes      | The instrument ID row identifier.    |

The `identifier` column is appended by `schema_for_type_with_identifier` in
`crates/serialization/src/arrow/json.rs`; it is a column, not schema metadata.

## Batch metadata

The batch schema metadata keys are defined in `crates/serialization/src/arrow/mod.rs` and set by
`metadata_for_type` (`type_name`) and `CorporateAction::get_metadata` (`instrument_id`):

| Key             | Value                                              |
| --------------- | -------------------------------------------------- |
| `type_name`     | `CorporateAction`, the catalog type discriminator. |
| `instrument_id` | The instrument ID, such as `AAPL.XNYS`.            |

Decoding reads `instrument_id` from this metadata, and the remaining fields from the columns above.

## Calendar data

Calendar data is a separate immutable input, not a catalog family. The file format is JSON under
`crates/model/resources/calendars/` with the schema identifier `nautilus-trading-calendar/v1`,
loaded and validated by `crates/model/src/calendars/mod.rs`. The format is documented in
`docs/concepts/trading_calendars.md`.
