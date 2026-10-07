# The volatility system: a bet whose size is set by how much the price has been moving

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                             |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Crypto coin futures contracts on a crypto exchange, in both directions, with two dollars of position for every dollar put up                                                                                                                                                                      |
| How often it trades       | A few times a week on one-hour candles; the signal is measured on three-hour blocks                                                                                                                                                                                                               |
| What you need             | A spreadsheet and a column of high, low and closing prices                                                                                                                                                                                                                                        |
| Where the rules come from | [VolatilitySystem.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/futures/VolatilitySystem.py), the file in the Freqtrade community strategy repository                                                                                                      |
| The underlying research   | Richard Bookstaber's volatility system, as described in the [TradingView entry](https://www.tradingview.com/script/3hhs0XbR/) the file cites; the sizing idea is measured in Harvey and others, [The Impact of Volatility Targeting](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=3175538) |
| How well it held up       | Weak: no result is published for this file, the author of the rule it copies writes that the rule has no stop loss and no profit target and that current noise destroys it on small timeframes, and the measured evidence for volatility sizing covers shares and credit rather than crypto       |
| Also appears in           | Nothing else in this collection                                                                                                                                                                                                                                                                   |

## The idea in one paragraph

This strategy does not try to guess the direction of the market. It measures how large a three-hour
move in the price usually is, doubles that figure, and takes a position whenever the last three hours
moved further than that, in the direction the price moved. A move bigger than usual is treated as a
sign that something is happening, and the bet is that it continues. The position is held until the
same test fires in the opposite direction, which closes the trade and opens the other one. The size
of the bet is tied to the money the bot offers rather than to the size of the move, but two extra
settings matter: the first entry uses only half the offered money, at most one later signal doubles
it, and the whole position is twice the size of the money put up, because the leverage is set to two.

## Why anyone believed it

Markets move in bursts. A quiet week is followed by a violent one, and the violence is not random:
it comes from a piece of news, a forced liquidation, or a crowd of orders arriving together, and it
takes time to be worked through. If a move is already larger than the recent average, the odds shift
toward more of the same, because the reason for the move is still in the market.

The counterparty is whoever is caught on the other side of that burst: a seller whose stop loss has
just been triggered into a falling market, a leveraged position being closed by an exchange, or a
buyer who has to buy today no matter the price. Those forced orders keep the burst going, which is
what a rule that buys or sells on an unusually large move is trying to collect. The rule also needs
the rise to be real rather than momentary, which is why the move is measured over three hours rather
than one.

## An everyday comparison

Think of a commuter deciding how much luggage to carry. On a calm day the train is on time and the
walk from the platform is easy, so a light bag and a quick step are enough to catch the connection.
On a day when the timetable has collapsed the same journey takes unpredictable amounts of time, so
the sensible response is to carry less, leave earlier, and plan for a longer walk. Nothing about the
destination changed; only the amount of uncertainty did, and the preparation is scaled to the
uncertainty. Volatility sizing does the same to a bet: when the market is wild, hold less of it.

## The rules, step by step

1. Use candles covering one hour of trading. Combine every three of them into one three-hour block:
   the block's closing price is the close of the third hour, its highest price is the highest of the
   three, and its lowest price is the lowest of the three.
2. For each block compute the true range: the largest of the block's high minus its low, the block's
   high minus the previous block's close, and the previous block's close minus the block's low. The
   last two terms are written without a sign.
3. Average the last fourteen true ranges. That average is the average true range. Multiply it by two.
4. Compute the price change of the block: the block's close minus the previous block's close.
5. Take a long position, meaning owning the contract in the hope that the price rises, when the price
   change is larger than the doubled average true range of the previous block. Take a short position,
   meaning selling it in the hope that the price falls, when the price change is smaller than minus
   that same figure.
6. Hold the position until a signal fires the other way. Because the exit rules are the entry rules
   read in reverse, the opposite signal closes the trade and opens the new one at the same price and
   the same moment.
