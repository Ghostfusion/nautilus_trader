# Turn of the month: buying just before the month ends and selling a few days into the next one

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                                                                 |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | A fund or futures contract that tracks a broad American share index, held for only a few days each month                                                                                                                                                                                                                                                                                              |
| How often it trades       | Twice a month: one purchase near the end of a month and one sale a few days into the next month                                                                                                                                                                                                                                                                                                       |
| What you need             | A spreadsheet and a long history of daily index closing prices                                                                                                                                                                                                                                                                                                                                        |
| Where the rules come from | [QuantConnect strategy library, turn of the month in equity indexes](https://www.quantconnect.com/tutorials/strategy-library/turn-of-the-month-in-equity-indexes) and the [Quantpedia entry](https://quantpedia.com/strategies/turn-of-the-month-in-equity-indexes) it cites                                                                                                                          |
| The underlying research   | Xu and McConnell, [Equity Returns at the Turn of the Month](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=917884), following Lakonishok and Smidt (1988)                                                                                                                                                                                                                                        |
| How well it held up       | Mixed: the pattern is documented over more than a century and in most of the thirty-plus countries studied, with costs small enough to leave a margin, but the source paper's own tests reject the cash-flow story that is usually offered and the rule is one of very many calendar patterns that have been searched, so the grade rests on the breadth of samples rather than on any accepted cause |
| Also appears in           | Nothing else in this collection                                                                                                                                                                                                                                                                                                                                                                       |

## The idea in one paragraph

The long-run rise of a share index is not spread evenly across the days of the month. A large share
of the gain has, historically, arrived in a short window around the change of month, roughly the last
trading day of one month and the first three trading days of the next. This strategy holds an index
fund only during that window, four trading days or so, and keeps the money out of the market for the
rest of the month. The claim is old and widely reported: it comes from a pattern measured over more
than a century of American daily prices. The rest of this page is about how to apply it, what was
measured, and why a pattern this simple is hard to believe.

## Why anyone believed it

Salaries, interest and dividends are mostly paid at the end of a month, and large pension funds
receive contributions on a monthly schedule and invest them. If money arrives on a predictable date,
buyers arrive on a predictable date, and prices can be pushed up by that arrival rather than by any
change in what the companies are worth. Funds also tend to tidy their books at month ends, which
means selling and buying at the same point in every calendar. The counterparty, on that story, is the
seller who needs cash on schedule, or the fund adjusting its holdings for reasons that have nothing
to do with the outlook.

The honest wrinkle is that the story has a hole in it. The paper behind the rule tested the
month-end-cash-flow explanation and rejected it, and it also ruled out higher risk at the turn of the
month and any special role for the end of the year or the end of a quarter. What is left is a pattern
that is real in the data and has no agreed reason to exist, which is why the paper calls it a puzzle.

## An everyday comparison

A bakery on a street where most employers pay wages on the last Friday of the month. The bread is the
same on every day of the month, but the queue on payday is longer and the shelves empty sooner,
because the money arrives on a known schedule rather than because the bread is better that Friday.
Someone who only looks at the payday queue and concludes that bread is especially good that day has
made the mistake this strategy depends on: a repeated pattern produced by the timing of money, not by
any change in the product.

## The rules, step by step

1. Choose one index to trade. The QuantConnect example uses one exchange-traded fund that tracks the
   largest American companies; the Quantpedia figures are for the American index itself.
2. Collect the daily closing prices, and mark which day is the last trading day of each month.
3. On the day before that last trading day, buy the fund. Some papers start the window four days
   before the month end, and the source pages differ on the exact day, so this is a choice to record.
4. Hold the fund. Do nothing in between.
5. Sell at the close of the third trading day of the new month.
6. Keep the money in cash until the next month end, then repeat from step 3.
7. Do this every month. There are two trades a month and no other decisions.

Two variations are worth knowing. Some studies buy on the last trading day itself rather than the day
before, which produces a three-day window instead of a four-day one. And some authors first demand
that the index be above its own average over the past several months, so that the rule is not run in
a falling market; that filter is a different strategy and is not part of the plain version.

## The maths, with every symbol named

Write the last trading day of a month as day `d`, and the fund's closing price on a day as `P` with a
subscript for the day. The return earned by holding the fund through the window, using the day before
the month end as the entry and the third trading day of the new month as the exit, is:

```text
R_tom = P_(d+3) / P_(d-1) - 1
```

- `R_tom` is the return over the four-day window, written as a decimal: 0.006 means 0.6 percent.
- `P_(d-1)` is the closing price on the trading day before the last trading day of the month.
- `P_(d+3)` is the closing price on the third trading day of the following month.

The rest of the month is then whatever the whole month did minus what the window did. If a month's
total close-to-close return, measured from one month-end to the next, is `R_month`, then:

```text
R_rest = (1 + R_month) / (1 + R_tom) - 1
```

- `R_month` is the fund's return over the whole month, from the close of the day before the previous
  month end to the close of the day before this month end.
- `R_rest` is the return over the other sixteen or so trading days, the part the strategy misses.
- Dividing rather than subtracting is the correct way to peel one part of a compounded return out of
  the whole; subtracting would be slightly wrong.

The claim the strategy rests on is that `R_tom` is, on average, positive and large while `R_rest` is,
on average, close to zero. Quantpedia reports the window's average daily return as about 0.15 percent
arithmetically, which over four days and twelve months is about 7.2 percent a year from the days the
strategy is actually invested.

Finally the cost. Every month the strategy buys once and sells once:

```text
Cost_per_month = 2 * c
```

- `c` is the cost of one trade as a fraction of the money traded, covering the gap between the buying
  and selling price plus any commission. One basis point is one hundredth of one percent; a large
  index fund regularly trades at a spread of one or two basis points, so `c` of 0.0002, that is two
  basis points, is a reasonable stand-in and is used in the example below.
- `2 * c` is the round trip: one purchase and one sale.

At `c` of 0.0002 the round trip is 0.0004 a month, which is about 0.48 percent a year, charged only
on the small window of days the strategy is in the market.

## A worked example

Eight months of invented but plausible numbers for one index fund. The four-day window return and the
rest-of-month return are given; the whole-month return is their compound, worked out as
`(1 + window) * (1 + rest) - 1`. The strategy earns the window and pays 0.04 percent a month in cost,
which is `2 * 0.0002`.

| Month | Window return | Rest-of-month return | Whole-month return | Strategy net  |
| ----- | ------------- | -------------------- | ------------------ | ------------- |
| 1     | +0.85 percent | -0.35 percent        | +0.50 percent      | +0.81 percent |
| 2     | +0.40 percent | +0.10 percent        | +0.50 percent      | +0.36 percent |
| 3     | +1.20 percent | -0.20 percent        | +1.00 percent      | +1.16 percent |
| 4     | -0.30 percent | +0.25 percent        | -0.05 percent      | -0.34 percent |
| 5     | +0.90 percent | -0.60 percent        | +0.29 percent      | +0.86 percent |
| 6     | +0.60 percent | +0.05 percent        | +0.65 percent      | +0.56 percent |
| 7     | +0.50 percent | -0.40 percent        | +0.10 percent      | +0.46 percent |
| 8     | +0.75 percent | -0.10 percent        | +0.65 percent      | +0.71 percent |

Check one row. Month 1: the whole month is `1.0085 * 0.9965 = 1.004970`, which is +0.50 percent, and
the strategy net is `0.85 - 0.04 = 0.81` percent. Month 4 is the one month in the window that lost
money, and the strategy still paid the cost on top of the loss, which is `-0.30 - 0.04 = -0.34`
percent.

Compounding the strategy's eight monthly net returns gives `1.0081 * 1.0036 * 1.0116 * 0.9966 *
1.0086 * 1.0056 * 1.0046 * 1.0071 = 1.0467`, so about +4.67 percent over eight months. Compounding
the whole-month returns gives `1.004970 * 1.005004 * 1.009976 * 0.9994925 * 1.002946 * 1.006503 *
1.000980 * 1.006493 = 1.036907`, about +3.69 percent. In this invented run the strategy wins by
roughly one point over eight months while being in the market about four days out of every
twenty-one. That is the shape the anomaly claims, and it is also why the next section matters: a
result this close depends on the window being right and the cost being low.

## What the research actually found

The published record is long, and the disagreement is about the cause rather than the existence.

| Source                                                                         | What it measured                                                             | Result                                                                                                                                                                                                                                                                                  |
| ------------------------------------------------------------------------------ | ---------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Lakonishok and Smidt (1988), the original result, as summarised by Quantpedia  | The four days at the turn of the month, Dow Jones index, 1897 to 1986        | Essentially all of the index's positive return over the period was earned in those four days                                                                                                                                                                                            |
| Xu and McConnell (2008), the source paper                                      | American daily returns, 1926 to 2005, and thirty-five other countries        | The effect persisted over 1987 to 2005 too; it was not confined to small or cheap shares, to the December to January turn, to quarter ends, or to one country; risk was not higher in the window; the authors call it a puzzle, and their tests reject the payday cash-flow explanation |
| Quantpedia, the entry the rules come from                                      | The index, 1926 to 2005                                                      | Indicative performance 7.2 percent a year, volatility 6.9 percent, worst fall 20.79 percent, reward to risk 1.04, over a sample that includes several deep bear markets                                                                                                                 |
| Carchano and Tornero (2012), cited by Quantpedia                               | 188 possible calendar rules, S&P 500, DAX and Nikkei futures, 1991 to 2008   | Of all the calendar rules tested, the turn of the month in S&P 500 futures was the only effect both statistically and economically significant and persistent                                                                                                                           |
| Reschenhofer (2010), cited by Quantpedia                                       | A test designed not to favour any particular pattern, applied to the S&P 500 | Confirms within-month patterns in daily returns, which matters because it answers the objection that the pattern was spotted by looking for one                                                                                                                                         |
| Dzhabarov and Ziemba, and Grimbracher, Swinkels and Vliet, cited by Quantpedia | Seasonal effects in futures, and the interaction of five calendar effects    | The effect was still present in the early 2000s; when the turn of the month or the Halloween effect was present the equity excess return was 7.2 percent a year, and minus 2.8 percent in all other cases                                                                               |

Read together: the pattern shows up in long samples, in many countries, and survives a test designed
not to hunt for it, and the trading costs are among the lowest of any active strategy. What is not
settled is why it exists. A paper that measures a pattern carefully and then reports that the obvious
explanation fails is evidence for the pattern and against the story; a reader should hold the two
apart.

## How this project relates to it

This repository does not implement a calendar or seasonality strategy, and no file in it trades the
turn of the month. The closest thing is the repository's own machinery for time, which is honest
about the boundary. The calendar code described in
[trading calendars](../../../docs/concepts/trading_calendars.md) answers when a market is open and
when a session ends; it is deliberately a description of the exchange, not a claim that any day of
the month behaves differently. A calendar rule as a strategy is the other kind of statement, and the
repository has nothing that makes it.

Two of the repository's research briefs bear directly on whether a calendar rule should be believed.
The first is
[Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
which records that mining 240 accounting variables produced 18,113 candidate strategies of which
30.17 percent cleared a two-sigma threshold by chance, and which recommends a much higher threshold
once a search has tried many rules. The second is
[Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), whose
1,022-rule experiment found that 132 of 1,022 rotation rules beat buy and hold, a rate the authors
read as the result of trying so many rules. A calendar anomaly is exactly the kind of claim those two
results say to treat with care.

## Where it goes wrong

- The rule is one of very many. Every weekday, every day of the month, every holiday and every turn
  of a quarter has been proposed as a market-beating calendar. A pattern that survives a test
  designed not to favour it, as Reschenhofer's does, is stronger evidence than a pattern found by
  scanning, but the family of calendar claims that have been tried is enormous and the thresholds are
  usually not adjusted for it.
- The exact days are chosen after the fact. Whether the window starts four days before the end or on
  the last day, and whether it ends on the second, third or fourth trading day, are all free choices,
  and the best-performing choice is the one that gets published. Move the window by a day and the
  result changes.
- The explanation does not survive its own test. The month-end cash story is the reason usually
  given, and the source paper's tests reject it. With no accepted cause there is no reason the
  pattern must persist once enough money knows about it.
- Costs and the idle cash. The strategy is in cash most of the month. Two trades a month at two basis
  points per side is about 0.48 percent a year, which the window's average gain covers, but a wider
  spread or a commission ends the margin. The example above also ignores any interest earned on the
  cash, which makes it slightly conservative.
- The effect can move. Quantpedia's own note says that calendar effects tend to vanish or rotate to
  different days once they are widely known, so a rule fitted to a particular window in a particular
  decade may simply be describing that decade.
- Execution at the close. The rule trades at the closing price on two specific days, when many other
  calendar traders would be doing the same if the effect were widely used. The measured record is on
  closing prices, and the return a real trader would capture at the moment of the close is a
  different number.

## Try it yourself

You need a spreadsheet and a public history of daily closing prices for a broad index, for example
twenty years of the S&P 500 or of one large index fund.

1. Make a sheet with one row per trading date and columns `date`, `close`, and `daily_return`, where
   `daily_return` is today's close divided by yesterday's close minus one.
2. Add a column `window_day` and mark each date as "yes" if it falls on the last trading day of a
   month or one of the first three trading days of a month, and "no" otherwise.
3. Using a pivot or a simple average, compute the average `daily_return` for the "yes" rows and for
   the "no" rows.
4. Multiply each average by the number of days of that kind in a year to compare them on the same
   footing.
5. Build a second sheet that compounds the "yes" days at the cost of two basis points per side, once
   a month, and compares the result with simply holding the index all month.
6. Finally, shift the window by one day, so that it runs from two days before the month end, and see
   how much the answer moves.

What to notice: the "yes" days carry a higher average and the "no" days an average close to zero,
which is the pattern. What should give you pause is how much the numbers change when the window moves
by a single day, and how few trading days the whole result depends on. A finding that rests on four
days a month is fragile in a way that a finding resting on every day is not.

## Where this came from

- [QuantConnect strategy library: turn of the month in equity indexes](https://www.quantconnect.com/tutorials/strategy-library/turn-of-the-month-in-equity-indexes),
  the rules as implemented: buy the index fund on the last trading day of the month and sell three
  trading days later, every month.
- [Quantpedia: turn of the month in equity indexes](https://quantpedia.com/strategies/turn-of-the-month-in-equity-indexes),
  the performance figures, the instrument count, the sample period and the list of underlying papers.
- Xu and McConnell, [Equity Returns at the Turn of the Month](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=917884),
  the source paper for the rules and the tables most of the figures come from.
- Carchano and Tornero, [Calendar Anomalies in Stock Index Futures](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1958587),
  the study that found the turn of the month the only significant calendar effect among 188 tested.
- Reschenhofer, [Further Evidence on the Turn of the Month Effect](http://astonjournals.com/manuscripts/Vol2010/BEJ-16_Vol2010.pdf),
  the test designed not to favour any particular within-month pattern.
- [Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md)
  and [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), the two
  briefs in this repository on how many rules were tried and what that does to a threshold.

## Words used in this tutorial

- anomaly: a pattern in prices that the usual explanations do not account for.
- buy and hold: buying an index and keeping it, rather than trading in and out.
- drawdown: the fall from a peak to the following low, measured in percent.
- index: a single number that tracks the average movement of a group of shares.
- round trip: one purchase and the later sale of the same thing.
- seasonality: a pattern that repeats with the calendar.
- trading day: a day the market is open and prices are recorded.
- volatility: how much a price moves around its average, measured as a percentage per year.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
