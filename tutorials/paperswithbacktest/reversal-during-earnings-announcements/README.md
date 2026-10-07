# Reversal during earnings announcements: buying the shares that just fell and betting against the ones that just jumped

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                               |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of the biggest American companies, bought or borrowed for a few days around their profit report                                                                              |
| How often it trades       | Every few days, as companies report; each position is held for three days                                                                                                           |
| What you need             | A spreadsheet, a list of report dates and the last week of prices for each company                                                                                                  |
| Where the rules come from | [Quantpedia, reversal during earnings announcements](https://quantpedia.com/strategies/reversal-during-earnings-announcements/), the page the list's implementation is written from |
| The underlying research   | So and Wang, [News-Driven Return Reversals: Liquidity Provision Ahead of Earnings Announcements](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2275982)                       |
| How well it held up       | Mixed: the in-sample effect is large and appears in more than one study, but the vendor's own out-of-sample run of the same rules was slightly negative                             |
| Also appears in           | [Short-term reversal](../../quantconnect/short-term-reversal/README.md), the same bounce-back idea with no earnings date attached                                                   |

## The idea in one paragraph

Prices often overshoot in the days just before a company reports its profit. A share that has jumped
in the week before the report tends to give some of that move back in the three days around the
report, and a share that has fallen tends to bounce. This strategy measures each big company's return
over the three days ending two days before its report, then buys the shares that fell the most and
sells short the shares that rose the most, holding both for the three days around the report. It is a
bet that the pre-report move was noise rather than news, and that the report brings the price back
towards where it started. Every position is closed after three days.

## Why anyone believed it

Someone has to be on the other side of a trade, and before an announcement the people taking the
other side are the market makers, the firms that stand ready to buy and sell all day. Holding a
position through a report is risky for them, because a single number can move the price sharply and
their inventory can turn against them overnight. To be paid for that risk, they widen the prices at
which they are willing to trade, and that widening pushes the share further in the direction it was
already moving.

The counterparty, then, is the market maker managing inventory, not an investor with a view. The
pre-report push is a temporary price for bearing risk, and it should unwind once the risk of the
report has passed. The study behind the rules measured the unwinding directly: it treats the size of
the reversal as the price market makers were charging. If that compensation shrinks, for example
because more firms compete to provide liquidity before reports, the effect shrinks with it.

## An everyday comparison

Think of a taxi rank outside a station at the moment a delayed train arrives. For a few minutes the
fare is higher because the drivers know they can charge it, and the queue of passengers is long. Once
the crowd clears, the fare drops back to normal. The higher fare was not about the journey at all; it
was the price of a temporary shortage. This strategy sells at the peak fare, so to speak, and buys
back after the crowd has gone.

## The rules, step by step

1. Start with shares listed on the New York, American and Nasdaq exchanges that have daily prices.
2. Sort all of them into five equal groups by company size, measured by market value. Keep only the
   largest group. The study restricts the test to big companies because their shares are easier to
   trade, and the fixed 20 long and 20 short slots in the implementation are chosen from this group.
3. For a company reporting on day `t`, look at its return over the three days from `t-4` to `t-2`.
   That is the price change from four days before the report to two days before it. Call this the
   pre-report return.
4. Sort the companies that are due to report within the next two days by their pre-report return.
   Buy the bottom fifth, the ones that fell the most, and sell short the top fifth, the ones that
   rose the most.
5. Hold each position for the three days from `t-1` to `t+1`: the day before the report, the report
   day, and the day after. Then close it.
6. Weight every position equally within its leg. The implementation holds at most 20 longs and 20
   shorts at any moment, so if more companies qualify it takes the first ones that appear.
7. Repeat continuously, since large companies are always reporting somewhere. The study assumed about
   ten such episodes in a quarter.

## The maths, with every symbol named

The pre-report return, the quantity that decides which side each company goes on:

```text
P = Price_(t-2) / Price_(t-4) - 1
```

- `P` is the pre-report return, written as a decimal: -0.06 is a fall of 6 percent.
- `Price_(t-2)` is the share price two trading days before the report.
- `Price_(t-4)` is the share price four trading days before the report.

Companies are ranked by `P`; the lowest values go in the long leg and the highest in the short leg.

The return of one position over the three-day hold:

```text
R_i = Price_(t+1) / Price_(t-1) - 1
```

- `R_i` is the return of position `i` over the hold.
- `Price_(t-1)` is the price the day before the report, when the position is opened.
- `Price_(t+1)` is the price the day after the report, when the position is closed.

The long-short account's return for one episode, with both legs fully invested and equally weighted:

```text
R = average(R over the long leg) - average(R over the short leg)
```

- The first term is the average return of the shares that were bought.
- The second term is the average return of the shares that were borrowed; subtracting it means a fall
  in a shorted share adds to the account.

The cost of one episode, for a book holding one unit of money long and one unit short:

```text
Cost = 4 * c + 3 * (d / 365)
```

- The factor 4 counts the four trades: opening and closing the long leg, and opening and closing the
  short leg.
- `c` is the cost of one side, about 0.001, that is 0.10 percent, for large American shares.
- `d` is the yearly fee for borrowing the shorted shares, for example 0.006, that is 0.6 percent a
  year, charged for the three days the short is held.

## A worked example

Eight large companies report on the same day, so they compete for the 20 long and 20 short slots
together. The prices are invented but the size of the moves is realistic.

| Company | Return from t-4 to t-2 | Side taken | Return from t-1 to t+1 |
| ------- | ---------------------- | ---------- | ---------------------- |
| A       | -6.0 percent           | long       | +0.9 percent           |
| B       | -4.5 percent           | long       | +1.1 percent           |
| C       | -3.0 percent           | long       | +0.8 percent           |
| D       | -1.5 percent           | long       | +0.5 percent           |
| E       | +1.5 percent           | short      | -0.4 percent           |
| F       | +3.0 percent           | short      | -0.9 percent           |
| G       | +5.0 percent           | short      | -1.2 percent           |
| H       | +7.0 percent           | short      | -1.5 percent           |

With only eight companies the two legs would each hold about four names. The long leg's average
return is:

```text
Long average = (0.9 + 1.1 + 0.8 + 0.5) / 4 = 3.3 / 4 = 0.825 percent
```

The short leg's average return is negative, which is what the strategy wants, because a short
position gains when the price falls. The account's gain from the short leg is the negative of the
average:

```text
Short average = (-0.4 - 0.9 - 1.2 - 1.5) / 4 = -4.0 / 4 = -1.0 percent
Account gain from shorts = +1.0 percent
```

So the episode's gross return, holding one dollar long and one dollar short, is:

```text
R = 0.825 percent + 1.0 percent = 1.825 percent
```

Now the costs. Four trades of one dollar each at 0.10 percent per side, plus the borrow fee for three
days:

```text
Trading cost = 4 * 0.001 = 0.004, that is 0.40 percent of the one dollar of capital
Borrow cost  = 3 * (0.006 / 365) = 0.0000493, under 0.01 percent
Net episode  = 1.825 - 0.40 - 0.01 = 1.415 percent
```

So the gross return of about 1.83 percent becomes about 1.42 percent after costs, and the trading
cost is the largest single item after the return itself. That is the pattern the published figure
below describes: a per-episode return near 1.5 percent, against a random-period baseline near 0.2
percent, before the roughly 0.4 percent round-trip cost is subtracted.

## What the research actually found

| Source                                  | What it measured                                                           | Result                                                                                                                                                                                                                 |
| --------------------------------------- | -------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| So and Wang, the paper behind the rules | American shares, 1996 to 2011, the three-day window around the report      | The long-short return was 1.45 percent per episode, six times the 0.22 percent the same rule earned in randomly chosen periods                                                                                         |
| Quantpedia, summarising that paper      | The same rule, big companies only                                          | An annualised return of 6.5 percent, volatility of 3.76 percent, a reward-to-risk ratio of 1.73, and a worst fall of 65.88 percent, with the annual figure framed as an arithmetic average over ten episodes a quarter |
| Quantpedia's reliability note           | The vendor's own out-of-sample backtest of the same rules                  | The vendor records that this out-of-sample run was slightly negative and states that the effect appears to be deteriorating after the sample period                                                                    |
| Jansen and Nikiforov, a related Study   | Companies with extreme returns in the week before a report, two-day window | A reversal strategy was profitable in 40 of the last 42 years and earned more than 1.3 percent over two days                                                                                                           |

The 1.45 percent per episode is the strongest number and it comes with a clean comparison: the same
rule applied to periods chosen at random earned only 0.22 percent. That baseline is what makes the
result worth taking seriously, because it rules out the possibility that any short-term reversal rule
would have done as well. The warning is equally clear. The vendor's own out-of-sample run was
slightly negative, and the worst fall of 65.88 percent shows that a strategy earning a fraction of a
percent per episode can still lose most of the account when the reversals stop arriving. The 6.5
percent a year is the vendor's arithmetic summary, not a compounded return, and it is before costs.

## How this project relates to it

The completed tutorial on the plain version of this idea,
[short-term reversal](../../quantconnect/short-term-reversal/README.md), buys shares that fell and
sells shares that rose without asking whether a report is due. It is the same bounce, measured
without the earnings date, and reading it first makes the earnings version easier to place: the
report is what concentrates the effect into a few days.

The repository's brief on text and earnings signals,
[strategies/books2/11_language_models_news_and_text.md](../../../strategies/books2/11_language_models_news_and_text.md),
notes that realised weekly returns show short-term reversals, which is exactly the pattern this
strategy trades. It also shows that attention around earnings is a measurable thing, which is the
bridge between the two halves of this tutorial.

## Where it goes wrong

- The effect decays. The vendor's own out-of-sample test was slightly negative, which is the
  clearest single warning. A signal that worked from 1996 to 2011 need not work afterwards.
- Cost is a large slice of the return. The per-episode gross return is near 1.5 percent and the
  round trip costs about 0.4 percent, so a small change in the spread or the borrow fee changes the
  answer materially.
- The short leg is fragile. Some of the shares that rose the most before a report are hard to borrow,
  and the fee can exceed the return the short leg is chasing.
- The size filter is doing a lot of work. The study restricts the test to the largest companies, and
  the effect is different in smaller ones, where the analogy of a single market maker breaks down.
- The pre-report window is a choice. Three days ending two days before the report is one definition;
  a different window measures a different thing, and choosing the best window after seeing the
  results is how a rule is fitted rather than discovered.
- A high worst fall sits under a modest average. With a reward-to-risk ratio of 1.73 but a worst fall
  of 65.88 percent, the shape of the returns is not symmetric, and a bad stretch can arrive before
  the average has time to show itself.

## Try it yourself

You need a spreadsheet, a list of report dates for large companies, and daily closing prices.

1. Build one row per company per report, with columns for the report date and for the prices on days
   `t-4`, `t-2`, `t-1` and `t+1`.
2. Add a column for the pre-report return: the `t-2` price divided by the `t-4` price minus one.
3. Add a column for the hold return: the `t+1` price divided by the `t-1` price minus one.
4. For each report day, sort the companies due to report in the next two days by pre-report return,
   and mark the bottom fifth as long and the top fifth as short.
5. Average the hold return of the long marks and the hold return of the short marks, and subtract the
   second from the first to get the episode return.
6. Subtract 0.4 percent from the episode for the round trip, and repeat for a year of report days.

What to notice: the average episode return of your sheet will vary a lot from one report day to the
next, and the cost is large enough that many single episodes lose money after it. If you split the
data into an earlier half and a later half, you may find the effect stronger in one than the other,
which is the pattern the vendor's out-of-sample note describes.

## Where this came from

- [Quantpedia, reversal during earnings announcements](https://quantpedia.com/strategies/reversal-during-earnings-announcements/),
  the page the list's implementation is written from, and the source of the 6.5 percent, the 3.76
  percent volatility, the 1.73 ratio, the 65.88 percent fall and the out-of-sample note.
- So and Wang, [News-Driven Return Reversals: Liquidity Provision Ahead of Earnings Announcements](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2275982),
  the source of the 1.45 percent episode return, the 0.22 percent baseline and the market-maker
  explanation.
- The implementation the list carries:
  [reversal-during-earnings-announcements.py](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/reversal-during-earnings-announcements.py),
  which states the size sort, the pre-report return, the three-day hold and the 20-name leg limits.
- [strategies/books2/11_language_models_news_and_text.md](../../../strategies/books2/11_language_models_news_and_text.md),
  this repository's brief on earnings text and short-term reversals.
- [standardized unexpected earnings](../../quantconnect/standardized-unexpected-earnings/README.md),
  a completed tutorial on the drift that follows a report without the reversal.

## Words used in this tutorial

- earnings announcement: the scheduled quarterly report in which a company states its profit.
- long: owning something, so that a rise in its price makes money.
- market maker: a firm that stands ready to buy and sell a share throughout the day, holding
  inventory of it.
- reversal: a price move that goes back the way it came after an earlier move.
- short selling: borrowing something you do not own, selling it now, and buying it back later, which
  makes money if the price falls.
- spread: the gap between the price at which something can be bought and the price at which it can be
  sold.
- trading day: a day on which the exchange is open.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
