# Tiny profits many times over: the scalping files and the cost that eats them

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                     |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Crypto pairs on a crypto exchange, such as Bitcoin or a smaller coin priced in United States dollars                                                                                                                                                                      |
| How often it trades       | Very often; the scalp file watches one-minute candles and is written to hold many positions at once                                                                                                                                                                       |
| What you need             | A spreadsheet and one day of one-minute prices for a single pair                                                                                                                                                                                                          |
| Where the rules come from | [SmoothScalp.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/SmoothScalp.py) and [SmoothOperator.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/SmoothOperator.py) |
| The underlying research   | None; both files are a practitioner's rule of thumb, and the scalp claim rests on the idea that most fast price moves are noise rather than information                                                                                                                   |
| How well it held up       | Weak: neither file reports a measurement, the smoothing file's own comment says not to use it, and the small targets are close to the size of a realistic round-trip cost                                                                                                 |
| Also appears in           | [The ROI ladder](../roi-target-ladder/README.md), which explains the profit target these files set, and [the smallest strategies](../minimal-examples/README.md), whose scalp file follows the same idea                                                                  |

## The idea in one paragraph

A scalping rule tries to behave like a printing press: make a very small profit, then do it again and
again. `SmoothScalp.py` buys a sharp dip on a one-minute chart and aims to sell one percent higher,
and it recommends holding at least sixty trades at once so the small wins arrive steadily enough to
cover the occasional loss. `SmoothOperator.py` is a slower cousin that combines several indicators
into one smoothed curve and tries to sell near the top of a bump rather than at a fixed target. Both
rest on the same premise: the many small moves in a fast market are mostly noise, and a rule that
takes a little from each one can add them up. The idea this page is really about is what the tiny
target costs.

## Why anyone believed it

Cryptocurrency trades around the clock on many exchanges, so there is always a market open, and on
one-minute candles the price is constantly making small jumps and small dips. Some of those dips are
caused by a single careless seller hitting the exchange, not by any change in the coin's prospects,
and the price often springs back within minutes. A rule that buys the dip and sells the spring can
argue that it is paid for providing the other side of somebody else's hurry.

The counterparty is that hurried seller. A trader forced to sell quickly, or one who panicked, or a
program that has to sell a fixed amount by a deadline, cannot wait for a good price and takes
whatever the market offers. The scalper is buying from them at a slight discount and selling back a
minute later. The belief is that this happens often enough and cheaply enough to matter.

## An everyday comparison

Think of a fruit stall beside a supermarket that closes at the same moment each evening. Just before
closing, the supermarket dumps crates of ripe fruit that will not keep, and the small stall buys them
for almost nothing, then sells them the next morning at a small markup. Each trade earns pennies.
The business works only if the buying price stays well below the selling price and if there are not
too many crates that spoil. If the gap between what the stall pays and what it charges becomes as
small as the gap the scalper needs to earn, the whole business stops making anything, and each sale
starts to lose a little even though every crate was sold.

## The rules, step by step

`SmoothScalp.py`, on one-minute candles:

1. Compute a five-candle exponential moving average of the high, of the low and of the close; a fast
   stochastic oscillator, whose two parts run from 0 to 100 and compare the closing price with the
   recent range; ADX, a number for how strong a trend is; a commodity channel index over fourteen
   candles, which swings above and below zero; a fourteen-candle RSI; and a money flow index, which
   is like an RSI that also uses trading volume.
2. Buy when all of these are true at once: the candle opened below the five-candle average of the
   lows, the ADX is above 30, the money flow index is below 30, both parts of the stochastic are
   below 30, the faster stochastic part has just crossed above the slower one, and the commodity
   channel index is below minus 150.
3. Sell when both of these are true: the candle opened at or above the five-candle average of the
   highs, or one of the stochastic parts just crossed above 70; and the commodity channel index is
   above plus 150.
