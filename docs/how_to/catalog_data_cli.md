# Inspect, validate, and convert a data catalog

The `nautilus catalog` command exposes data subcommands over the Parquet catalog and its existing
loaders: `inspect`, `validate`, and `convert`, alongside the existing `migrate-parquet`. The
`download` and `generate` subcommands are declared but not yet available; see
[download and generate](#download-and-generate).

Every subcommand writes one JSON document to standard output and signals failure through the process
exit status, so a CI step can tell a failure from an empty result. Console logging is disabled for
these subcommands so the document is the only standard output; set the `NAUTILUS_LOG` environment
variable to configure logging and keep console output.

## Machine-readable output

Each invocation prints exactly one JSON document:

```json
{
  "schema": "nautilus.catalog.cli/v1",
  "command": "inspect",
  "status": "ok",
  "catalog": "/data/catalog",
  "data_types": [],
  "instruments": []
}
```

The `status` is one of:

| Status        | Exit code | Meaning                                                      |
| ------------- | --------- | ------------------------------------------------------------ |
| `ok`          | 0         | The command completed; a listing may still be empty.         |
| `invalid`     | 1         | Validation found a malformed catalog.                        |
| `unsupported` | 1         | The subcommand has no loader or generator surface available. |
| `error`       | 1         | The command failed, for example the catalog cannot be read.  |

An empty listing is `ok` with an empty array, so it is distinguishable from any failure. Fields are
the same across the subcommands: `schema`, `command`, `status`, and a command-specific payload. On
failure the payload carries `problems` or `prerequisite` instead of the success fields.

## inspect

Lists the data types, identifiers, and coverage a catalog holds, plus the instrument definitions the
reader resolves.

```bash
nautilus catalog inspect /data/catalog
```

```bash
cargo run -p nautilus-cli -- catalog inspect /data/catalog
```

The payload lists each data type directory under `data/` with its immediate identifier directories,
the number of Parquet files, and the `ts_init` intervals parsed from the file names. A separate
`instruments` array lists each resolved instrument `id` and `class`.

## validate

Checks the catalog's layout and Arrow schemas, then decodes each built-in data family through the
catalog reader.

```bash
nautilus catalog validate /data/catalog
```

The schema and layout check reuses the migration preflight planner; the decode sweep reads through
`CatalogReader`. A malformed catalog is reported with `status: "invalid"`, the reader's own error in
`problems`, and a non-zero exit status. A valid catalog is `status: "ok"` with an empty `problems`
array.

## convert

Copies supported data families from a source catalog into a separate destination catalog. The source
is never modified, and the destination must be new or empty.

```bash
nautilus catalog convert /data/catalog-old /data/catalog-new
```

Each family is read with `CatalogReader` and written with `CatalogWriter`, so the destination holds
files with the current encoder, and a backtest or a `ParquetDataCatalog` reads them directly.

| Flag              | Meaning                                                                   |
| ----------------- | ------------------------------------------------------------------------- |
| `--data-type`     | A data family to convert. Repeatable; defaults to every supported family. |
| `--identifier`    | Restrict the conversion to an identifier. Repeatable.                     |
| `--source-option` | A source object-store option in `key=value` form. Repeatable.             |
| `--target-option` | A destination object-store option in `key=value` form. Repeatable.        |

Families the reader cannot round-trip are reported in `skipped` with a reason rather than converted.
This includes custom data that needs a registered Arrow decoder and record families such as account
state. Select a custom family explicitly with `--data-type Custom:<type_name>` to attempt it.

Both locations can be local paths or supported object-store URIs. Pass native object-store settings
with repeated `--source-option key=value` and `--target-option key=value` arguments, as for
[Parquet migration](migrate_parquet_catalog.md).

## download and generate

These subcommands are declared as part of the data surface but cannot run with the dependencies
`nautilus-cli` carries. Each reports `status: "unsupported"`, the missing prerequisite, and a
non-zero exit status; neither fetches nor writes any data.

- `download` would fetch provider market data and needs a provider data client reachable from
  `nautilus-cli`. The crate depends on no adapter crate, so no provider loader or network client is
  available. Such a client would read its provider API key from the environment; nothing is written
  to disk.
- `generate` would produce synthetic market data and needs a Rust market-data generator to delegate
  to. The workspace has none: the `generate_*` helpers in `crates/backtest/benches` are
  benchmark-local, and `TestOrdersGenerator` emits orders rather than market data.

## Credentials and licensing

No command embeds, writes, or echoes provider credentials, and no data is bundled into the
repository. The read-only commands and `convert` only read and write catalog files and object-store
settings that the caller passes explicitly. `download`, which would need a provider API key, is not
available and reads no environment variables.

## Data contract

The catalog layout, the Arrow schema, and the batch metadata these commands rely on are documented
in [Data catalog](../concepts/data/catalog.md) and the colocated
[catalog data contract](https://github.com/nautechsystems/nautilus_trader/blob/develop/crates/persistence/src/catalog/README.md).
