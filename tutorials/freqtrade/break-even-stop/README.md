# The break-even stop: closing a trade at the price you paid, and what that costs

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                      |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Nothing. It opens no positions; it manages positions that are already open, closing each one at a small profit or at break even                                            |
| How often it trades       | Continuously, on five-minute candles, until every open position has been closed                                                                                            |
| What you need             | Nothing but this page and a pencil; the arithmetic is one profit percentage plus the fees                                                                                  |
| Where the rules come from | [BreakEven.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/BreakEven.py)                                                              |
| The underlying research   | None, this is a practitioner's rule of thumb; the file's author describes it as a way to clear a bot's open positions, not as a source of return                           |
| How well it held up       | Weak: no measurement is published, the file chooses no trades of its own, and the rule can be shown on paper to convert a winning trade into a smaller win or a small loss |
| Also appears in           | Nothing else in this collection                                                                                                                                            |

## The idea in one paragraph

Sometimes a person has already bought something and wants to be out of it, but not by selling at a
loss. This file is a rule for that situation. It is not a strategy that decides what to buy; it opens
no positions at all. It watches the positions that are already open and closes each one as soon as it
is slightly ahead, or as soon as it has climbed back to the price that was paid. Positions that are
still falling are held, up to a hard stop five percent below the entry, and are closed the moment they
recover to break even. The name describes the main act: waiting for a losing position to come back to
the entry price and leaving at zero rather than at a loss.

## Why anyone believed it

The appeal is emotional and it is real. A person who wants to stop trading does not want to end the
day with a position that is down, because that position keeps moving while it is unattended. Closing
everything at once would lock in the loss. Waiting for each position to recover means the account
finishes flat instead of behind. The rule turns a loss that is happening now into a zero that may
happen later.

There is a second, more technical reason the rule is written as a profit ladder rather than a single
number. A trade that is still open after ten minutes has not moved the way the entry expected, so the
file lowers the profit it asks for as the clock runs, from one percent at the start to zero after ten
minutes. That is the same falling-ladder idea as the sibling tutorial's
[profit target](../roi-target-ladder/README.md), used here to sweep up positions that are only just
ahead.

The counterparty is whoever buys the shares or coins that this rule sells. The rule sells into small
rises, which is a service to buyers who want to enter. The cost of that service is paid by the person
running the rule, and it is the point of the next two sections.

## An everyday comparison

Think of a person holding a concert ticket they no longer want. They paid 100 for it. There are two
ways to be rid of it: sell now for whatever the market offers, or put it back on sale at 100 and wait.
Waiting feels better because it avoids a loss on paper, but it has two hidden costs. The ticket may
never sell at 100 and the person is left holding it when the concert starts, worth nothing. And while
waiting, the person who would have paid 90 for it has bought from someone else, so the chance to
recover part of the money is gone. Selling at 100 is not free either: the ticket site takes a fee, so
the seller who "broke even" is a little behind.

## The rules, step by step

1. Run this file against positions that are already open. It generates no buy and no sell signals, so
   on an empty account it does nothing at all.
2. Set the profit ladder to one percent from the start and zero after ten minutes. So a position that
   is one percent ahead within the first ten minutes is closed; after ten minutes, any position that
   is not behind is closed.
3. Set the stop at five percent below the entry. A position that falls that far is closed, whatever
   the clock says.
4. Re-check every five-minute candle, and close a position the moment its current gain reaches the
   target in force at that moment.
5. Expect the outcome for each position to be one of three: a small gain if it rises early, about zero
   if it recovers later, or a five percent loss if it falls first and keeps falling.
6. Know that the ladder in the file can be replaced by the bot's configuration file, and that the file
   itself shows two more aggressive settings in comments: a ladder of zero, which closes anything not
   behind, and a ladder of minus one, which is the same as telling the bot to sell everything at once.

The one sentence to hold on to: the rule never lets a trade get worse while it is ahead, but it also
never lets a trade get better once it is ahead, and it does nothing at all for a trade that falls.

## The maths, with every symbol named

