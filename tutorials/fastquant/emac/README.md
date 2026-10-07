# The exponential moving average crossover: the same rule with a shorter memory

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                          |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | One company's shares, or one fund that tracks an index, bought and sold whole                                                                                                                                                                                                  |
| How often it trades       | A few times a year, usually slightly more often than the simple version of the same rule                                                                                                                                                                                       |
| What you need             | A spreadsheet                                                                                                                                                                                                                                                                  |
| Where the rules come from | The strategy table in the [fastquant](https://github.com/enzoampil/fastquant) README (alias `emac`) and its [source file](https://github.com/enzoampil/fastquant/blob/master/python/fastquant/strategies/ma_crossover.py)                                                      |
| The underlying research   | none, this is a practitioner's rule of thumb; the rule it imitates was tested by Brock, Lakonishok and LeBaron (1992), [Simple Technical Trading Rules and the Stochastic Properties of Stock Returns](https://onlinelibrary.wiley.com/doi/10.1111/j.1540-6261.1992.tb04681.x) |
| How well it held up       | Weak: tests of moving average rules generally used the plain average and lost their power out of sample; the exponential version has no separate record that changes the picture                                                                                               |
| Also appears in           | [smac](../smac/README.md), the same rule on the plain average, in this same library                                                                                                                                                                                            |

## The idea in one paragraph

This is the moving average crossover rule again, with one change: instead of averaging the last few
closing prices equally, the average gives the most recent day the largest weight and each earlier day
a little less. When the price turns, an average built that way turns sooner, because it listens more
to yesterday than to a fortnight ago. The rule still buys when the short average crosses above the
long one and sells when it crosses back below. The whole difference from the plain version is how
much of the past each day is allowed to influence the line.

## Why anyone believed it

The economic story is the same as for the plain crossover: news spreads through a crowd slowly, some
buyers act late, some sellers are forced or reluctant, and the price therefore moves in runs rather
than jumping straight to a new level. The exponential version adds a second, weaker belief: that the
most recent days carry more information than the older ones. If a rising price is itself evidence that
more buying is on the way, then yesterday's high price is more informative than the price a fortnight
ago, and a rule that reacts to yesterday sooner will act nearer the start of the run.

The counterparty, once again, is the slow seller and the late buyer. What the exponential version
cannot avoid is that its counterparty can be just as aware of the pattern; buying earlier is worth
something only if there is still something left to buy.

## An everyday comparison

Ask two people to describe how a friend's health has been going. The first reads the last three
medical notes and treats them equally. The second reads the same notes but believes the newest one
matters most, the one before it roughly half as much, and the one before that half again, and so on
backwards. Both are describing the same record. When the news turns bad, the second person notices the
change in the summary earlier; when the news is noisy and contradictory, the second person also swings
around more, because the latest note carries more of the total.

## The rules, step by step

1. Pick one thing to trade: one company's shares, or one fund that tracks an index.
2. Get the daily closing price. Every rule below uses closes only.
3. Choose the short period, called `fast_period`, and the long period, called `slow_period`. The
   fastquant defaults are 10 and 30, exactly as in the plain version.
4. Compute the exponential average of the last `fast_period` closes and the exponential average of the
   last `slow_period` closes, as described in the next section.
5. Buy when the short line crosses from at or below the long line to above it.
6. Sell when the short line crosses from at or above the long line to below it.
7. Hold, doing nothing, at every other time. There is no separate stop loss or profit target unless
   you add one.
8. Review every day, at the close.

The library buys with all the available cash and sells the whole position by default, through
`buy_prop` and `sell_prop`. The rule is otherwise character for character the same as the simple
version; only the averaging differs.

## The maths, with every symbol named

The exponential moving average of the last `N` closes:

```text
alpha = 2 / (N + 1)
EMA_t = alpha * P_t + (1 - alpha) * EMA_(t-1)
```

- `EMA_t` is the exponential average on day `t`.
- `P_t` is today's closing price.
- `EMA_(t-1)` is the same average calculated yesterday.
- `N` is the number of days, `fast_period` or `slow_period`.
- `alpha` is the weight given to today's price, between 0 and 1.

The first value of the line is not a special case to memorise: it is simply the plain average of the
first `N` prices. After that, each day blends the new price with the old line, and the older prices in
that line are worth less every day. The weight on the price from `k` days is `alpha * (1 - alpha)^k`,
which decays instead of stopping, so unlike the plain average this one never completely forgets a
price; it just discounts it.

For a three-day period, `alpha` is `2 / 4`, which is 0.5, and the weights look like this. This is the
whole difference between the two kinds of average, computed by hand:

| Days back | Weight in the plain 3-day average | Weight in the exponential 3-day average |
| --------- | --------------------------------- | --------------------------------------- |
| 0 (today) | 0.3333                            | 0.5000                                  |
| 1         | 0.3333                            | 0.2500                                  |
| 2         | 0.3333                            | 0.1250                                  |
| 3         | 0.0000                            | 0.0625                                  |
| 4         | 0.0000                            | 0.0313                                  |
| older     | 0.0000                            | 0.0312                                  |

- The plain column adds to 1 because three equal thirds are a whole.
- The exponential column multiplies down by half each step: 0.5, then 0.25, then 0.125, and so on.
- The remainder of 0.0312 sits on every price older than four days, so the exponential average still
  remembers the past even as it mostly listens to today.

The crossing test is the same comparison as before, now between two exponential lines:

```text
Buy  when EMA_t(fast) >  EMA_t(slow) and EMA_(t-1)(fast) <= EMA_(t-1)(slow)
Sell when EMA_t(fast) <  EMA_t(slow) and EMA_(t-1)(fast) >= EMA_(t-1)(slow)
```

- `EMA_t(fast)` is the short exponential line today, and `EMA_t(slow)` the long one.
- The conditions on two consecutive days are what make it a crossing rather than a state.

## A worked example

Ten made-up closes, with `fast_period` set to 3 and `slow_period` set to 5. The same closes are shown
twice on the right, once through the plain averages and once through the exponential ones, so the two
can be compared line by line.

| Day | Close | Plain SMA(3) | Plain SMA(5) | Exp EMA(3) | Exp EMA(5) |
| --- | ----- | ------------ | ------------ | ---------- | ---------- |
| 1   | 100   | -            | -            | -          | -          |
| 2   | 100   | -            | -            | -          | -          |
| 3   | 99    | 99.67        | -            | 99.67      | -          |
| 4   | 100   | 99.67        | -            | 99.83      | -          |
| 5   | 99    | 99.33        | 99.60        | 99.42      | 99.60      |
| 6   | 99    | 99.33        | 99.40        | 99.21      | 99.40      |
| 7   | 99    | 99.00        | 99.20        | 99.10      | 99.27      |
| 8   | 100   | 99.33        | 99.40        | 99.55      | 99.51      |
| 9   | 104   | 101.00       | 100.20       | 101.78     | 101.01     |
| 10  | 108   | 104.00       | 102.00       | 104.89     | 103.34     |

The first six days are a quiet spell around 99, where the two lines are close and swap order for small
reasons. On day 8 the price moves from 99 to 100. The plain three-day average rises to 99.33 while the
plain five-day is 99.40, so the plain pair has not crossed. The exponential pair has: the three-day
line reaches 99.55 and the five-day 99.51, because the newest price carries half the weight in the
short line but only a third of it in the long line. The exponential rule buys on day 8 at 100. The
plain rule buys a day later, on day 9, at 104.

Mark both positions at day 10, the last price, 108, and charge 0.10 percent per side:

```text
Exponential: bought at 100, marked at 108. Gross gain 800.00, costs 20.80, net 779.20.
             That is 7.8 percent of the 10,000.00 invested.
Plain:       bought at 104, marked at 108. Gross gain 400.00, costs 21.20, net 378.80.
             That is 3.6 percent of the 10,400.00 invested.
```

Two honest notes. The numbers are invented, and they were chosen to make the crossing days differ; on
other invented numbers the two rules cross on the same day, and on real prices the difference between
them is usually a fraction of a day's move. Acting earlier is not the same as acting better: the same
speed that enters the trade sooner also exits sooner when the move reverses, and the exponential rule
pays for its extra sensitivity with more false signals.

## What the research actually found

The published tests of moving average rules almost always use the plain average, so what follows is
evidence about the family the exponential version belongs to, not about the exponential version by
name.

Brock, Lakonishok and LeBaron (1992) tested 26 simple moving average and related rules on the Dow
Jones Industrial Average from 1897 to 1986 and found that all 26 beat the benchmark of holding cash.
Sullivan, Timmermann and White (1999) then corrected for the fact that those 26 were drawn from a much
larger family of rules and extended the data by ten years:

| What was measured                          | What came out                                                                                               |
| ------------------------------------------ | ----------------------------------------------------------------------------------------------------------- |
| Best of the original 26, over 1897 to 1996 | About 9.4 percent a year, still beating the benchmark in sample                                             |
| The same rules on 1987 to 1996             | The result stopped being convincing once the searching over rules was accounted for                         |
| 7,846 rules on the full century            | The best in-sample rule earned 17.2 percent a year but won only 2,501 of its 6,310 trades, about 40 percent |
| S&P 500 futures, 1984 to 1996              | No evidence at all that the rules added anything                                                            |

Strobel and Auer (2018) measured the same family of rules across many developed markets and individual
shares from 1972 to 2015 and found the predictive power declining across the period, which they link
to the declining tendency of prices to follow their own recent direction. Lento and Gradojevic (2022)
tested technical rules on five markets during the crash of early 2020 and found that the moving
average rules, plain or otherwise, did not survive their costs. I did not find a study that isolates
the exponential version and reports an edge over the plain one; the honest position is that the choice
between them changes the timing of trades, not the evidence about whether the rule works.

The defaults deserve the same warning as in the plain version. The fastquant defaults of 10 and 30 are
two round numbers, and the README reports them turning 100,000 into 90,976.00 on one Philippine stock
over 2018, which is a loss of about 9 percent, computed with a commission of zero. That is one stock,
one year, one pair of numbers. The same README's plain-average example on the same year lost less.
A single backtest of that shape cannot tell you whether the exponential average is better; it can only
tell you which of two particular numbers happened to fit one particular year.

## How this project relates to it

The repository implements the exponential average in Rust, in
[crates/indicators/src/average/ema.rs](../../../crates/indicators/src/average/ema.rs). That file is the
recursion `EMA_t = alpha * P_t + (1 - alpha) * EMA_(t-1)` from the maths section, together with the
same seeding rule, the plain average of the first `N` prices. Reading it beside the plain average in
[crates/indicators/src/average/sma.rs](../../../crates/indicators/src/average/sma.rs) makes the single
line of difference between the two tutorials concrete. The wider survey of what these rules have
delivered is
[08_predictability_and_trading_strategies.md](../../../strategies/books2/08_predictability_and_trading_strategies.md).

## Where it goes wrong

- Faster is not the same as better. The exponential line turns sooner on a real turn and also sooner
  on noise, so it buys earlier into genuine moves and more often into false ones. Over a long sample
  the two effects largely cancel, which is why studies rarely find one version beating the other.
- The signal is still built from the past. The line reacts faster to prices that have happened; it
  contains no information about prices that have not.
- The parameters decide the answer. The short and long periods, and the choice between the plain and
  exponential form, are all free choices. Testing many combinations and keeping the best is the same
  trap as before, with one more dimension.
- Whipsaw costs. In sideways markets the lines cross repeatedly, and every crossing is a trade with a
  cost. The exponential version crosses at least as often as the plain one.
- The out-of-sample failure is not about the average type. The 1897 to 1986 result did not survive the
  next ten years or the adjustment for searching over rules. Changing the weighting does not change
  that.
- What would have to be true. The rule needs the move to continue long enough after the crossing to pay
  for entering, exiting and the one reversal in between. If moves are shorter than that, the rule pays
  costs forever and never collects the trend.

## Try it yourself

You need a spreadsheet and twenty to thirty daily closes.

1. Put dates in column A and closes in column B.
2. In C3 enter `=AVERAGE(B1:B3)` as the seed of a three-day exponential average, then in C4 enter
   `=0.5*B4+0.5*C3` and drag it down. The 0.5 is `2/(3+1)`.
3. In D5 enter `=AVERAGE(B1:B5)`, then in D6 enter `=0.333*B6+0.667*D5` and drag it down. The 0.333 is
   `2/(5+1)`.
4. In E3 enter `=AVERAGE(B1:B3)` for the plain average and drag it down; this is the comparison line.
5. In F5 write `=IF(C5>D5,1,0)` and in G5 `=F5-F4`, so a 1 marks a day the fast exponential line
   crossed above the slow one, and a -1 a crossing down.
6. Count the crossings on the exponential pair and on the plain pair.

What to notice: the exponential fast line is usually further from the slow one than the plain fast
line, in the direction the price is moving. That gap is the extra sensitivity, and it is worth exactly
one thing: entering a little sooner. It is also worth exactly one cost: more crossings in a market
that goes nowhere.

## Where this came from

- The [fastquant](https://github.com/enzoampil/fastquant) README, for the alias `emac`, the parameter
  names `fast_period` and `slow_period`, and the one-stock example result.
- The [moving average crossover source](https://github.com/enzoampil/fastquant/blob/master/python/fastquant/strategies/ma_crossover.py),
  which contains both the simple and the exponential versions and shows that the rule itself is
  identical.
- The [backtest documentation](https://github.com/enzoampil/fastquant/blob/master/docs/docusaurus/docs/backtest.md),
  for the parameters that control how much is bought and sold.
- Brock, Lakonishok and LeBaron (1992), [Simple Technical Trading Rules and the Stochastic Properties of Stock Returns](https://onlinelibrary.wiley.com/doi/10.1111/j.1540-6261.1992.tb04681.x).
- Sullivan, Timmermann and White (1999), [Data-Snooping, Technical Trading Rule Performance, and the Bootstrap](https://www.kevinsheppard.com/files/teaching/mfe/advanced-econometrics/Sullivan_Timmermann_White.pdf),
  for the out-of-sample collapse and the trade statistics.
- Strobel and Auer (2018), [Does the predictive power of variable moving average rules vanish over time and can we explain such tendencies?](https://doi.org/10.1016/j.iref.2017.10.012).
- Lento and Gradojevic (2022), [The Profitability of Technical Analysis during the COVID-19 Market Meltdown](https://www.mdpi.com/1911-8074/15/5/192).
- The repository's own survey,
  [08_predictability_and_trading_strategies.md](../../../strategies/books2/08_predictability_and_trading_strategies.md).

## Words used in this tutorial

- benchmark: the thing a strategy is compared with, usually simply owning the index.
- closing price: the price of the last trade of the day, used as that day's price.
- commission: the fee a broker charges for buying or selling.
- exponential moving average: an average in which newer prices count for more than older ones.
- moving average: the plain average of the last few prices, recomputed each day.
- seed: the first value used to start a running calculation.
- whipsaw: a signal followed quickly by the opposite signal, usually at a small loss.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
