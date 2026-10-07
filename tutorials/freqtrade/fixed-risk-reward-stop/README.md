# The fixed risk-reward stop: risking one amount to aim at a chosen multiple of it

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                    |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | A cryptocurrency pair on the spot market; the file opens a position on every candle, so the size of the bet is decided by the stop, not by a signal                                                                                                      |
| How often it trades       | Whenever the bot's configured timeframe produces a candle; the file itself sets no timeframe, so it runs on whatever the configuration says, commonly five minutes                                                                                       |
| What you need             | Nothing but this page and a pencil; the arithmetic is a subtraction, a multiplication and a division                                                                                                                                                     |
| Where the rules come from | [FixedRiskRewardLoss.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/FixedRiskRewardLoss.py)                                                                                                                        |
| The underlying research   | None directly: the risk multiple is a practitioner convention, the volatility distance comes from Wilder's average true range, and the closest local treatment is the [Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md) |
| How well it held up       | Weak: no sample is published with the file, the entry rule is a placeholder that buys unconditionally, and the ratio alone tells you nothing until the win rate is measured                                                                              |
| Also appears in           | [Risk boundaries and stops](../../project/risk-boundaries-and-stops/README.md), which places a fixed stop and a profit target side by side                                                                                                               |

## The idea in one paragraph

A trade has two prices that matter before it starts: the price at which you give up, below the entry,
and the price at which you take the profit, above it. This strategy sets the first from how much the
pair has recently been moving, so a calm pair gets a close stop and a wild pair gets a distant one,
and then sets the second at a fixed multiple of the first. The multiple here is three and a half, so
the profit aimed at is three and a half times the amount risked. Along the way the stop is dragged up
in two steps: once the trade is a little ahead it moves to the entry price plus the fees, and once the
profit target is nearly reached it moves to the target itself. The point is to fix, in advance, the
ratio of what could be lost to what could be won.

## Why anyone believed it

A trader who only knows how to enter has no way to compare two trades. A trader who fixes the risk
first can compare them: a trade where one unit of risk aims at three and a half units of reward looks
better than one where it aims at half a unit, whatever the market. Fixing the ratio also fixes how
much of the account is at stake, because the distance to the stop is known before the position is
opened, so the same money can be risked on a calm pair and on a wild one by adjusting the size.

The volatility distance adds a practical reason. A stop placed a fixed percentage below the entry is
too tight for a pair that swings five percent an hour and too loose for one that swings half a
percent; the pair will hit the tight stop by ordinary noise and the loose stop only on a real
reversal. Sizing the distance by the recent average movement makes the stop mean the same thing on
both.

The counterparty is the noise itself. Every trade has to survive the ordinary back-and-forth before
the target can be reached, and the trader who sets the ratio is betting that the specific target, not
the market's average, is what happens next. When a pair spends most of its time making small moves,
the three-and-a-half target is rarely reached and the stop is, which is the failure the ratio cannot
see.

## An everyday comparison

Think of a bookmaker setting odds on a football match. The bookmaker does not need to know the result,
only to price the two outcomes so that the money coming in on the winner covers the money paid to the
losers. A punter who is offered three and a half to one only needs to be right more than one time in
four and a half to come out ahead. That number, not the odds alone, is what makes the bet good or bad,
and it is exactly the number this strategy's ratio implies. A trader who fixes three-and-a-half to one
is saying the same thing as the punter: I will be right at least about a quarter of the time. Whether
that is true is a question about the market, and the odds say nothing about it.

## The rules, step by step

1. On each candle of the configured timeframe, measure the average true range, the same measure of how
   far a price moves that the sibling [supertrend tutorial](../supertrend/README.md) explains. The
   file uses the standard setting of the last fourteen candles, and a smoothed version of the average.
2. Record, for every candle, a provisional stop price as the close of that candle minus twice the
   average true range. This is a loose distance of two typical moves.
3. Open a position at the next candle. The file's entry rule buys on every candle; it is a placeholder
   and not a reason to trade.
4. When the position opens, look up the provisional stop price recorded for that candle. That is the
   initial stop, and it does not change afterwards.
5. Work out the risk distance: the entry price minus that initial stop. Call it `R`.
6. Work out the profit target: the entry price plus three and a half times `R`.
7. Hold the position. While its gain is below one `R`, the stop stays at the initial stop.
8. Once the gain reaches one `R`, move the stop up to the entry price plus the round-trip fees, which
   locks in roughly break even if the trade then falls.
9. Once the gain reaches three and a half `R`, move the stop up to the profit target itself, which
   locks in about the full reward.
10. Close the position whenever the price reaches the stop. The file has no sell signal, so the stop is
    the only exit.