4. Also sell when the trade's gain reaches one percent, and sell when the trade is 50 percent below
   its entry price.

`SmoothOperator.py`, on five-minute candles:

1. Compute a twenty-candle commodity channel index, a fourteen-candle RSI, ADX, a money flow index,
   and an eleven-candle smoothed copy of each of those three; also Bollinger bands over twenty
   candles at one point six standard deviations for entries, and a slowly varying band width.
2. Blend the smoothed RSI, money flow index and commodity channel index into one number, with the
   RSI and money flow index weighted slightly more, and then smooth that blend heavily.
3. Buy when one of three oversold patterns appears and the price is also higher than the previous
   candle. The three patterns are: a price that has been falling for four candles and turned up; a
   price below the middle Bollinger band with an extremely low commodity channel index and RSI; or
   an extremely low money flow index with the RSI below it.
4. Sell when the smoothed blend has peaked and turned down, or when there have been eight rising
   candles in a row, or when the commodity channel index is above plus 200 and the RSI above 70.
5. Also sell when the trade's gain reaches 10 percent, and sell when the trade is 5 percent below
   its entry price.

Two things in the files are worth stating exactly. `SmoothScalp.py` sets its stop loss at 50 percent,
which is enormous next to a one-percent target, so a single loss is worth about fifty wins. And
`SmoothOperator.py` carries the comment "DO NOT USE, just playing with smooting and graphs!" at the
top of the file, and its own note says it sells after 100 percent rather than the 10 percent the
settings actually state.

## The maths, with every symbol named

Every trade in these files is judged by one number, the gain, and then reduced by the cost of trading
it.

```text
gain = (price_now / entry_price) - 1
```

- `gain` is the trade's progress as a fraction: 0.01 means one percent.
- `price_now` is the current price of the pair.
- `entry_price` is the price at which it was bought.

The target is reached when `gain` is at least the target the file sets:

```text
sell when gain >= target
```

- `target` is the profit target as a fraction: 0.01 for `SmoothScalp.py`, 0.10 for its slower cousin.

The cost of one round trip, the pair of a buy and the matching sell:

```text
cost = spread + 2 * fee
```

- `spread` is the gap between the best price at which you can buy and the best price at which you
  can sell, paid once for each round trip, because you buy at the higher price and sell at the lower.
- `fee` is the exchange's charge per side as a fraction of the amount traded, so `2 * fee` covers the
  buy and the sell.

What reaches the account is the gain less the cost:

```text
net = gain - cost
```

- `net` is the result of the trade after costs, as a fraction. It can be negative even when `gain`
  is positive, and that is the case the example below is built to show.

The rule of thumb follows immediately: a trade only makes money when the gain is larger than the
round-trip cost, `gain > cost`. A target of one percent on a pair whose round trip costs one point
four percent fails that test on every winning trade.

## A worked example

Take a small, thinly traded coin pair, meaning one that few people trade, so the gap between its
buying and selling price is wide. Suppose the gap is 1.2 percent of the price and the exchange
charges 0.10 percent per side, so one round trip costs `1.2 + 2 * 0.1 = 1.4` percent. The file aims
for a one-percent gain. Ten trades, each of which reaches the target, each losing the same amount:

| Trade | Amount  | Gross gain before costs | Spread paid | Fees paid | Net    |
| ----- | ------- | ----------------------- | ----------- | --------- | ------ |
| 1     | 1,000.0 | +10.00                  | -12.00      | -2.00     | -4.00  |
| 2     | 1,000.0 | +10.00                  | -12.00      | -2.00     | -4.00  |
| 3     | 1,000.0 | +10.00                  | -12.00      | -2.00     | -4.00  |
| 4     | 1,000.0 | +10.00                  | -12.00      | -2.00     | -4.00  |
| 5     | 1,000.0 | +10.00                  | -12.00      | -2.00     | -4.00  |
| 6     | 1,000.0 | +10.00                  | -12.00      | -2.00     | -4.00  |
| 7     | 1,000.0 | +10.00                  | -12.00      | -2.00     | -4.00  |
| 8     | 1,000.0 | +10.00                  | -12.00      | -2.00     | -4.00  |
| 9     | 1,000.0 | +10.00                  | -12.00      | -2.00     | -4.00  |
| 10    | 1,000.0 | +10.00                  | -12.00      | -2.00     | -4.00  |
| Total |         | +100.00                 | -120.00     | -20.00    | -40.00 |

