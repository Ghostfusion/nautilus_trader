# Volatility regimes: detecting when the market changes character

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Spot gold (XAUUSD) on daily and fifteen-minute bars, and in a few variants a small basket of shares, bonds, gold and commodities held through funds                                                  |
| How often it trades       | Rarely. The signals are deliberately filtered, so a strategy may change position only a handful of times a year                                                                                      |
| What you need             | Python and a data file for the full backtests; a spreadsheet is enough for the arithmetic on this page                                                                                               |
| Where the rules come from | [Strategy Compendium, article 06, volatility_systems](https://backtrader.readthedocs.io/en/latest/strategies-series/en/06-volatility-systems.html)                                                   |
| The underlying research   | Mandelbrot's 1963 observation that large price changes cluster together, reported in the category article, and [hmmlearn](https://hmmlearn.readthedocs.io/), the hidden Markov library the tests use |
| How well it held up       | Mixed: volatility clustering is one of the best-documented patterns in finance, but the specific regime gates and cycle filters here rest on single backtests with no independent replication        |
| Also appears in           | [Telling which kind of market you are in](../../project/measuring-the-regime/README.md) and [Whether sector rules are allowed](../../project/regime-verdict/README.md) in this collection            |

## The idea in one paragraph

Markets do not behave the same way all the time. Some stretches are calm, with prices drifting
slowly; others are violent, with prices lurching. This group of strategies tries to work out which
kind of stretch it is in before it risks any money. It measures how wildly prices have been moving,
and some of the tests fit a statistical model that says the market is currently in one of a few
hidden states, such as calm, normal or wild. Only when the measure is confident, and has stayed
confident for several days, does the rule place a trade, and it makes that trade smaller when it is
less sure. The aim is not to forecast the next move, but to be small or flat when the market is in a
state the rule does not trust.

## Why anyone believed it

The reason is a pattern so old and so widespread that it has a name: volatility clustering. Large
price changes tend to be followed by more large changes, and small changes by more small ones. If
that is true, then today's level of movement tells you something about tomorrow's, even though it
tells you nothing about whether the price will rise or fall. That is a modest but real piece of
information, and it is enough to change how much money you want at risk.

The counterparty matters too. In a violent market, many participants are forced to act rather than
choosing to: a leveraged fund has to sell as prices fall to meet its lender's demands, and a trader
who has run out of margin is closed out whether or not that is a good moment. Those forced sellers
push prices around, which is part of why a turbulent stretch stays turbulent for a while. If you are
small, or out entirely, during their forced selling, you avoid paying for their problem.

There is a second reason specific to the machine-learning tests. A market that switches between a
few moods is naturally described by a model with a few hidden states, and a computer can estimate
which state is most likely now from the recent data. That is an appealing picture, and it is why
hidden-state models became popular even before the evidence for trading them was gathered.

## An everyday comparison

Think of a coastal ferry that decides each morning whether to sail. The captain does not forecast
whether it will be raining at noon; nobody can. The captain looks instead at the sea state, the wind
and the swell, and asks a different question: is the water in a passable mood or a dangerous one? On
a calm day the boat goes out at full speed. On a rough day it stays in harbour, giving up the fare
rather than risking the passengers. The decision is based on the conditions right now, plus a
sensible confirmation that the conditions are not about to flip, not on a forecast of the weather.

## The rules, step by step

The category shares one shape: measure a state, demand a confirmation, and let the state set the
size of the position. Two representative strategies make it concrete.

The regime model, which is the archetype of the group:

1. Collect a long series of daily closing prices. The tests use daily gold from 2008 to 2025, and the
   regime test itself runs on the two years 2024 to 2025.
2. From each day, build three numbers: the daily change in the log of the price; the annualised
   volatility of returns over the last 20 days; and the momentum, meaning today's price divided by
   the price 60 days ago minus one.
3. Rescale each of the three numbers so that, over the training window, it has average zero and
   standard deviation one. This stops a large-numbered feature from dominating the model.
4. Every day, fit a three-state Gaussian hidden Markov model to the trailing 252 days of those
   rescaled numbers, and refit it every 63 days. The three states are hidden: the model estimates
   them from the data rather than being told them.
5. Name the states by their average return. The state with the highest average return is called
   BULL, the lowest is called BEAR, and the third is NEUTRAL. The state numbers themselves mean
   nothing and change on every refit, which is why they are renamed each time.
6. Each day, ask the model which state is most likely right now and with what probability, called
   the confidence. The signal is only allowed through when the confidence is at least 0.55 and the
   same state has held for the last 5 days.
7. When the state is BULL, aim for a long position worth the confidence as a fraction of the
   account, capped at the whole account. When it is BEAR, aim for a short worth half the confidence,
   capped at half the account. Otherwise hold nothing.
8. Trade only when the target changes; otherwise wait.

The asymmetric Bollinger breakout, which is the simplest member of the group:

1. Each day compute the average of the last 100 closing prices, and their standard deviation.
2. The entry band is that average plus 3.0 standard deviations.
3. The exit band is the same average minus 1.0 standard deviation.
4. If you hold nothing and the close is above the entry band, buy.
5. If you hold a position and the close is below the exit band, sell the whole position.
6. Size the position at the whole account. The entry is deliberately rare, so the strategy spends
   most of its life waiting.

## The maths, with every symbol named

The first quantity the group rests on is the annualised volatility of returns, which is the standard
deviation of recent returns scaled to a year:

```text
sigma = sqrt( (1 / (N - 1)) * sum from i = 1 to N of (r_i - r_bar)^2 ) * sqrt(252)
```

- `sigma` is the annualised volatility, written as a decimal: 0.20 means 20 percent a year.
- `r_i` is the return on day `i`, found by dividing that day's close by the previous close and
  subtracting one.
- `r_bar` is the average of the `N` returns in the window.
- `N` is the number of days in the window, 20 in the regime model.
- `252` is the approximate number of trading days in a year, used to turn a daily figure into a
  yearly one.

What it means: a large `sigma` says recent prices have been moving far from their own average, which
is the "violent" reading; a small `sigma` says they have been steady.

The second is the rescaling step, usually called a z-score, which puts every feature on the same
footing:

```text
z = (x - mu) / s
```

- `z` is the rescaled value, in units of standard deviations from the average.
- `x` is the raw feature, such as the 20-day volatility.
- `mu` is the average of that feature over the training window.
- `s` is its standard deviation over the same window; if `s` is zero it is replaced by one.

What it means: after rescaling, a `z` of plus two means "two typical steps above normal", whichever
feature it came from, so the model compares like with like.

The third is the Bollinger band, which turns a mean and a spread into two prices:

```text
upper_entry = mu_close + k_entry * sigma_close
lower_exit  = mu_close - k_exit * sigma_close
```

- `mu_close` is the average of the last 100 closing prices.
- `sigma_close` is the standard deviation of those 100 closes.
- `k_entry` is 3.0, the number of spreads above the average that counts as a breakout.
- `k_exit` is 1.0, the number of spreads below the average that counts as a breakdown.
- `upper_entry` and `lower_exit` are the two trigger prices.

What it means: when prices are quiet, both bands sit close to the average and only a genuinely large
move crosses one; when prices are wild, the bands widen so that ordinary noise does not.

## A worked example

First, the regime gate over seven days, starting flat with an account of 1,000,000. The model
reports a state and a confidence each day, and the rule needs the confidence to reach 0.55 and the
state to have held for 5 days.

| Day | State | Confidence | Same state for | Gate open | Target exposure | Action         |
| --- | ----- | ---------- | -------------- | --------- | --------------- | -------------- |
| 1   | BULL  | 0.60       | 1 day          | no        | 0.00            | hold nothing   |
| 2   | BULL  | 0.58       | 2 days         | no        | 0.00            | hold nothing   |
| 3   | BULL  | 0.62       | 3 days         | no        | 0.00            | hold nothing   |
| 4   | BULL  | 0.57       | 4 days         | no        | 0.00            | hold nothing   |
| 5   | BULL  | 0.61       | 5 days         | yes       | 0.61            | buy 61 percent |
| 6   | BULL  | 0.59       | 6 days         | yes       | 0.59            | trim to 59     |
| 7   | BEAR  | 0.66       | 1 day          | no        | 0.00            | sell out       |

The gate opens on day 5, when both conditions hold at once, so the target becomes the confidence:
`min(1.0, 0.61) = 0.61`. On an account of 1,000,000 that is a long position worth 0.61 times
1,000,000 = 610,000. Day 7 fails the confirmation test because the state has changed, so the account
goes flat. Notice how few of the seven days produced any action; in the real test, 205 bars and
4 refits produced 27 signal changes, of which the gate admitted only 2 trades.

Second, the Bollinger breakout over seven days. The average and standard deviation are recomputed
each day from the last 100 closes; the numbers below are the values those windows produce.

| Day | Close   | Mean    | Std   | Upper entry | Lower exit | Signal                  |
| --- | ------- | ------- | ----- | ----------- | ---------- | ----------------------- |
| 1   | 2130.00 | 2000.00 | 40.00 | 2120.00     | 1960.00    | close above upper, buy  |
| 2   | 2200.00 | 2010.00 | 44.00 | 2142.00     | 1966.00    | hold                    |
| 3   | 2300.00 | 2050.00 | 50.00 | 2200.00     | 2000.00    | hold                    |
| 4   | 2400.00 | 2100.00 | 55.00 | 2265.00     | 2045.00    | hold                    |
| 5   | 2450.00 | 2160.00 | 58.00 | 2334.00     | 2102.00    | hold                    |
| 6   | 2380.00 | 2200.00 | 60.00 | 2380.00     | 2140.00    | hold                    |
| 7   | 2200.00 | 2250.00 | 45.00 | 2385.00     | 2205.00    | close below lower, sell |

The band arithmetic is checkable day by day: on day 1, `2000.00 + 3.0 * 40.00 = 2120.00` and
`2000.00 - 1.0 * 40.00 = 1960.00`. The trade buys at 2130.00 on day 1 and sells at 2200.00 on day 7,
a gross gain of 70.00 per unit. The reason a profit is possible even though the exit is below the
average is that the average rose from 2000 to 2250 while the trade was open, so the exit band rose
above the entry price.

Now the costs. The tests charge a commission of 0.02 percent, and gold carries a spread between the
buying and selling price, here taken as 0.30 a unit. On one unit:

```text
gross gain      = 2200.00 - 2130.00 = 70.00
commission      = (2130.00 + 2200.00) * 0.0002 = 0.87
spread          = 0.30
net gain        = 70.00 - 0.87 - 0.30 = 68.83
return          = 68.83 / 2130.00 = 0.0323, that is 3.23 percent
```

The cost line is small here because the trade was held for several days. On a rule that trades often,
the same costs would be paid many times over.

## What the research actually found

Volatility clustering is one of the most replicated facts in finance. A survey of 150 years of
monthly, daily and minute data across a dozen markets found that the autocorrelation of absolute
returns decays as a power law, with a fitted exponent of 0.38 for monthly data and 0.45 for daily
data (`2504.08611v1`, p.7). In plain words, big moves follow big moves, and the effect fades slowly.

What the hidden-state models actually capture is narrower than their name suggests. When researchers
look inside the states, they are almost always volatility bands, not direction: a four-state model
of Bitcoin split into a high-volatility state with a daily spread of 5.91 percent, two bull states
with 2.49 and 1.69 percent, and a calm state with 0.53 percent (`2011.03741v2`, p.11). The label
"bull" is doing very little work; the model is mostly measuring how wild the market is.

The one study in the project's own research that traded a regime model out of sample won with an
online filter that re-estimated as data arrived, not with a state read off a full-history fit. It
returned 15.18 percent annualised with a reward-to-risk ratio of 1.18, against minus 2.44 percent
for a backward-looking rule (`2309.00875v3`, p.21). The same slice warns that costs decide whether
any of this is tradeable: the round-trip cost was 53.71 basis points on one futures market against
5.80 on another (`2309.00875v3`, p.12).

The category article's own numbers show how differently the members behave on the same idea. The
regime test over 2024 to 2025, using the gates above, admitted just 2 trades, both winners, for a
final value of 1,014,553.76 against 1,000,000, a gain of 1.46 percent, with a worst fall of 4.22
percent. The Bollinger breakout over 2008 to 2025 made 7 entries, of which 3 were wins and 3 were
losses once closed, a win rate of 42.9 percent, yet finished at 3,076,810.25, a gain of 207.7
percent, with a worst fall of 23.1 percent. The Fisher cyber-cycle system finished at 996,022.30, a
loss of 0.40 percent, over its three-month window.

One habit is worth copying before reading any of these numbers as a result. Every backtest in the
compendium asserts three things against a baseline captured when the test was migrated: the final
portfolio value, the reward-to-risk ratio, and the worst fall. Passing that assertion proves the
engine computes exactly what the file says it computes, in both of its modes, and nothing more. It
does not say the strategy earns anything. The two-trade regime result, in particular, is a
two-trade sample; the assertion fixes it so the engine cannot drift, but two trades cannot settle
whether a rule works.

## How this project relates to it

This repository has its own study of whether market regimes justify acting on them,
[Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md). Its Section 3.2
states the condition a directional overlay needs, and Section 9 concludes that an overlay is
admitted only on significant, material, out-of-sample evidence. That is the same bar these regime
filters must clear, and it is a high one.

The measurement engine built on that study,
[the sector regime engine user manual](../../../implementation/sector-regime-engine/USER_MANUAL.md),
does not forecast returns. It reports a verdict on whether the data shows a usable pattern, and it
refuses to suggest a rule when it does not. It is the honest cousin of the HMM filters here: both
label a market, but the engine reports the evidence behind the label rather than a profit figure.

The design for this repository's entry and exit engine is explicit about regime models. Its Section
8 rejects hidden Markov models, Bayesian regime targets and reinforcement-learning exits, on the
grounds that they are model classes with no validated performance on the project's own data. So the
HMM here is studied as a subject, not adopted as a component.

## Where it goes wrong

- The state is mostly a volatility reading, not a direction. A model that calls a state "bull" is
  labelling a stretch by its highest average return after the fact, and the label can split the
  spread of returns rather than the direction of them. Trading it as a forecast uses it for
  something the fit does not support.
- The estimated state is fragile. A hidden Markov fit depends on how many states are assumed, on the
  starting values, and on the window. The choice of the number of states has been called subjective
  in the research itself, and the same data can be two states or four depending on the setting.
- Out-of-sample is the whole question. A state read off a full-history fit uses information from the
  future; only an online filter that re-estimates as data arrives is honest, and only one study in
  the project's research did that.
- Costs can erase the signal. A regime filter trades infrequently by design, but when it does trade
  it pays the spread, and in the research the round-trip cost varied by a factor of nine between two
  comparable futures markets.
- Very few trades make a very weak test. The regime test's two trades cannot distinguish a real edge
  from luck, however precisely the engine reproduces them.

## Try it yourself

You need a spreadsheet and a column of daily closing prices; any finance website will give you gold
or a broad share index.

1. Add a column for the daily return: today's close divided by yesterday's close, minus one.
2. Add a column for the 20-day volatility: take the standard deviation of the last 20 returns and
   multiply it by the square root of 252.
3. Add a column for the 100-day average close, and a column for the 100-day standard deviation.
4. Add two columns: the entry band, the 100-day average plus 3 times the 100-day standard deviation;
   and the exit band, the average minus 1 times that standard deviation.
5. In the next column, write "buy" on the first day the close is above the entry band and you are
   flat, "sell" on the first day the close is below the exit band and you hold a position, and
   "hold" otherwise.
6. Finally, add a column that subtracts a cost of about 0.02 percent of the traded amount on every
   buy and every sell.

What to notice: the entry band is crossed very rarely, often not at all in a year, while the exit
band is crossed more often once a trade is open. That imbalance is deliberate, and it is why the
strategy holds few positions and lets them run. If your sheet produces an entry most weeks, you have
probably taken the standard deviation over too short a window.

## Where this came from

- [Strategy Compendium, article 06, volatility_systems](https://backtrader.readthedocs.io/en/latest/strategies-series/en/06-volatility-systems.html),
  the category inventory, the two deep dives and the reported baselines.
- [hmmlearn](https://hmmlearn.readthedocs.io/), the library used to fit the three-state Gaussian
  hidden Markov model, and the reason the test skips cleanly when it is not installed.
- [Stylized facts, tails and scaling](../../../strategies/books2/13_stylized_facts_and_scaling.md),
  this repository's brief, for the 150-year test of volatility clustering (`2504.08611v1`).
- [Regimes, breakpoints and state switching](../../../strategies/books2/24_regimes_and_change_points.md),
  for the finding that a hidden state is a volatility state (`2011.03741v2`) and that the only
  out-of-sample winner was an online filter (`2309.00875v3`).
- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), Sections 3.2
  and 9, for the evidence bar an overlay must clear.

## Words used in this tutorial

- volatility: how much a price moves around its average, quoted here as a yearly percentage.
- volatility clustering: the tendency of large price changes to be followed by more large changes.
- regime: a stretch in which a market behaves a certain way, such as calm or turbulent, which may
  then change.
- hidden state: a condition a model believes the market is in but cannot observe directly, only
  infer from the data.
- confidence: the probability a model attaches to its current best guess.
- Bollinger band: a pair of lines drawn a number of standard deviations above and below a moving
  average, used as trigger levels.
- standard deviation: a measure of how far values typically sit from their average.
- annualised: converted to an average rate per year, so that periods of different lengths compare.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