7. Size the first entry at half of whatever stake the bot offers. If the same signal appears again on
   a new block while the trade is open, add the same amount again, but only once: the bot is told to
   stop after two successful entries.
8. Use two times leverage. Every dollar of stake controls two dollars of the contract, so both the
   gain and the loss on the position are twice the move of the price.
9. Note what is missing. The stop loss is set to minus one, that is minus 100 percent, so no stop
   loss ever fires, and the profit target is set to 100, that is 10000 percent, so the profit target
   never fires either. Only the opposite signal closes a trade.

## The maths, with every symbol named

The rule is built on the average true range, whose purpose is to state how far the price typically
travels in one block, using the high and low as well as the close.

```text
TR_t = max(high_t - low_t, |high_t - close_(t-1)|, |low_t - close_(t-1)|)
ATR_t = (TR_t + TR_(t-1) + ... + TR_(t-13)) / 14
threshold_t = 2 * ATR_(t-1)
```

- `TR_t` is the true range of block `t`: the largest of three gaps, one inside the block and two
  against the previous close. The two vertical bars mean "without a sign".
- `ATR_t` is the average true range: the plain average of the last fourteen true ranges.
- `threshold_t` is the hurdle for a signal on block `t`. It is the doubled average from the block
  before, because the current block's average is not complete until the block has closed. The file
  writes this as a one-block shift.
- The factor of 2 is a choice, not a measurement; the file multiplies the average by two.

The signals:

```text
change_t = close_t - close_(t-1)
enter long   when change_t > threshold_t
enter short  when -change_t > threshold_t
exit long    when the short signal fires
exit short   when the long signal fires
```

- `change_t` is the price change of the block.
- `-change_t` is that change with its sign reversed, so the short test fires on falls and the long
  test on rises.
- Both tests use the same hurdle, so a fall of the same size as a rise is treated exactly the same
  way; the strategy has no built-in opinion about direction.

The size of the position, which is what makes this a volatility rule rather than a directional one:

```text
value_t = account * target / volatility
```

- `value_t` is the money the position controls.
- `account` is the money in the account.
- `target` is the level of movement the account is willing to live with, written as a fraction.
- `volatility` is an estimate of the market's recent movement, written as a fraction.
- The formula says the same thing in words: when the market's movement doubles, the position halves.
  That is volatility targeting. It is what Harvey and others call scaling by volatility, and it is
  the reason two very different markets can be compared at all: a plain return figure just tells you
  which market moved more, while a return measured at a fixed level of movement tells you which
  market paid better for the risk taken.
- The file itself does not implement this formula in full. It ties the size to the bot's stake and to
  the manual settings described in the rules: half the offered stake first, at most one doubling of
  the position, and leverage of two. The volatility only decides when to enter, not how much.

## A worked example

Six three-hour blocks. The doubled average true range is given for each block as the value the
strategy had already computed from the previous fourteen blocks; the price changes are the ones a
crypto contract can produce in three hours.

| Block | Close  | Change | Doubled ATR of this block | Hurdle in force (previous block) | Test        | Result     |
| ----- | ------ | ------ | ------------------------- | -------------------------------- | ----------- | ---------- |
| 1     | 100.00 |        | 1.40                      |                                  |             |            |
| 2     | 101.50 | +1.50  | 1.45                      | 1.40                             | 1.50 > 1.40 | buy        |
| 3     | 100.60 | -0.90  | 1.50                      | 1.45                             | 0.90 > 1.45 | nothing    |
| 4     | 98.40  | -2.20  | 1.60                      | 1.50                             | 2.20 > 1.50 | sell short |
| 5     | 97.30  | -1.10  | 1.70                      | 1.60                             | 1.10 > 1.60 | nothing    |
| 6     | 96.10  | -1.20  | 1.80                      | 1.70                             | 1.20 > 1.70 | nothing    |