Every trade won, meaning the price rose one percent each time, and the account still ends forty
units down, a loss of four percent of the thousand it began with. The gain and the cost use the same
arithmetic at the top of each column: `1,000 * 0.01 = 10.00` of gain, `1,000 * 0.012 = 12.00` of
spread and `1,000 * 0.001 = 1.00` on each side of fees.

Now run the same ten trades on a liquid pair, meaning one that many people trade, so its gap is
narrow. Suppose the gap is 0.02 percent and the fee is still 0.10 percent per side, so a round trip
costs `0.02 + 0.2 = 0.22` percent.

| Pair   | Gross gain per trade | Round-trip cost | Net per trade | Net over ten |
| ------ | -------------------- | --------------- | ------------- | ------------ |
| Thin   | +10.00               | -14.00          | -4.00         | -40.00       |
| Liquid | +10.00               | -2.20           | +7.80         | +78.00       |

The two rows are the same rule on the same ten winning moves, and they land on opposite sides of
zero. The only difference is the pair. Two further facts complete the picture. First, the scalp
file's stop loss is 50 percent, so one losing trade at `1,000 * 0.50 = 500.00` cancels about sixty
four of the liquid-pair wins at `7.80` each. Second, the exit signal can fire before the target, so
many real wins are smaller than one percent and the thin-pair loss grows larger, not smaller. This
example ignores compounding between trades and treats the result as a simple sum; the point it makes
does not depend on that choice.

## What the research actually found

No study is published for either file, so there is no measured result to report. What can be said is
about the arithmetic and the file's own words.

The scalp premise is that fast moves are mostly noise, and that is a claim about market
microstructure, the study of how orders are matched and how prices move tick by tick. The repository
brief on market impact and trading cost explains that the costs a fast strategy pays, the spread and
the effect of your own orders on the price, are exactly the quantities that decide whether such a
rule can earn anything after the fact
([strategies/books/02_market_impact_and_trading_cost.md](../../../strategies/books/02_market_impact_and_trading_cost.md)).
The two files here never measure those costs; they set a target and leave the cost to the reader.

The repository's own README states that the files "mostly should serve as a starting point for your
own strategies, not as ready to use strategies", and that results "heavily depend on the pairs,
timeframe and timerange used to backtest". `SmoothOperator.py` adds its own warning, "DO NOT USE",
which is as direct a statement of confidence as a file can make.

What would settle the question is a single comparison on one set of entries: close them at the
one-percent target, count the realised spread and fees on the same pairs and dates, and report the
net figure. That comparison does not exist here.

## How this project relates to it

This repository has no Freqtrade engine and no code that runs these scalp rules, so nothing here
implements them. What it does have is the arithmetic of why they are hard.
[Costs, fees and taxes](../../foundations/06_costs-fees-and-taxes.md) builds the same small-example
the section above uses, showing a strategy that trades twenty times a year paying four percent in
costs and turning a four percent gross year into nothing. A scalp rule trades far more often than
twenty times a year, so the same arithmetic is applied many more times.

The wider question, whether a fast price move carries information or is only noise, is the subject of
this repository's brief on order flow and toxicity
([strategies/books/07_order_flow_and_toxicity.md](../../../strategies/books/07_order_flow_and_toxicity.md)),
which studies who is trading and whether their orders predict the next move. The consequence for a
coin's price when somebody must trade regardless of price is covered in the brief on crypto trading
venues ([strategies/books/13_crypto_amm_and_perpetuals.md](../../../strategies/books/13_crypto_amm_and_perpetuals.md)).

