# Power Tower: comparing a price with an earlier price raised to a power

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                 |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Cryptocurrency pairs on a spot exchange, long only (it buys, and it never sells short)                                                                                                                                                                |
| How often it trades       | Almost constantly on a cheap coin: the rule fires whenever the price has not collapsed, which for a coin priced below one unit is nearly always                                                                                                       |
| What you need             | A spreadsheet with a power function, and a table of five-minute close prices                                                                                                                                                                          |
| Where the rules come from | [PowerTower.py in the freqtrade strategies repository](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/PowerTower.py)                                                                                                |
| The underlying research   | none, this is a practitioner's rule of thumb; the file says it is inspired by the Three Black Crows candlestick pattern, described in Nison's [Japanese Candlestick Charting Techniques](https://archive.org/details/japanesecandlest0000niso) (1991) |
| How well it held up       | Weak: a new rule with no published test, one author run over sixty-seven trades, and arithmetic that only fires for coins priced below one unit                                                                                                       |
| Also appears in           | nothing else in this collection                                                                                                                                                                                                                       |

## The idea in one paragraph

The rule does not use any indicator. Each five-minute candle it makes three comparisons between
today's price and an earlier price, where the earlier price has first been raised to the power 3.849
(or, on the way out, 3.798). A power is a number multiplied by itself a set number of times: a
power of 2 means squared, and a power of 3.849 is a little more than cubed. If all three comparisons
say the recent price is above the powered earlier price, the rule buys. The author says it is a new
rule inspired by the Three Black Crows candlestick shape, which is three falling candles in a row,
but with the idea turned around to hunt for coins that are rising. The trouble, shown in the
arithmetic below, is what raising a price to a power above one does to coins on either side of one
unit.

## Why anyone believed it

A run of three strong moves in the same direction is a striking sight, and the Three Black Crows
pattern uses exactly that: three falling candles in a row are read as a strong downward push. The
author's idea was to look for the same kind of push on the way up, and to measure "strong" not by
the size of a candle but by a mathematical comparison with an earlier price. When a coin is being
pushed up hard, the story goes, the buyers are still arriving, and a rule that detects the push
early rides it. The counterparty is the last buyer in the door, the one who sees the move after it
has happened and pays the top price; a rule that buys into strength is trying to be ahead of that
person rather than being that person.

## An everyday comparison

Imagine a competition in which each day's target is set by raising your previous day's score to a
power of about four. If yesterday you scored 0.5, today's target is about 0.07, and passing is
trivial. If yesterday you scored 2.0, today's target is about 14.4, and passing is close to
impossible. The rule is that competition, applied to prices. The name is apt: for a price below one
unit the bar is a tower so short you step over it, and for a price above one unit the bar is a tower
so tall you cannot reach the first floor. That single fact decides which coins the rule can ever
select.

## The rules, step by step

1. Collect five-minute candles for a cryptocurrency pair: for each candle, the open, the highest
   price, the lowest price and the close. The rule needs only the close.
2. For each candle, compute three comparison values, using a power of 3.849. Write the current close
   as `close[0]`, the previous close as `close[1]`, and so on:
3. First comparison: is `close[0]` greater than `close[2]` raised to the power 3.849?
4. Second comparison: is `close[1]` greater than `close[3]` raised to the power 3.849?
5. Third comparison: is `close[2]` greater than `close[4]` raised to the power 3.849?
6. Buy only when all three comparisons are true at the same candle.
7. Hold the position. Close it when any one of three opposite comparisons is true, using a power of
   3.798: is `close[0]` less than `close[2]` to the power 3.798, or `close[1]` less than `close[3]`
   to the power 3.798, or `close[2]` less than `close[4]` to the power 3.798? Because it takes only
   one of the three on the way out but all three on the way in, the exit is far easier to trigger
   than the entry.
8. Profit target ladder. Measured from the opening price: 21.3 percent at once, 4.8 percent after 39
   minutes, 2.9 percent after 56 minutes, and any profit at all after 159 minutes.
9. Loss limit. If the position is down 28.8 percent from the opening price, it is sold.
10. No trailing stop is used. The strategy needs thirty candles of history, looks once every five
    minutes, and cannot sell short, so it sits in cash between trades.

The two powers, 3.849 on the way in and 3.798 on the way out, are not fixed ideas; the file
declares them as search parameters that may take any value from 0 to 4, and the numbers shown are
the ones left in the file. The author's own comment in the file calls the rule completely new and
says it is much more effective than Three Black Crows, but no test accompanies that claim.

## The maths, with every symbol named

For a price `P` and a power `p`, `P^p` means `P` multiplied by itself in the way a power describes.
The three powers used here are close to four, so `P^3.849` is `P` raised almost to the fourth
power, which is a large number for any `P` above one and a very small number for any `P` below one.

The entry condition, written in full, is:

```text
close[0] > (close[2])^3.849
close[1] > (close[3])^3.849
close[2] > (close[4])^3.849
```

- `close[0]` is the closing price of the candle being tested, `close[1]` the one before it, and so
  on back to `close[4]`.
- `^` means "raised to the power of", so `(close[2])^3.849` is the close two candles ago, raised to
  the power 3.849.
- All three lines must be true at once for the rule to buy.

The exit condition, using the other power, is:

```text
close[0] < (close[2])^3.798   or
close[1] < (close[3])^3.798   or
close[2] < (close[4])^3.798
```

- Any one of the three being true closes the position.

What these formulas do to a price of different sizes is the whole story:

| Price `P` | `P^3.849` | Does `P > P^3.849` hold?                          |
| --------- | --------- | ------------------------------------------------- |
| 0.02      | 0.0000003 | yes, easily: the threshold is far below the price |
| 0.05      | 0.0000098 | yes, easily                                       |
| 0.50      | 0.069     | yes: 0.50 is above 0.069                          |
| 1.00      | 1.000     | no: the two sides are equal, so `>` fails         |
| 2.00      | 14.41     | no: 2.00 is far below its own fourth power        |
| 50.00     | 3,462,000 | no: hopelessly out of reach                       |

- `P` is any price in the pair's own units.
- The middle column is what the comparison demands the current price to exceed.

The table says the rule fires for cheap coins almost regardless of what the price is doing, and
cannot fire at all for coins priced above one unit, because a number above one raised to a power
above one is larger than the number itself. The profit ladder and the loss limit are step functions
of the elapsed time `T` in minutes, exactly as written in the file:

```text
required(T) = 0.213 if T < 39
              0.048 if 39 <= T < 56
              0.029 if 56 <= T < 159
              0.0   if T >= 159
stop price  = entry_price * (1 - 0.288)
```

- `T` is minutes since the position was opened.
- `required(T)` is the smallest gain that closes the position.
- `entry_price` is the price at which the position was opened, and `0.288` is the 28.8 percent loss
  limit.

The cost of a completed round trip is `2 * c`, where `c` is the charge per side as a fraction of the
amount traded. A realistic figure on a large crypto exchange is 0.0005 to 0.001, that is 0.05 to
0.10 percent per side, the level the cost primer in this collection uses.

## A worked example

Two coins, one cheap and one expensive, on the same five-minute candles. The cheap coin trades near
0.02 units, the kind of price in the pair list the author left in the file; the expensive coin
trades near 2.00 units.

| Candle | Cheap coin close | Cheap close to the power 3.849 | Expensive coin close | Expensive close to the power 3.849 |
| ------ | ---------------- | ------------------------------ | -------------------- | ---------------------------------- |
| 1      | 0.0195           | 0.000000260                    | 1.980                | 13.60                              |
| 2      | 0.0198           | 0.000000277                    | 1.985                | 13.78                              |
| 3      | 0.0200           | 0.000000289                    | 1.990                | 13.96                              |
| 4      | 0.0197           | 0.000000272                    | 1.995                | 14.14                              |
| 5      | 0.0201           | 0.000000295                    | 2.000                | 14.41                              |

For the cheap coin, testing candle 5 means checking 0.0201 against 0.000000289, 0.0197 against
0.000000272, and 0.0200 against 0.000000260. All three are true by an enormous margin, so the rule
buys. Notice that it would buy even if the price had fallen slightly, so the condition is barely a
condition at all for a coin at this price. For the expensive coin, testing candle 5 means checking
2.000 against 13.96, 1.995 against 14.14 and 1.990 against 13.60. None can be true, so the rule
never buys this coin no matter how fast it rises. That is the arithmetic consequence of the power,
and it is why the author's pair list is full of coins priced at a fraction of a unit.

Now follow one trade on the cheap coin. The rule fires on candle 5, and the position is opened at
the next candle's open, 0.0202. The price rises to 0.0210 once the position has been held for 56
minutes. The rung of the ladder at that point is 2.9 percent, and the gain is 3.96 percent, above
the rung, so the position is sold.

```text
Gross gain      = 0.0210 / 0.0202 - 1 = 0.0396, that is 3.96 percent
Round-trip cost = 2 * 0.001 = 0.002, that is 0.20 percent
Net gain        = 3.96 - 0.20 = 3.76 percent
```

The 28.8 percent loss limit was never reached. The example shows the arithmetic, not that the rule
earns anything. Two features deserve a second look. First, because the entry fires so easily on a
cheap coin, the rule will often be holding a position while the price falls, which is what the large
loss limit allows. Second, the exit fires when any one of three comparisons breaks, and for a cheap
coin those comparisons are also nearly always satisfied, so on paper the exit is just as loose as
the entry; the author's own note reports an average holding time of about eleven minutes, which is
two candles, consistent with a rule that buys and sells almost immediately.

## What the research actually found

There is no published study of this rule, and the file records only one author's search.

| Source                                    | What it measured                                                               | Result                                                                                                                                                                                                                                         |
| ----------------------------------------- | ------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The file's own comment                    | A search the author ran, reported as the thirty-eighth of one hundred settings | Sixty-seven trades with thirty-two wins, thirty-four draws and one loss, an average profit of 1.23 percent per trade, a total profit of 81.51 percent, an average holding time of 10 hours 58 minutes, and a search objective of minus 9.86920 |
| Nison (1991)                              | The Three Black Crows pattern the author cites as the inspiration              | A descriptive pattern from the Japanese candlestick tradition; the book itself does not test it against a sample with costs                                                                                                                    |
| The pair list the author left in the file | The coins the search was run over                                              | A long list of small coins with prices at a fraction of a unit, which is exactly the set the power comparison can fire on                                                                                                                      |

The one number that stands out is the loss count: one loss in sixty-seven trades. A rule that buys
nearly always on a cheap coin and sells nearly always on the next comparison will produce many small
wins and a few large losses, and the loss limit of 28.8 percent is the size of those losses. The
author's note is a single run of a search over one hundred settings; it is not a test a reader can
check, and it does not record the period, the fees, or what happened when the same numbers were run
on data the search did not see.

## How this project relates to it

The three-way relationship between a wide search, a chosen parameter set, and a result that looks
good is the subject of
[overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md).
The powers 3.849 and 3.798, and the four ladder rungs, are five free choices picked together, which
is the pattern that brief warns about. The cost that decides whether a rule trading every few
minutes survives is set out in
[market impact and trading cost](../../../strategies/books/02_market_impact_and_trading_cost.md).
The collection's primer
[how a backtest lies](../../foundations/07_how-a-backtest-lies.md)
explains, without code, why a run over one pair list and one period proves little.

## Where it goes wrong

- A price that has risen is observed in the data; that it will keep rising is a prediction, and this
  file offers no measurement of how often it came true.
- Four conditions on the same candles fit almost any past. Here three conditions use the same five
  closing prices, and the powers that make them fit were chosen by a search over the same data, so
  the rule was shaped to the sample it was tested on.
- The arithmetic is degenerate. For prices below one unit the entry is almost always true, and for
  prices above one unit it is almost never true, so the rule is really a filter that says "trade
  cheap coins", not a measure of strength. Any apparent edge may simply be the behaviour of the
  cheap coins in the sample.
- One loss in sixty-seven trades is not safety. A strategy that wins often and loses rarely can
  still lose a large share of an account on the rare occasion the loss limit is hit, and a single
  sample of sixty-seven trades cannot distinguish that from luck.
- The exit is nearly as permissive as the entry. Because the same power comparison is used on the
  way out, a cheap coin will tend to sell very soon after buying, which turns the strategy into one
  that pays the exchange fee and the gap between prices over and over.
- The coin list is a choice too. The author's list of cheap coins was selected along with everything
  else, and a coin that has since risen above one unit, or a pair that has stopped trading, changes
  both the trades and the record.

## Try it yourself

You need a spreadsheet with a power function and a table of five-minute closes for one coin pair.

1. Make columns: candle number, close, close two candles ago, close three candles ago, close four
   candles ago.
2. Add three columns that raise the last three prices to the power 3.849.
3. Add three columns that compare the current close with each powered value, giving true or false.
4. Add a signal column: buy when all three comparisons are true.
5. Add three more columns with the power 3.798 and the reversed comparisons, and a sell column that
   fires when any one is true.
6. Alongside, record each candle's close divided by one unit, so you can see at a glance whether the
   coin is above or below one.

What to notice: if the coin trades below one unit, almost every candle is a buy, and the sell column
is almost as busy, so the sheet says "trade constantly" rather than "trade when strength appears".
Change one coin for a pair priced above one unit and the buy column goes silent. That difference is
the power, not the market.

## Where this came from

- [PowerTower.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/PowerTower.py),
  the file that states the rules: the five-minute timeframe, the three entry comparisons with the
  power 3.849, the three exit comparisons with the power 3.798, the ladder, the loss limit and the
  author's own search note.
- Steve Nison, [Japanese Candlestick Charting Techniques](https://archive.org/details/japanesecandlest0000niso)
  (1991), the description of the Three Black Crows pattern the author names as the inspiration.
- [Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md)
  and [market impact and trading cost](../../../strategies/books/02_market_impact_and_trading_cost.md),
  this repository's studies of the two forces that decide this rule's fate.

## Words used in this tutorial

- candlestick: one period of trading drawn from its open, high, low and close prices.
- long only: buying and later selling what you own, never selling something you do not own.
- loss limit: a price below the entry at which the position is sold, also called a stop loss.
- power: a number multiplied by itself a stated number of times, so a power near four is close to
  the price times itself four times.
- search: running the same rule many times with different numbers and keeping the one that looked
  best on the past.
- Three Black Crows: a candlestick shape of three falling candles in a row, read as a strong
  downward push.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