11. The stop set in the file is 90 percent below the price; it applies before the first calculation and
    as an absolute floor afterwards. The bot also refuses to lower a stop, so the stop only ever moves
    up.

## The maths, with every symbol named

The initial stop is the entry candle's close less twice the average true range:

```text
initial_stop = Close_entry - 2 * ATR_entry
```

- `Close_entry` is the closing price of the candle on which the position opened.
- `ATR_entry` is the average true range at that candle, the smoothed average size of the last fourteen
  candles' true ranges.

The risk distance and the target follow:

```text
R = entry_price - initial_stop
target = entry_price + 3.5 * R
```

- `R` is the amount risked per unit, in price terms: how far the price can fall before the stop.
- `entry_price` is the price paid.
- `3.5` is the chosen ratio of reward to risk.
- `target` is the price aimed at.

The ratio tells you the win rate you need before the two outcomes balance:

```text
required win rate = 1 / (1 + ratio) = 1 / 4.5 = 0.2222, about 22 percent
```

- The `1` in the denominator is the single unit lost on a losing trade; the `ratio` is the units won on
  a winning one.
- 22 percent is the rate with no costs. Add the costs and the rate rises, as the worked example shows.

The two later stops, as the bot applies them, are absolute prices:

```text
break_even_stop = entry_price * (1 + fee_open + fee_close)
take_profit_stop = target
```

- `fee_open` and `fee_close` are the fees paid on each side; the file reads them from the trade.
- Both are expressed as a fraction of the current price when the bot is asked for a stop, but the bot
  ignores any value that would lower the stop, so in practice each of these is a fixed price once
  reached.

## A worked example

The position was bought at 100.00 dollars. The average true range at that candle was 2.00 dollars and
the close was 100.00, so the initial stop is 96.00, the risk distance is 4.00, and the target is
114.00. Fees are 0.10 percent on each side, so the break-even stop is 100.20.

| Time | Price  | Gain    | Stage reached      | Stop price | Why                                                              |
| ---- | ------ | ------- | ------------------ | ---------- | ---------------------------------------------------------------- |
| t0   | 100.00 | 0.00%   | first              | 96.00      | initial stop, fixed from the entry candle                        |
| t1   | 102.50 | +2.50%  | first              | 96.00      | gain below one risk distance, so the first stop still applies    |
| t2   | 104.00 | +4.00%  | break-even         | 100.20     | gain above one risk distance, stop moved above the entry         |
| t3   | 110.00 | +10.00% | break-even         | 100.20     | target not yet reached                                           |
| t4   | 116.00 | +16.00% | profit target      | 114.00     | gain above three and a half risk distances, stop moved to target |
| t5   | 113.50 | +13.50% | closed at the stop | 114.00     | price fell through the stop and the position closed              |

The threshold for the break-even step is a little below four percent. The gain reaches one risk
distance of 4.00 when the price `r` satisfies `r / 100 - 1 = 4.00 / r`; solving gives `r` of about
103.85, which is why t2 at 104.00 arms it and t1 at 102.50 does not. The threshold for the profit
target is exactly 14 percent, which is the ratio times the 4 percent risk.

At t5 the price of 113.50 is below the target of 114.00, so the file computes the break-even stop
again, which would be 100.20, lower than the stop already set. The bot refuses to lower a stop, so the
stop stays at 114.00 and the position closes there. Counting the fees and the gap between prices at
about 0.30 percent for the round trip, the step-by-step result is:

```text
gross = (114.00 - 100.00) / 100.00 = +14.00 percent
net = 14.00 - 0.30 = +13.70 percent
```

The loss case is the mirror image. Had the price fallen to 96.00 at t1, the position would have closed
at the initial stop, for a gross of -4.00 percent and a net of about -4.30 percent. One unit of risk,
three and a half units of reward. The win rate needed to break even before costs is `1 / 4.5`, about
22 percent; with the costs above, the winning trade earns 13.70 and the losing trade loses 4.30, so
the break-even rate becomes `4.30 / 18.00`, about 24 percent. The strategy has to be right on more
than about one trade in four, and the file publishes no number for how often it is.

## What the research actually found

There is no experiment behind this file. The risk multiple is a convention that long predates it, and
the file states no sample, no period and no result. Its entry rule buys on every candle, so as written
it produces a stream of small trades whose only source of variation is the stop placement.

