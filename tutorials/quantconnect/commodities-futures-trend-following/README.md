# Commodity trend following: buy what has been rising, sell what has been falling

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                     |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Futures contracts on raw materials: crude oil, natural gas, corn, wheat, sugar, live cattle and copper                                                                                    |
| How often it trades       | Once a month, and only when a contract's direction changes                                                                                                                                |
| What you need             | A spreadsheet                                                                                                                                                                             |
| Where the rules come from | [QuantConnect strategy library, commodities futures trend following](https://www.quantconnect.com/tutorials/strategy-library/commodities-futures-trend-following)                         |
| The underlying research   | Lemperiere, Deremble, Seager, Potters and Bouchaud, [Two Centuries of Trend Following](https://arxiv.org/abs/1404.3274)                                                                   |
| How well it held up       | Mixed: two hundred years of positive evidence with strong statistics, but the paper itself reports that shorter trends have withered, and the library page's own ten-year test lost money |
| Also appears in           | [Sector momentum](../sector-momentum/README.md) in this collection, the cross-sectional form of the same momentum idea                                                                    |

## The idea in one paragraph

Take seven raw materials, from crude oil to corn, each traded through a futures contract, which is an
agreement to buy or sell the material at a fixed price on a future date. Every month, compare each
contract's latest price with a slowly moving average of its own past prices. If the price is above
that average, buy the contract; if it is below, sell it, which means betting on a fall. Size each bet
so that every contract contributes roughly the same risk, by dividing the desired risk by how much
the contract usually moves. Hold the positions for a month and look again. The bet is that a price
which has been drifting in one direction tends to keep drifting that way for a while.

## Why anyone believed it

Trend following grew out of the commodity futures markets, and it grew out of insurance. A farmer
who plants a crop, or a mining company that will dig up copper later, does not know what the material
will fetch when it is ready. To remove that uncertainty the producer sells a futures contract today.
The buyer of that contract carries the price risk and is compensated for it, and the compensation is
built into the price the producer accepts. The first trend followers were simply firms that took the
other side of these hedges, and the economic story is that they earned a premium for insuring the
people who produce physical goods.

There is a second, modern reading of the same trade. A rule that buys when prices rise and sells when
they fall is long volatility: it does best when markets move a long way in either direction, and it
does well when share prices fall for months, because then it is short. That makes it a form of
insurance for an ordinary portfolio of shares, which is why funds selling this idea advertise it as
protection in a crash. The counterparty is the hedger who wants certainty, and the investor who
sells in a panic and pushes the fall further.

## An everyday comparison

Think of a bus route rather than a single bus. If buses on a route have been arriving later and later
every day, a commuter who plans for them to be late will be caught out less often than one who
assumes the timetable is exact. The commuter does not need to know why the buses are late; the fact
that they have been getting later is enough to be useful for a while. When the delays start to
shrink, the same rule flips and the commuter plans for them to be early. This strategy is that habit:
follow the direction of travel, adjust the size of the plan to how erratic the service has been, and
give it up when the direction changes.

## The rules, step by step

1. Choose seven liquid commodity futures: crude oil, natural gas, corn, wheat, sugar, live cattle and
   copper. The library page uses exactly these, and the underlying paper calls them a well balanced
   pool.
2. Collect the monthly closing price of each contract, for as long a history as you can find.
3. For each contract, compute a reference level: a moving average of its prices over the past five
   months, weighted so that recent months count more than older ones. This is an exponential moving
   average, and it turns slowly.
4. For each contract, compute a measure of how much it usually moves: a moving average, over the same
   five months, of the size of each monthly price change, ignoring the sign.
5. Compute the signal for each contract: the latest price minus the reference level, divided by the
   movement measure. A positive signal means the price is above its reference; a negative signal
   means below.
6. Decide the direction from the sign of the signal: buy if the signal is positive, sell short if it
   is negative.
7. Decide the size from the last monthly price change divided by the movement measure, given the
   direction's sign. A big recent move or a quiet market produces a bigger position; a big recent
   move in a choppy market produces a smaller one.
8. Trade at the start of each month and hold for the month. Allow each contract a leverage of up to
   three times its value, as the library page does.
9. Pay the cost of every change of direction, described below. When a contract rolls from one dated
   contract to the next, the old position must also be closed and reopened.

## The maths, with every symbol named

The signal is the distance of the price from its own slow average, measured in units of how much the
price typically moves.

```text
s(t) = (p(t) - EMA(t)) / sigma(t)
```

- `s(t)` is the signal at the start of month `t`, a plain number.
- `p(t)` is the latest monthly closing price.
- `EMA(t)` is the exponential moving average of past prices, the reference level.
- `sigma(t)` is the exponential moving average of the absolute monthly price changes, the size of a
  typical move.

The exponential moving average weights the newest price most and older prices less, with the weight
decaying by a fixed fraction each month. With a five-month averaging period the newest price gets a
weight of `2 / (5 + 1)`, that is one third, and the previous average gets the other two thirds:

```text
EMA(t) = a * p(t) + (1 - a) * EMA(t - 1)
```

- `a` is the weight of the newest price, equal to `2 / (n + 1)` for an `n`-month average.
- `EMA(t - 1)` is last month's reference level.

The size of the position is the direction times the last monthly change divided by the movement
measure:

```text
q(t) = sign(s(t)) * (p(t) - p(t - 1)) / sigma(t)
```

- `q(t)` is the number of contracts, before multiplying by the leverage and dividing by the contract
  multiplier.
- `sign(s(t))` is `+1` if the signal is positive and `-1` if it is negative.
- `p(t) - p(t - 1)` is the change in price over the last month.

The paper writes the position as one divided by the movement measure, `1 / sigma(t)`, so that every
contract contributes the same risk. The library implementation multiplies that by the last monthly
change, which grows the position when the trend is moving fast and shrinks it when the market is
quiet. Each month's contribution to the strategy, in units of its own risk, is the direction times
the next month's price change divided by the movement measure:

```text
contribution(t) = sign(s(t)) * (p(t + 1) - p(t)) / sigma(t)
```

The average of these contributions divided by their standard deviation, and multiplied by the square
root of twelve, is the yearly reward-to-risk figure that the sources quote.

## A worked example

One contract, copper, with monthly closing prices below. The averaging period is five months. The
reference level starts as the simple average of the first five prices, 101.8, and the movement
measure starts as the average of the first four absolute changes, 2.75.

| Month | Price | Change | EMA   | sigma | Signal | Direction | Next change | Contribution | Cost  |
| ----- | ----- | ------ | ----- | ----- | ------ | --------- | ----------- | ------------ | ----- |
| 1     | 100   |        |       |       |        |           | +2          |              |       |
| 2     | 102   | +2     |       |       |        |           | -3          |              |       |
| 3     | 99    | -3     |       |       |        |           | +4          |              |       |
| 4     | 103   | +4     |       |       |        |           | +2          |              |       |
| 5     | 105   | +2     | 101.8 | 2.75  |        |           | -1          |              |       |
| 6     | 104   | -1     | 102.5 | 2.17  | +0.68  | buy       | +4          | +1.85        | 0.019 |
| 7     | 108   | +4     | 104.4 | 2.78  | +1.31  | buy       | +2          | +0.72        |       |
| 8     | 110   | +2     | 106.2 | 2.52  | +1.49  | buy       | -4          | -1.59        |       |
| 9     | 106   | -4     | 106.2 | 3.01  | -0.05  | sell      | -2          | +0.66        | 0.038 |
| 10    | 104   | -2     | 105.4 | 2.68  | -0.54  | sell      | +2          | -0.75        |       |
| 11    | 106   | +2     | 106.6 | 3.45  | -0.18  | sell      |             |              |       |

Each contribution is the direction times the next month's change divided by the movement measure.
For month 6 it is `+1 * 4 / 2.17 = +1.85`; for month 8 it is `+1 * -4 / 2.52 = -1.59`. The five
contributions from months 6 to 10 add to `1.85 + 0.72 - 1.59 + 0.66 - 0.75 = +0.89`.

To turn this into money, choose a risk budget: suppose one unit of position is sized so that a
one-month move equal to `sigma` costs the account one percent of its value. Then each contribution of
1.00 is one percent of the account, and the five months made `+0.89` percent. The position changed
direction once, from buy to sell, which means closing the long and opening the short: two one-way
trades, plus the initial entry, three in all. The cost of a one-way trade is the cost rate `c`, about
0.05 percent, multiplied by the position's value. With the price near 106 and the movement measure
near 3, that is about `0.0005 * 106 / 3 = 0.018` percent of the account per one-way trade, so about
`0.053` percent for three trades:

```text
Net for five months = 0.89 - 0.05 = 0.84 percent of the account
```

Two things are worth noticing. First, the strategy lost money in month 8, when copper fell while the
signal was still positive; a trend rule is always late at the turning point. Second, of the five
months, only one direction change was needed, so the cost was small. That is the central attraction
of the monthly version, and it is also why the rule is slow: it needs a long move to pay for its
lateness.

## What the research actually found

| Source                                             | What it measured                                                   | Result                                                                                                                                                                                            |
| -------------------------------------------------- | ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Lemperiere, Deremble, Seager, Potters and Bouchaud | Four asset classes, futures from 1960 and spot prices back to 1800 | The trend signal earned excess returns with a statistical strength of about 10 standard errors over two hundred years, and positive returns in every decade, after removing the market's own rise |
| The same paper, on recent performance              | The last few years of the sample, and the short-horizon version    | No statistically detectable decay of long trends, but the three-day version had completely disappeared since about 2003                                                                           |
| Quantpedia, momentum effect in commodities         | Commodity futures, 1979 to 2004                                    | 14.6 percent a year, volatility 25.57 percent, worst fall 79.75 percent, reward-to-risk 0.57, for a twelve-month ranking held one month                                                           |
| QuantConnect's own implementation                  | The seven contracts, January 2010 to January 2020                  | Reward-to-risk of minus 0.131, against 0.805 for simply holding the American share index                                                                                                          |

The disagreement here is sharp and worth stating plainly. On two centuries of data the effect is one
of the most statistically solid patterns in finance, and the paper reports that the recent poor
performance of commodity trading advisers is consistent with ordinary chance. Yet the library page's
own ten-year test on the same rules lost money, and the commodity momentum entry reports a worst fall
of nearly eighty percent. The reconciliation is that trend following spends long stretches not
working: with a reward-to-risk below one, the paper notes, typical losing periods last two years and
four-year stretches are not unusual. A ten-year sample can easily be one of those stretches. The grade
is Mixed because the effect appears in the long samples and not in the recent one, not because the
long evidence is weak.

## How this project relates to it

This repository has a brief on commodity futures,
[strategies/books2/18_energy_and_commodities.md](../../../strategies/books2/18_energy_and_commodities.md).
Its Sections 6 and 7 report a commodity curve signal that is a close cousin of trend following: the
slope of the futures curve continued from one day to the next, earning 1.77 percent a year with
reward-to-risk 1.41 before costs, then 0.55 to 1.31 percent a year after three cost scenarios, with
the reward-to-risk falling from 1.47 before 2000 to 0.20 after 2009. Section 7 of that brief also
gives the cost realism this tutorial only sketches: futures costs differ by contract, so a single
cost number across a commodity book is not honest.

The second related piece is the repository's
[predictability brief](../../../strategies/books2/08_predictability_and_trading_strategies.md).
Its Section 3 reports that a trend-following rule's returns are positively skewed: the position is
largest after the strategy has already made money, so the rule wins less than half its trades and is
still worth measuring. That is the statistical shape behind the two-year losing stretches above, and
it is why a short sample of this strategy tells a reader almost nothing.

## Where it goes wrong

- Spent years. A reward-to-risk below one means the strategy can lose for years while still being
  what it claims to be. A reader who stops after two bad years may be stopping in the middle of the
  pattern, not at the end of it.
- At the turning point the rule is always late. It buys after the rise and sells after the fall, so
  every reversal costs it the first part of the new move. In a market that oscillates rather than
  trends, this is the whole story.
- Costs and rolls. Each contract must be rolled from one dated month to the next as it expires, which
  is a trade even when the direction has not changed. Contracts in quiet materials also cost more to
  trade relative to their size.
- Short horizons have decayed. The paper is explicit that the three-day version, which used to work,
  had disappeared by 2003. Nothing guarantees that the multi-month version will not follow.
- Crowding. Managed futures grew to hundreds of billions of dollars, and much of that money runs
  similar rules. More money crowding into a signal moves the buying earlier and splits the reward.
- The insurance story can be wrong. If the premium that producers pay for certainty is not the source
  of the returns, then the returns may be a coincidence of the sample, and would not be expected to
  continue.

## Try it yourself

You need nothing but a spreadsheet and a public source of monthly futures prices for one material,
say corn.

1. Build a sheet with one row per month and a column for the closing price.
2. Add a column for the monthly change: this month's price minus last month's, and a column for its
   size, ignoring the sign.
3. Add a column for the reference level: seed it with the simple average of the first five prices,
   then update each month as one third of the new price plus two thirds of the previous reference.
4. Add a column for the movement measure the same way, using the size of the changes.
5. Add a column for the signal: price minus reference, divided by the movement measure.
6. Add a column for the direction: buy if the signal is positive, sell if it is negative.
7. Add a column for the contribution: direction times next month's change, divided by the movement
   measure.
8. Count the months in which the direction changed, and subtract 0.05 percent for each one-way trade.

What to notice: the strategy is long through the long rises and short through the long falls, and it
gives back part of every reversal. Over ten years you will see long flat stretches and a few years
that carry the whole result. If your sheet looks steadily profitable, check whether the direction was
decided using the next month's price, which would be a look into the future.

## Where this came from

- [QuantConnect strategy library: commodities futures trend following](https://www.quantconnect.com/tutorials/strategy-library/commodities-futures-trend-following),
  the rules as implemented: seven contracts, a five-month reference and movement measure, monthly
  trading, leverage of three.
- Lemperiere, Deremble, Seager, Potters and Bouchaud, [Two Centuries of Trend Following](https://arxiv.org/abs/1404.3274),
  the paper the rules are built from. This tutorial cites its results from `1404.3274v1`, including
  the t-statistic of about 10 since 1800 (p.9) and the decay of the three-day trend since 2003 (p.13).
- [Quantpedia: momentum effect in commodities](https://quantpedia.com/strategies/momentum-effect-in-commodities),
  the indicative performance and the worst fall.
- [strategies/books2/18_energy_and_commodities.md](../../../strategies/books2/18_energy_and_commodities.md),
  this repository's own study of commodity curve signals and their costs.

## Words used in this tutorial

- exponential moving average: an average that weights the newest value most and older values less.
- futures contract: an agreement to buy or sell something at a fixed price on a future date.
- hedge: a trade made to reduce the risk of another holding.
- leverage: using borrowed money, or the equivalent, so that a price move is magnified.
- roll: closing an expiring futures contract and opening the next dated one.
- selling short: borrowing something you do not own, selling it, and buying it back later.
- skewness: a measure of whether returns are lopsided, with more big gains than big losses or the reverse.
- volatility: how much a price moves around its average, usually quoted per year.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