At block 2 the price rose 1.50, which beats the 1.40 hurdle, so the strategy buys at 101.50. At
block 4 the price fell 2.20, which beats the 1.50 hurdle, so the strategy sells short at 98.40, and
because the exit rules are the entry rules in reverse, that same sale also closes the long position.

Two trades, then. The long went from 101.50 to 98.40:

```text
Long gross = 98.40 / 101.50 - 1 = -0.0305, that is -3.05 percent
```

The short went from 98.40 to the block 6 close of 96.10, where the example stops:

```text
Short gross = 1 - 96.10 / 98.40 = +0.0234, that is +2.34 percent
```

Now the money. Suppose the bot offers a stake of 1,000 units of currency and the strategy takes half
of it, 500, with leverage of two, so the position controls 1,000:

```text
Long loss in money  = -3.05 percent * 1,000 = -30.50
Long fee            = 2 sides * 0.075 percent * 1,000 = -1.50
Long net            = -32.00, which is -6.40 percent of the 500 put up
Short gain in money = +2.34 percent * 1,000 = +23.40
Short fee           = 2 sides * 0.075 percent * 1,000 = -1.50
Short net           = +21.90, which is +4.38 percent of the 500 put up
```

Three things stand out. First, the leverage doubles the effect of every price move, so a 3 percent
adverse move costs more than 6 percent of the money committed. Second, a fee of 0.075 percent per
side costs 1.50 on a 1,000 position, which is 0.3 percent of the 500 actually committed: the same
trading rule looks cheaper the larger the stake, which is why small accounts suffer more. Third, the
example happens to end ahead, and that means nothing at all; it is a demonstration of the arithmetic,
not a result.

## What the research actually found

Nothing was measured for this file. The repository publishes no backtest, no trade count and no
result for `VolatilitySystem.py`. The entry the file cites does make a claim about the rule, and it
is a discouraging one: its author writes that the rule yields good results on some very specific
charts, that it has no stop loss and no profit target, and that current noise levels destroy it,
especially on short timeframes. That is one practitioner's opinion about a chart, not a measurement.

Where measurement does exist is for the sizing half of the idea. Harvey, Hoyle, Korgaonkar, Rattray,
Sargaison and Van Hemert studied volatility targeting across more than 60 assets with data starting
as early as 1926, holding each position so that its movement was the same fraction of the account at
all times, with a target of 10 percent a year. For American shares over 1927 to 2017 the reward per
unit of risk improved from 0.40 without scaling to between 0.48 and 0.51 with it, the variation of
the volatility itself fell from about 4.6 percent to between about 1.7 and 2.2 percent, and the
average of the worst 1 percent of months improved from about -11.4 percent to between -8.3 and -10.1
percent. For bonds, currencies and commodities the effect on the reward per unit of risk was
negligible. Moreira and Muir found the same improvement for a set of share strategies. Both results
concern shares and credit, not crypto, and both measure the sizing decision rather than the entry
rule used here.

## How this project relates to it

This repository implements the average true range the rule is built on, in code with tests around
it, at [crates/indicators/src/volatility/atr.rs](../../../crates/indicators/src/volatility/atr.rs).
A reader who wants to see the true range written precisely, including the treatment of the first bar
where no previous close exists, should start there. The wider question of what market noise does to
a rule like this one is covered by this repository's own brief,
[strategies/books/06_volatility_and_microstructure_noise.md](../../../strategies/books/06_volatility_and_microstructure_noise.md),
which collects the measurements on how much of a short-horizon price move is signal and how much is
the mechanics of the market.

## Where it goes wrong

- No stop loss and no profit target. The file sets the stop loss to minus 100 percent and the target
  to 10000 percent, so between entry and the next opposite signal the position runs fully exposed.
  A single large adverse move, or a liquidation at two times leverage, ends the account rather than
  the trade.
