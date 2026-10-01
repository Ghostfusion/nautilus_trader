# Options: the beginner manual

An option is a contract that gives you a right to buy or to sell something at a fixed
price, on or before a fixed date. You pay a small amount now, called the premium, and in
return you decide later whether to use the right. An option is not a promise to trade but a
choice, and that choice is why options are used to protect a position, to bet on a move, or
to earn a fee for accepting an obligation someone else wants to avoid.

This manual teaches options through the NautilusTrader engine. You will see how an option
contract is described in code, how a whole row of contracts at one expiry, called an
option chain, is streamed and aggregated, how the Greeks measure an option's
sensitivities, and how the repository builds a volatility surface from chain prices. You
will run a complete option-chain backtest on your machine with no network and no
credentials.

## Who should read this

- You have never traded an option and want the words and the arithmetic first.
- You write Python and want to see how option instruments, Greeks, and chains appear in
  this repository.
- You want to know exactly which parts of option pricing exist in Python and which exist
  only in Rust, so you do not waste time looking for an import that is not there.

## Prerequisites

- A working Python environment for this repository. See the first-run lecture for the
  exact commands.
- No financial background. Every term is defined the first time it appears.
- No Rust toolchain is required for the Python parts. One lecture runs Rust tests to show
  the pricing engine; the commands are given in full.

## Lectures

| File                                                     | What it covers                                                                                 | Reading | Working |
| -------------------------------------------------------- | ---------------------------------------------------------------------------------------------- | ------- | ------- |
| [01-what-is-options.md](01-what-is-options.md)           | Calls, puts, strikes, expiries, premiums, and three worked payoffs by hand.                    | 20 min  | 20 min  |
| [02-the-engine-view.md](02-the-engine-view.md)           | The instrument types, the data types, the config, and the crate and doc paths.                 | 15 min  | 10 min  |
| [03-first-run.md](03-first-run.md)                       | The smallest complete option-chain backtest, with its real output.                             | 15 min  | 25 min  |
| [04-sample-data.md](04-sample-data.md)                   | The committed chain and underlying CSVs, their columns, and how to inspect them.               | 15 min  | 15 min  |
| [05-build-the-strategy.md](05-build-the-strategy.md)     | Exercise style, a tree price, the implied forward check, and a Greeks subscription.            | 25 min  | 40 min  |
| [06-measure-and-evaluate.md](06-measure-and-evaluate.md) | Greeks in plain words, surface checks, fit confidence, and observation counts.                 | 20 min  | 20 min  |
| [07-risks-and-limits.md](07-risks-and-limits.md)         | Time decay, implied volatility moves, the wrong exercise assumption, and serialization limits. | 20 min  | 15 min  |
| [08-exercises.md](08-exercises.md)                       | Six exercises with solutions and one break-it-on-purpose task.                                 | 10 min  | 60 min  |
| [09-go-further.md](09-go-further.md)                     | Honest gaps, the concept docs to read next, and what to learn after that.                      | 10 min  | 0 min   |

## Sample data

The committed files live in [sample_data/](sample_data/). They are two small, deterministic
CSV snapshots: a Deribit-style BTC option chain and the matching underlying quotes. Every
column, unit, row count, and generator command is documented in
[sample_data/README.md](sample_data/README.md). No network is needed to read them.

## How to use this manual

1. Read 01 and 02 once, without running anything.
2. Run the program in 03 and read its output.
3. Open the CSVs in 04 and check the columns by hand.
4. Work through 05, which is the core of the manual.
5. Read 06 and 07 before you size any real position.
6. Do the exercises in 08 with the solutions covered.

The manual never says "simply", "just", or "obviously". If a step is unclear, the fault is
the explanation, not you.
