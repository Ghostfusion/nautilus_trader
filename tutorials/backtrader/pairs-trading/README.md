# Pairs trading: two related things drift apart, and the bet that they come back

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                 |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Gold against silver, and a few other linked pairs such as gold against platinum or the Canadian dollar against oil; the two legs are bought and sold together                                                                                         |
| How often it trades       | The hourly backtests trade a few times a day; the daily backtests trade a few times a month                                                                                                                                                           |
| What you need             | A spreadsheet for the simple version, and Python with a data file for the Kalman-filter and copula versions                                                                                                                                           |
| Where the rules come from | [The compendium's pairs trading article](https://backtrader.readthedocs.io/en/latest/strategies-series/en/11-pairs-trading.html)                                                                                                                      |
| The underlying research   | Gatev, Goetzmann and Rouwenhorst, [Pairs Trading: Performance of a Relative Value Arbitrage Rule](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=227415)                                                                                         |
| How well it held up       | Disputed: the original study found profits over 1962 to 2002, while the compendium's three representative gold/silver backtests on modern data each lost a little                                                                                     |
| Also appears in           | [Pairs trading with stocks](../../quantconnect/pairs-trading-with-stocks/README.md) and [Intraday dynamic pairs trading](../../quantconnect/intraday-dynamic-pairs-trading-using-correlation-and-cointegration-approach/README.md) in this collection |

## The idea in one paragraph

Some things are linked: gold and silver have been mined, held and traded together for centuries, so
their prices tend to keep a roughly stable relationship. Usually they move up and down together. But
sometimes one runs ahead of the other, and the gap between them opens wider than usual. Pairs trading
notices that gap. When gold has become unusually expensive relative to silver, the rule sells gold
and buys silver in matched amounts; when the gap closes again, it closes both. Because the two trades
are opposite and roughly equal in size, a market-wide move up or down tends to cancel out, and what
is left is the movement of the gap itself. The bet is not that either metal will rise. It is that the
gap will return to its usual size.

## Why anyone believed it

The counterparty is anyone buying one metal and not the other for reasons that have nothing to do
with the relationship - a jeweller stocking up before a holiday, a fund forced to sell part of a
position, a producer hedging its future output. Those flows push one leg around and leave the other
behind, opening the gap. The trader who sells the expensive leg and buys the cheap one provides the
other side of that flow and is paid, in effect, for taking the risk that the gap keeps widening. The
Morgan Stanley trading desk that popularised the idea in the 1980s bought into exactly this story:
the two legs share a market's overall direction, so the shared part cancels and only the relationship
is left to trade.

## An everyday comparison

Two walkers leave the same village each morning and take different paths to the same hilltop. On a
windy day they drift apart for a while, but because they are heading for the same place, the distance
between them tends to shrink again by the time they arrive. A rule that walked a mile along whichever
path was lagging, and offset it by walking back along the other, would make money every time the two
paths rejoin and lose it on the days when one walker has genuinely settled somewhere else. That last
case is the whole risk: the paths can diverge for good.

## The rules, step by step

The shared mechanism is: define the gap between two linked prices, measure how unusual the gap is
right now, trade against it when it is unusual, and stop when it returns to normal or runs away.

1. Choose two things with a real reason to be linked. Gold and silver are canonical because both are
   precious metals held as stores of value; oil and the Canadian dollar are linked because Canada
   exports oil.
2. Build the gap, called the spread. The simple version uses the difference of the two logarithms of
   the prices, so the gap is a ratio rather than a subtraction of different units. A small number of
   units of one thing per unit of the other is fixed as the hedge ratio.
3. Measure how unusual the gap is with a z-score: the number of standard deviations the gap sits from
   its own recent average, where a standard deviation measures how far readings typically scatter. A
   z-score of plus two means the gap is unusually wide; minus two means unusually narrow.
4. Trade against the gap. When it is unusually wide, sell the first leg and buy the second; when it is
   unusually narrow, buy the first and sell the second. Selling something you do not own is called
   short selling, and it profits if the price falls.
5. Size each leg to the same amount of money (about five percent of the pot in these files), so the
   two legs offset each other.
6. Close both legs when the z-score comes back near zero, or when it runs beyond a stop level, which
   means the gap kept widening instead of closing.

Three variants of step 2 and step 3 are worth reading.

The fixed-ratio gold/silver spread (`test_0002_gold_silver_pairs_trading.py`). It uses a hedge ratio
of exactly 1.0, so the spread is the logarithm of the gold price minus the logarithm of the silver
price, which is the logarithm of the number of ounces of silver one ounce of gold buys. The z-score
uses a 192-bar window on hourly bars. It buys the spread when the z-score is at or below -2.0, sells
when it is at or above +2.0, closes when the size of the z-score is 0.5 or less, and stops out when
it reaches 3.0. Each leg is five percent of the pot. Over the second half of 2025 it made 102 round
trips, of which 46 won and 56 lost, ending at 990,238 from 1,000,000, a small loss. The article's
diagnosis is precise: the true ratio between gold and silver has drifted from around 60 to around 120
over two decades, so a fixed ratio of one turns that drift into a slow loss.

The Kalman-filter version (`test_0001_gold_kalman_filter_pairs_trading.py`). If the ratio itself
drifts, stop fixing it. A Kalman filter is a running estimate of one unknown number that is updated a
little with every new observation: it weighs what it already believed against the surprise in the
newest prices and moves the estimate toward whichever deserves more trust. Here the unknown is the
ratio beta, roughly "ounces of silver per ounce of gold". It starts at 78 and adapts bar by bar. The
variant adds a gate: it trades only when the ratio has been stable, meaning the coefficient of
variation (the standard deviation divided by the average) of the last 96 readings is below 0.03. If
the relationship itself is wandering, the rule stands aside. The thresholds are an entry at a
z-score of 2.0, an exit at 0.35 and a stop at 3.25. Over the same half year this made 103 round
trips, 61 winners against 42 losers, ending at 997,507 from 1,000,000 - still a small loss, but a
lower one with a higher hit rate. The article's summary is that the dynamic ratio's value is not
earning more but being wrong less.

The copula version (`test_0007_copula_pairs_trading.py`). A z-score quietly assumes the gap's swings
are well behaved and symmetric. Gold and silver, however, are most tightly coupled at the extremes,
in panics. A copula is a way of describing how two things move together that keeps the extremes
intact. This file fits a Clayton copula (a formula tuned to lower-tail dependence) over a rolling
year of daily returns, then computes a conditional probability: given how gold moved today, how
unusual is silver's move? A probability near 0.05 means "gold barely moved while silver fell hard",
which flags silver as the cheap leg. Entries occur when that probability is below 0.05 or above 0.95,
and exits when it is within 0.10 of one half. Over 2018 to 2025 this made 292 trades, 136 winners,
ending at 986,607 from 1,000,000.

## The maths, with every symbol named

Two prices are cointegrated when each one wanders on its own but a particular blend of them stays
tethered to a stable average. Cointegration is not correlation. Correlation says two things move
together day to day; cointegration says the gap between them never drifts far away over the long run.
The blend, using logarithms and a ratio beta, is:

```text
S = log(P_gold) - beta * log(P_silver)
```

- `S` is the spread, a single number that is large when gold is expensive relative to silver.
- `P_gold` and `P_silver` are the two prices.
- `beta` is the hedge ratio, the number of units of the second leg that offset one unit of the first.
- `log` is the natural logarithm, which turns a ratio into a difference.

The fixed-ratio file sets `beta` to 1.0, so `S` is the logarithm of the price ratio. A rising `S`
means gold is gaining on silver. The usual way to estimate `beta` is a straight-line fit, called a
regression, of one log price on the other; the part the line does not explain is the spread `S`.

The z-score, which turns the spread into a comparable reading:

```text
z = (S - mean(S over the window)) / std(S over the window)
```

- `z` is how many standard deviations today's spread sits from its own recent average.
- `mean(...)` is the average of the spread over the look-back window, such as 192 bars.
- `std(...)` is the standard deviation of those values, a measure of how widely they have scattered.
- If the spread is centered on 4.40 with standard deviation 0.02, a reading of 4.43 gives z of 1.5.

The Kalman filter, written as scalar updates. There is no matrix algebra here; it is one equation per
line.

```text
P_before = P + Q
K = (P_before * P_silver) / (P_before * P_silver^2 + R)
beta_new = beta + K * (P_gold - beta * P_silver)
P_new = (1 - K * P_silver) * P_before
```

- `beta` is the current estimate of the hedge ratio before this update.
- `P` is the current estimated error of `beta`: large means "I am unsure", small means "confident".
- `Q` is the process noise, error added because the true ratio drifts each day; the file uses 0.0005.
- `R` is the observation noise, the error in each new price reading; the file uses 1.0.
- `P_before` is the error after allowing for the drift.
- `K` is the Kalman gain, between 0 and 1, saying how much of the newest surprise to accept.
- `(P_gold - beta * P_silver)` is the innovation, the surprise: what gold did minus what the current
  ratio expected, and `beta_new` moves the old estimate toward it in proportion to `K`.

When prices are noisy (`R` large) or the estimate is confident (`P` small), `K` is small and the
estimate barely moves. When the estimate is uncertain, `K` is larger and it chases the data.

The copula's dependence parameter and its conditional probability, both scalar formulas:

```text
theta = 2 * tau / (1 - tau)
probability(V <= v given U = u) = u^(-(theta+1)) * (u^(-theta) + v^(-theta) - 1)^(-(theta+1)/theta)
```

- `tau` is Kendall's tau, a rank correlation between the two return series, from -1 to +1.
- `theta` is the Clayton dependence parameter, larger when the two series are more tightly coupled in
  the lower tail.
- `u` and `v` are the two returns converted to percentile positions within their own past year.
- The second line gives the conditional probability that the second series sits at or below `v` given
  the first sits at `u`; a value near 0 flags the second series as the cheap leg.

## A worked example

First, building the spread and its z-score by hand, with a fixed hedge ratio of one. The last 192
bars gave a mean spread of 4.38203 and a standard deviation of 0.025. The ratio is the number of
ounces of silver one ounce of gold buys.

| Period | Ratio | Spread = log(ratio) | z-score | Rule reading                        |
| ------ | ----- | ------------------- | ------- | ----------------------------------- |
| 1      | 80.0  | 4.38203             | 0.000   | gap is exactly average              |
| 2      | 85.0  | 4.44265             | 2.425   | gap unusually wide: sell the spread |
| 3      | 83.0  | 4.41884             | 1.472   | hold                                |
| 4      | 81.5  | 4.40060             | 0.743   | hold                                |
| 5      | 80.5  | 4.38826             | 0.249   | inside 0.5: close both legs         |
| 6      | 82.0  | 4.40672             | 0.988   | flat; gap not wide enough to enter  |

"Sell the spread" means sell gold and buy silver, because the gap is wide and the rule expects it to
narrow. Now the money. The pot is 100,000, and each leg is five percent, so 5,000 per leg. At period
2 gold is 2,400 and silver is 2400/85, which is 28.2353. Selling gold means selling 5,000/2,400,
which is 2.0833 ounces; buying silver means buying 5,000/28.2353, which is 177.0833 ounces. At period
5 gold is 2,280 and silver is 2,280/80.5, which is 28.3230.

```text
Gold leg (sold):     (2400 - 2280) * 2.0833             = 250.00
Silver leg (bought): (28.3230 - 28.2353) * 177.0833     =  15.53
Gross profit = 250.00 + 15.53 = 265.53
Traded = 4 legs of 5,000 = 20,000; cost = 20,000 * 0.0005 = 10.00
Net = 265.53 - 10.00 = 255.53, or 0.26 percent of the pot
```

Notice how the two legs mostly cancel: gold fell 5 percent and silver was almost unchanged, so the
profit is close to gold's own fall times the size of the gold leg. That is what "hedged" means here,
and it is also why the win is modest - the size is deliberately small.

Second, two steps of the Kalman filter with the file's settings: process noise 0.0005, observation
noise 1.0, starting beta 78 and starting error 1.0.

| Step | Gold | Silver | Kalman gain | Innovation | New beta | New error | Spread  |
| ---- | ---- | ------ | ----------- | ---------- | -------- | --------- | ------- |
| 1    | 2400 | 30.0   | 0.03330     | +60.00     | 79.9978  | 0.001110  | +0.0666 |
| 2    | 2405 | 30.1   | 0.01971     | -2.93      | 79.9400  | 0.000655  | -1.1931 |

In step 1 the ratio implied by the prices is 2400/30, which is 80, well above the starting guess of
78, so the filter moves beta almost all the way to 80 in one update. Its error estimate collapses
from 1.0 to about 0.001, so it now trusts its own number and will move it only gently. In step 2 the
implied ratio is 2405/30.1, which is 79.90, slightly below the new estimate, and the filter nudges
beta down a little.

## What the research actually found

The founding study is Gatev, Goetzmann and Rouwenhorst, who in the late 1990s formed pairs by finding
the two stocks whose prices had historically moved most closely together, and traded the gap when it
opened. On American stocks from 1962 to 2002 they reported top-quintile pairs earning roughly eleven
percent a year before costs, with the return shrinking as the sample moved toward the present. Their
result was widely taken as evidence that the mechanism is real, and it launched a generation of
statistical-arbitrage desks.

The compendium's own files tell the other half of the story, and the article does not hide it: all
three representative gold/silver backtests lost money on the data they were run on - minus 0.98
percent, minus 0.25 percent and minus 1.34 percent from a starting 1,000,000 over their respective
windows. The article's own reading is that plain statistical arbitrage stopped printing money long
ago in increasingly efficient markets, and that the regression library records these files to give
"is pairs trading easy?" an honest, asserted answer.

One thing must be said plainly, because it applies to every file in this category and to every other
backtest in this compendium. Each backtest asserts its final value, its reward-to-risk ratio and its
worst fall against a stored baseline, and it must produce identical numbers in the engine's two
calculation modes. The reward-to-risk ratio is the average return divided by how much the pot swung,
and the worst fall is the largest drop from a peak to the following low. Passing those assertions
proves the engine computes exactly what the file says it computes, on the stored data. It does not
prove the strategy earns anything. The assertion is a test of the software, not a claim about the
market. A file can pass every assertion and still describe a plan that loses money - as three of the
files here do.

## How this project relates to it

The repository's own [predictability brief](../../../strategies/books2/08_predictability_and_trading_strategies.md)
has a section on exactly this idea. It reports a study of crude oil futures, `2309.00875v3`, that
treats pairs trading as cointegration plus a mean-reverting spread and lets the spread switch between
hidden states. On daily and weekly Brent, WTI and Shanghai crude futures from 2018 to 2023, with costs
estimated from the average daily half-spread (5.80 basis points for Brent, 20.24 for WTI and 53.71
for Shanghai), the out-of-sample test returned 15.18 percent annualised with a reward-to-risk ratio
of 1.18 and a worst fall of minus 5.73 percent. The brief's caution is as important as the number:
three contracts, one year of testing, and a cost that differs by an order of magnitude between
instruments.

Two finished tutorials in this collection cover the same ground from other datasets: the
[pairs trading with stocks](../../quantconnect/pairs-trading-with-stocks/README.md) tutorial and the
[intraday pairs](../../quantconnect/intraday-dynamic-pairs-trading-using-correlation-and-cointegration-approach/README.md)
tutorial. Both walk through a full gapped-pair trade, including the cost of opening and closing both
legs.

## Where it goes wrong

- The relationship can break for good. A hedge ratio stable for decades can step to a new level when
  the world changes - a new use for one metal, a shift in which country mines it, a change in tax or
  transport. The fixed-ratio file's slow loss on drifting gold and silver is a small version of this.
- Costs hit both legs twice. Every round trip trades four times, so a strategy that looks profitable
  before costs can lose after them. On the hourly gold/silver file the costs are a large share of the
  small per-trade profit.
- The gap can widen, not close, which is what the stop is for. A stop is a rule to close a losing
  position at a preset level; without one, a broken relationship can produce an unbounded loss.
- Cointegration is estimated on a window that may already be over. A pair that passed a test of
  tethering on the last two years can fail it in the next two, and choosing the pair and the window
  after seeing the data is a way of looking into the future.
- The two legs may not be equally easy to sell. If one leg is hard to trade when markets are stressed,
  the hedge does not work exactly when it is most needed, and the "market-neutral" promise leaks.

## Try it yourself

You need a spreadsheet and daily closing prices for gold and silver for the last two years. Most
finance websites publish both.

1. One row per day, with a column for gold and a column for silver.
2. Add a column for the ratio: gold divided by silver.
3. Add a column for the 60-day average of that ratio, and a column for its 60-day standard deviation.
4. Add a z-score column: (today's ratio minus the average) divided by the standard deviation.
5. Add a column that marks a "sell gold, buy silver" whenever the z-score is above 2, and a "buy
   gold, sell silver" whenever it is below minus 2.
6. Add a column that marks an exit whenever the z-score crosses back through zero.

What to notice: the ratio spends most of its time near its average and only rarely reaches plus or
minus two, so the rule trades a few times a year, not every day. Then look at the ratio's path over
the two decades before this - if it has been drifting in one direction for years, the average you
compute is a moving target, and a rule that assumes it stands still will slowly lose, which is the
finding that motivated the Kalman version.

## Where this came from

- [The compendium's pairs trading article](https://backtrader.readthedocs.io/en/latest/strategies-series/en/11-pairs-trading.html),
  the rules, the inventory and the file-level results quoted above.
- Gatev, Goetzmann and Rouwenhorst, [Pairs Trading: Performance of a Relative Value Arbitrage Rule](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=227415),
  the study that established the pattern and its decay.
- [predictability brief](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's own, whose section 6 covers regime-switching cointegration and its costs.

## Words used in this tutorial

- cointegration: two prices each wandering, but a blend of them staying tethered to a stable average.
- correlation: a number from minus one to one saying how much two things move together.
- hedge ratio: the number of units of one leg that offset one unit of the other.
- long: owning something, so you profit if its price rises.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- spread: the gap between two prices, measured as a single number.
- z-score: how many standard deviations a reading sits from its own recent average.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