The rule is a comparison between the trade's current gain and a target that depends on the time held.

```text
gain = current_price / entry_price - 1
```

- `gain` is the profit on the position so far, as a fraction; 0.01 means one percent.
- `entry_price` is the price that was paid when the position was opened.
- `current_price` is the price now.

The target at a given time is read from the ladder: one percent before ten minutes have passed, zero
after.

```text
exit if gain >= target_for_time_held
```

The stop is a separate comparison, and it does not depend on time:

```text
stop if gain <= -0.05
```

- `-0.05` is the five percent loss the file is willing to take.

Finally, the net result after costs, which is where the name becomes misleading:

```text
net_gain = gain - fee_open - fee_close - spread_cost
```

- `fee_open` and `fee_close` are the exchange fees on each side, 0.001 each at a tenth of a percent.
- `spread_cost` is the cost of crossing the gap between the best buy and best sell price on each side,
  taken as 0.0005 each way.

So a trade that exits at a gain of exactly zero comes out about 0.30 percent behind, not at zero.

## A worked example

Everything below is invented but of a realistic size. The position was bought at 100.00 dollars, the
fee is 0.10 percent of the amount on each side, and crossing the gap between prices costs 0.05 percent
each side, so a full round trip costs about 0.30 percent.

| Trade | Time held | Price now | Gain   | Target in force | Decision               | Gross  | Net    |
| ----- | --------- | --------- | ------ | --------------- | ---------------------- | ------ | ------ |
| A     | 3 min     | 101.20    | +1.20% | 1%              | sell, target reached   | +1.20% | +0.90% |
| B     | 4 min     | 99.40     | -0.60% | 1%              | hold                   |        |        |
| B     | 7 min     | 100.70    | +0.70% | 1%              | hold, below target     |        |        |
| B     | 12 min    | 100.10    | +0.10% | 0%              | sell, any gain will do | +0.10% | -0.20% |
| C     | 5 min     | 95.00     | -5.00% | 1%              | sell, stop reached     | -5.00% | -5.30% |

Trade A did what the rule wants: it caught a quick rise and left with a small gain. Trade B is the
case the rule is named for, and it is the one to study. The position fell to a small loss, the rule
held it, and twelve minutes later it was fractionally ahead, so the rule closed it. The headline
number is "break even", plus 0.10 percent, but after fees and the gap between prices the account is
0.20 percent behind. Trade C fell straight through the stop, and the loss was five percent of the
position plus costs. Read the three together: the rule converted a small gain into a smaller gain, a
recovery into a small loss, and a larger loss into a smaller one.

Now the expectation, which is the honest part. Suppose the four outcomes below occur equally often,
which keeps the arithmetic short rather than describing any real market.

| Case as time passes                          | If the position is left alone | If the break-even rule is applied |
| -------------------------------------------- | ----------------------------- | --------------------------------- |
| Runs on to +8 percent                        | +8.00%                        | +1.00%                            |
| Rises to +1 percent, fades                   | -1.00%                        | +1.00%                            |
| Dips to -3 percent, recovers to +0.2 percent | +0.20%                        | +0.20%                            |
| Falls to -8 percent                          | -8.00%                        | -5.00%                            |
| Average of the four                          | -0.20%                        | -0.70%                            |

The average is worse with the rule, not better, and only because the first row is capped. The rule
cuts the large loss in the last row, which is its purpose, and it cuts the large gain in the first
row, which is its cost. Which of those is larger is not something the rule can know; it depends
entirely on whether big winners or big losers are more common in the market being traded. The one
case where the rule clearly helps is the second row, where a small rise that would have faded is
taken before it fades. Change the numbers and the answer changes sign, which is the whole problem
with a rule that has no measurement behind it.

## What the research actually found

Nothing has been measured for this file. It ships no backtest result, it was written by its author as
a housekeeping tool, and the repository's own README calls its strategies starting points rather than
strategies to trade. The file's comment says as much: the author wants to stop the bot quickly without
leaving positions open.

