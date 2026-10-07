# Adding to a position as the price moves against you

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                           |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | A cryptocurrency priced in a stablecoin, on five-minute candles; one of the two source files also bets on falling prices through a futures-style contract                                                                                                                                                       |
| How often it trades       | Both source files buy on dips, so often; this page is about what happens when a second purchase is made on the way down                                                                                                                                                                                         |
| What you need             | Nothing but this page, or a spreadsheet to follow the average-entry arithmetic                                                                                                                                                                                                                                  |
| Where the rules come from | [futures/FReinforcedStrategy.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/futures/FReinforcedStrategy.py) and [berlinguyinca/ReinforcedQuickie.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/ReinforcedQuickie.py) |
| The underlying research   | [Polson and Witte, A Bellman View of Jesse Livermore](https://arxiv.org/abs/1407.2642), which derives the opposite habit, and [Almgren and Chriss, Optimal execution of portfolio transactions](https://www.risk.net/journal-risk/2161150/optimal-execution-portfolio-transactions)                             |
| How well it held up       | Weak: neither source file adds to a position at all and neither reports a measurement, and the one careful treatment found here derives the reverse rule                                                                                                                                                        |
| Also appears in           | Nothing else in this collection; [A profit target that falls the longer a trade is held](../roi-target-ladder/README.md), in this same group, covers the selling side of the same trade                                                                                                                         |

## The idea in one paragraph

You buy a coin, the price falls, and instead of selling you buy more at the lower price. The second
purchase lowers the average price you paid, so the coin has to rise less before the position climbs
back to even. This page sets out that mechanism, its arithmetic, and what it costs. The two source
files behind it buy dips in an uptrend, so they produce repeated buy signals on the way down, and a
second purchase on the same coin is what turns those signals into a larger and cheaper position. The
attraction is real: adding at lower prices moves the break-even price down. The danger is just as
real: the amount of money exposed grows with every add, and if the fall continues there is no
recovery to catch.

## Why anyone believed it

The person who adds to a losing position believes the fall is a temporary disagreement, not a change
in the coin. If the price has fallen because one large holder had to sell, or because the whole market
sneezed, the coin may be worth what it was worth yesterday, and buying it ten percent cheaper is a
better version of the decision that was made at the higher price. The arithmetic supports the feeling:
each add lowers the average, so the position needs a smaller rally to get back to where it started.

The counterparty is the forced seller described above: a leveraged holder being closed out, an
exchange's automatic selling after a liquidation, or someone who simply panics. Their selling pushes
the price below what the asset is worth for a short stretch. A rule that does not depend on being
right about the timing, and only on the price coming back, can collect that stretch.

## An everyday comparison

A greengrocer buys a crate of apples for a hundred pounds. The next week the same crate is ninety
pounds, so the greengrocer buys a second one, and the week after a third at eighty. The average price
paid for the three crates is now below ninety, so the apples can be sold at a profit at a lower price
than if only the first crate had been bought. What the average does not say is that three times as
much money is now tied up in apples, that they rot, and that if the price keeps falling the shop has
three crates of a shrinking asset. Whether the second crate was a good idea depends entirely on
whether the price recovers, which is the part the average entry price hides.

## The rules, step by step

1. Decide what one unit of a purchase is, in money terms, and decide the price steps at which you
   will add, such as each further ten percent fall from your average.
2. Decide how much to add at each step. The same money each time is the mildest form; doubling each
   time, the betting system called a martingale, makes the tenth purchase five hundred and twelve
   times the first.
3. Set a maximum total amount, and stop adding when it is reached. Without this limit the plan has no
   floor, and the position size is decided by the falling price rather than by you.
4. Buy the first amount, and record the units bought, the price paid and the fee paid. The fee
   matters: it is part of the cost of the position, not a separate expense.
5. On every later candle, compute the average entry price as the total cost divided by the total
   units. That number, not the first purchase price, is what the position has to beat.
6. If the price has fallen past the next step and the total is still below the maximum, buy the
   chosen amount and recompute the average.
7. When the position is ahead, reduce it in the same spirit: sell part and keep the rest, so the
   money already earned is no longer at risk. Selling half at one target and the rest at a later one
   is called scaling out.
8. Two rules bound the whole thing: a stop loss, the price at which the position is closed whatever
   the plan says, and a maximum total, the money beyond which the position must not grow. Averaging
   down without a limit is a bet whose size is chosen by the market.
9. What the two source files actually encode is narrower than the mechanism above: neither of them
   adds to a position. The word "reinforced" here means an extra, slower confirmation before buying,
   not a second purchase. Both build a one-hour chart from the five-minute candles and require the
   price to be above that slow average before they buy; the adding itself is left to the person or
   the program running them.
10. In the Freqtrade bot, adding to an open position is a separate feature called position
    adjustment, off by default, and the documentation warns that every adjustment is another order
    with another fee. The older switch that let a backtest hold more than one position in the same
    coin is not reproducible in live trading.

The two files, stated exactly. FReinforcedStrategy trades five-minute candles and can bet in both
directions: it buys when the close is above the fifty-period average of a one-hour chart and the
eight-period average crosses above the twenty-one-period average, and bets on a fall when both point
the other way, with a fourteen-period ADX below thirty closing either side. Its profit ladder is five
percent at once, ten percent after thirty minutes and seven point five percent after sixty, and its
stop loss is five percent. ReinforcedQuickie buys a dip below the lower Bollinger band at a
twelve-period low, or a V-shaped turn after a fall, and requires the one-hour average to be below the
price and rising; it sells at one percent profit with a five percent stop loss. Its author calls the
idea buying only in an upward-tending market, and the sibling reinforced file calls itself a proof of
concept that "doesn't really perfom that well".

## The maths, with every symbol named

Every add changes two numbers: the cost of the position and its average entry price.

```text
C = sum over all purchases of (units_i * price_i * (1 + c))
U = sum over all purchases of units_i
A = C / U
```

- `C` is the total cost of the position, including the fees paid to buy.
- `units_i` is the number of coins bought in purchase `i`, and `price_i` is the price paid.
- `c` is the fee as a fraction of the amount traded, 0.0010 for 0.10 percent.
- `U` is the total number of coins held.
- `A` is the average entry price. It is the price the position must beat, not the first price paid.

The progress of the position at a current price `P` is then:

```text
value = U * P * (1 - c)
loss_money = value - C
loss_fraction = (value - C) / C
```

- `P` is the price now.
- The `(1 - c)` is the fee charged when the position is sold, which is why a position is a little
  behind even at the average entry price.
- `loss_money` is what would be lost in money if the position were closed now.
- `loss_fraction` is that loss as a share of the money that was put in.

Two numbers show what adding has done. The break-even price is the price at which `value` equals
`C`, which is close to `A`; the money at risk is `C`, which grows with every add. Adding always moves
the break-even price down and the money at risk up, and that trade-off is the whole mechanism; if the
price never returns to the break-even price, the money is what decides the outcome.

## A worked example

A position in a coin we call CCC. The fee is 0.10 percent on each side. The plan is to add after
falls, with a maximum of four units.

| Step             | Price paid | Units bought | Cost including fee | Total units | Total cost | Average entry |
| ---------------- | ---------- | ------------ | ------------------ | ----------- | ---------- | ------------- |
| First purchase   | 100.00     | 1            | 100.10             | 1           | 100.10     | 100.1000      |
| Add after a fall | 90.00      | 1            | 90.09              | 2           | 190.19     | 95.0950       |
| Add after a fall | 80.00      | 2            | 160.16             | 4           | 350.35     | 87.5875       |

The third row is the full calculation: two units at 80.00 cost 160.00, the fee is 0.10 percent of
that, 0.16, so the row adds 160.16 to the cost and 2 to the units. The total cost becomes 190.19 +
160.16 = 350.35, the units become 4, and the average is 350.35 / 4 = 87.5875. At that moment the
position is worth 4 * 80.00 = 320.00 before the exit fee, so it is 320.00 - 350.35 = -30.35 behind,
which is -8.66 percent. Without the third purchase the position would have been one unit worth 80.00
against a cost of 100.10, a loss of 20.10, which is -20.08 percent.

Now the part that the average entry price hides. The same two positions, priced at a range of falling
prices, before the exit fee:

| Price | Four units: value | Loss in money | Loss as a share | One unit: value | Loss in money | Loss as a share |
| ----- | ----------------- | ------------- | --------------- | --------------- | ------------- | --------------- |
| 80.00 | 320.00            | -30.35        | -8.66 percent   | 80.00           | -20.10        | -20.08 percent  |
| 70.00 | 280.00            | -70.35        | -20.08 percent  | 70.00           | -30.10        | -30.07 percent  |
| 60.00 | 240.00            | -110.35       | -31.50 percent  | 60.00           | -40.10        | -40.06 percent  |
| 50.00 | 200.00            | -150.35       | -42.91 percent  | 50.00           | -50.10        | -50.05 percent  |
| 40.00 | 160.00            | -190.35       | -54.33 percent  | 40.00           | -60.10        | -60.04 percent  |

Read the table in two directions. Across a row, the averaged position is always the more comfortable
one in percentage terms, and the pattern never reverses: at every price above zero the three extra
purchases make the loss look smaller. Down the money columns, the averaged position loses more money
at every price, and the gap grows with the fall: at 50.00 it has lost 150.35 against 50.10, three
times as much. The percentage improved because the total grew, which is the same trick as measuring a
loss against a larger and larger number.

The break-even price makes the attraction concrete. With four units the position needs a price of
350.35 / 4 = 87.5875 to be level before the exit fee, so from 80.00 it needs a rise of 87.5875 / 80.00
- 1 = 9.48 percent. The single-unit position needs a rise of 100.10 / 80.00 - 1 = 25.13 percent to
reach its break-even. The averaged position is closer to recovery and further from safety at the same
time.

Reducing as the price moves for you works the same way in reverse. Suppose the price rallies to
100.00 and half the position is sold:

| Action         | Units | Price  | Proceeds after fee | Cost of those units | Result   |
| -------------- | ----- | ------ | ------------------ | ------------------- | -------- |
| Sell half      | 2     | 100.00 | 199.80             | 175.1750            | +24.6250 |
| Sell the rest  | 2     | 80.00  | 159.84             | 175.1750            | -15.3350 |
| Total for four | 4     |        | 359.64             | 350.35              | +9.29    |

The half sold after the rally costs 2 * 87.5875 = 175.175 of the total, and brings back 199.80, so it
banks 24.625. If the remaining half is later sold at 80.00 it loses 15.335, and the whole position
still finishes ahead by 9.29, which is 9.29 / 350.35 = 2.65 percent. Holding all four units until the
price came back to 80.00 instead would have produced a loss of about 8.75 percent, because the whole
four units would be sold for 320.00 less the exit fee. Selling all four at 100.00 would have made
49.25, about 14.06 percent. The early reduction gave up about eleven points of a further rise in
exchange for protection against the fall.

## What the research actually found

Neither source file reports a measurement: no backtest result, no period, no fees, no comparison. The
honest report is therefore about what has been measured elsewhere, and the nearest careful treatment
points the other way. Polson and Witte rebuild the trading rules of Jesse Livermore, published in
1923, inside Bellman's framework for choosing the best action step by step. One of their remarks is
that because the market's information is better than the trader's, the best action when the price is
rising is to do nothing, and when it is falling it is to think about selling: in Livermore's phrase,
one should fear, not hope. Adding to a position that has moved against you is the opposite of that
derived policy, and the fall should update the estimate, not be defended against.

The second measurement is about cost. Almgren and Chriss worked out the best way to buy or sell a
large amount over time, and showed that the problem has two opposite forces: trading quickly moves
the price against you because your own orders consume the available supply, and trading slowly leaves
you exposed to the price moving while you wait. The optimal schedule is a balance between them, and
it depends on how risky the asset is and how much you dislike risk. Adding in pieces is therefore not
free execution: every piece is a separate order paying the fee and the gap between the buying and
selling price, and the plan is deliberately holding a losing position for longer, which is exactly
the risk that schedule is trying to price.

Two arithmetic facts complete the picture. Adding after a loss does not change the expected value of
the remaining money: if the second purchase was not worth making on its own terms, a lower price does
not repair the reason. What does change is the size of the bet, and the doubling version is famous
for turning a long run of small wins into a single total loss.

## How this project relates to it

This repository runs no trading bot, so it cannot reproduce an averaging rule. It does hold the two
quantitative briefs that bear on the decision.
[Market making and inventory](../../../strategies/books/04_market_making_and_inventory.md) is about a
trader whose whole problem is what to do with a position that is too large, and its central result is
that the price quoted must be shifted to discourage the position from growing further; a rule that
deliberately grows a position as it goes against you is the case such models are built to penalise.
[Optimal execution and liquidation](../../../strategies/books/01_execution_and_liquidation.md) is the
brief that contains the Almgren-Chriss trade-off in full, including the closed-form schedule and the
case where the asset does not move as the model assumes. The thresholds in the futures file were
chosen by a search over ranges rather than fixed by argument, and
[overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md)
is this repository's account of what a search of that kind is worth.

## Where it goes wrong

- The percentage flatters and the money does not. Every add improves the loss percentage and worsens
  the loss in money, as the table above shows. A plan that is judged by the percentage is a plan that
  can lose three times as much and look better while doing it.
- Once the maximum is spent, the plan freezes. If the price keeps falling after the last add, the
  average can no longer be improved and the full loss lands at once.
- Borrowed money ends the story. The futures file can bet on falling prices with borrowed money, and
  on a leveraged position an adverse move is closed automatically when the deposit is gone. Averaging
  down into a liquidating position is how a loss becomes a total loss.
- Averaging down against the signal's own logic. Both files buy dips inside an uptrend, so a second
  purchase is a larger bet that the uptrend is intact. When the trend has actually broken, the second
  purchase buys the break.
- Costs compound. Each add pays the fee and crosses the spread, and the spread is widest when the
  market is falling, which is exactly when the adds happen.
- The names and the backtest mislead. Neither file averages down, the sibling "reinforced" file calls
  itself a proof of concept, and the switch that lets a backtest hold more than one position in the
  same coin is not reproducible in live trading, so such a backtest is not a forecast of the live
  program.

## Try it yourself

You need a spreadsheet and nothing else.

1. Make columns `step`, `price paid`, `units bought`, `fee`, `cost`, `total units`, `total cost` and
   `average entry`.
2. Put the first purchase in row one: one unit at 100.00, with a fee column of 0.10 percent of the
   amount, and fill the running totals.
3. Add rows for a second purchase of one unit at 90.00, then two units at 80.00, then four units at
   70.00, each time updating the running totals and recomputing the average as total cost divided by
   total units.
4. In a second block, list prices from 100.00 down to 30.00 in a column, and beside each one compute
   the value of the final position, the loss in money and the loss as a share of the total cost.
5. In a third block do the same for a single unit bought at 100.00, and put the two loss columns side
   by side, together with the break-even price of each position and the rise each would need from the
   lowest price.

What to notice: the share-loss column for the averaged position is always the smaller of the two,
and yet the money-loss column is always the larger, and it keeps growing as the price falls. Then
look at the last purchase and ask what the plan would have done at the next ten percent fall, and at
the one after that. The answer is the reason a maximum total is part of the rules and not an
afterthought.

## Where this came from

- [futures/FReinforcedStrategy.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/futures/FReinforcedStrategy.py)
  and [berlinguyinca/ReinforcedQuickie.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/ReinforcedQuickie.py),
  the two files whose entry conditions, profit ladders and stop losses are stated above, together with
  the search ranges the futures file leaves open.
- [The Freqtrade strategy callbacks documentation](https://www.freqtrade.io/en/stable/strategy-callbacks/),
  which describes position adjustment, the switch that turns a repeated signal into an added position,
  the fee on every adjustment, and the warning that a position-stacking backtest cannot be reproduced
  in live trading.
- Nick Polson and Jan Hendrik Witte, [A Bellman View of Jesse Livermore](https://arxiv.org/abs/1407.2642),
  arXiv 1407.2642 (2014), for the derived rule that a position moving against you should be reduced,
  not increased.
- Robert Almgren and Neil Chriss,
  [Optimal execution of portfolio transactions](https://www.risk.net/journal-risk/2161150/optimal-execution-portfolio-transactions),
  Journal of Risk 3(2), pages 5 to 39 (2000), for the cost of buying in pieces.
- [Market making and inventory](../../../strategies/books/04_market_making_and_inventory.md),
  [Optimal execution and liquidation](../../../strategies/books/01_execution_and_liquidation.md) and
  [Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's own briefs.

## Words used in this tutorial

- average entry price: the total cost of a position divided by the number of units held, which is the
  price the position has to beat.
- break-even price: the price at which a position is worth what was paid for it.
- leverage: using borrowed money so that a given price move produces a larger gain or loss.
- liquidation: the automatic closing of a leveraged position when its deposit is exhausted.
- spread: the gap between the best buying price and the best selling price, which is paid on each
  round trip.
- stop loss: the fixed loss at which a trade is closed, here five percent in both source files.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
