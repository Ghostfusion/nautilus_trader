# Gaps, calendars and Kelly: where size itself is the strategy

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                             |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Gold, quoted in American dollars per ounce, on a daily timetable; the sizing rule trades a gold fund, and the allocation rule trades a small group of assets                                                                                                                                                      |
| How often it trades       | From six times in eighteen years for the gap rule to every day for the sizing rule, which adjusts the amount held constantly                                                                                                                                                                                      |
| What you need             | A spreadsheet and a table of daily open, high, low and close prices, plus a column of arithmetic                                                                                                                                                                                                                  |
| Where the rules come from | [Strategy compendium, category 05, others](https://backtrader.readthedocs.io/en/latest/strategies-series/en/05-others.html)                                                                                                                                                                                       |
| The underlying research   | J. L. Kelly, [A New Interpretation of Information Rate](https://www.princeton.edu/~wbialek/rome/refs/kelly_56.pdf) (1956); Harold Hurst's work on river levels, and Harry Markowitz, [Portfolio Selection](https://www.jstor.org/stable/2975974) (1952)                                                           |
| How well it held up       | Mixed: the sizing mathematics and the mean-variance framework are settled results, but they are not sources of return, and the gap, overnight and calendar signals rest on single samples, several of which the compendium's own runs contradict                                                                  |
| Also appears in           | [Position sizing](../../project/position-sizing/README.md) in this collection for the Kelly criterion, [overnight anomaly](../../quantconnect/overnight-anomaly/README.md) for the overnight split, and [january effect in stocks](../../quantconnect/january-effect-in-stocks/README.md) for the calendar strand |

## The idea in one paragraph

This category collects the strategies that do not fit a theme: the small gap between yesterday's
close and today's open, the overnight hours as opposed to the daytime hours, the calendar, the
mathematics of how much to bet, the memory of a price series, and the search for the best mix of
holdings. They are joined only by their misfit status. Some trade a signal, such as a gap that
should close or a day of the month that should be strong. Others do not trade a signal at all:
Kelly sizing decides how much of the account to put at risk, and mean-variance allocation decides
which holdings to combine. The shared claim is that a fact about the market's structure, rather
than about a company, can be turned into a rule.

## Why anyone believed it

The gap rule rests on a piece of folk wisdom, that gaps get filled, and on the observation that a
gap after a long decline is often a burst of emotion rather than new information. The overnight
rule rests on the timing of news: most announcements arrive while the market is shut, so the price
jump that reflects them happens between the close and the next open. Kelly sizing rests on
something stronger, a mathematical proof: if you know the edge and the odds exactly, there is one
bet size that grows your money fastest in the long run, and it is not the largest one. Mean
variance rests on a different proof: for a given level of risk, one mix of holdings gives the
highest expected return, and it can be computed. In each case the counterparty is whoever is
forced by rules or fear to trade at the wrong moment.

## An everyday comparison

Think of a weekly card game where you are sure the deck is slightly in your favour. Betting a
fixed small amount every hand leaves money on the table. Betting everything wins the most when you
win and ends the game when you lose once. Somewhere in between is a fraction that grows your pile
fastest over many hands, and it depends on how big the edge is. That fraction is the Kelly
fraction. The rest of this category is the same idea applied to gaps, to the clock and to mixes of
holdings: each one is an attempt to find a small, repeatable edge and then decide how much of it
to take.

## The rules, step by step

This category holds sixty-nine strategies. The ones a reader would meet are listed below, with one
clause on each.

| Strategy family            | What it does                                                                               |
| -------------------------- | ------------------------------------------------------------------------------------------ |
| Gap N Go Fade              | After a fifty-day low, fades a strong gap up with a short, held two days                   |
| Gap Down                   | The mirror image: buys a gap down beyond one percent and holds five days                   |
| Overnight versus intraday  | Holds only when the average overnight return of the last twenty days is positive           |
| Monday drop bounce         | Buys after a fall of more than two percent on a Monday that follows three down days        |
| Day-of-month timing        | Votes with a long moving average and month-specific multipliers for size                   |
| January effect             | Holds the previous year's worst performers through January                                 |
| Kelly and optimal fraction | Recomputes the bet size each day from a rolling window of returns, then halves and caps it |
| Hurst exponent             | Picks a trend rule or a reversal rule according to the memory of the price series          |
| Markowitz allocation       | Rebalances a small mix using a rolling risk-adjusted return as the gate                    |
| Turbulence index           | Measures how unusual the current market is and maps it to one of three fixed mixes         |
| Omega ratio                | Ranks a strategy by the ratio of its gains above a threshold to its losses below it        |

The shared mechanism across the trading rules is a condition on daily bars plus a fixed hold, and
across the sizing and allocation rules it is a number recomputed from a rolling window. State the
window, the threshold and the hold, and the rule is complete.

1. The gap rule. Work out the gap as today's open minus yesterday's close. Call it significant
   when it is larger than three tenths of a percent of yesterday's close, or larger than half the
   fourteen-day average true range, whichever test is easier to pass. The setup also needs
   yesterday to have made a new fifty-day low, today's close to be above yesterday's close, and
   today's close to be above today's open.
2. When the setup fires, sell short at the next day's open and hold for exactly two days. There is
   no stop and no target; the exit is the calendar.
3. The overnight rule. Compute the average overnight return of the last twenty days, which is
   today's open divided by yesterday's close, minus one. If that average is positive, hold the
   asset overnight; otherwise hold cash. The compendium's version runs this on a futures contract
   with heavy borrowing, which is stated in the results below.
4. The Kelly rule. From a rolling window of one hundred and twenty-six daily returns, compute the
   mean return and the variance. The raw fraction is the mean divided by the variance. Multiply it
   by one half, then cap it at one fifth, then set it to zero whenever the sixty-three-day trend
   is not positive.
5. Rebalance to that fraction of the account each day, using a target size rather than a fixed
   number of shares.
6. The Hurst rule. Estimate the Hurst exponent on a rolling window of one hundred and fifty days.
   Above 0.55, treat the market as trending: hold long if the price is above its fifty-day
   average, otherwise short. Below 0.45, treat it as reverting: buy when a fourteen-day momentum
   reading is below 30 and sell when it is above 70. In between, do nothing.
7. The allocation rule. Rebalance a small set of holdings quarterly, using a rolling risk-adjusted
   return as the gate for how much to hold.
8. Review the sizing and allocation rules on their stated schedule, and the trading rules once a
   day at the close.

## The maths, with every symbol named

The Kelly fraction is the heart of the category.

```text
f* = mean / variance
```

- `f*` is the fraction of the account to bet, written as a number of times the account, so 0.2
  means one fifth and 15.6 means fifteen times the account or more.
- `mean` is the average return over the rolling window, as a decimal: 0.01 means one percent.
- `variance` is the average of the squared distance of each return from that mean; it is a measure
  of how spread out the returns are, and its square root is the usual volatility.
- The formula says bet more when the average return is large and less when the returns are wild.
  The implementation then applies brakes: `f = min(0.5 * f*, 0.2)`, and `f = 0` when the trend is
  not positive.

The Hurst exponent measures whether a series wanders or persists.

```text
tau(lag) = standard deviation of ( log(price_t) - log(price_(t-lag)) )
H        = slope of log(tau) against log(lag), for lags 2 to 20
```

- `price_t` is the price today and `price_(t-lag)` is the price `lag` days earlier.
- `tau(lag)` is how far the price typically moves over a gap of `lag` days, measured in log terms.
- The slope `H` is the Hurst exponent. Near 0.5 the series behaves like a random walk, near 1 it
  trends, and near 0 it alternates up and down.

The mean-variance framework gives the mix of holdings.

```text
return_portfolio = w1 * r1 + w2 * r2 + ... + wn * rn
variance_portfolio = sum over all pairs (i, j) of wi * wj * covariance(i, j)
Sharpe = (return_portfolio - cash_rate) / sqrt(variance_portfolio)
```

- `wi` is the fraction of the account placed in holding `i`, and the weights add to one.
- `ri` is the average return of holding `i`.
- `covariance(i, j)` measures how much holdings `i` and `j` move together; when it is negative they
  offset each other.
- `cash_rate` is the interest rate on holding cash, and `Sharpe` is the reward per unit of risk.

## A worked example

Ten invented monthly returns, in percent, for a fund. The Kelly fraction is computed from them.

| Month | Return | Distance from the mean of 1.0 | Distance squared |
| ----- | ------ | ----------------------------- | ---------------- |
| 1     | +2.0   | +1.0                          | 1.00             |
| 2     | -1.0   | -2.0                          | 4.00             |
| 3     | +3.0   | +2.0                          | 4.00             |
| 4     | -2.0   | -3.0                          | 9.00             |
| 5     | +1.0   | 0.0                           | 0.00             |
| 6     | +2.0   | +1.0                          | 1.00             |
| 7     | -1.0   | -2.0                          | 4.00             |
| 8     | +4.0   | +3.0                          | 9.00             |
| 9     | -3.0   | -4.0                          | 16.00            |
| 10    | +5.0   | +4.0                          | 16.00            |

The returns add to 10.0 percent, so the mean is 1.0 percent a month, or 0.01 as a decimal. The
squared distances add to 64.00, and the average of those is the variance in percent-squared terms,
6.4, which is 0.00064 as a decimal.

```text
f*        = 0.01 / 0.00064 = 15.63
half      = 0.5 * 15.63    = 7.81
after cap = min(7.81, 0.20) = 0.20
```

So the raw Kelly fraction says to bet fifteen times the account, or borrow fifteen dollars for
every dollar you own. This is the most important number in the category, and it is why full Kelly
is far too aggressive for a real account. The formula assumes the mean and the variance are known
exactly. From ten months they are not: change one month and the estimate moves a lot. Betting
fifteen times an account on an estimate that could be wrong by a factor of two is a way to lose
everything. The implementation therefore halves the figure to be safer and then caps it at one
fifth of the account, which is the version worth remembering.

Now the gap rule, on invented daily prices for gold. Yesterday closed at 2050.00 and made a new
fifty-day low. Today opens at 2062.00, a gap of 12.00, which is 0.585 percent of the previous
close, above the 0.3 percent test. Today closes at 2068.00, above both the previous close and
today's open, so the setup fires. The short is opened at the next day's open, 2065.00, and held two
days, exiting at 2040.00.

```text
Gross gain for a short = (2065.00 - 2040.00) / 2065.00 = 0.012106, that is 1.211 percent
Round-trip cost = 2 * 0.0005 = 0.001, that is 0.10 percent
Net gain   = 1.211 - 0.10 = 1.11 percent
```

Both examples are arithmetic only. The first shows that the Kelly fraction is enormous before the
brakes; the second shows how a gap trade is measured. Neither says whether the rules earn
anything.

## What the research actually found

| Source                                      | What it measured                                               | Result                                                                                                                                                                                         |
| ------------------------------------------- | -------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Kelly (1956), and Thorp's later application | The bet size that maximises long-run growth given a known edge | A mathematical result, not a market measurement. Full Kelly is the fastest-growing fraction when the edge is known exactly; half Kelly gives three quarters of the growth with half the swings |
| The compendium, gap rule                    | Eighteen years of daily gold, 4,588 bars                       | The setup fired six times, three wins and three losses, final value 1,030,141.98, a gain of 3.01 percent, a profit factor of 1.56 and a worst fall of 3.27 percent                             |
| The compendium, Hurst rule                  | Eighteen years of a gold fund, 4,370 bars                      | One hundred eight trades, fifty-nine wins and forty-nine losses, final value 669,247.06 from one million, a loss of 33.1 percent                                                               |
| The compendium, overnight rule              | Daily gold                                                     | A large gain reported, but with ten times borrowing and a worst fall of 30.27 percent; the return comes from leverage, not from the signal                                                     |
| The compendium, allocation rule             | Daily gold, rebalanced quarterly                               | Nine buys and eight sells in eighteen years, final value 5,203,300 from one million, again with heavy borrowing                                                                                |
| The compendium, day-of-month rule           | Daily gold plus a cash fund                                    | Four wins and sixteen losses across the trades, with the return living in the path rather than the trades                                                                                      |

Read together, the picture is this. The gap rule is a frozen reference with six trades in eighteen
years, which is far too few to distinguish from luck. The Hurst rule lost a third of the account,
and the diagnosis is more useful than the loss: gold spent those years trending, and the rule kept
choosing the reversal leg. The rules that earned the largest numbers borrowed heavily, and their
gains are a reward for borrowing rather than for the signal. The Kelly and mean-variance
mathematics are settled, but they are not sources of return; they tell you how much to risk and
how to combine holdings once a signal exists.

This is the honest heart of this group, and it applies to every file here. Every backtest in the
compendium asserts its final value, its reward-to-risk ratio and its worst fall against a
baseline. Passing that assertion proves the engine computes exactly what the file says. It does
not prove the strategy earns anything. A rule that fired six times in eighteen years can pass its
assertion and still be a coin flip dressed as a strategy.

## How this project relates to it

The Kelly criterion has its own treatment in this collection,
[position sizing](../../project/position-sizing/README.md), which states the engine's version of
the same formula and its warning: a win rate estimated from a small sample of past trades is
fitting noise, so fractional Kelly at a low fraction is the only defensible use. That page and
this one agree, and neither promises that sizing creates a return.

The overnight strand and the calendar strand are covered by
[overnight anomaly](../../quantconnect/overnight-anomaly/README.md), which finds the split between
the overnight and daytime halves of a day across many markets and concludes it is not tradable
after the cost of trading twice a day, and
[january effect in stocks](../../quantconnect/january-effect-in-stocks/README.md), which reports
what remains of the January pattern once costs and the choice of sample are counted.

The framework behind the allocation rule is
[portfolio and allocation](../../../strategies/books2/10_portfolio_and_allocation.md). Its first
finding is that a mean-variance weight vector is only as aligned with expected returns as the
covariance matrix is well conditioned; a badly estimated matrix rotates the bets away from the
forecast. The brief also records that unfiltered mean-variance is fragile, which is why the
compendium's own version collapses it to a rolling risk-adjusted return gate. The memory of a
price series, behind the Hurst rule, is treated in
[stylized facts and scaling](../../../strategies/books2/13_stylized_facts_and_scaling.md), which
notes that long-memory estimates are window-dependent and regime-dependent, so a single Hurst
value should not be read as evidence of an exploitable pattern.

## Where it goes wrong

- Sizing cannot rescue a signal. Kelly tells you how much of a good edge to take; if the edge is
  zero, every fraction loses at the rate of the costs.
- The Kelly inputs are estimated, not known. Full Kelly assumes the mean and variance are exact;
  from a rolling window they are guesses, and errors in the mean move the bet size a great deal,
  which is why the whole industry halves and caps it.
- Six trades prove nothing. The gap rule's entire record is six decisions over eighteen years, and
  a sample that small cannot separate the rule from chance.
- Borrowing is not a signal. Two of the largest results here come with ten times borrowing and
  worst falls above thirty percent; the gain is the reward for leverage, and so is the loss.
- Long-memory estimates are unstable. The Hurst exponent depends on the window and on the market's
  regime, and the compendium's own result is a loss on a market that trended while the rule chose
  the reversal leg.
- The calendar samples overlap. Day-of-month, January and Monday rules all use the same daily
  prices, so their apparent successes are not independent confirmations of each other.
- Small samples and many rules. With sixty-nine files in this folder and each one fitted on one
  instrument, some rule will look good by chance alone, and the assertion will still pass because
  the assertion measures arithmetic, not truth.

## Try it yourself

You need a spreadsheet and a public source of daily prices for one index or commodity.

1. Make columns: date, open, high, low, close, and a helper column holding yesterday's close.
2. Add a `gap` column: today's open minus yesterday's close, and a `gap percent` column dividing
   it by yesterday's close.
3. Add a `fifty-day low` column: the rolling minimum of the close over the last fifty rows.
4. Flag a row when the previous row set a new fifty-day low, today's gap percent is above 0.3, and
   today's close is above both yesterday's close and today's open.
5. In the next rows, add the return over the following one, two and three days.
6. Average those returns over every flagged row, and over every other row, and count the flagged
   rows.
7. Separately, build a `returns` column, and compute its mean, its variance, and the mean divided
   by the variance over a rolling sixty rows.

What to notice: the flagged-row count will be tiny on a single instrument, which is the honest
problem with the whole gap idea. The rolling mean-over-variance figure will swing wildly from one
window to the next, and it will often be far above one, which shows in plain arithmetic why the
cap is needed.

## Where this came from

- [Strategy compendium, category 05, others](https://backtrader.readthedocs.io/en/latest/strategies-series/en/05-others.html),
  the rules and the backtest numbers quoted above, including the six-trade gap rule, the losing
  Hurst rule and the borrowed results.
- J. L. Kelly, [A New Interpretation of Information Rate](https://www.princeton.edu/~wbialek/rome/refs/kelly_56.pdf)
  (1956), the origin of the growth-optimal bet size.
- Harry Markowitz, [Portfolio Selection](https://www.jstor.org/stable/2975974) (1952), the origin
  of the mean-variance framework.
- [Position sizing](../../project/position-sizing/README.md), this collection's treatment of the
  Kelly criterion and its warning about fitted win rates.
- [Overnight anomaly](../../quantconnect/overnight-anomaly/README.md) and
  [january effect in stocks](../../quantconnect/january-effect-in-stocks/README.md), this
  collection's treatments of the overnight and calendar strands.
- [Portfolio and allocation](../../../strategies/books2/10_portfolio_and_allocation.md) and
  [stylized facts and scaling](../../../strategies/books2/13_stylized_facts_and_scaling.md), the
  briefs behind the allocation and memory rules.

## Words used in this tutorial

- edge: an advantage that makes a bet favourable on average, such as a slightly better than even
  chance of winning.
- gap: the difference between yesterday's closing price and today's opening price.
- Hurst exponent: a number between zero and one describing whether a price series trends or
  alternates.
- leverage: using borrowed money so a given price move produces a larger gain or loss.
- mean-variance: choosing a mix of holdings from their average returns and their co-movement.
- overnight return: the move from one day's close to the next day's open.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- variance: how spread out a set of returns is, measured as the average squared distance from the
  mean.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