## Where it goes wrong

- The target can be smaller than the cost. The worked example is the whole failure mode: on a thinly
  traded pair the round trip costs more than the one-percent gain, so every win is a small loss.
- Costs scale with trades while the target does not. Holding sixty positions open means sixty round
  trips paid, so a rule that fires constantly pays its costs constantly.
- The stop loss dwarfs the target. A 50 percent stop against a one-percent target means one loss
  erases dozens of wins, so the rule needs an unusually high win rate just to break even.
- The exit can fire early. The stochastic exit closes a trade before the target whenever either
  stochastic part crosses 70, so the average win is smaller than the file's stated one percent.
- The smoothing file is untested by its own author. Its header says not to use it, and it mixes
  seven indicators into one curve that few readers could reproduce by hand.
- The gap widens when it matters. The pairs where these rules fire most are often the thin ones,
  which are also the pairs with the widest buying-and-selling gap, so the cheapest-looking trades
  are the most expensive to complete.

## Try it yourself

You need a spreadsheet and one day of one-minute prices for a single crypto pair from any public
chart. The exercise is to find the pair's round-trip cost and compare it with a one-percent target.

1. Find the best buying price and the best selling price for the pair at one moment, and write them
   down. Their gap, divided by the middle price, is the spread as a fraction.
2. Add twice the exchange's fee per side to the spread. That is the round-trip cost.
3. Build a column of one-minute closing prices for the day.
4. Add a column that subtracts each price from the next, to see the size of the typical one-minute
   move.
5. Add a column that marks every minute in which the price is at least one percent above the price
   twenty minutes earlier.
6. On a spare row, write the round-trip cost and the number 0.01 side by side.

What to notice: on a liquid pair the cost is a small fraction of one percent, and the one-percent
target clears it comfortably; on a thin pair the cost can be one percent or more on its own, and the
target no longer clears it. The same rule, the same day and the same prices can be profitable on one
pair and lose money on the other, purely because of a gap you never chose.

## Where this came from

- [SmoothScalp.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/SmoothScalp.py),
  the one-minute scalp rule, its one-percent target, its 50 percent stop and its sixty-position
  suggestion.
- [SmoothOperator.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/SmoothOperator.py),
  the smoothed multi-indicator rule, its 10 percent target, its 5 percent stop and its own "DO NOT
  USE" warning.
- [The Freqtrade strategies README](https://github.com/freqtrade/freqtrade-strategies/blob/main/README.md),
  which states that the files are starting points and that results depend on the pairs, timeframe and
  period chosen.
- [strategies/books/02_market_impact_and_trading_cost.md](../../../strategies/books/02_market_impact_and_trading_cost.md),
  this repository's brief on the costs a fast strategy pays.
- [Costs, fees and taxes](../../foundations/06_costs-fees-and-taxes.md), this repository's own
  demonstration that a busy strategy pays its gross result away in costs.
- The spread of 0.02 percent and 1.2 percent, the fee of 0.10 percent, and the ten-trade tables are
  made up for teaching; the arithmetic is the only claim being made.

## Words used in this tutorial

- candle: the opening, highest, lowest and closing price of one fixed interval of trading.
- fee: the exchange's charge per trade, usually a small percentage of the amount traded, paid on each
  side.
- liquid: easy to buy and sell quickly without moving the price much; a liquid pair has a narrow gap
  between its buying and selling prices.
- market microstructure: the study of how orders are matched and how prices move from trade to trade.
- scalping: trading for very small profits very often.
- spread: the gap between the best price at which you can buy and the best price at which you can
  sell, which you pay on each round trip.
- stochastic oscillator: an indicator whose two parts run from 0 to 100 and compare the closing price
  with the recent range.
- stop loss: the fixed loss at which a trade is closed to limit the damage.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
