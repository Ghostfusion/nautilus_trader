# Catalogue of the tutorial collection

Date: 2026-10-07. Revision 1.

Every tutorial in this collection, grouped by the source library it came from. A strategy that
appears in several libraries is listed once, and its own page names the other libraries where
it appears. Open a tutorial from the link in its row.

## Foundations

Primers for a reader with no background.

| Tutorial                                                                                                        | What it trades | How well it held up |
| --------------------------------------------------------------------------------------------------------------- | -------------- | ------------------- |
| [What a market is](foundations/01_what-a-market-is.md)                                                          |                |                     |
| [What you can buy and sell](foundations/02_what-you-can-buy-and-sell.md)                                        |                |                     |
| [Orders and how they execute](foundations/03_orders-and-how-they-execute.md)                                    |                |                     |
| [What a strategy is](foundations/04_what-a-strategy-is.md)                                                      |                |                     |
| [Return, risk and drawdown](foundations/05_return-risk-and-drawdown.md)                                         |                |                     |
| [Costs, fees and taxes: what a trade costs before it can make anything](foundations/06_costs-fees-and-taxes.md) |                |                     |
| [How a backtest lies: the standard ways a history can fool you](foundations/07_how-a-backtest-lies.md)          |                |                     |
| [Regimes, and why nothing lasts](foundations/08_regimes-and-why-nothing-lasts.md)                               |                |                     |
| [What the evidence says](foundations/09_what-the-evidence-says.md)                                              |                |                     |
| [How to read a claim](foundations/10_how-to-read-a-claim.md)                                                    |                |                     |

## QuantConnect strategy library

Published strategies with named sources, written up by QuantConnect staff and contributors.

| Tutorial                                                                                                              | What it trades                                                                                        | How well it held up                                                                                                                                                                                                                 |
| --------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Sector momentum: buying what has already been winning, one sector at a time](quantconnect/sector-momentum/README.md) | Baskets of American shares, one basket per industry sector, held through funds that track each sector | Mixed: a long published sample with a modest reward for the risk, replications that survive costs with funds, and independent tests that shrink the effect once the market exposure and the number of rules tried are accounted for |

## This project

The ideas this repository implements and the measurements it runs on them.

| Tutorial                                                                                 | What it trades                                                                                                     | How well it held up                                                                                                                                                                                                |
| ---------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| [Whether sector rules are allowed: the regime verdict](project/regime-verdict/README.md) | Nothing. It decides whether rules that buy and sell groups of shares are allowed to be used on a given set of data | Mixed: the measuring tool is accurate on invented data whose answer is known, but the real-market patterns it looks for are weak in the studies the project reviewed and several are indistinguishable from chance |

Tutorials listed: 12.
