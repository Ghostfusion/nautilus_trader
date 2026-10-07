# Trading only during chosen hours of the day

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                           |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | A cryptocurrency priced in a stablecoin, watched on one-hour candles                                                                                                                                            |
| How often it trades       | Whatever the hour rule allows; with the settings that ship in the file, a trade can open in twenty of the twenty-four hours of the day                                                                          |
| What you need             | Nothing but this page, or a spreadsheet and about a month of hourly prices to repeat the arithmetic                                                                                                             |
| Where the rules come from | [HourBasedStrategy.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/HourBasedStrategy.py), which states the rules and the settings its author searched for                  |
| The underlying research   | [Eross, McGroarty, Urquhart and Wolfe, The Intraday Dynamics of Bitcoin](https://doi.org/10.1016/j.ribaf.2019.01.008), the measurement of how activity changes through the day                                  |
| How well it held up       | Weak: the daily rhythm of activity is well measured, but the differences in return between hours are not stable, and the file's own results come from a thousand-round search on one coin over one hundred days |
| Also appears in           | Nothing else in this collection                                                                                                                                                                                 |

## The idea in one paragraph

A market is not equally busy at all times. This strategy picks the hours of the day in which it is
willing to buy, and does nothing the rest of the time. On one-hour candles it reads the hour on the
clock, and opens a position whenever that hour falls inside a chosen window, with no other condition
at all. The window that ships with the file runs from four in the morning to midnight, so the four
hours after midnight are the only ones it refuses. A profit ladder and a ten percent stop loss close
whatever it opens. The bet is that some hours are better to own a coin than others.

## Why anyone believed it

A cryptocurrency is bought and sold around the clock, but the people doing the buying are not spread
evenly around the planet, and neither are the institutions. When Europe and North America are awake
at the same time, more money and more news arrive at the market, so more trades happen and prices
move further. That activity is not necessarily a forecast; it is a fact about who is present. A rule
that is only switched on during the busy hours is really a rule that says: do not hold through the
dead hours, because that is when a thin market can move for reasons that have nothing to do with the
coin.

The counterparty is easiest to see in the quiet hours. At four in the morning in Europe, which is
late evening in New York, the number of resting orders is small, so any single seller moves the price
more than the same seller would at noon. Whoever is forced to sell then, and whoever is careless
then, pays more for the privilege than someone who waits. Standing aside during those hours avoids
being on the wrong side of that, and it also avoids paying the wider gap between the buying and
selling price that a thin hour carries.

## An everyday comparison

A seaside cafe looks at the clock, not the weather. It opens at nine because the promenade fills from
nine, stays open through lunch because that is when the queues are, and closes at four when the crowd
walks back to the car park. The owner is not forecasting anything; the shop is open when the custom
exists and shut when the staff and the electricity cost more than the trade. The hour rule here does
the same: it does not predict the price, it selects the times when the market has enough custom to
make trading worthwhile.

## The rules, step by step

1. Trade on one-hour candles. A candle is the opening, highest, lowest and closing price of one
   interval, so an hourly candle covers one hour and is labelled with the hour at which it starts;
   the candle labelled 04:00 covers 04:00 to 04:59.
2. Read the hour from each candle's timestamp. The file takes the timestamp the exchange supplied,
   and crypto exchanges timestamp in coordinated universal time, written UTC, so hour four means
   04:00 UTC wherever the reader happens to live.
3. Choose a buying window: a lowest hour and a highest hour. The window includes both ends.
4. Buy whenever the hour of the candle falls inside the window, and take no other condition into
   account. There is no filter for price, trend or volume.
5. The file ships with a lowest hour of four and a highest hour of twenty-four. Because an hour can
   only be nought to twenty-three, this means buy during hours four to twenty-three, that is, twenty
   of the twenty-four hours, and stand aside only during hours nought to three.
6. Choose a selling window in the same way, and sell whenever the hour falls inside it. The file
   ships with a lowest selling hour of twenty-two and a highest of twenty-one. Since twenty-two is
   greater than twenty-one, no hour can fall inside that window, so this sell rule never fires. Its
   exits in practice come from the two rules below.
7. Sell when the trade is ahead by fifty-two point eight percent at any moment, by eleven point three
   percent after one hundred and sixty-nine minutes, by eight point nine percent after five hundred
   and twenty-eight minutes, or by anything at all after one thousand eight hundred and thirty-seven
   minutes. This is the profit ladder, called a minimal return on investment table.
8. Sell when the trade is behind by ten percent, at any time. This is the stop loss.
9. The author's note at the top of the file says the file requires a parameter search before it is
   used, and that is how the hours and the ladder were chosen. The settings above are one such
   search's result, not a measured rule.

## The maths, with every symbol named

The whole entry rule is one comparison:

```text
enter when buy_hour_min <= hour <= buy_hour_max
```

- `hour` is the hour of the candle's timestamp, a whole number from nought to twenty-three.
- `buy_hour_min` and `buy_hour_max` are the two settings, here four and twenty-four.
- `<=` means "is not greater than", so both end hours are included.

The number of hours the rule allows is then:

```text
hours_allowed = buy_hour_max - buy_hour_min + 1 = 24 - 4 + 1 = 21, of which hour 24 never occurs
```

so twenty hours of the twenty-four are open to buying, and four are closed: hours nought, one, two
and three.

The size of the search behind those two numbers is worth writing down, because it is where the
settings come from. Each of the four hour settings can take the whole numbers nought to twenty-four,
which is twenty-five values, so the four together allow:

```text
search_space = 25 * 25 * 25 * 25 = 390,625
```

- The first two factors are the buying window's lowest and highest hour.
- The second two are the selling window's lowest and highest hour.
- Twenty-five, not twenty-four, because nought and twenty-four are both allowed values.

Three hundred and ninety thousand possibilities, each one a different rule, before the profit ladder
and the stop loss are searched as well. Choosing the best of them after seeing the results is the
practice called data snooping, and it makes the winner look better than it is.

Finally the profit ladder, which is a list of pairs of a number of minutes and a profit:

```text
target(elapsed) = ladder[K], where K is the largest key in the ladder that is not greater than elapsed
```

- `elapsed` is how many whole minutes have passed since the position was opened.
- `ladder` is `{"0": 0.528, "169": 0.113, "528": 0.089, "1837": 0}`.
- `K` is the largest key already passed. With this ladder, `K` is nought until minute 168, then 169
  until minute 527, then 528 until minute 1836, and 1837 afterwards.
- A value of nought means the ladder accepts any profit, however small.

The cost line is the usual one. If each side costs a fraction `c` of the amount traded, a round trip
costs `2 * c`; at 0.10 percent a side that is 0.20 percent of the position.

## A worked example

Six hourly candles of a coin we call BBB. The trade opens as soon as an allowed hour arrives.

| Hour (UTC) | Price | Elapsed minutes | Key in force | Target | Gain   | Action          |
| ---------- | ----- | --------------- | ------------ | ------ | ------ | --------------- |
| 4          | 1.000 | 0               | 0            | 52.8%  | 0.00%  | buy             |
| 5          | 0.970 | 60              | 0            | 52.8%  | -3.0%  | hold            |
| 6          | 0.950 | 120             | 0            | 52.8%  | -5.0%  | hold            |
| 7          | 0.930 | 180             | 169          | 11.3%  | -7.0%  | hold            |
| 8          | 0.905 | 240             | 169          | 11.3%  | -9.5%  | hold            |
| 9          | 0.900 | 300             | 169          | 11.3%  | -10.0% | stop loss fires |

The sell window never fires, so the ladder would have to close this trade and it never gets close:
for the first one hundred and sixty-eight minutes the ladder demands a gain of fifty-two point eight
percent, which almost no trade reaches, and after that it demands eleven point three percent while
the coin is falling. The stop loss does the closing, at minus ten percent.

```text
Buy   1,000 coins at 1.000 = 1,000.00, fee 0.10% = 1.00, total paid 1,001.00
Sell  1,000 coins at 0.900 = 900.00, fee 0.10% = 0.90, total received 899.10
Loss  899.10 - 1,001.00 = -101.90
Return -101.90 / 1,001.00 = -0.1018, about minus 10.18 percent
```

Two things stand out. First, the stop loss level is ten percent below the entry, and the two fees
push the realised loss to 10.18 percent. Second, there are twenty allowed buying hours and each one
opens a trade if the previous one has closed, so the rule can fire up to twenty times a day. The
hour filter is not really a filter: it removes four hours out of twenty-four, and the worked example
shows that the four it removes are precisely the quiet ones the research below describes.

## What the research actually found

The daily rhythm of a crypto market is measured, and the measurements are consistent. Eross,
McGroarty, Urquhart and Wolfe studied the Bitstamp exchange in New York dollars, using every trade
from January 2014 to December 2017, grouped into five-minute intervals, and timestamped in
coordinated universal time. They found that volume is low until about 07:00, rises until about 10:30,
peaks at about 14:00 and then declines, a pattern they attribute to European and North American
traders being present. Recorded volatility is highest from about 07:00 and declines after 18:00, and
liquidity is at its best from about 10:00 onward, when all three of the main stock exchanges are
open, while the market is at its least liquid in the early morning. So the four hours the shipped
settings exclude, nought to three, are indeed the thinnest ones in the data that was studied.

What is not measured is a usable return difference between hours. That is the day-of-week cousin of
this problem, and it has been tested repeatedly. Caporale and Plastun examined several
cryptocurrencies from 2013 to 2017 and found a day-of-the-week effect only for Bitcoin, whose Monday
returns were unusually high. They then ran a trading simulation that bought Bitcoin on Monday and
sold it at the end of the day: over the whole sample it made money and beat random trading, but in
individual years the results were mostly indistinguishable from random, and the authors conclude
there is no clear evidence that the market was inefficient. Mueller revisited the question on five
hundred coins and reported that the positive Monday effect for Bitcoin does not survive in the data
after 2015, that no return anomaly is robust, and that what is robust is lower trading activity at
weekends. The pattern of the evidence is therefore: the busy hours are real, the returns attached to
them are not stable.

The file's own numbers deserve a plain reading. Its comments record searches of one thousand rounds
on single coins over one hundred days. The best result on SHIB against the dollar, reached at round
one hundred and fifty-eight of a thousand, reported fifty-one trades, wins, draws and losses of
twenty-nine, nineteen and three, and a total profit of 486.75 percent. The best result on KDA
against the dollar, reached at round seven of a thousand, reported sixty-five trades and a total
profit of 4,112 percent. The author rates each run by its objective and keeps the best. A result that
is the best of a thousand attempts, on one coin, over one hundred days, in a period when that coin
rose enormously, is a description of the search rather than of the rule.

## How this project relates to it

This repository runs no Freqtrade strategy, so it cannot reproduce the hour rule. It does hold the
two pieces of this project's own research that bear on it.
[Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md)
is the brief on how many rules may be tried before a winner means anything, which is exactly what a
search of 390,625 hour windows is. [Market data quality](../../../strategies/books2/27_market_data_quality.md)
studies what a thin hour does to recorded prices: it reports that order flow that pushed a five-minute
crypto contract around before its close was concentrated in illiquid periods, with fifty-six percent
of the pushes overnight and forty-four percent at weekends, and that the price reverted within ten
seconds afterwards. An hour rule that stands aside during the thin hours is standing aside during the
hours when recorded prices are least trustworthy, and that brief is where the measurement lives.

## Where it goes wrong

- The window is not a window. The shipped settings allow buys in twenty of the twenty-four hours.
  Whatever the search was doing, it was not selecting a narrow time of day.
- The search is enormous and the sample is tiny. Three hundred and ninety thousand hour windows, on
  one hundred days of one coin, means the winner was almost certainly luck. The reported returns,
  from 486 percent to more than four thousand percent in a hundred days, are of a size that cannot be
  real for a rule that buys in nearly every hour.
- The seller is switched off. The selling window is empty as written, so exits come only from the
  stop loss and a ladder whose first rung asks for 52.8 percent. That is not a strategy the author
  tested as stated; it is the result of a search that was free to produce a broken window.
- Time zones move the rule. Exchange candles are timestamped in coordinated universal time, and a
  reader in another zone will find that hour four is not their four in the morning. Daylight saving
  changes shift local labels twice a year while the data stays in universal time, and an exchange that
  timestamps differently shifts the whole rule silently.
- The busy hours change. The measurement above comes from 2014 to 2017; the mix of who trades and
  from where has changed since, and an hour pattern that was measured once is not a permanent feature.
- Overnight is where manipulation hides. The brief on market data quality finds the least trustworthy
  prints in the thinnest hours, so the quiet hours are not simply low-activity, they are also the
  hours in which the recorded price is least meaningful.
- There is no reason on the page. The file gives no economic argument for its hours, and the search
  is the only justification offered.

## Try it yourself

You need a spreadsheet and about a month, ideally a year, of hourly closes for one coin.

1. Build columns `time`, `hour` and `close`, with `hour` taken from the timestamp exactly as the
   source supplies it, without converting it to your own clock.
2. Add a column `return` that is each hour's close divided by the previous hour's close, minus one.
3. Add a pivot table or an average-by-hour column, so each of the twenty-four hours gets one average
   return and one count of how many hours went into it.
4. Do the same for volume, so you have an average volume per hour beside the average return.
5. Split the data in half by date, and compute the average return per hour separately for each half.
6. Rank the hours in the first half and mark where the second half's best hours sit in that ranking.
7. Finally, take the best four hours from the first half, apply the shipped rule that buys only in
   those hours, and see what the second half would have paid, after 0.20 percent per round trip.

What to notice: the volume column has a clear shape, with a busy stretch in European and American
hours and a quiet stretch overnight, and that shape is similar in the two halves. The average-return
column usually has no stable shape at all: the hours that look good in the first half are typically
scattered through the ranking in the second. That gap, between a rhythm in activity and no rhythm in
return, is the honest finding about time-of-day rules.

## Where this came from

- [HourBasedStrategy.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/HourBasedStrategy.py),
  the source of the hour windows, the profit ladder, the ten percent stop, and the notes recording
  the author's searches.
- [The Freqtrade documentation on hyperopt](https://www.freqtrade.io/en/stable/hyperopt/), the
  parameter search the file's own instructions tell the reader to run, including how many rounds are
  involved.
- A. Eross, F. McGroarty, A. J. Urquhart and S. S. Wolfe,
  [The Intraday Dynamics of Bitcoin](https://doi.org/10.1016/j.ribaf.2019.01.008), Research in
  International Business and Finance 49, pages 71 to 81 (2019), for the volume, volatility and
  liquidity patterns through the day.
- G. M. Caporale and A. Plastun,
  [The day of the week effect in the cryptocurrency market](https://doi.org/10.1016/j.frl.2018.11.012),
  Finance Research Letters 31 (2019), for the Monday effect and how little a trading rule based on it
  was worth.
- L. Mueller, [Revisiting seasonality in cryptocurrencies](https://doi.org/10.1016/j.frl.2024.105429),
  Finance Research Letters 64 (2024), for the failure of the Monday effect to persist and the robust
  weekend drop in activity.
- [Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md)
  and [Market data quality](../../../strategies/books2/27_market_data_quality.md), this repository's
  own briefs.

## Words used in this tutorial

- candle: the opening, highest, lowest and closing price of one fixed interval of trading.
- coordinated universal time: the clock that crypto exchanges timestamp their candles with, written
  UTC, which does not change with daylight saving.
- data snooping: searching the same data with many rules until one looks good by luck, then reporting
  only that one.
- hyperopt: the parameter search built into the Freqtrade bot, which tries many settings and keeps
  the best according to a chosen score.
- liquidity: how easily something can be bought or sold quickly without moving its price.
- seasonality: a pattern that tends to appear at the same time of year, month or day.
- stop loss: the fixed loss at which a trade is closed, here ten percent below the entry price.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
