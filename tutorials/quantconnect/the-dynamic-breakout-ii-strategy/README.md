# A breakout rule whose trigger level moves with how much the price is jumping

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                               |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | One currency pair at a time, bought or sold short, chosen from the most heavily traded pairs                                                                                        |
| How often it trades       | At most once every few days, whenever the price breaks a level set shortly before                                                                                                   |
| What you need             | A spreadsheet and daily records of the high, low and closing price of one pair                                                                                                      |
| Where the rules come from | [QuantConnect strategy library, the Dynamic Breakout II strategy](https://www.quantconnect.com/tutorials/strategy-library/the-dynamic-breakout-ii-strategy)                         |
| The underlying research   | Pruitt, Hill and Russak, [Building Winning Trading Systems](https://www.wiley.com/WileyCDA/WileyTitle/productCd-1118168275.html) (2008 edition), page 126, where the rule is stated |
| How well it held up       | Weak: one practitioner book states the rule, the library tested two pairs with opposite outcomes, and no independent study repeats it                                               |
| Also appears in           | [Dual Thrust](../dual-thrust-trading-algorithm/README.md), another rule that sets a breakout level before the day begins, on a fixed trigger rather than a moving one               |

## The idea in one paragraph

A breakout rule buys when a price rises above the highest level it has reached for a while, betting
that a move through that level means the price is going further. The trouble is that the right "a
while" depends on how jumpy the price is: in a quiet stretch a small move is meaningful, and in a
wild stretch the same move is just noise. This strategy makes the waiting time longer when the price
becomes jumpier and shorter when it calms down, so the level it must break is adjusted to the mood of
the market. It also adds a second condition, a band of recent prices, and closes the trade using an
average rather than a fixed stop.

## Why anyone believed it

Prices that have been climbing for months often keep climbing, because investors update their views
slowly and because money tends to follow money. This is the trend effect, and it is one of the best
documented patterns in markets: a study of two centuries of prices found trend-following returns
across commodities, currencies, share indices and bonds that were stable over time and overwhelmingly
unlikely to be chance.

If trends exist, then the start of a trend is worth catching, and the moment a price passes its recent
high is a natural place to look for one. The counterparty is the trader who sells into a rising
market because the price "looks high", and the hedger or exporter who must transact on a schedule
regardless of direction. Their steady selling can only absorb so much, and when a breakout exhausts
it, the price moves faster.

## An everyday comparison

Imagine the queue at a shop whose doorway sensor records the longest queue each day. On a normal
Saturday the queue rarely passes twenty people, so a queue of twenty-five is news and people join it.
On a sale weekend the queue often reaches forty, so twenty-five means nothing. If you always treated
twenty-five as a signal, you would misread the sale weekend badly. The strategy's move is to look at
how big the queues have been lately and raise its threshold during busy times, so only genuinely
unusual queues count. A second check confirms the crowding is real before acting.

## The rules, step by step

1. Choose one currency pair, for example EURUSD or GBPUSD, and obtain daily records of its high, low
   and closing price.
2. Start the waiting time, called the lookback, at 20 days. Each day, measure how jumpy the price is:
   take the standard deviation, which is the typical distance of the closes from their average, of the
   last 30 daily closing prices.
3. Compare today's jumpiness with yesterday's. If it rose, lengthen the lookback; if it fell, shorten
   it, in proportion to the size of the change. Keep the lookback between 20 and 60 days.
4. Draw two bands around the price. The middle of the band is the average of the last few closing
   prices, using the lookback as the length, and the upper and lower edges sit two standard deviations
   above and below the middle.
5. A buy setup needs two things on the same day: yesterday's close was above the upper band, and
   today's price is above the highest high reached over the lookback. A sell setup is the mirror: a
   close below the lower band and a price below the lowest low over the lookback.
6. Buy or sell short when the setup fires.
7. Close the trade when the price crosses to the other side of the average of the last lookback closes:
   a long closes if the price drops below that average, and a short closes if it rises above it.
8. Repeat every day.

## The maths, with every symbol named

The jumpiness and the lookback:

```text
vol = standard deviation of the last 30 daily closing prices
delta = (vol_today - vol_yesterday) / vol_today
lookback_today = the whole number nearest lookback_yesterday * (1 + delta), held between 20 and 60
```

- `vol` is the standard deviation of the last 30 closing prices, a plain measure of how jumpy the
  price has been.
- `delta` is the fractional change in jumpiness from one day to the next; a positive `delta` means the
  price is getting jumpier.
- `lookback` is the number of recent days the rule looks at. It grows when jumpiness grows, so the
  level the price must break is set further from the recent range and fewer false signals pass.

The bands:

```text
middle = average of the last N closing prices
upper  = middle + k * spread
lower  = middle - k * spread
```

- `N` is the lookback from the step above.
- `spread` is the standard deviation of those N closing prices.
- `k` is how many standard deviations wide the band is; the rule uses `k = 2`.

The entry and exit conditions:

```text
buy  when close_yesterday > upper  and  price_today > highest high of the last N days
sell when close_yesterday < lower  and  price_today < lowest low  of the last N days
exit a long when price_today < average of the last N closing prices
exit a short when price_today > average of the last N closing prices
```

The page uses an exponentially weighted average for the middle of the band and a plain average for the
exit, which means the two averages are not the same number even though both use the lookback length.

The cost of a completed trade:

```text
Cost = t * c
```

- `t` is the value traded, counting the entry and the exit, divided by the account value; a full
  round trip on a fully invested position gives `t = 2.0`.
- `c` is the cost of one trade as a fraction of the amount traded, covering the gap between the buying
  and selling price. One basis point, that is 0.0001, is a cautious figure for the most heavily traded
  currency pairs; one basis point is one hundredth of one percent.

## A worked example

The numbers are invented, and the lookback is shrunk to a handful of days so the arithmetic can be done
by hand; the real rule uses 20 to 60. First the lookback update. If the recent jumpiness rose from
0.0050 to 0.0060 and yesterday's lookback was 20, then:

```text
delta = (0.0060 - 0.0050) / 0.0060 = 0.1667
lookback_today = 20 * (1 + 0.1667) = 23.33, rounded to 23
```

23 sits inside the allowed range, so it becomes the new lookback. A rising jumpiness lengthens the
lookback, which pushes the breakout level further away and demands a stronger move to trigger.

Now the band, using six days where the first five closes are all 1.1000 and the sixth is 1.1020:

| Day | Close  | Distance from average | Squared distance |
| --- | ------ | --------------------- | ---------------- |
| 1   | 1.1000 | -0.000333             | 0.00000011       |
| 2   | 1.1000 | -0.000333             | 0.00000011       |
| 3   | 1.1000 | -0.000333             | 0.00000011       |
| 4   | 1.1000 | -0.000333             | 0.00000011       |
| 5   | 1.1000 | -0.000333             | 0.00000011       |
| 6   | 1.1020 | +0.001667             | 0.00000278       |

The average is 1.100333 and the standard deviation is 0.000745, so:

```text
upper = 1.100333 + 2 * 0.000745 = 1.101824
```

Day 6's close of 1.1020 is above 1.101824, so the first condition holds. The highest close in the
window is 1.1020, and suppose day 7's price is 1.1030, which is above it, so the second condition holds
too. The rule buys at 1.1030. Suppose the price then runs up and pulls back over the next days:

| Day | Close  |
| --- | ------ |
| 8   | 1.1050 |
| 9   | 1.1070 |
| 10  | 1.1090 |
| 11  | 1.1080 |
| 12  | 1.1070 |
| 13  | 1.1060 |

The exit is checked against the average of the last six closes. On day 13 that average is
(1.1050 + 1.1070 + 1.1090 + 1.1080 + 1.1070 + 1.1060) / 6 = 1.107000, and the price of 1.1060 is below
it, so the trade closes at 1.1060:

```text
Return before costs = 1.1060 / 1.1030 - 1 = +0.272 percent
t = 2.0  (one entry and one exit on a fully invested position)
Cost = 2.0 * 0.0001 = 0.0002, that is 0.02 percent
Net return for the trade = 0.272 - 0.02 = +0.25 percent
```

Notice that the trade gave back part of its best price before exiting, because the exit waits for the
price to cross an average rather than a fixed level. A year of the published rule made only a couple
of percent, so a single trade of this size repeated a handful of times is the order of magnitude
involved, and many of those trades lose.

## What the research actually found

| Source                                                             | What it measured                                                           | Result                                                                                                                                                                                                                                                                       |
| ------------------------------------------------------------------ | -------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| QuantConnect tutorial, its own tests                               | EURUSD and GBPUSD, six years of daily data                                 | EURUSD returned about 2.3 percent a year with a reward-for-risk figure of 0.31 and a worst fall near 14 percent, mostly in May to December 2015; GBPUSD returned a negative amount a year with a fall near 19 percent. The page concludes it works best in a trending market |
| Pruitt, Hill and Russak, Building Winning Trading Systems          | The book the rule comes from, page 126                                     | States the adaptive rule as a practitioner's system; it gives no independent measurement of profit net of costs                                                                                                                                                              |
| Lemperiere, Deremble, Seager, Potters and Bouchaud (`1404.3274v1`) | Trend following across commodities, currencies, indices and bonds, to 1800 | The trend effect is one of the most statistically significant patterns in markets, with a reliability score near 5 since 1960 and near 10 since 1800 after removing the general upward drift; long trends show no decay, though shorter trends have weakened considerably    |
| This repository's volatility brief (`2402.01354v2`)                | How persistent volatility is, on energy futures                            | Volatility's persistence drifts over time rather than holding still, so a rule that tunes itself to a fixed estimate of jumpiness is tuning to a moving target                                                                                                               |
| This repository's predictability brief (`2206.12282v1`)            | Indicator-only rules on American index shares, 2015 to 2021                | Plain technical rules won under half their trades, and the best parameters found by a search differed for every index and were driven by a few outsized trades, which is the classic sign of fitting noise                                                                   |

The pattern is a well-documented family effect with a poorly documented member. That trends exist, and
have existed for a long time, is strongly supported. That this particular adaptive breakout rule earns
anything after costs is supported by one book and one library test that succeeded on one pair and
failed on another. The grade is Weak, resting on that single, mixed test and the absence of any
independent replication of this exact rule.

## How this project relates to it

This repository has a design document for exactly the kind of machinery this rule uses,
[Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md). It catalogues the
volatility bands, the channel of recent highs, and the stops that appear here, and it takes the
important step this tutorial's rule does not: it puts the whole directional overlay behind a gate that
can refuse to trade at all, and it insists a volatility-scaled stop must prove itself on data it was
not fitted to. Reading it shows how much of a rule like this is risk machinery rather than edge.

The teaching manual [Intraday and medium-frequency systematic trading](../../../docs/usermanauls/intraday-systematic/README.md)
is where a reader learns to turn a rule like this into a tested program and to read the result
honestly. The volatility facts behind the adaptive lookback are collected in the
[volatility and microstructure noise brief](../../../strategies/books2/14_volatility_and_microstructure_noise.md),
and the warning about fitting indicator parameters is in the
[predictability brief](../../../strategies/books2/08_predictability_and_trading_strategies.md).

## Where it goes wrong

- Breakouts fail in choppy markets. A price that pokes above a level and falls back costs the entry
  and the exit for nothing, called a whipsaw. The library's own EURUSD run lost its worst 14 percent
  in a single stretch where a long trend reversed.
- The best price is given back. Because the exit waits for the price to cross an average, the trade
  surrenders the top of the move before closing. The worked example shows a quarter of a percent won
  where the price had earlier touched a higher level.
- The adaptive settings are themselves guesses. Which window measures jumpiness, whether to use
  prices or returns, the constant `k`, and the 20-to-60 range are all choices, and trying several and
  keeping the best is how a rule is fitted to its own past. The library page itself suggests a return
  series would measure jumpiness better than a price series, which is an admission the measure is
  arbitrary.
- The jumpiness estimate lags. Volatility's persistence drifts, so a rule that lengthens its lookback
  after a jumpy day is reacting to something that may already be over, entering late and exiting late.
- One instrument, no diversification. The result depends entirely on the one pair chosen, and the two
  pairs the library tested had opposite outcomes. A rule with one leg has no cushion.
- Costs on every round trip. Spreads widen exactly when the price is moving fast, which is when the
  rule trades, so the one-basis-point assumption is optimistic at the worst moment.

## Try it yourself

You need a spreadsheet and daily high, low and close prices for one currency pair, for about a year.

1. Build columns for the date, the high, the low and the close.
2. Add a column for the average close over the last five days, and one for the standard deviation of
   those five closes.
3. Add an upper band column: the average plus twice the standard deviation, and a lower band: the
   average minus twice it.
4. Add a column for the highest close over the last five days, and one for the lowest.
5. Mark a buy on any day where the previous day's close is above the upper band and today's close is
   above the five-day high. Mark a sell on the mirror case.
6. On paper, hold each trade until the close crosses the five-day average against you, and subtract
   one basis point on each side.
7. Then change the window from five days to ten and repeat.

What to notice: the number of trades collapses as the window lengthens, and the days that produced the
wins change. A rule whose results depend that much on one setting has not been shown to work; it has
been shown to have a favourite setting, which is a different and much weaker claim.

## Where this came from

- [QuantConnect strategy library: the Dynamic Breakout II strategy](https://www.quantconnect.com/tutorials/strategy-library/the-dynamic-breakout-ii-strategy),
  the rules as implemented, the volatility-driven lookback, the Bollinger-band condition, the exit
  rule and the two pair tests.
- Pruitt, Hill and Russak, [Building Winning Trading Systems](https://www.wiley.com/WileyCDA/WileyTitle/productCd-1118168275.html),
  page 126, where the adaptive breakout rule is stated.
- Robert Miner, [High Probability Trading Strategies](https://www.amazon.com/High-Probability-Trading-Strategies-Tactics/dp/0470181664)
  (2008), the second book the library page cites for entry and exit tactics.
- `1404.3274v1`, Two centuries of trend following, the long-horizon evidence that trends are real as a
  family effect.
- `2402.01354v2`, on drifting volatility persistence, cited through
  [Volatility and microstructure noise](../../../strategies/books2/14_volatility_and_microstructure_noise.md).
- `2206.12282v1`, on technical indicator rules and parameter fitting, cited through
  [Predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md).
- [Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md), this
  repository's design for volatility bands, channels and the gate that can refuse a trade.

## Words used in this tutorial

- Bollinger band: a band drawn two standard deviations above and below a moving average of prices.
- breakout: a trade that opens when a price passes a level it has not exceeded for a chosen time.
- channel: the range between the highest recent price and the lowest recent price, sometimes called a
  Donchian channel.
- exponential moving average: an average that puts more weight on recent prices than on older ones.
- lookback: the number of recent days a rule examines.
- volatility: how much a price moves around its average.
- whipsaw: a price that pokes past a rule's level and immediately falls back, costing the entry and
  the exit for nothing.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