The reason a break-even stop is treated with caution rather than enthusiasm is a property of any rule
that cuts a distribution in half. In this repository's own review of
[risk measures and drawdowns](../../../strategies/books2/22_risk_measures_and_drawdowns.md), the
consequence of a path-dependent exit is that it changes which prices you experience without changing
the average of the prices themselves. A stop can make a series look smoother and win more often while
leaving the average the same or worse. That review is about risk measurement generally, not about
this file, and it is the closest thing to evidence that exists here.

## How this project relates to it

The closest material in this repository is
[Risk boundaries and stops](../../project/risk-boundaries-and-stops/README.md), which puts a
break-even style of exit next to fixed stops and a trailing stop and argues the same point made above:
a stop reshapes the outcomes rather than creating a return, and it should be reported with the win
rate and both tails beside it. That tutorial is a design document for an engine in this repository,
not a result about crypto pairs, and it says so.

[Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md) lists break-even
exits in the catalogue of formulas it reviewed, and keeps them in the restricted group that may only
be switched on when a market state admits a bet. The restriction is the well-considered answer to the
question this file asks: a rule that only closes positions is a cost model, not a reason to trade.

## Where it goes wrong

- The name overstates the result. After fees and the gap between prices, a break-even exit is a small
  loss, so the account does not finish flat even when every position comes back to its entry price.
- It caps the winners. The trade that would have kept rising is closed at the first one percent, and
  the profit it would have made is the price paid for the safety on the losing side.
- It holds losers waiting for a recovery that may not come. A position that keeps falling is held
  until it is five percent down, and the recovery to break even is never guaranteed.
- It has no measurement. No sample, no costs, no comparison with simply selling everything at once,
  and therefore no way to know whether the rule helped over any real period.
- It can be gamed by its own ladder. Because the target falls to zero after ten minutes, a position
  that is fractionally ahead at the eleventh minute is closed, even though a rising price might carry
  it to the first target a minute later.

## Try it yourself

You need nothing but a spreadsheet and a public price history; a free charting site will give you
daily or hourly closing prices.

1. Pick one price series and a starting date, and write the entry price in a cell.
2. Add one row per period with the price, and a column for the gain, being the price divided by the
   entry price minus one.
3. Add a column for the target: one percent for the first ten minutes of holding, zero afterwards.
4. Add a column that marks the first row where the gain reaches the target, and another that marks the
   first row where the gain falls to minus five percent. Whichever is marked first is how the trade
   ends.
5. Subtract 0.30 percent from the end gain for the round trip, and write the net result.
6. Repeat for twenty different starting dates, then count how often each of the three outcomes occurs:
   an early small win, a recovery to about zero, and a five percent stop.

What to notice: the pattern of outcomes is not one thing. On a rising series, market, most trades end
in the early small win and the stop is never touched, so the rule looks harmless and the costs are the
main cost. On a falling market the stop is hit constantly and the small wins vanish. Do the same
twenty starts leaving the position alone until a fixed end date, without the rule, and compare the
two columns. That comparison, not the rule's name, is the answer to whether it helped.

## Where this came from

- [BreakEven.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/BreakEven.py),
  the file whose rules, ladder and stated purpose are described above.
- The freqtrade documentation on
  [minimal ROI](https://www.freqtrade.io/en/stable/strategy-customization/), which states that the
  ladder's key is the minutes held and its value the profit required.
- [Risk boundaries and stops](../../project/risk-boundaries-and-stops/README.md) and
  [Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md), this
  repository's own account of why a stop reshapes outcomes rather than adding return.
- [Risk measures, tails and drawdowns](../../../strategies/books2/22_risk_measures_and_drawdowns.md),
  the closest local evidence on path-dependent measures.

## Words used in this tutorial

- break even: being neither ahead nor behind, before the costs of trading are counted.
- profit ladder: a set of profit targets that starts high and falls as a trade is held longer.
- stop loss: an instruction that closes a position once its price reaches a chosen level, limiting the loss.
- round trip: opening a position and later closing it, which pays the costs of trading twice.
- spread: the gap between the best price at which you can buy and the best price at which you can sell.
- entry price: the price paid when a position is opened.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
