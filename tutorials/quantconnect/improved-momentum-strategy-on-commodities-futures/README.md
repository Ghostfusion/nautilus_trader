# Improved commodity momentum: scaling each bet by trend strength and by choppiness

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                     |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Futures contracts on raw materials: crude oil, natural gas, corn, wheat, sugar, live cattle, copper and similar                                                                                                           |
| How often it trades       | Once a month                                                                                                                                                                                                              |
| What you need             | A spreadsheet                                                                                                                                                                                                             |
| Where the rules come from | [QuantConnect strategy library, improved momentum strategy on commodities futures](https://www.quantconnect.com/tutorials/strategy-library/improved-momentum-strategy-on-commodities-futures)                             |
| The underlying research   | Baltas and Kosowski, [Demystifying Time-Series Momentum Strategies](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2140091)                                                                                          |
| How well it held up       | Mixed: the long published sample rewards the refinements, but a short out-of-sample test in the library page only beats its own plain version and not the market, and later work disputes how much of the effect survives |
| Also appears in           | [Sector momentum](../sector-momentum/README.md) in this collection, its cross-sectional cousin                                                                                                                            |

## The idea in one paragraph

Trend following on raw materials buys a contract when its price has been rising and sells it when the
price has been falling. The plain version bets the same amount whenever its direction is up, which
wastes money: a market racing away from its average deserves a bigger bet than one that is drifting,
and a market that swings wildly needs a smaller bet to carry the same risk. This version replaces the
on-or-off decision with a number between minus one and plus one that measures how convincing the rise
or fall has been, and it measures how choppy the market is with a formula that reads the daily high,
low, open and close rather than only the closing price. It then divides the whole portfolio's
leverage by a measure of how much the contracts move together, so that a basket of things which all
fall at once is not a hidden pile of the same bet. The result is a set of weights, one per contract,
rebuilt once a month.

## Why anyone believed it

A farmer who plants wheat does not know what it will fetch at harvest, so he sells a futures
contract in advance, and a bakery that needs wheat buys one, locking in a price. Both of them are
paying to remove uncertainty, and the speculator on the other side is paid for taking it. This is
the oldest story in commodity futures: the people who make a physical product would rather not carry
the price risk, and the people who take that risk earn a premium for doing so.

On top of that payment, information arrives slowly. A poor harvest is reported region by region, a
new pipeline is built over years, and an airline hedging its fuel bill buys in stages. Buyers who
hear news late push a price that has already started moving, so a rise tends to continue for a while.
The counterparty to this strategy is therefore the hedger who is happy to pay for certainty, and the
slow buyer who is still reacting to old news.

## An everyday comparison

Think of a greengrocer ordering fruit. If mangoes have sold faster every week for three months, the
grocer orders more of them than usual. But the grocer also watches how erratic the daily sales are:
if some days sell out and some days nothing moves, a rising weekly trend is less trustworthy, and it
would be reckless to order a large crate on the strength of it. Finally, the grocer notices that
mangoes, papayas and pineapples all sell well on the same hot days, so piling more credit into all
three is not three separate bets; it is one bet on hot weather. This strategy is that grocer's
reasoning, written as numbers.

## The rules, step by step

1. Choose a universe of liquid commodity futures traded on the main exchanges. The library page uses
   contracts on CME and ICE; a smaller set of seven to twelve contracts is enough to see the idea.
2. Collect daily data for each contract for at least twelve months: the open, the high, the low and
   the closing price of each day.
3. For each contract, compute the average of its daily percentage changes in price over the past
   twelve months, divide that average by its standard deviation divided by the square root of the
   number of days, and cap the result between minus one and plus one. This capped number is the
   trend signal, written `X`.
4. For each contract, estimate how choppy it has been over the past month, using the Yang and Zhang
   formula described below, which mixes four different views of the same month. Multiply the result
   by the square root of the number of trading days in a year (about 16) to put it on a yearly scale.
5. Compute the average correlation of all the contracts over the past three months. Correlation is a
   number between minus one and plus one saying how much two things move together; the average is
   taken over every pair, and each pair's correlation is multiplied by the signs of the two trend
   signals.
6. Turn that average correlation into a correlation factor, using the formula below. When the
   contracts move together, the factor is small; when they are independent, the factor is close to
   the square root of their number.
7. Give each contract the weight given by the weighting formula below, and cap each weight between
   minus one and plus one. A positive weight means buy the contract; a negative weight means sell it
   (called selling short: borrowing the contract, selling it, and buying it back later).
8. Choose a target for the whole portfolio's yearly choppiness, for example twelve percent, meaning
   that in a typical year the portfolio's value is expected to swing about twelve percent around its
   trend. The weights are scaled so that the portfolio aims at that number.
9. Rebuild the weights at the start of each month. Pay the cost of every change described below.

## The maths, with every symbol named

The trend signal is the statistical confidence of the recent drift.

```text
t = m / (s / sqrt(n))
```

- `t` is the trend strength, a plain number, negative for a fall.
- `m` is the average daily percentage change in the contract's price over the past twelve months.
- `s` is the standard deviation of those daily changes: how far a typical day sits from the average.
- `n` is the number of trading days in the twelve months, about 252.

The signal `X` is `t` itself, but no larger than one and no smaller than minus one. So a very strong
trend and a merely strong trend give the same bet, and a weak trend gives a smaller bet.

The Yang and Zhang estimate of how choppy a contract is:

```text
sigma_YZ^2 = sigma_OJ^2 + k * sigma_SD^2 + (1 - k) * sigma_RS^2
```

- `sigma_OJ` is the standard deviation of the overnight changes, from yesterday's close to today's open.
- `sigma_SD` is the standard deviation of the usual daily changes, close to close.
- `sigma_RS` is the average of the Rogers and Satchell range term, which reads the day's high, low,
  open and close and asks how far the price travelled inside each day.
- `k` is a weight that depends only on the number of days used, equal to `0.34 / (1.34 + (D + 1) / (D - 1))`.
- `D` is the number of days in the estimation window, about 21 for one month.

The estimate is a squared number, so the square root of the whole expression is the standard
deviation. Reading the daily high and low, not only the close, is what makes the estimate react
faster: a day can open, run up and fall back to close unchanged, which looks like no movement at all
if only closes are read.

The correlation factor:

```text
CF = sqrt(N / (1 + (N - 1) * rho_bar))
```

- `CF` is the correlation factor, a number of about one or more.
- `N` is the number of contracts in the portfolio.
- `rho_bar` is the average pairwise correlation, between minus one and plus one.

When the contracts are independent, `rho_bar` is near zero and `CF` is close to the square root of
`N`, which allows more leverage. When they all move together, `rho_bar` is near one and `CF` is near
one, which removes the accidental leverage.

The weight of each contract:

```text
w_i = X_i * sigma_target * CF / (N * sigma_i)
```

- `w_i` is the fraction of the account placed in contract `i`, positive for a buy, negative for a sell.
- `X_i` is that contract's trend signal.
- `sigma_target` is the target yearly choppiness of the whole portfolio, for example 0.12.
- `sigma_i` is that contract's estimated yearly choppiness from the Yang and Zhang formula.
- `N` and `CF` are as above.

The portfolio's return over the following month is then the sum of each weight multiplied by that
contract's return. The whole scheme is one sentence: divide the intended risk by the measured risk,
tilt the result by the confidence in the trend, and shrink everything by the tendency of the
contracts to move together.

## A worked example

Take one contract, copper, and one monthly rebuild. First the signal:

```text
m = 0.0005, s = 0.011, n = 252
t = 0.0005 / (0.011 / sqrt(252)) = 0.0005 / 0.000693 = 0.721
```

So `X` is 0.721, comfortably inside the cap of one. Now the choppiness, from four daily ingredients
over 21 days: `sigma_OJ` is 0.005, `sigma_SD` is 0.011, `sigma_RS` is 0.012, and `k` is
`0.34 / (1.34 + 22/20) = 0.34 / 2.44 = 0.139`:

```text
sigma_YZ^2 = 0.005^2 + 0.139 * 0.011^2 + 0.861 * 0.012^2
          = 0.000025 + 0.0000168 + 0.000124 = 0.000166
sigma_YZ  = 0.0129 per day
sigma_i   = 0.0129 * 16 = 0.206 per year, that is 20.6 percent
```

Now suppose seven contracts, with an average pairwise correlation of 0.10:

```text
CF = sqrt(7 / (1 + 6 * 0.10)) = sqrt(7 / 1.6) = sqrt(4.375) = 2.092
w  = 0.721 * 0.12 * 2.092 / (7 * 0.206) = 0.181 / 1.442 = 0.125
```

So copper gets 12.5 percent of the account. Repeat the calculation for the other six contracts and
the portfolio is built. The table below runs six monthly rebuilds of this one contract, using
invented but plausible numbers, with the composition of the choppiness estimate held at a typical
value and the correlation factor held at 2.092.

| Month | t      | Signal X | Yearly choppiness | Weight  | Next-month return | Contribution |
| ----- | ------ | -------- | ----------------- | ------- | ----------------- | ------------ |
| 1     | 0.721  | 0.721    | 20.6 percent      | 0.1250  | +4.0 percent      | +0.5000      |
| 2     | 0.900  | 0.900    | 18.0 percent      | 0.1793  | +2.0 percent      | +0.3586      |
| 3     | 1.200  | 1.000    | 19.5 percent      | 0.1839  | -3.0 percent      | -0.5517      |
| 4     | 0.400  | 0.400    | 22.0 percent      | 0.0652  | +1.0 percent      | +0.0652      |
| 5     | -0.600 | -0.600   | 21.0 percent      | -0.1024 | -2.0 percent      | +0.2048      |
| 6     | -1.400 | -1.000   | 23.0 percent      | -0.1559 | -5.0 percent      | +0.7795      |

Every contribution is the weight multiplied by the next month's return, in percent of the account.
The six months add up to +1.3564 percent from this one contract. The positions changed every month,
so some cost was paid. Counting the change in weight each month and a cost of 0.10 percent of the
amount traded:

```text
traded fraction = 0.125 + 0.054 + 0.005 + 0.119 + 0.168 + 0.054 = 0.525
Cost = 0.525 * 0.001 = 0.000525, that is 0.0525 percent
Net contribution = 1.3564 - 0.0525 = 1.304 percent over six months
```

Two things are worth noticing. First, in month 3 copper fell and the book lost money, but in the
down-trends of months 5 and 6 the strategy was short and made money; a plain long-only commodity
allocation would not have. Second, the weights move a lot month to month, so the cost is not
negligible. The example says nothing about whether the strategy works; it only shows how to apply the
rules and how the arithmetic behaves.

## What the research actually found

| Source                                                      | What it measured                                                             | Result                                                                                                                                                                                                                          |
| ----------------------------------------------------------- | ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Baltas and Kosowski, the underlying paper                   | Twelve futures contracts, intraday quotes from November 1999 to October 2009 | A signal built from the confidence of the trend, rather than its mere direction, gave the best out-of-sample performance while trading least, and the Yang and Zhang estimate minimised both bias and turnover                  |
| Quantpedia, summarising the time-series momentum literature | 58 futures across commodities, currencies, indices and bonds, 1965 to 2009   | Indicative performance of 20.7 percent a year, volatility 15.74 percent, worst fall 33.87 percent, reward-to-risk 1.31, for the simpler version without these refinements                                                       |
| QuantConnect's own implementation                           | The same family of contracts, January 2018 to September 2019                 | Reward-to-risk 0.198 for the improved version against minus 0.746 for the plain version and 0.46 for simply holding the market index                                                                                            |
| Huang, Li, Wang and Zhou                                    | A large cross-section of assets, in and out of sample                        | The statistical case for time-series momentum is weak once the standard test is corrected for the many ways it could be tried; the strategy still makes money, but no more than a rule that simply assumes past averages repeat |

Read together: the refinements do what they were designed to do, trading less and sizing each bet by
a better measure of risk, and in the long samples the plain strategy already earned a return. But the
advantage over simply holding the market is not established by the library page's short test, and a
serious line of research argues the statistical foundation is thinner than it appears. Hence Mixed
rather than Strong: the effect appears in a long sample, but the out-of-sample record is contested.

## How this project relates to it

This repository has a brief on commodity futures curves,
[strategies/books2/18_energy_and_commodities.md](../../../strategies/books2/18_energy_and_commodities.md).
Its Sections 6 and 7 report a related but different commodity signal: the previous day's movement in
the futures curve, fitted with level, slope and curvature terms. The slope leg earned an annualised
mean excess return of 1.77 percent with reward-to-risk 1.41 before costs, but after three
transaction-cost scenarios it netted 0.55 to 1.31 percent a year and its reward-to-risk fell from
1.47 to 0.20 after 2009. That is the same lesson as this tutorial's cost line: a commodity signal
that looks strong before costs can be reduced to a fraction of itself afterwards.

The second related piece is the repository's
[predictability brief](../../../strategies/books2/08_predictability_and_trading_strategies.md).
Its Section 3 treats momentum as a design problem and finds that the skewness of a momentum
strategy's returns is a property of the position rule, rising to a maximum at a horizon related to
the trend filter's response time. That is why the choice of estimation window and the shape of the
signal here matter as much as the direction of the trade.

## Where it goes wrong

- Crowding. Once a rule is published and cheap to trade, more money runs it, the buying arrives
  earlier, and the reward is split. The library page's test covers only twenty-one months.
- The choppiness estimate is still an estimate. The Yang and Zhang formula reacts faster than a
  simple close-to-close measure, but a sudden volatility shock, such as a war or a harvest failure,
  is not in last month's data at all.
- The correlation factor is backward-looking. It is built from the past three months, so it lowers
  leverage after a period of high co-movement, not before it.
- Costs on a monthly rebuild. Every month the weights change and the portfolio pays the gap between
  the buying and selling price; a universe of twenty noisy contracts would trade much more.
- A short sample flatters and misleads. A test of twenty-one months can make any rule look good or
  bad by chance; the library page's reward-to-risk of 0.198 is a single draw.
- The whole idea fails if the premium paid by hedgers disappears. If producers stop needing to lay
  off price risk, the drift that the trend rule measures would not exist.

## Try it yourself

You need nothing but a spreadsheet and a public source of daily commodity futures prices; the
library page and any finance website will give you the daily open, high, low and close.

1. Pick one contract, say copper, and paste five years of daily dates, opens, highs, lows and closes
   into a sheet.
2. Add a column for the daily change: today's close divided by yesterday's close, minus one.
3. Add a column for the twelve-month trend signal: the average of the last 252 daily changes, divided
   by their standard deviation divided by the square root of 252, capped between minus one and one.
4. Add a column for a one-month choppiness estimate: the standard deviation of the last 21 daily
   changes, multiplied by 16.
5. Add a column for the weight: the signal times 0.12, divided by the choppiness.
6. In the next row, multiply that weight by the next day's change from step 2. That is the daily
   contribution of this position.
7. Sum the contributions month by month, and subtract 0.10 percent each month the weight changes.

What to notice: most of the return comes from a few months, and in the falling months the weight is
negative, so the sheet makes money exactly when a long-only commodity position loses. If your sheet
shows a large gain, check whether the signal used data from the future, for example by looking
forward when the trend turned.

## Where this came from

- [QuantConnect strategy library: improved momentum strategy on commodities futures](https://www.quantconnect.com/tutorials/strategy-library/improved-momentum-strategy-on-commodities-futures),
  the rules as implemented: the capped trend signal, the Yang and Zhang estimate, the correlation
  factor, the twelve percent target, monthly rebuilding.
- Baltas and Kosowski, [Demystifying Time-Series Momentum Strategies](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2140091),
  the paper the rules are built from, and its companion on volatility estimators.
- Yang and Zhang, [Drift-Independent Volatility Estimation Based on High, Low, Open, and Close Prices](https://www.jstor.org/stable/10.1086/209650),
  the source of the choppiness estimate.
- [Quantpedia: time series momentum effect](https://quantpedia.com/strategies/time-series-momentum-effect),
  the indicative performance figures and the instrument count.
- [Quantpedia: momentum effect in commodities](https://quantpedia.com/strategies/momentum-effect-in-commodities),
  the commodity-specific version and its underlying papers.
- [strategies/books2/18_energy_and_commodities.md](../../../strategies/books2/18_energy_and_commodities.md),
  this repository's own study of commodity curve signals and their transaction costs.

## Words used in this tutorial

- correlation: a number between minus one and plus one saying how much two prices move together.
- drawdown: the fall from a peak to the following low, measured in percent.
- futures contract: an agreement to buy or sell something at a fixed price on a future date.
- hedge: a trade made to reduce the risk of another holding.
- selling short: borrowing something you do not own, selling it, and buying it back later.
- standard deviation: a measure of how far a typical value sits from the average.
- volatility: how much a price moves around its average, usually quoted per year.
- weight: the share of your money placed in one holding.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