- The exit is the entry of the opposite bet, not a judgement that the first bet was wrong. After a
  violent drop and a violent partial recovery, the position flips repeatedly in a choppy market, and
  each flip pays the fee and the gap between the buying and selling price.
- Two times leverage turns ordinary noise into large account moves. The example above loses 6.4
  percent of the money committed on a 3 percent price move, and crypto contracts can move further
  than that in a single three-hour block.
- The hurdle is a fitted number. The rule as published allows the multiple of the average to be
  chosen; this file doubles it, and nothing published says two is better than one and a half or
  three.
- Funding costs on perpetual futures are not in the chart. A position held through a funding payment
  pays that payment, which is charged every few hours and can exceed the fee on the trade.
- The comparison across markets is only as good as the volatility estimate. If the estimate is
  computed over the whole history rather than the recent past, the position is sized on information
  that was not available at the time, which is a form of looking into the future and is the subject
  of the [look-ahead bias tutorial](../look-ahead-bias/README.md) in this group.

## Try it yourself

You need a spreadsheet and sixty days of daily high, low and closing prices for any cryptocurrency
or share.

1. Column A is the date, columns B, C and D are the high, the low and the close.
2. Column E is the true range: `=MAX(B2-C2, ABS(B2-D1), ABS(C2-D1))`.
3. Column F is the fourteen-day average of column E: `=AVERAGE(E2:E15)` once twelve prior rows exist.
4. Column G is the doubled average: `=2*F2`.
5. Column H is the price change: `=D2-D1`.
6. Column I is the signal: `=IF(H2>G1, "buy", IF(-H2>G1, "sell", ""))`, which uses the previous
   day's doubled average, exactly as the file does.
7. Column J is the size you would have been able to take, under a rule you choose: for example
   `=1000/F2`, which gives a position that moves by 1,000 currency units for every move of one
   average true range.

What to notice: in the calm stretches column J gives a large position and in the wild stretches a
small one, while column I fires just as readily in both. That is the arithmetic of volatility
targeting, and it is also why the same rule produces very different account swings depending on
which stretch of history it is applied to. Run the sheet on two different coins and compare the two
columns of signals before you compare the two columns of sizes.

## Where this came from

- [VolatilitySystem.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/futures/VolatilitySystem.py),
  the rules as implemented: the three-hour blocks, the doubled average true range, the signals, the
  half stake, the single top-up, the leverage of two, and the absent stop loss and profit target.
- [Volatility System on TradingView](https://www.tradingview.com/script/3hhs0XbR/), the entry the
  file cites, which attributes the rule to Richard Bookstaber in 1984 and states the author's own
  doubts about it.
- Harvey, Hoyle, Korgaonkar, Rattray, Sargaison and Van Hemert,
  [The Impact of Volatility Targeting](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=3175538),
  the measurement of what scaling positions by volatility does across more than 60 assets from 1926.
- Moreira and Muir, [Volatility-Managed Portfolios](https://onlinelibrary.wiley.com/doi/10.1111/jofi.12513),
  the paper that first reported higher reward per unit of risk from scaling share exposure down when
  recent movement was high.
- Welles Wilder, New Concepts in Technical Trading Systems (1978), the book that introduced the true
  range and the average true range used here.

## Words used in this tutorial

- average true range: the average size of a price move over a set number of bars, counting the gap
  from the previous close as well as the move inside the bar.
- leverage: borrowing so that a position is larger than the money put up, which multiplies both the
  gain and the loss.
- liquidation: the exchange closing a leveraged position because the money backing it has run out.
- long: owning something, so that a rise in its price is a gain.
- short: selling something borrowed, so that a fall in its price is a gain.
- stake: the amount of the account committed to one trade.
- volatility: how much a price moves around its average, usually written as a percentage.
- volatility targeting: choosing the size of a position so that its expected movement is a fixed
  fraction of the account, whatever the market is doing.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
