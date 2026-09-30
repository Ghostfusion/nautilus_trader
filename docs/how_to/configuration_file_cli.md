# Validate and resolve a configuration file

The `nautilus config` command validates a JSON configuration file against an existing typed
configuration and prints the resolved configuration that the loader produced. The typed
constructors remain the canonical configuration API; the file is a view of them, so the file schema
is exactly the `Serialize`/`Deserialize` surface of the selected Rust type and loading never
introduces a second configuration model or precedence rule. See
[Configuration](../concepts/configuration.md) for the typed configuration surface.

Every subcommand writes one JSON document to standard output and signals failure through the
process exit status, so a CI step can tell a failure from a success. Console logging is disabled for
these subcommands so the document is the only standard output; set the `NAUTILUS_LOG` environment
variable to configure logging and keep console output.

## Choosing the configuration

The file is loaded as one of three existing typed configurations, selected by `--schema`:

| `--schema` | Typed configuration      |
| ---------- | ------------------------ |
| `kernel`   | `KernelConfig`           |
| `backtest` | `BacktestEngineConfig`   |
| `live`     | `LiveNodeConfig`         |

`--schema` defaults to `kernel`. A file whose schema does not match the selected type is rejected,
because each typed configuration denies unknown fields.

## Machine-readable output

Each invocation prints exactly one JSON document:

```json
{
  "schema": "nautilus.config.cli/v1",
  "command": "validate",
  "config": "kernel.json",
  "config_schema": "KernelConfig",
  "status": "ok"
}
```

`resolve` adds a `config_resolved` object holding the typed configuration after loading: the
built-in defaults with the file applied. On failure the document carries `status: "invalid"` and an
`error` field with the loader's own message, and the process exits non-zero.

| Status    | Exit code | Meaning                                                        |
| --------- | --------- | -------------------------------------------------------------- |
| `ok`      | 0         | The file loaded into the selected typed configuration.         |
| `invalid` | 1         | The file could not be read, decoded, or contained unknown keys. |

## validate

Checks that a file loads into the selected typed configuration. The check is the existing loader's
decoding and unknown-key rejection, not a separate schema check.

```bash
nautilus config validate kernel.json
```

Validation fails when the file cannot be read, when it is not valid JSON, or when it contains a key
the typed configuration does not define.

## resolve

Prints the resolved configuration so an operator can see exactly what the loader produced, including
the built-in defaults for any field the file omits.

```bash
nautilus config resolve kernel.json --schema kernel
```

```bash
cargo run -p nautilus-cli -- config resolve kernel.json --schema kernel
```

The printed `config_resolved` object is the typed configuration after loading, not a re-reading of
the file: field defaults are materialized, and the file's explicit values take precedence.

## Loading

The loader layers the built-in defaults, then the file. Unknown keys are rejected rather than
ignored, and there are no environment profiles or environment variables in the configuration
precedence. The file must be valid JSON; comments and trailing commas are not accepted.
