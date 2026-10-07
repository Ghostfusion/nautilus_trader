# Momentum: buying what has been rising, one month at a time

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                              |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | Gold, and in the ranking versions a small menu of four funds: spot gold, an American share index fund, a Treasury bond fund and a gold fund                                                                                                                        |
| How often it trades       | About once a month, when the ranking is recomputed                                                                                                                                                                                                                 |
| What you need             | A spreadsheet and a year of monthly prices                                                                                                                                                                                                                         |
| Where the rules come from | [backtrader strategy compendium, Momentum](https://backtrader.readthedocs.io/en/latest/strategies-series/en/03-momentum.html)                                                                                                                                      |
| The underlying research   | Jegadeesh and Titman, [Returns to Buying Winners and Selling Losers](https://onlinelibrary.wiley.com/doi/10.1111/j.1540-6261.1993.tb04702.x), and Moskowitz, Ooi and Pedersen, [Time Series Momentum](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2089463) |
| How well it held up       | Mixed: momentum is among the most replicated patterns in finance, but it suffers violent reversals, has weakened after publication, and most of the forty-five variants here run on a single gold series                                                           |
| Also appears in           | [Asset class momentum](../../quantconnect/asset-class-momentum/README.md), [Sector momentum](../../quantconnect/sector-momentum/README.md) and [Momentum effect in stocks](../../quantconnect/momentum-effect-in-stocks/README.md)                                 |

## The idea in one paragraph

Look at how much each thing on a short list has gained over the past year. Buy the one that gained
the most, hold it for a month, then rank the list again and repeat. That is relative momentum: it
always holds whichever member has been strongest. A simpler cousin, absolute momentum, looks at one
thing only and asks whether it has risen over the past year at all; if it has, hold it, and if it has
not, hold cash. Both rest on the same observation, that strength tends to persist for a few months.

## Why anyone believed it

News spreads slowly. A company earning more, a metal in short supply or a currency with higher
interest rates does not reprice in one jump; it takes weeks for analysts, funds and savers to notice
and act. Whoever buys after the first move pushes the price further, so early strength is followed by
more strength. Order flow is persistent too: large traders split one big order into many small ones,
and those hidden orders keep pressing in the same direction for a while.

The counterparty is the investor who is slow, or who is forced to act for reasons unrelated to the
outlook. A fund facing withdrawals sells whatever it can, a manager trims a position that has grown
too large, and a saver takes a profit simply because the price has risen. If those sellers keep
appearing while the buyers are still arriving, the winner keeps winning.

## An everyday comparison

Think of a school class and its end-of-year examination. The pupil who has topped every test so far
is a good bet to be near the top of the next one, not because of luck but because ability carries
over. The class list is the ranking, the past tests are the one-year return, and the next
examination is the coming month. The strategy bets on the current leader each time. It works while
ability persists, and it fails in the term when a pupil who was merely lucky stops being lucky.

## The rules, step by step

1. Write down a menu of things you can buy. The compendium uses four: spot gold, IVV (an American
   share index fund), IEF (a Treasury bond fund) and GLD (a gold fund), plus cash.
2. For each, compute the return over the past twelve months: today's price divided by the price
   twelve months ago, minus one. A fund that went from 100.00 to 112.00 has a return of 0.12.
3. Put the menu in order, largest return first.
4. If the largest return is above zero, put all the money into that one asset and none in the rest.
   If even the largest return is zero or below, put the money in cash. In the single-asset version,
   hold the asset while its own twelve-month return is positive and cash while it is not.
5. Hold for one month, then recompute step 2 and repeat. Sell whatever is no longer chosen and buy
   whatever is. Review once a month and no more often.
6. Every added check, such as requiring the leader to be above its own average price, is a new rule
   and a new way for the result to be accidental.

### What is in this category

Forty-five backtests on gold and a few related funds over 2008 to 2025. Almost every one is a
variation of two questions: should I be in the market at all, and if so what should I hold.

| Strategy                                | What it does                                                                                               | Source                                   |
| --------------------------------------- | ---------------------------------------------------------------------------------------------------------- | ---------------------------------------- |
| Dual momentum (switch)                  | Checks 252 days of momentum once a month and holds gold or cash                                            | `test_0001_dual_momentum.py`             |
| Gold dual momentum (four assets)        | Ranks four assets on twelve-month return, holds the leader, or cash if the leader is falling               | `test_0002_gold_dual_momentum.py`        |
| Time-series momentum, volatility-scaled | Twelve-month direction, position scaled to a fifteen percent volatility target, with an eight percent stop | `test_0005_gold_time_series_momentum.py` |
| Antonacci classic                       | Gold against equities head to head, the dual momentum of the book                                          | `test_0015_dual_momentum_strategy.py`    |
| 52-week high effect                     | Buys between seventy-five and ninety-eight percent of the rolling high and above a slow average            | `test_0014_52week_high_effect.py`        |
| ESG momentum                            | Momentum for direction, a low-volatility ranking for quality, rebuilt every sixty-three days               | `test_0025_esg_momentum.py`              |
| Precious-metals rotation                | Ranks gold, silver, platinum and palladium on a blended twenty-one, sixty-three and 252-day return         | `test_0010_momentum_rotation_roc.py`     |
| Alpha momentum                          | Ranks assets on the return left over after the market's move and holds the high ones                       | `test_0017_alpha_momentum.py`            |
| Two-period RSI                          | A short-horizon relative-strength rule on a single share                                                   | `test_101_rsi_long_short_strategy.py`    |

The count is larger than the number of ideas. A ranking on six months instead of twelve, a two-asset
menu instead of four, or a stop added to the same signal is a new file but not a new strategy.

### Deep dive: absolute momentum as a switch

`test_0001_dual_momentum.py` is the smallest form. Its parameters are a 252-day lookback, a risk-free
threshold of 0.0 and monthly rebalancing.

1. Each day compute the 252-day momentum: today's close divided by the close 252 trading days ago,
   minus one.
2. If the day is the first bar of a new month and no order is alive, act; otherwise do nothing.
3. If the momentum is above 0.0 and nothing is held, buy. The size is the account value divided by
   price times the contract multiplier of 100, so the position's value is roughly the account.
4. If the momentum is at or below 0.0 and something is held, sell everything and hold cash.
5. Do not short, do not use a stop, and do not act between months.

### Deep dive: relative momentum as a ranking

`test_0002_gold_dual_momentum.py` widens the menu to four assets and adds cash, with a twelve-month
formation period and 0.05 percent commission.

1. Resample each asset's daily prices to month-end and line the dates up.
2. Compute each asset's return over the past twelve months.
3. Find the largest return and the asset that produced it.
4. If that largest return is above zero, the choice is its owner; otherwise the choice is cash.
5. If the choice changed since last month, sell what is no longer chosen and buy the new choice,
   driving each asset to all of the money or none of it.
6. Do not short and do not use a stop.

Two variants are worth naming here even though they are not worked through below. The 52-week high
rule buys when the close sits between 0.75 and 0.98 of its 130-day rolling high while above the
200-day average, and sells when the ratio falls below 0.70, the close drops under the average, or the
position has been held 63 days. The precious-metals rotation ranks four metals on a blended 21, 63
and 252-day return and switches monthly. Both ask the same momentum question with a different score.

## The maths, with every symbol named

The whole family is one calculation, one sort and one comparison.

The return over the formation period:

```text
M = P_today / P_formation - 1
```

- `M` is the momentum score of one asset, as a decimal: 0.12 means twelve percent.
- `P_today` is its price now and `P_formation` its price one formation period ago, twelve months in
  the compendium's version. A positive `M` means the asset rose over the window.

Ranking with a cash option:

```text
choice = the asset with the largest M, if that largest M is greater than zero
choice = cash, otherwise
```

- The first line says buy the strongest member of the menu.
- The second line is the absolute filter: when even the strongest member has fallen over the year,
  hold cash. It is what turns a ranking into a switch, and it is why the four-asset version spent 11
  of its 204 months in cash.

The rolling high and the proximity ratio used by the 52-week rule:

```text
H = the largest high over the last L days, ignoring today
ratio = P_today / H
```

- `H` is a reference level, not a prediction; `L` is the lookback, 130 trading days in the file.
- A ratio near 1 means price is close to its recent high; 0.75 means it is a quarter below.

The cost of rebuilding the portfolio:

```text
Cost = s * c * f
```

- `s` is the number of sides traded: 2 when the old holding is sold and a new one bought, 1 when a
  holding is entered or left for cash.
- `c` is the cost of one side as a fraction of the amount traded, covering the gap between the
  buying and the selling price plus commission; the files use 0.0002 to 0.0005.
- `f` is the fraction of the account traded, 1 when the whole account moves.

## A worked example

The examples use invented but plausible prices and shortened lookbacks so the arithmetic fits
on the page. Costs follow the files: 0.02 to 0.05 percent per side.

### The absolute switch

Twelve monthly closes and a six-month lookback in place of twelve. The account starts at 1,000.00 and
pays 0.02 percent on each side.

| Month | Close  | Six-month return | Decision for the next month | That month's move | Account after |
| ----- | ------ | ---------------- | --------------------------- | ----------------- | ------------- |
| Jul   | 126.00 | +26.00%          | hold the asset              | +4.76%            | 1,047.41      |
| Aug   | 132.00 | +28.16%          | hold the asset              | -3.03%            | 1,015.67      |
| Sep   | 128.00 | +20.75%          | hold the asset              | -6.25%            | 952.19        |
| Oct   | 120.00 | +9.09%           | hold the asset              | -6.67%            | 888.71        |
| Nov   | 112.00 | -2.61%           | hold cash                   | -10.71%           | 888.53        |

July's close of 126.00 against January's 100.00 gives 126/100 - 1 = 0.26, so the decision is to
hold. The account ends July at 1,000.00 times 1.0476 times 0.9998, the last factor being the buying
cost, which is 1,047.41. November's 112.00 against May's 115.00 gives -0.0261, so the decision is
cash; the sale costs 0.02 percent and the account ends at 888.53. The asset itself fell 20.63 percent
from July to December, so the switch lost 11.15 percent where holding lost 20.63 percent. It did not
predict the fall; it stopped holding after the fall had begun and avoided the worst month in cash.

### The four-asset ranking

Two-month returns stand in for twelve-month returns. The account starts at 1,000.00 and the previous
holding was the gold fund, so the switch pays both sides at 0.05 percent.

| Asset       | Price month 1 | Price month 3 | Formation return | Forward return |
| ----------- | ------------- | ------------- | ---------------- | -------------- |
| Spot gold   | 100.00        | 108.00        | +8.00%           | +3.00%         |
| Share index | 200.00        | 210.00        | +5.00%           | +1.00%         |
| Bond fund   | 100.00        | 101.00        | +1.00%           | +0.50%         |
| Gold fund   | 100.00        | 99.00         | -1.00%           | -2.00%         |

Spot gold has the largest formation return, 8.00 percent, and it is above zero, so it is chosen.
Selling the gold fund and buying spot gold costs 2 times 0.0005, or 0.10 percent. The account then
earns spot gold's 3.00 percent: 1,000.00 times 0.9990 times 1.0300 = 1,028.97. Had the largest
formation return been negative, the choice would have been cash and the account would have ended at
1,000.00 times 0.9995, or 999.50.

## What the research actually found

Momentum has one of the longest published records of any trading rule. Jegadeesh and Titman, using
American shares from 1965 to 1989, measured that shares which had done best over the previous three
to twelve months beat the previous losers by roughly one percent a month over the following three to
twelve months. Moskowitz, Ooi and Pedersen extended the idea to fifty-eight futures markets and found
that the sign of an asset's own past twelve-month return predicted its next month's return. Both
results are before the costs a real account pays.

Before turning those findings into expectations, it is worth stating what the compendium's own
numbers are and are not. Every backtest here asserts three numbers against a baseline: the final
portfolio value, the reward-to-risk ratio, and the worst fall from a peak. Passing those assertions
proves that the engine computes what the file says it computes. It does not prove that the strategy
earns anything, that the number would survive a different market, or that a real account could have
captured it. The assertions are a correctness test for the software, not evidence of profit.

| Source                                  | What it measured                                    | Result                                                                                                               |
| --------------------------------------- | --------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| Jegadeesh and Titman (1993)             | American shares, 1965 to 1989                       | Past winners beat past losers by about one percent a month over the following three to twelve months, before costs   |
| Moskowitz, Ooi and Pedersen (2012)      | Fifty-eight futures markets                         | The sign of an asset's own twelve-month return predicted its next month's return                                     |
| McLean and Pontiff, in this collection  | Ninety-seven published predictors after publication | The effect falls by about 26 percent out of sample once everyone knows the rule                                      |
| Chen and Zimmermann, in this collection | 207 academic predictors from 140 papers             | 183 clear a t-statistic of 2.0, 74 clear 4.0 and 26 clear 6.0                                                        |
| This repository's sector rotation study | 1,022 rotation rules on American sectors            | The average rule returned 0.86 percent a month against 0.89 percent for simply holding the market                    |
| `test_0001_dual_momentum.py`            | Gold daily, 2008 to 2025                            | 14 trades, 5 wins and 8 losses, a 35.71 percent win rate, 3,789,720.30 on 1,000,000, worst fall 33.71 percent        |
| `test_0002_gold_dual_momentum.py`       | Four funds monthly                                  | 204 months, 52 switches, 2,078,226.32 on 1,000,000, worst fall 12.08 percent, reward-to-risk 0.51, 11 months in cash |
| `test_0014_52week_high_effect.py`       | Gold daily                                          | 98 trades, 32 wins and 65 losses, a 32.65 percent win rate, 2,992,578.56 on 1,000,000, worst fall 30.61 percent      |

Read the last three rows with the window in mind. Gold rose for most of 2008 to 2025, so a rule that
is invested whenever gold is rising looks good on that window. The single-asset switch won fewer than
four trades in ten and still finished ahead, the shape of momentum: many small losses, a few large
gains. The four-asset version gave up return but cut the worst fall from 33.71 to 12.08 percent,
because holding cash when everything is falling is built into the rule.

## How this project relates to it

This repository holds its own study of the same question:
[Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md). Section 3.2
states the condition the idea needs, that leaders stay leaders for several months, and Section 3.3
collects the tests, including the 1,022-rule experiment quoted above and a cross-sector regression
whose test statistics came out consistent with chance.

[Portfolio construction and allocation](../../../strategies/books2/10_portfolio_and_allocation.md)
records the out-of-sample decline of published predictors, about 26 percent of the in-sample return
once the sample ends, and warns that the way a backtest is built flatters it: net returns inflate the
S&P 500 reward-to-risk ratio by nearly 30 percent, and compounding an arithmetic mean overstates the
1960 to 2020 terminal value by 89 percent. A monthly momentum backtest reporting a smooth annual
average is exactly the kind of number that warning is about.

## Where it goes wrong

- Momentum crashes. After a market-wide fall, the assets that fell furthest are often the ones that
  led the previous ranking, and they bounce hardest. A rule that buys them before the fall takes the
  loss twice, which is where a worst fall near a third of the account comes from.
- Publication decay. McLean and Pontiff measured that ninety-seven replicated predictors lose about
  26 percent of their return once the paper is out and everyone can trade the rule. The strength you
  measure is partly the reward for noticing something before others did.
- The number of rules tried. Sorting shares on functions of 240 accounting variables produced 18,113
  strategies, of which 8.40 percent cleared a t-statistic of 4.0 against a chance rate of 0.0063
  percent. The forty-five momentum files here are the same arithmetic on a smaller scale.
- Costs on a monthly rebuild. Every change of leader pays the gap between buying and selling on both
  sides. The ranking version switched 52 times in 204 months, roughly every four months.
- One market, one window. Almost all forty-five files run on gold from 2008 to 2025, a period in
  which gold rose. A rule tested only where the signal worked is not evidence that it works elsewhere.
- Variants counted as discoveries. A twelve-month ranking, a six-month ranking and a ranking with a
  stop are three files and one hypothesis, which makes the evidence look wider than it is.

## Try it yourself

1. Put one column per fund and one row per month for the last five years.
2. Add a column that computes each fund's twelve-month return: this month's price divided by the
   price twelve rows up, minus one.
3. Add a column naming the fund with the largest value in that row, and writing cash instead if the
   largest value is zero or below.
4. In the next row down, write the chosen fund's next-month return. That is the strategy's return.
5. Do the same for the share index fund alone, then subtract 0.10 percent from the strategy column
   in any month in which the name changed.

What to notice: in some months the name is unchanged and nothing trades, while in others every
holding changes, and the months that decide the difference are the few in which the leader fell
sharply. If the strategy wins by a wide margin, check whether the fund list contains only funds that
existed for the whole period, because funds that disappeared would have been a losing month you never
saw.

## Where this came from

- [backtrader strategy compendium, Momentum](https://backtrader.readthedocs.io/en/latest/strategies-series/en/03-momentum.html),
  the category inventory and the deep dives on the switch, the ranking and the 52-week high.
- `test_0001_dual_momentum.py`, `test_0002_gold_dual_momentum.py`,
  `test_0005_gold_time_series_momentum.py` and `test_0014_52week_high_effect.py` in
  [tests/functional/strategies/momentum](https://github.com/cloudQuant/backtrader/tree/development/tests/functional/strategies/momentum),
  the files from which the rules and the asserted numbers above are taken.
- Jegadeesh and Titman, [Returns to Buying Winners and Selling Losers](https://onlinelibrary.wiley.com/doi/10.1111/j.1540-6261.1993.tb04702.x),
  and Moskowitz, Ooi and Pedersen, [Time Series Momentum](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2089463).
- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md) and
  [Portfolio construction and allocation](../../../strategies/books2/10_portfolio_and_allocation.md),
  this repository's own studies, the source of the 1,022-rule experiment, the 26 percent decline and
  the 18,113-strategy figure.

## Words used in this tutorial

- absolute momentum: the rule that asks only whether one asset has risen over the lookback.
- cash: money not invested, which earns nothing here.
- drawdown: the fall from a peak to the following low, measured in percent.
- formation period: the stretch of past prices used to score an asset.
- momentum: the tendency of something that has been rising to keep rising for a while.
- position: the holding an account owns at a moment in time.
- relative momentum: the rule that ranks a menu of assets and holds the strongest.
- reward-to-risk ratio: the return earned per unit of the price's up-and-down movement, called the
  Sharpe ratio when it is computed in this way.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
