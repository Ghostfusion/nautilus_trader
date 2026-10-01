# Factor research and portfolio construction

A **factor** is a number you compute for each instrument from data you already have, so that the
numbers put the instruments in an order. A **portfolio** is the set of positions you hold, and a
factor portfolio is one built by holding more of the good-ranked instruments and less of the
bad-ranked ones, then rebalancing on a schedule. The whole craft is making the ranking honest: the
factor must use only information that existed at the decision instant, and the result must survive
being tested on data the ranking never saw.

This manual builds one complete factor study. You compute a five-day momentum factor over four
instruments from a committed price panel, turn the ranking into target weights, label the outcome,
split the period so training never sees the future, and correct the reported Sharpe ratio for the
number of things you tried.

## Who should read this

You have no finance background and no programming background. You can run a command in a terminal.
Every program is given whole and explained line by line, so you do not need to write code from
scratch. If you have never seen a price chart, [01](01-what-is-factor-portfolio.md) starts from
what a price is.

## The lectures

| Lecture                                                       | What it covers                                                                   | Time   |
| ------------------------------------------------------------- | -------------------------------------------------------------------------------- | ------ |
| [01-what-is-factor-portfolio](01-what-is-factor-portfolio.md) | What a factor and a portfolio are, the vocabulary, three hand-worked examples    | 30 min |
| [02-the-engine-view](02-the-engine-view.md)                   | How this repository models the style: the research crate, the Python surface     | 20 min |
| [03-first-run](03-first-run.md)                               | Setup and the smallest complete program that ranks and weights                   | 25 min |
| [04-sample-data](04-sample-data.md)                           | The committed price panel: its columns, units, and how to inspect it             | 20 min |
| [05-build-the-strategy](05-build-the-strategy.md)             | The numbered build: factor, membership, weights, labels, split, target pipeline  | 60 min |
| [06-measure-and-evaluate](06-measure-and-evaluate.md)         | The split contract, purge and embargo, the deflated Sharpe ratio, report reading | 45 min |
| [07-risks-and-limits](07-risks-and-limits.md)                 | How a factor study lies to you, and what the engine does and does not enforce    | 35 min |
| [08-exercises](08-exercises.md)                               | Six exercises with solutions, plus one deliberate breakage                       | 45 min |
| [09-go-further](09-go-further.md)                             | Honest gaps, the concept pages to read next, and what to learn after this        | 15 min |

## Prerequisites

- A working NautilusTrader development environment. See the
  [developer guide](../../developer_guide/).
- Comfort running a command in a terminal.
- The sample panel in [`sample_data/`](sample_data/README.md). It is committed, so you need no
  market data subscription and no network access to follow the whole manual.

The factor *pipeline* (features, panels, membership series, datasets) is **Rust only** in this
repository: there is no Python binding for it. The lectures say so at the point of use and
demonstrate that part by running the crate's own tests. The research *statistics* (split contracts,
leakage policies, labels, significance, identity) are Python and are demonstrated with Python you
can run.

## Conventions used here

- Prices and quantities are decimal. Time is integer nanoseconds since the Unix epoch (1 January
  1970), which is how the engine stores every timestamp.
- Every code block is a complete program or a complete step of one, and the output below it is real
  output from running it.
- A statement about what the engine does cites the file that does it.
- Where a feature exists only in Rust, the lecture says so instead of showing Python that does not
  work.

## Sample data inventory

| File                                                         | Rows | What it is                                 |
| ------------------------------------------------------------ | ---- | ------------------------------------------ |
| [`sample_data/price_panel.csv`](sample_data/price_panel.csv) | 144  | Daily closes for four instruments, 36 days |

See [`sample_data/README.md`](sample_data/README.md) for every column, its units, the generator
command, and the provenance.
