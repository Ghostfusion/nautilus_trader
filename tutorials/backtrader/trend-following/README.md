# Trend following: buying above a slow reference line

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                          |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | Gold priced in American dollars, and in some variants futures and share-index contracts; one position at a time                                                                |
| How often it trades       | Rarely. The classic version traded thirteen times in eighteen years; the busiest variants a few times a week                                                                   |
| What you need             | A spreadsheet and a column of daily closing prices                                                                                                                             |
| Where the rules come from | [backtrader strategy compendium, Trend Following](https://backtrader.readthedocs.io/en/latest/strategies-series/en/01-trend-following.html)                                    |
| The underlying research   | Moskowitz, Ooi and Pedersen, [Time Series Momentum](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2089463)                                                               |
| How well it held up       | Mixed: the broad family replicates across a century and dozens of markets after costs, but its returns have shrunk since 2010, and every variant here rests on one gold series |
| Also appears in           | [Momentum](../momentum/README.md) in this same group, which follows one asset's own past rather than a reference line drawn across it                                          |

## The idea in one paragraph

Draw a slow line through the recent prices of one thing, for example the average of the last two
hundred daily closes. When the price or a faster line moves above it, buy and hold. When the price
falls back below it, sell and wait. The line is not a forecast; it is a divider between the days when
the thing has been rising and the days when it has been falling. The bet is that a move which has
already carried the price above its own recent average tends to continue.

## Why anyone believed it

Prices move because people buy and sell, and people act on news at different speeds. A mine closes,
a central bank changes course, a currency's interest rate rises; the information arrives gradually and
is acted on over months. Whoever buys after the first move pushes the price further, and the slow line
registers the result. Some flow is mechanically persistent too: large traders split one big order into
many small ones so as not to move the price against themselves.

The counterparty is the investor who is slow to update, or who is forced to sell for reasons
unrelated to the outlook: a fund facing withdrawals, a manager trimming a position that has grown too
large, a saver taking a profit because the price has risen. A trend rule does not need those sellers
to be wrong; it needs them to keep appearing until the buyers outnumber them.

## An everyday comparison

Think of a heavy freight train leaving a station. Once it is rolling it takes a long time to stop, and
the people who hop aboard while it is still slow get the ride for free. The slow line is the signal
that the train is moving: it changes direction only long after the engine did, which is why you are
never first on board and never first off. That delay is the cost and the protection at the same time,
and a train that only shunts back and forth in the yard gives many small false starts.

## The rules, step by step

1. Choose one thing to trade, such as gold, and a price series for it, such as daily closes.
2. Compute a slow reference line, the arithmetic average of the last two hundred closing prices. The
   tests also try a faster line, the average of the last fifty.
3. Compare the faster line, or the price itself, with the slow line, always using values computed on
   the previous completed day, never including the day you are deciding on.
4. Buy when the fast line crosses from below the slow line to above it, and sell when it crosses back.
5. Decide the size in advance; the simplest version makes the position's value about equal to the
   account, and the Turtle version risks one percent of the account per unit, adds a unit each time
   the trade earns one average day's move, and exits on a shorter channel.
6. Check once at the close of each day and act on the next bar. Every added stop, target or filter is
   a different strategy and another rule you tried.

### What is in this category

The category holds 340 backtests; these are nine a reader would meet, and many files blend two
indicators.

| Strategy                        | What it does                                                   | Source file                                      |
| ------------------------------- | -------------------------------------------------------------- | ------------------------------------------------ |
| Golden cross                    | The 50-day average crosses the 200-day; 13 trades in 18 years  | `test_0175_golden_cross.py`                      |
| Price above the 200-day average | Holds while the close is above the average; 65 trades          | `test_0001_sma_trend_following.py`               |
| Original Turtle rules           | 20-day and 55-day channel entries, ATR-sized units, pyramiding | `test_0074_0776_original_turtle_rules_trader.py` |
| Donchian colour system          | A dual-timeframe channel state machine                         | `test_0078_0855_donchian_channels_system.py`     |
| MACD sample                     | Two exponential averages with a zero-axis filter               | `test_0116_1107_macd_sample.py`                  |
| ADX plus moving average         | An ADX threshold gates every crossover                         | `test_0064_0687_adx_ma.py`                       |
| SuperTrend                      | An average-range band flips from support to resistance         | `test_0139_1232_supertrend.py`                   |
| Woodies CCI                     | A fast and slow commodity-channel-index cloud                  | `test_0082_0887_cci_woodies.py`                  |
| Gold hidden-state trend         | A model labels the regime and gates its own confidence         | `test_0002_gold_hmm_trend_following.py`          |

### Deep dive: the golden cross

`test_0175_golden_cross.py` is the classic crossing of two averages, with a fast 50-day average, a
slow 200-day average and a position target of the whole account.

1. Compute the 50-day and 200-day averages of the daily close.
2. Call it a golden cross when the fast average was at or below the slow average on the previous bar
   and is strictly above it on this bar; call it a death cross in the opposite case.
3. If you hold nothing and a golden cross has just happened, buy, sized so the position's value is
   roughly the account (the account value divided by the price times the multiplier of 100). If you
   hold something and a death cross has just happened, sell everything. Do not short.

### Deep dive: the original Turtle rules

`test_0074_0776_original_turtle_rules_trader.py` ports the rules Richard Dennis gave his students in
the 1980s. The entry signal is only a small part; the system is mostly position engineering.

1. Define an entry channel of the last 20 days and a wider backup channel of the last 55 days.
2. Buy when price breaks above the 20-day channel; if that first breakout fails and a later breakout
   of the 55-day channel occurs, take that one instead.
3. Size each unit so the distance from the entry to the stop costs one percent of the account; the
   stop is one average true range away, so the size is the risk budget divided by that range.
4. Add a unit every time the trade moves one average true range in your favour, up to a handful, and
   exit when price breaks the 10-day low.

### Deep dive: the hidden-state regime gate

`test_0002_gold_hmm_trend_following.py` replaces the drawn line with a probability. It fits a
three-state statistical model, a hidden Markov model, to gold's daily returns.

1. Each day describe the market with two numbers: the logarithmic return and the last 20 days'
   annualised volatility, the day-to-day spread of returns scaled to a year.
2. Every 21 days refit a three-state model on the last 252 days and label the states as bull, bear
   and neutral by their average return in that training window.
3. Ask the model for the probability that today belongs to the bull state, the confidence.
4. Hold the smaller of 0.10 and 0.03 times the confidence times a volatility factor, and hold
   nothing below 0.70 confidence, when the state has not repeated for three days, or when stickiness
   is weak. Once a position is 8 percent in profit, move the stop up to break-even.

## The maths, with every symbol named

Three calculations carry the category: the slow line and its crossing, the average true range and
channel, and the hidden-state probability.

```text
SMA_N(t) = (P_t + P_(t-1) + ... + P_(t-N+1)) / N
golden at t  when  fast(t-1) <= slow(t-1)  and  fast(t) > slow(t)
death  at t  when  fast(t-1) >= slow(t-1)  and  fast(t) < slow(t)
```

- `SMA_N(t)` is the average of the last `N` prices as of day `t`, `P_t` the close on day `t`, and `N`
  the lookback, 200 in the golden cross; `fast` and `slow` are the two averages.
- The average lags the price, and using `t-1` on the left of each test stops a day's own close from
  generating its own signal.

```text
TR_t = max(H_t - L_t, |H_t - C_(t-1)|, |L_t - C_(t-1)|)
ATR_n(t) = (TR_t + TR_(t-1) + ... + TR_(t-n+1)) / n
upper(t) = max(H_(t-1), ..., H_(t-N))    lower(t) = min(L_(t-1), ..., L_(t-N))
unit = (equity * risk) / (ATR * multiplier)
```

- `H_t`, `L_t` and `C_t` are the day's high, low and close; `TR_t` is how far the price travelled,
  counting opening gaps, and `ATR_n(t)` is the average travel over `n` days, 20 in the Turtle file.
- `upper` and `lower` are the highest high and lowest low of the last `N` completed days, the
  Donchian channel; the entry uses `N = 20` and the exit `N = 10`.
- `equity` is the account value, `risk` the fraction lost if the stop is hit (0.01), and `multiplier`
  converts a contract's price move into money, so a wider daily range buys a smaller position.

```text
f(x) = 1 / (sigma * sqrt(2 * pi)) * exp( -(x - mu)^2 / (2 * sigma^2) )
posterior = f_bull(x) / ( f_bull(x) + f_bear(x) )
```

- `x` is the day's return; `mu` is a state's average return and `sigma` its spread, both fitted from
  the data; `f(x)` is the bell-shaped likelihood of seeing `x` in that state, and `posterior` is the
  share of the likelihood belonging to the bullish state, between 0 and 1.

## A worked example

The examples use invented but plausible numbers and shortened lookbacks so the arithmetic fits on the
page. Each charges the gap between the buying and the selling price, here a half-unit each side,
because a buyer pays the higher asking price and a seller receives the lower offered price.

### The golden cross

Ten daily closes, with a 2-day and a 4-day average in place of 50 and 200. The account starts at
1,000.00 and the whole of it is invested when the rule is long.

| Day | Close  | Fast (2-day) | Slow (4-day) | Fast against slow | Action           |
| --- | ------ | ------------ | ------------ | ----------------- | ---------------- |
| 1   | 100.00 | -            | -            | -                 | none, warming up |
| 2   | 99.00  | 99.50        | -            | -                 | none, warming up |
| 3   | 98.00  | 98.50        | -            | -                 | none, warming up |
| 4   | 99.00  | 98.50        | 99.00        | below             | none             |
| 5   | 100.00 | 99.50        | 99.00        | above, was below  | buy at 100.05    |
| 6   | 102.00 | 101.00       | 99.75        | above             | hold             |
| 7   | 104.00 | 103.00       | 101.25       | above             | hold             |
| 8   | 103.00 | 103.50       | 102.25       | above             | hold             |
| 9   | 101.00 | 102.00       | 102.50       | below, was above  | sell at 100.95   |
| 10  | 99.00  | 100.00       | 101.75       | below             | flat             |

On day 4 the fast average, 98.50, sits below the slow average, 99.00, and on day 5 the fast average,
99.50, is above. That is a golden cross, so the rule buys at the asking price of 100.05. On day 9 it
sells at the offered price of 100.95, a move of +0.90 percent, and the account ends at 1,000.00 times
1.0090, or 1,009.00. Only one round trip occurred; the real 50/200 system made thirteen in eighteen
years.

### The Turtle unit and channel

Nine bars, with a 3-day entry channel and a 3-day average true range in place of 20. The account is
100,000 and the entry is the first day whose close exceeds the highest high of the three days before.

| Day | High   | Low    | Close  | True range | 3-day ATR | Previous 3-day high | Action                |
| --- | ------ | ------ | ------ | ---------- | --------- | ------------------- | --------------------- |
| 1   | 101.00 | 99.00  | 100.00 | -          | -         | -                   | none                  |
| 2   | 102.00 | 100.00 | 101.00 | 2.00       | -         | -                   | none                  |
| 3   | 103.00 | 101.00 | 102.00 | 2.00       | -         | -                   | none                  |
| 4   | 105.00 | 103.00 | 104.00 | 3.00       | 2.33      | 103.00              | break above, buy      |
| 5   | 108.00 | 105.00 | 107.00 | 4.00       | 3.00      | 105.00              | hold                  |
| 6   | 111.00 | 108.00 | 110.00 | 4.00       | 3.67      | 108.00              | hold                  |
| 7   | 114.00 | 111.00 | 113.00 | 4.00       | 4.00      | 111.00              | hold                  |
| 8   | 112.00 | 109.00 | 110.00 | 4.00       | 4.00      | 114.00              | hold                  |
| 9   | 110.00 | 107.00 | 107.00 | 3.00       | 3.67      | 114.00              | break below 108, sell |

On day 4 the close of 104.00 is above the previous three days' highest high of 103.00, so the rule
buys. The 3-day average true range is the average of 2.00, 2.00 and 3.00, which is 2.33. The unit size
is one percent of 100,000, or 1,000, divided by 2.33, which is 428 units; as a check, 428 units times
the 2.33 stop distance is 998.67, almost exactly the risk budget, and the stop sits at 101.67. The
buying price is 104.05 and on day 9 the close of 107.00 is below the previous three days' lowest low
of 108.00, so the rule sells at 106.95. The profit is 428 times (106.95 - 104.05), or 1,241.20, and
the account ends at 101,241.20, up 1.24 percent.

### The hidden-state gate

A two-state version, with a state whose average day is +0.15 percent and spread 0.40 percent, and one
whose average day is -0.15 percent and spread 1.00 percent. The likelihoods come from the Gaussian
formula above with an equal prior each day; the file fits three states, collapsed to two here.

| Day | Return | Bull likelihood | Bear likelihood | Posterior of bull | Exposure |
| --- | ------ | --------------- | --------------- | ----------------- | -------- |
| 1   | +0.15% | 99.74           | 38.14           | 0.72              | 2.17%    |
| 2   | +0.60% | 52.97           | 30.11           | 0.64              | 0.00%    |
| 3   | -0.30% | 52.97           | 39.45           | 0.57              | 0.00%    |
| 4   | +0.15% | 99.74           | 38.14           | 0.72              | 2.17%    |
| 5   | +0.05% | 96.67           | 39.10           | 0.71              | 2.14%    |

On day 1 the posterior is 99.74 / (99.74 + 38.14) = 0.72, which clears the 0.70 gate, so the exposure
is 0.03 times 0.72 times a volatility factor of 1, or 2.17 percent of the account. Day 2 is the
instructive row: +0.60 percent is unusual for the calm bull state and ordinary for the wild bear
state, so the posterior falls to 0.64 and the rule holds nothing. Over the five days the account gains
0.0076 percent, ending at 1,000.08; the real file was similarly flat, 1,001,059.99 after six trades on
245 days. The model's interesting property is that it can be unsure.

## What the research actually found

Before the results, the caveat that applies to every page in this group. Each backtest in the
compendium asserts three numbers against a baseline: the final portfolio value, the reward-to-risk
ratio, and the worst fall from a peak. The golden cross file pins its final value at 3,571,828.03
within a few dollars, its worst fall at 37.54 percent and its reward-to-risk ratio at 0.63. Passing
those assertions proves that the engine computes what the file says it computes, in both engine
modes. It does not prove that the strategy earns anything, that the number would survive a different
market, or that a real account could have captured it; the assertions are a software test.

The published record for trend following is unusually long. Moskowitz, Ooi and Pedersen found that,
across fifty-eight futures markets, the sign of an asset's own past twelve-month return predicted its
next month's return, and volatility scaling improved the trade-off; the golden cross approximates that.

| Source                                           | What it measured                                  | Result                                                                                                        |
| ------------------------------------------------ | ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| Moskowitz, Ooi and Pedersen (2012)               | Fifty-eight futures markets, up to 2009           | The sign of an asset's own twelve-month return predicted its next month's return, before and after costs      |
| `test_0175_golden_cross.py`                      | Gold daily, 2008 to 2025, 0.02 percent commission | 13 trades, 4 wins and 8 losses, a 30.77 percent win rate, 3,571,828.03 on 1,000,000, worst fall 37.54 percent |
| `test_0001_sma_trend_following.py`               | Gold daily, same window                           | 65 trades, 11 wins and 53 losses, a 16.92 percent win rate, 3,686,124.79, worst fall 32.83 percent            |
| `test_0074_0776_original_turtle_rules_trader.py` | Gold fifteen-minute bars, three months            | 345 trades, 173 wins and 172 losses, a 50.14 percent win rate, 1,190,431.17, worst fall 8.08 percent          |
| `test_0002_gold_hmm_trend_following.py`          | Gold daily, 2024 to 2025                          | 6 trades, final value 1,001,059.99, roughly flat after commissions                                            |

The golden cross made 257 percent while winning under a third of its trades, the signature of the
family: many small losses and a few large gains. The article records a MACD-and-KDJ combination that
turned 100,000 into 5,870.49, a 98.63 percent loss, purely because it bet everything on every signal.
What broke was the position size, not the indicator.

## How this project relates to it

[Stylized facts and scaling](../../../strategies/books2/13_stylized_facts_and_scaling.md) collects the
order-flow evidence behind the trend premise: about a quarter of traders split their orders but issue
about 80 percent of market orders, and the length of those hidden orders follows a power law with an
exponent near 1.62 for a liquid Japanese name. The same brief supplies the counterweight: an impact
exponent below 0.5 is what keeps prices diffusive despite all that predictable flow, so visible trends
in order flow need not become trends in price.

[Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md)
records that sorting shares on functions of 240 accounting variables yields 18,113 strategies, of
which 8.40 percent clear a t-statistic of 4.0 against a chance rate of 0.0063 percent. A category with
340 moving-average and channel variants is the same arithmetic on a smaller scale.

## Where it goes wrong

- Whipsaw. In a market that drifts sideways, the reference line is crossed over and over and each
  crossing pays the gap between the buying and the selling price. The price-above-average version
  traded 65 times for a result barely different from trading 13 times.
- The signal is late. A 200-day average needs the trend well under way before it turns, so the rule
  always gives up the first part of a move and much of the last. That is the price of avoiding false
  starts, not a defect to be tuned away.
- Parameter sweeps. The choice of a 20-day versus a 55-day channel, or a 50-day versus a 100-day
  average, changes the trade distribution. Trying all of them and reporting the best is how a chance
  result becomes a strategy-shaped object.
- One market, one window. Almost all the daily files run on gold from 2008 to 2025, when gold rose for
  much of the time, so a rule that is long above its average is partly a rule that was long in a bull
  market. A hidden-state model that never sees a new regime in training can also be confidently wrong.

## Try it yourself

1. Fill one column with daily closing prices for one thing, over at least ten years.
2. Add a second column, the 200-day average: the average of the current row and the 199 above it.
3. Add a third column that is 1 when the close is above the average and 0 when it is below.
4. Add a fourth column that is 1 only where the third changed from 0 to 1; those are the buys. For
   each buy, find the next row where the third returns to 0, that close is the selling price, and
   subtract 0.10 percent for the gap on both sides. Sort the trades into winners and losers.

What to notice: the losers are many and small, clustered in the sideways stretches, and the winners
are few and large. Then change the average to 100 days and repeat; if two neighbouring settings give
opposite verdicts, the verdict was never about the market.

## Where this came from

- [backtrader strategy compendium, Trend Following](https://backtrader.readthedocs.io/en/latest/strategies-series/en/01-trend-following.html),
  the category inventory and the deep dives.
- `test_0175_golden_cross.py`, `test_0001_sma_trend_following.py`,
  `test_0074_0776_original_turtle_rules_trader.py` and `test_0002_gold_hmm_trend_following.py` in
  [tests/functional/strategies/trend_following](https://github.com/cloudQuant/backtrader/tree/development/tests/functional/strategies/trend_following),
  the files from which the rules and the asserted numbers above are taken.
- Moskowitz, Ooi and Pedersen, [Time Series Momentum](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2089463),
  and this repository's own [stylized facts](../../../strategies/books2/13_stylized_facts_and_scaling.md)
  and [overfitting](../../../strategies/books2/28_overfitting_and_research_integrity.md) briefs.

## Words used in this tutorial

- average true range: the average size of a day's move including gaps, used to place stops.
- drawdown: the fall from a peak to the following low, measured in percent.
- hidden Markov model: a statistical model that assumes the market is in one of a few unseen states
  and infers the state from the data.
- position: the holding an account owns at a moment in time.
- reward-to-risk ratio: the return earned per unit of the price's up-and-down movement, called the
  Sharpe ratio when it is computed in this way.
- whipsaw: a stretch of small losses caused by a signal that keeps reversing.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
