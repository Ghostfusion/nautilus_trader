# Calendar effects: the month, the weekday and the year that always seems special

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                               |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Spot gold on daily bars, a fund holding gold bars, Bitcoin through a fund, and 15-minute gold                                                                                                                                                                                                       |
| How often it trades       | 18 times in 17 years for Sell in May, about once a month for the turn-of-month window, and 69 times in 18 years for the meeting-date rule                                                                                                                                                           |
| What you need             | A spreadsheet and a calendar                                                                                                                                                                                                                                                                        |
| Where the rules come from | [The Strategy Compendium, article 08, calendar effects](https://backtrader.readthedocs.io/en/latest/strategies-series/en/08-calendar-effects.html)                                                                                                                                                  |
| The underlying research   | Bouman and Jacobsen, [The Halloween Indicator, Sell in May and Go Away](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=300700), and Xu and McConnell, [Equity Returns at the Turn of the Month](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=917884)                                    |
| How well it held up       | Disputed: long samples in many countries support Sell in May and the turn of the month, while the library's own meeting-date rule loses money, and the number of calendars that have been searched makes any single month or weekday unconvincing on its own                                        |
| Also appears in           | [Turn of the month](../../quantconnect/turn-of-the-month-in-equity-indexes/README.md), [pre-holiday effect](../../quantconnect/pre-holiday-effect/README.md) and [the same calendar month](../../quantconnect/seasonality-effect-based-on-same-calendar-month-returns/README.md) in this collection |

## The idea in one paragraph

Some days and some months have, historically, been better for markets than others. This category
turns that observation into rules: hold the market from early November to early May and sit out the
summer; hold it for the last few days of a month and the first few days of the next; hold it in the
days before a central-bank meeting; hold it during the week options expire. There is no model of the
economy behind any of them, only a pattern in the calendar and, sometimes, a story about who is
forced to trade on a particular date. The rules are simple enough to check by hand, which is exactly
why they are also easy to find by accident.

## Why anyone believed it

The stories differ by calendar. For Sell in May, the explanation is that money leaves the market in
the summer, when desks are thin and investors are away. For the turn of the month, salaries, pension
contributions and dividends are paid at the month end, so buyers arrive on a predictable date. For
central-bank meetings, uncertainty resolves and positions are adjusted before the announcement; for
option expiry, the firms that sold insurance adjust the shares they hold as the contracts die.

What the stories share is a counterparty who transacts for a reason unrelated to value: a person
selling to meet a bill, a fund tidying its books, a dealer unwinding a hedge.

## An everyday comparison

A bakery on a street where most employers pay wages on the last Friday of the month. The bread is the
same every day, but the queue on payday is longer and the shelves empty sooner, because the money
arrives on a known schedule rather than because the bread is better that Friday. A shopper who only
ever sees paydays and concludes that the bread is special has made the mistake this category depends
on.

## The rules, step by step

The category holds 28 backtests. They share one skeleton: read the date, be invested inside a window,
be flat or short outside it. The table lists the ones a reader is most likely to meet.

| Strategy                          | Data                     | What it does                                                              |
| --------------------------------- | ------------------------ | ------------------------------------------------------------------------- |
| Sell in May                       | Gold daily, 2008 to 2025 | Buy in early November, sell in early May, flat all summer                 |
| Turn of month                     | Gold daily, 2008 to 2025 | Fully invested in the last 3 and first 3 days of each month               |
| Gold meeting-day effect           | Gold daily, 2008 to 2025 | Position 5 days before a synthetic central-bank date, with a trend filter |
| Gold calendar effect              | Gold daily, 2008 to 2025 | Holdings grouped by month of the year                                     |
| Gold turn of month (two)          | Gold daily               | The same window with two different parameter sets                         |
| Gold seasonality                  | Gold daily               | Historical monthly returns decide the direction                           |
| Seasonal windows and rotation     | Gold daily               | Fixed month windows, and several windows rotated together                 |
| End-of-month seasonality          | Gold daily               | Only the last days of each month                                          |
| Thanksgiving                      | Gold daily               | A holiday-window drift                                                    |
| December expiry and quad witching | Gold daily               | The volatility of the option-expiry week                                  |
| Bitcoin seasonal anomalies        | Bitcoin fund daily       | Monthly anomalies of a Bitcoin fund                                       |
| Pre-election drift                | Gold daily               | A long window ahead of American elections                                 |

1. Sell in May. Use daily gold. Mark every month by its number, 1 for January through 12 for
   December. Buy the whole account on the first trading day of November and sell on the first trading
   day of May. Hold November, December, January, February, March and April; be flat for May through
   October. One refinement in the file: it fires the buy only on the bar that first enters November,
   not on every November bar.
2. Turn of month. Use daily gold. Work out each day's place inside its month: the last three trading
   days of the month and the first three trading days of the next are inside the window, and a window
   runs continuously across the boundary. Buy the whole account on the day the window opens and sell
   on the day it closes. A 2 percent stop from the entry price is armed while the position is held.
3. Meeting-day rule. Use daily gold. The file cannot fetch a real central-bank calendar, so it
   builds a proxy: eight months of the year contain a meeting, and the third Wednesday of each is
   taken as the meeting date. Look back over the last four meetings. If the average price drift in
   the five days before those dates was positive and the price is also above its 20-day average,
   take a long position 5 days before the next proxy date, sized at 3 percent of the account. Exit
   the day after the event. The stop is two times recent volatility scaled to five days, held between
   1 and 5 percent.
4. The remaining rules repeat this shape with other dates: single months, holidays, election years,
   and the option-expiry week. Most are one long-or-flat window, with the same fixed size.
5. Every rule is reviewed once a day and acts on the close.

## The maths, with every symbol named

A calendar rule is a date test plus a return. The date test for Sell in May has to cope with a year
that wraps around:

```text
holding = true when month >= 11 or month <= 4
```

- `month` is the number of the month, 1 for January and 12 for December.
- The first condition catches November and December, the second catches January to April. Written
  this way, the six winter months sit on one side and the six summer months on the other.

The return earned by holding through a window is the ordinary one:

```text
R_window = P_exit / P_entry - 1
```

- `P_entry` is the price on the day the window opens and `P_exit` the price on the day it closes.
- `R_window` is written as a decimal: 0.013 means 1.3 percent.

The turn-of-month window is defined by counting backwards and forwards from a month boundary:

```text
inside = (rev_rank <= 3) or (fwd_rank <= 3)
```

- `fwd_rank` counts trading days from the first of the month, so it is 1 on the first trading day,
  2 on the second and 3 on the third.
- `rev_rank` counts trading days back from the last of the month, so it is 1 on the last trading day.
- A day is in the window when either count is 3 or less, so the window runs from three days before the
  boundary to three days after it, and the two pieces join into one holding of about six days.

The meeting-day rule sizes its stop from volatility, which is the one place a calendar rule in this
category uses a formula rather than a date:

```text
stop = 2.0 * vol_20 * sqrt(5 / 252), clipped to between 0.01 and 0.05
```

- `vol_20` is the standard deviation of daily gold returns over the last 20 days, expressed per day.
- `sqrt(5 / 252)` scales a one-day figure to five trading days, because volatility grows with the
  square root of time. The number 252 is the count of trading days in a year.
- The result is kept between 1 percent and 5 percent, so a quiet market still gets a usable stop.
- `2.0` is the library's multiplier: a wider stop when the market is more volatile.

## A worked example

First Sell in May, on one invented year. The strategy holds the six winter months and is flat for the
six summer months, and the file charges 0.05 percent commission per side, which the table adds to a
0.02 percent gap between buying and selling prices. The table's returns are the price moves of the
month; the strategy column is that move in the months it holds and zero in the months it does not.

| Month | Held | Price move | Strategy gross | Strategy net | Running value |
| ----- | ---- | ---------- | -------------- | ------------ | ------------- |
| Nov   | yes  | +2.00%     | +2.00%         | +1.86%       | 10,186.00     |
| Dec   | yes  | +1.50%     | +1.50%         | +1.36%       | 10,324.50     |
| Jan   | yes  | -0.50%     | -0.50%         | -0.64%       | 10,258.44     |
| Feb   | yes  | +0.80%     | +0.80%         | +0.66%       | 10,326.15     |
| Mar   | yes  | +1.20%     | +1.20%         | +1.06%       | 10,435.62     |
| Apr   | yes  | +0.40%     | +0.40%         | +0.26%       | 10,462.75     |
| May   | no   | -1.10%     | 0.00%          | 0.00%        | 10,462.75     |
| Jun   | no   | +0.30%     | 0.00%          | 0.00%        | 10,462.75     |
| Jul   | no   | +0.90%     | 0.00%          | 0.00%        | 10,462.75     |
| Aug   | no   | -0.20%     | 0.00%          | 0.00%        | 10,462.75     |
| Sep   | no   | +0.60%     | 0.00%          | 0.00%        | 10,462.75     |
| Oct   | no   | +1.40%     | 0.00%          | 0.00%        | 10,462.75     |

Check the account. The summer months made `0.30 + 0.90 - 0.20 + 0.60 + 1.40 = 3.00` percent after
subtracting May's fall, and the strategy missed all of it by sitting out. The winter months made 5.40
percent, and the strategy paid the round-trip cost twice in the year, `2 * 0.0007 = 0.0014`, that is
0.14 percent on each of the two trips, so its own year is a little over 4.6 percent. In this invented
year the calendar rule lost to simply holding the metal, which is exactly the possibility its
supporters have to face: skipping half the year also skips half the gains.

Now the turn-of-month window, on eight invented trading days across a boundary.

| Day | Trade date | Close   | In window | Action          |
| --- | ---------- | ------- | --------- | --------------- |
| 1   | Mon 26     | 1900.00 | yes       | buy at 1900.00  |
| 2   | Tue 27     | 1905.00 | yes       | hold            |
| 3   | Wed 28     | 1910.00 | yes       | hold            |
| 4   | Thu 29     | 1918.00 | yes       | hold            |
| 5   | Fri 30     | 1912.00 | yes       | hold            |
| 6   | Mon 2      | 1925.00 | yes       | hold            |
| 7   | Tue 3      | 1920.00 | no        | sell at 1920.00 |
| 8   | Wed 4      | 1915.00 | no        | flat            |

On day 7 the window has closed, so the rule sells at 1920.00.

```text
Gross = 1920.00 / 1900.00 - 1 = 0.010526, that is 1.053 percent
Cost  = 2 * (0.0005 + 0.0002) = 0.0014, that is 0.14 percent
Net   = 0.91 percent
```

For comparison, holding the same eight days from start to finish returns `1915.00 / 1900.00 - 1 =
0.789` percent before costs and about 0.65 percent after. The window beat the whole eight days here
because it stepped aside before the last two days fell, not because it captured anything special.

## What the research actually found

The library reports one run per rule. Its own numbers are below.

| Rule              | Sample               | Trades | Wins        | Final value | Reward for risk | Worst fall | Time in market |
| ----------------- | -------------------- | ------ | ----------- | ----------- | --------------- | ---------- | -------------- |
| Sell in May       | Gold daily, 17 years | 18     | 12 (66.7%)  | 2,875,338   | 0.55            | 28.94%     | half the year  |
| Turn of month     | Gold daily, 17 years | 210    | 115 (54.8%) | 2,000,333   | 0.56            | 22.60%     | 28% of days    |
| Meeting-day proxy | Gold daily, 18 years | 69     | 33 (47.8%)  | 994,992     | -0.17           | 1.29%      | 69 events      |

The three start from a million units of currency. Sell in May ends 187.5 percent higher over
seventeen years, but gold itself rose over those years and the rule held the metal for half of them,
so much of that figure is the metal rather than the calendar. The turn of month made 100 percent
while invested less than a third of the time, the strongest claim in the table. The meeting-day rule
lost half a percent over eighteen years with a worst fall of 1.29 percent, a failed hypothesis tested
cheaply, which is the most useful thing in the category.

The library's tests assert the final value, the reward for risk and the worst fall against numbers
captured when each strategy was migrated. Passing those assertions proves the engine computes exactly
what the file says, to the cent, in both engine modes. It proves nothing about whether the strategy
earns anything. A rule that loses money can pass its assertion perfectly, and the meeting-day rule
does exactly that.

The published record for the two big calendars is long, and it is why this category cannot be
dismissed outright. Bouman and Jacobsen studied the November-to-April half of the year against the
May-to-October half across thirty-seven countries and found the winter half ahead in most of them,
stronger outside the United States, which is why the pattern is also called the Halloween indicator.
Xu and McConnell measured the turn of the month on American daily returns from 1926 to 2005 and in
thirty-five other countries, found it persisted in the later part of the sample, ruled out higher
risk in the window, and then reported that their own tests rejected the payday explanation for it. A
pattern whose obvious story fails its own test is evidence for the pattern and against the story.

Now the honest heart of this tutorial, and the reason the grade is disputed rather than strong. There
are many calendars: twelve months, seven weekdays, about ten public holidays, four quarter ends,
twelve option expiries, and the days around each central-bank meeting, each of which can be split into
before, on and after. Search that space and something will look special, because with enough tries a
run of good luck is guaranteed. The repository's brief
[Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md)
measures the size of the problem: mining 240 accounting variables produced 18,113 candidate
strategies of which 30.17 percent cleared a two-sigma threshold against a chance rate of 4.55
percent, and the paper recommends raising the bar to three sigma once many rules have been tried
(`2209.13623v3`). The same brief records the other failure mode: a single erroneous data row turned
two published betting returns of 17.29 and 28.82 percent into losses of 7.36 and 6.31 percent
(`2306.01740v4`), and the corrected strategy then earned nothing on three further years of cleaned
data. A calendar rule lives on a handful of dates, so one bad row and one unreported trial count are
enough to make it or break it.

## How this project relates to it

This repository does not implement a calendar strategy. The closest thing is its research on how many
rules were tried, in the brief
[Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
the single most relevant document to this category. It gives the trial-count arithmetic that decides
whether a monthly pattern is real, the data-quality rule that a single bad row must be caught before
a model sees it, and the protocol a search must record: the dataset and its date range, the full
candidate list and the number of trials, the significance policy and the false-discovery rate it
implies, the in-sample and out-of-sample periods, and the metric used to rank candidates.

The repository's own [sector rotation study](../../../strategies/sector_rotation_strategies.md) is
the second link: its 1,022-rule experiment found that 132 rules beat buy and hold while the average
rule returned 0.86 percent a month against 0.89 percent for simply holding the market, and the authors
read the scattered winners as the outcome of so many tries.

The finished tutorial
[turn of the month](../../quantconnect/turn-of-the-month-in-equity-indexes/README.md) covers the
month-end window in detail, including the test designed not to favour any particular within-month
pattern, and the [pre-holiday effect](../../quantconnect/pre-holiday-effect/README.md) and
[same calendar month](../../quantconnect/seasonality-effect-based-on-same-calendar-month-returns/README.md)
pages cover two more calendars of the same family.

## Where it goes wrong

- There are many calendars, so one will always look special. Every month, weekday, holiday and
  quarter boundary has been tested, and the best of them on one sample is not the best on the next.
  A rule that reports its winner without the number of candidates has reported an undefined
  threshold.
- The days were chosen after the fact. Whether the window starts four days before a month end or on
  the last day, whether the stop is 1.5 or 2 percent, and whether October is weighted 1.2 or 1.0 are
  all free choices, and the version that gets published is the one that looked best in the sample.
- The story usually fails its own test: Xu and McConnell rejected the payday explanation for the turn
  of the month and kept the pattern, and Sell in May has several competing explanations and no agreed
  one.
- Being out of the market is a decision with a cost: Sell in May skips six months of the year, and in
  a rising market those are gains forgone.
- The samples are narrow: one metal, three windows, and two implementations of the same idea for
  cross-checking, where the turn-of-month evidence proper comes from equity indexes over a century.
- One bad row can make the rule. A calendar rule rests on a small number of dates, so a single wrong
  price on a single entry day can move the whole result from a loss to a profit; the betting study in
  the brief is the clean demonstration, and the fix is a data-quality check before the model sees the
  row.

## Try it yourself

You need a spreadsheet and twenty years of daily closing prices for any index fund or metal.

1. Put the date in one column and the close in the next.
2. Add a column `month` with the month number, and a column `held` that says yes when the month is 11,
   12, 1, 2, 3 or 4 and no otherwise.
3. Add a column `next_close` that is the next row's close, and a column `daily_return` that is
   `next_close / close - 1`.
4. In a separate small table, average `daily_return` separately for the held days and the not-held
   days, then multiply each by the number of trading days in a year to put them on the same footing.
5. Do the same exercise with `held` redefined to a single month, once for each of the twelve months,
   and write down all twelve averages, then repeat it for each weekday.

What to notice: at least one month and at least one weekday will look meaningfully better than the
others, in every dataset you try. That is the whole difficulty. Then look at how much the best month
moves when you use a different twenty-year window, or a different fund. If the winner changes when
the sample changes, you have found a fact about your sample rather than about the calendar.

## Where this came from

- [The Strategy Compendium, article 08, calendar effects](https://backtrader.readthedocs.io/en/latest/strategies-series/en/08-calendar-effects.html),
  the category inventory, the three deep dives and every performance figure quoted above.
- [Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's brief, including the 18,113-strategy mining result (`2209.13623v3`) and the
  single-bad-row correction study (`2306.01740v4`).
- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), this
  repository's 1,022-rule experiment.
- Bouman and Jacobsen, [The Halloween Indicator, Sell in May and Go Away](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=300700),
  the Sell in May study across thirty-seven countries.
- Xu and McConnell, [Equity Returns at the Turn of the Month](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=917884),
  the turn-of-month study, whose tests rejected the cash-flow explanation.
- [Turn of the month](../../quantconnect/turn-of-the-month-in-equity-indexes/README.md), the finished
  tutorial on the same window in this collection.

## Words used in this tutorial

- anomaly: a pattern in prices that the usual explanations do not account for.
- buy and hold: buying an index or an asset and keeping it, rather than trading in and out.
- drawdown: the fall from a peak to the following low, measured in percent.
- false discovery rate: the share of findings called significant that are in fact chance results.
- reward for risk (Sharpe ratio): the average return divided by how much it wobbled, usually written
  per year; zero means no reward once the wobble is counted.
- trading day: a day the market is open and prices are recorded.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