The one measured part is the distance it uses. Wilder's average true range is a standard volatility
estimate, and the idea of scaling a stop to a multiple of it appears throughout the practitioner
literature on stops. In this repository's review of
[risk measures and drawdowns](../../../strategies/books2/22_risk_measures_and_drawdowns.md), the
relevant finding is that a path-dependent exit changes the shape of the outcomes, not the average of
the underlying prices: a strategy can win more often and still earn nothing, because the stop is doing
the winning. That review is about measurement generally, not about this ratio, and no source anywhere
establishes that three and a half to one is a good number to choose.

## How this project relates to it

[Risk boundaries and stops](../../project/risk-boundaries-and-stops/README.md) is the tutorial in this
collection that treats exactly this machinery: a stop set from a volatility multiple, a profit target,
and a time limit, evaluated in a single layer. It makes the point this file illustrates: the stop and
the target decide the shape of the outcome, so the layer is reported with the win rate and both tails
of the distribution beside it, never as a source of return.

[Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md) is the document
behind that tutorial. Its Section 7 groups volatility-scaled stops and fixed targets as risk
boundaries that may only be enabled when a market state admits a bet, and requires held-out evidence
before anything but the cost model is switched on. The
[market impact and trading cost brief](../../../strategies/books/02_market_impact_and_trading_cost.md)
is where the cost side of the same document comes from.

## Where it goes wrong

- The ratio is silent about the win rate. Three and a half to one sounds generous, but it only pays if
  the target is reached more than about a quarter of the time, and the file has no measurement of that.
- The entry rule is a placeholder. Buying every candle is not a strategy, and any profit or loss it
  shows is a property of the stop and the market's drift, not of a signal.
- The stop is a price level, not a guarantee. A price can gap through the stop, so the actual loss can
  exceed the risk distance the ratio assumes, and in a thin pair the gap between prices makes it worse.
- The fixed multiple can be too far in a quiet market. If the pair rarely moves three and a half times
  its typical move before turning, the target is never reached and the trade drifts to the stop.
- The initial stop is fixed at entry and never reconsidered. If the market becomes calmer or wilder
  after the position opens, the stop does not adjust, even though the same size of movement now means
  something different.
- Costs are outside the ratio. The file's break-even step adds the round-trip fees, which shows the
  author knew costs matter, but the break-even win rate used to judge the ratio ignores them.

## Try it yourself

You need nothing but a spreadsheet and a public price history.

1. Pick a price series and write the entry price, say 100.00, in a cell.
2. Compute the average true range over the last fourteen candles, and write the initial stop as the
   close minus twice that number. Write the risk distance as the entry minus the stop.
3. Write the target as the entry plus three and a half times the risk distance.
4. Add a column of prices and a column of gains, being the price divided by the entry minus one.
5. Add a column for the stop in force: the initial stop until the gain reaches one risk distance, then
   the entry plus fees, then the target once the gain reaches three and a half risk distances, always
   taking whichever is highest so far.
6. Mark the first row where the price meets or crosses the stop; that is the trade's end. Subtract 0.30
   percent for costs.
7. Repeat for thirty different starting dates, count how often the trade ended at the target rather
   than the stop, and compare that rate with the 22 percent the ratio requires.

What to notice: the win rate you measure will rarely sit near the 22 percent the ratio needs, and it
will differ from series to series. Change the multiple from three and a half to one and repeat: the
target is reached far more often, but each win is small, and the number the ratio requires falls to
`1 / 2`, half the trades. A short target with a high hit rate and a long target with a low hit rate can
break even at the same point; the ratio alone does not tell you which side of that point you are on.

## Where this came from

- [FixedRiskRewardLoss.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/FixedRiskRewardLoss.py),
  the file whose ratio, break-even step, target step and placeholder entry are described above.
- The freqtrade documentation on
  [custom stoploss](https://www.freqtrade.io/en/stable/strategy-callbacks/), which states that a stop
  can only move upwards and that the returned value is a fraction of the current price.
- [Risk boundaries and stops](../../project/risk-boundaries-and-stops/README.md) and
  [Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md), this
  repository's own account of volatility-scaled stops and profit targets.
- [Risk measures, tails and drawdowns](../../../strategies/books2/22_risk_measures_and_drawdowns.md)
  and [Market impact and trading cost](../../../strategies/books/02_market_impact_and_trading_cost.md),
  the local material on path-dependent exits and on the costs that sit outside the ratio.

## Words used in this tutorial

- average true range: the average size of recent price moves, counting gaps between candles as well as each candle's own range.
- entry price: the price paid when a position is opened.
- profit target: the price at which a position is closed to take a gain.
- risk distance: how far the price can fall before the stop, the amount risked on one unit.
- risk multiple: the number of units aimed at in profit for each unit risked, here three and a half.
- win rate: the share of trades that end in a gain rather than a loss.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
