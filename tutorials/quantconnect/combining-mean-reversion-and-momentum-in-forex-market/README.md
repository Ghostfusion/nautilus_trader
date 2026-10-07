# Currencies that drift up for a while and then get pulled back

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                  |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | Four currency pairs against the United States dollar: the euro, the British pound, the Canadian dollar and the Japanese yen                                                                                                                            |
| How often it trades       | About once a month, when the strongest and weakest currencies are chosen again                                                                                                                                                                         |
| What you need             | A spreadsheet and several years of monthly exchange rates and, ideally, interest rates                                                                                                                                                                 |
| Where the rules come from | [QuantConnect strategy library, combining mean reversion and momentum in the forex market](https://www.quantconnect.com/tutorials/strategy-library/combining-mean-reversion-and-momentum-in-forex-market)                                              |
| The underlying research   | Alina Serban, [Combining mean reversion and momentum trading strategies in foreign exchange markets](https://ideas.repec.org/a/eee/jbfina/v34y2010i11p2720-2727.html) (2010), built on Balvers and Wu (2006)                                           |
| How well it held up       | Weak: the paper is one sample, the library's own re-run produced negative returns, and the page's stated result contradicts its own results table                                                                                                      |
| Also appears in           | [Currency momentum](../forex-momentum/README.md), the longer-horizon version of the same trend idea in currencies, and [Short-term reversal in stocks](../short-term-reversal-strategy-in-stocks/README.md) on the reversal half in a different market |

## The idea in one paragraph

An exchange rate wanders, but two forces pull on it. Over months it tends to drift in the direction
it has been drifting, which is momentum. Over longer stretches it drifts back toward a level that the
two countries' interest rates make sensible, which is mean reversion. This strategy measures both
forces for each of four currencies, uses them together to predict next month's move, buys the
currency with the highest predicted move and sells short the one with the lowest, and repeats each
month. The point of combining them is that momentum should catch the short drift while mean reversion
stops the strategy from chasing a currency that has already gone too far.

## Why anyone believed it

Interest rates differ between countries. In theory, if you borrow where interest is low and lend
where it is high, the exchange rate should move against you by exactly the interest you gained, so
the round trip earns nothing. This is called uncovered interest parity, and it is a straitjacket on
prices over the long run.

In practice the straitjacket is loose for months at a time. Currency markets are made of participants
who act at different speeds: banks and funds that react within seconds, exporters and importers who
trade on schedule, and slower investors who move only after a trend is obvious. When news arrives,
the fast group moves the price first and the slow group follows, so the price keeps going in one
direction for a while. The counterparty is that slow group, and also the firms that must convert
money on a calendar rather than at a price. Later, as the interest-rate gap asserts itself, the drift
reverses. The bet is that both parts of this picture are predictable.

## An everyday comparison

Picture someone walking a dog on a long leash. The owner crosses the park at a steady pace; that is
the long-run level the exchange rate is tied to. The dog, however, races ahead, sniffs, falls behind,
then catches up, and the leash keeps tugging it back toward the owner. Over a minute the dog's
position is a mix of its own dashing and the pull of the leash. Trying to trade only the dashing
misses the leash; trying to trade only the leash ignores where the dog actually is. This strategy
measures both and lets them argue.

## The rules, step by step

1. Choose the currencies. The page uses four pairs against the dollar: EURUSD, GBPUSD, USDCAD and
   USDJPY. The paper used the same four, with the German mark in place of the euro before 1999.
2. For each pair, take a long history of monthly exchange rates. Work with the natural logarithm of
   the rate rather than the rate itself, because the logarithm turns percentage moves into ordinary
   subtractions.
3. For each pair, compute two numbers from that history: a mean level, the average of the log rate,
   and a standard deviation, which measures how far the log rate typically sits from that average.
4. Compute the reversal number: how far the most recent month's log rate sits from the mean, measured
   in standard deviations. A large positive number means the currency is expensive relative to its
   own history.
5. Compute the momentum number: the change in the log rate over the last three months. A large
   positive number means the currency has been rising.
6. Fit one straight line for all four currencies at once, predicting next month's change in the log
   rate from the reversal number and the momentum number. This is done once, using all the history
   available before the starting date, and then held fixed.
7. At the start of each month, use the fitted line to predict next month's change for each of the four
   currencies.
8. Buy the currency with the highest predicted change and sell short the one with the lowest, each
   with the same amount of the account. If every prediction is positive, only buy the best one; if
   every prediction is negative, only sell short the worst one.
9. Hold for one month, then repeat from step 7.

## The maths, with every symbol named

Interest parity says a currency with higher interest should be expected to fall, so that the gain is
cancelled:

```text
r - r_foreign = expected change in the log exchange rate
```

- `r` is the home interest rate, quoted as a fraction per year.
- `r_foreign` is the foreign interest rate.
- The right side is the average change in the log exchange rate that the market expects.

The deviation from that expectation is what the strategy tries to predict:

```text
y = change in the log exchange rate next month
```

- `y` is the difference between the log exchange rate next month and this month.

The fitted line predicts `y` from the two forces:

```text
y = alpha + beta_rev * z + beta_mom * m + error
```

- `alpha` is the intercept, the average monthly change not explained by the two forces.
- `z` is the reversal number: `z = (x - mu) / sigma`, where `x` is the latest log rate, `mu` is its
  long-run average, and `sigma` is its standard deviation. The original paper used `(x - mu)` without
  dividing; the library divides by `sigma` so that currencies on different scales can be compared.
- `m` is the momentum number: the latest log rate minus the log rate three months earlier.
- `beta_rev` is the weight on reversal. In the paper's notation it equals `-(1 - delta)`, where
  `delta` is the speed at which the rate is pulled back to its mean, a number between zero and one.
- `beta_mom` is the weight on momentum, written `rho` in the paper, and it measures how much of a
  three-month rise continues next month.
- `error` is the part of next month's change that the two forces miss.

The paper's own fitted numbers, as reported by the library page, were a momentum weight of 0.042 and
a reversal weight of 0.9859, on monthly data from 1978 to 2008. The library's re-run on shorter, more
recent data reported a momentum weight of 0.0633 and a reversal weight of about 1.035. The page also
reports a statistical measure of reliability, the t-statistic, of -4.074 for reversal and 1.417 for
momentum; a value near or below two in size usually means the number could easily be a coincidence,
which is the case for momentum here. The page's own prose and its printed coefficients do not agree
on the sign of the reversal weight, which is noted here as a discrepancy in the source.

The trading step takes the four predicted values and sorts them. For the account's own return:

```text
R_account = w_long * R_long - w_short * R_short
```

- `w_long` and `w_short` are the fractions of the account placed on each side; the page uses 1.0 on
  each, so the account holds twice its own money.
- `R_long` is the return of the currency bought; `R_short` is the return of the currency sold short,
  so the minus sign turns a fall in that currency into a gain for the account.

The monthly cost of both sides is:

```text
Cost = t * c
```

- `t` is the value traded, counting both selling and buying, divided by the account value; replacing
  both sides of a two-sided position gives `t = 4.0`.
- `c` is the cost of one trade as a fraction of the amount traded, covering the gap between the buying
  and selling price. Two basis points, that is 0.0002, is a cautious figure for a major currency
  pair; one basis point is one hundredth of one percent.

## A worked example

The numbers are invented and the coefficients are chosen to be easy to read; they are not the paper's.
Four currencies, with the reversal number `z` and the momentum number `m` measured at the end of the
formation window, and `alpha = 0.001`, `beta_rev = -0.005`, `beta_mom = 0.15`:

| Currency | z     | m      | Predicted y        | Rank |
| -------- | ----- | ------ | ------------------ | ---- |
| EURUSD   | -1.60 | -0.010 | +0.0075, or 0.75%  | 1    |
| GBPUSD   | 0.40  | +0.030 | +0.0035, or 0.35%  | 2    |
| USDCAD   | 1.20  | +0.020 | -0.0020, or -0.20% | 3    |
| USDJPY   | 2.40  | +0.050 | -0.0035, or -0.35% | 4    |

The euro is below its long-run average (a negative `z`, so reversal is positive) and has fallen over
three months (negative momentum). The yen shows the opposite: it is far above its
average and has risen. The euro's predicted move is the highest and the yen's is the lowest, so the
strategy buys the euro and sells short the yen, each side with the whole account.

Suppose the previous month it had bought the pound and sold short the Canadian dollar, so all four
positions change. Over the coming month the euro rises 1.2 percent, and the dollar-yen rate falls
0.4 percent, so the short yen position gains 0.4 percent. Then:

```text
Return before costs = 1.0 * 1.2 + 1.0 * 0.4 = +1.6 percent
t = 4.0  (two positions closed and two opened, each the whole account)
Cost = 4.0 * 0.0002 = 0.0008, that is 0.08 percent
Net return for the month = 1.6 - 0.08 = +1.52 percent
```

Not one of these numbers is a forecast; they exercise the arithmetic only. Notice that the cost line
is small in currencies, which is one reason such strategies are tried there, and that the account is
again being run with borrowed money, since two positions each take the whole account. The page's own
implementation also ignores the interest you would earn or pay while holding each currency, which for
a strategy built on interest rates is a real omission.

## What the research actually found

| Source                                                                     | What it measured                                                             | Result                                                                                                                                                                                                                                                             |
| -------------------------------------------------------------------------- | ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Serban (2010), as reported by the library page                             | Monthly exchange rates for four currencies, 1978 to 2008                     | The model explained about 3.89 percent of next month's variation, with a momentum weight of 0.042 and a reversal weight of 0.9859; the paper reported abnormal returns from deviations from interest parity                                                        |
| QuantConnect tutorial, the re-run                                          | The same four pairs, available data from 2004, tested across several windows | Explained only 1.4 percent of the variation, with a reversal t-statistic of -4.074 and a momentum t-statistic of 1.417; annual returns in three test windows were -1.938, -4.008 and -0.958 percent, with worst falls near 20 percent                              |
| QuantConnect tutorial, stated headline                                     | The same run                                                                 | The page's abstract claims "a fairly stable annual return of 11 percent, a 0.8 Sharpe ratio and 11 percent drawdown", which its own results table and sensitivity table contradict; a reader should treat the tables, not the summary sentence, as the measurement |
| Quantpedia's own currency mean-reversion study                             | Six currency futures, monthly rebalancing, 2007 to 2024                      | A simple reversal rule was weak: a reward-to-risk figure of 0.12 with stable weights and 0.35 with aggressive weights, after costs                                                                                                                                 |
| Menkhoff, Sarno, Schmeling and Schrimpf, via the Quantpedia momentum entry | More than forty currencies, 1976 to 2010                                     | Winners beat losers by up to about 10 percent a year, but the returns are concentrated in hard-to-trade currencies and are eaten by costs; the tradable index returned 7.61 percent a year with a reward-to-risk figure of 0.3 and a worst fall of 45.87 percent   |
| This repository's stylized-facts brief                                     | Long memory in twelve currency pairs                                         | With a test that accounts for multifractality, eleven of the twelve pairs show no long memory at all, which is evidence against a simple persistent pull                                                                                                           |

The honest picture is a small modelled effect that survives in the paper's long sample and does not
survive the library's shorter re-run. The page says plainly that "the returns and Sharpe Ratios
obtained are not as good as what the paper claimed", and its tables show losses. There is broader
support for currency momentum as a real but cost-sensitive effect, and little support for a strong,
simple pull back to a level. The grade is Weak.

## How this project relates to it

This repository's [Macro, rates and foreign exchange brief](../../../strategies/books2/07_macro_rates_and_fx.md)
is the closest match: it collects measured evidence on what moves exchange rates, including a study
in which a large language model reading economic data releases produced a currency signal with a
reward-to-risk figure above 0.7 (`2608.00761v1`). The broader
[predictability brief](../../../strategies/books2/08_predictability_and_trading_strategies.md) records
the finding that such signals are mostly real but decay and crowd, which is the right way to read
this one. The no-long-memory result for currencies is in the
[stylized facts and scaling brief](../../../strategies/books2/13_stylized_facts_and_scaling.md).

There is no engine in this repository that implements this specific combined rule; the manual closest
in spirit is [Intraday and medium-frequency systematic trading](../../../docs/usermanauls/intraday-systematic/README.md),
which teaches how to turn one written rule into a tested program, and which is the tool a reader would
use to test a rule like this on recorded prices before believing anything.

## Where it goes wrong

- The sample is tiny. Four currencies, monthly data, and a test window of a few years leaves very few
  independent observations, so a fitted weight can look meaningful and still be noise. The momentum
  weight here did not clear the usual reliability bar even in the page's own run.
- Two forces can cancel. The reversal number and the momentum number often point in opposite
  directions, so the prediction can be near zero and the ranking arbitrary. When the regressors move
  together, this is called multicollinearity, and the page notes it as a hazard.
- The model is estimated once and frozen. The page fits the line on all history before the test, then
  never updates it, so a change in how currencies behave after the estimation window is invisible to
  it.
- Interest is ignored. The theory is about interest-rate differences, yet the implementation trades
  spot rates and pays no attention to the interest earned or owed while a position is held, which for
  this idea is a missing piece rather than a detail.
- Costs and market access. Currency deals are cheap for large players and much less so for a small
  account, and the spread widens exactly when markets are stressed. The momentum literature's own
  caveat is that its returns live where trading is hardest.
- The regime can end. The 1978 to 2008 period includes fixed and managed exchange rates, several
  currency unions and a financial crisis; a rule calibrated there need not describe a floating market.

## Try it yourself

You need a spreadsheet and any free source of monthly exchange rates for four currencies, plus
short-term interest rates if you want to add the missing piece.

1. Build a sheet with one column per currency, holding the natural logarithm of the monthly rate.
   Any spreadsheet has a function for the logarithm.
2. Add a column for the long-run average of each log rate, and one for its standard deviation over
   the same span.
3. Add a reversal column: the latest log rate minus the average, divided by the standard deviation.
4. Add a momentum column: this month's log rate minus the log rate three months earlier.
5. For a block of about sixty months, fit a line predicting the next month's change from the reversal
   and momentum columns, and note the two weights.
6. Apply the weights to the most recent month, predict each currency's move, and rank them.
7. Follow the top and bottom currency on paper for the next month, and subtract about two basis
   points on each side of each trade.

What to notice: the reversal column is almost always the larger of the two in size, and the two
columns often disagree on the sign, so the prediction is small and the ranking can flip on a rounding
difference. If the ranking looks decisive, you have probably fit the weights on the same months you
are judging them by.

## Where this came from

- [QuantConnect strategy library: combining mean reversion and momentum in the forex market](https://www.quantconnect.com/tutorials/strategy-library/combining-mean-reversion-and-momentum-in-forex-market),
  the rules as implemented, the four pairs, the fitted weights, the reliability figures and the
  sensitivity table.
- Alina Serban, [Combining mean reversion and momentum trading strategies in foreign exchange markets](https://ideas.repec.org/a/eee/jbfina/v34y2010i11p2720-2727.html)
  (2010), the paper behind the rule. The published PDF at the address the library page cites could not
  be reached, so the paper's numbers here are quoted from the library page's report of them.
- Ronald Balvers and Yangru Wu, [Momentum and mean reversion across national equity markets](https://pdfs.semanticscholar.org/c98c/533334b6962d06b44ae7a796b5fcffbe2fe4.pdf),
  the model the currency version adapts.
- [Currency momentum factor](https://quantpedia.com/strategies/currency-momentum-factor), the
  Quantpedia entry with the tradable momentum figures, and
  [How to build mean reversion strategies in currencies](https://quantpedia.com/how-to-build-mean-reversion-strategies-in-currencies/),
  its own currency reversal study.
- `1601.00903v1`, on long memory in currency pairs, as reported in
  [the stylized facts and scaling brief](../../../strategies/books2/13_stylized_facts_and_scaling.md).
- `2608.00761v1`, AI and exchange rate predictability, as reported in
  [the macro, rates and foreign exchange brief](../../../strategies/books2/07_macro_rates_and_fx.md).

## Words used in this tutorial

- carry: the interest earned or paid for holding a currency position, sometimes called the cost of
  funding.
- momentum: the tendency of something that has been rising to keep rising for a while.
- mean reversion: the tendency of a price that has moved far from its usual level to move back toward
  it.
- multicollinearity: the state in which two of the numbers used to make a forecast move together, so
  their separate weights are hard to pin down.
- short selling: borrowing something you do not own, selling it, and buying it back later, so a fall
  in its price is a gain.
- t-statistic: a number that says how large an estimate is compared with its own uncertainty; values
  near or below two in size are usually treated as too weak to trust.
- uncovered interest parity: the idea that a higher interest rate in one country should be cancelled
  by a fall in its currency, so a borrowing-and-lending round trip earns nothing.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
