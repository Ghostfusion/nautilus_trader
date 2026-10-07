# Earnings announcements with a share buyback: buying when managers put their money where the profit is

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                               |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of American companies that both announced a buyback and are about to report profits                                                                                                                          |
| How often it trades       | Positions are opened and closed every few weeks, whenever a qualifying company reports                                                                                                                              |
| What you need             | A spreadsheet, a list of buyback announcements and a list of earnings dates                                                                                                                                         |
| Where the rules come from | [Quantpedia, earnings announcements combined with stock repurchases](https://quantpedia.com/strategies/earnings-announcements-combined-with-stock-repurchases/), the page the list's implementation is written from |
| The underlying research   | Amini and Singal, [Are Earnings Predictable?](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2589966)                                                                                                          |
| How well it held up       | Mixed: the effect is clear in one long sample, but the headline reward is before costs and the strategy has no short leg to lean on                                                                                 |
| Also appears in           | Nothing else in this collection describes the buyback filter; [standardized unexpected earnings](../../quantconnect/standardized-unexpected-earnings/README.md) is the same event without the buyback condition     |

## The idea in one paragraph

A buyback is a decision by a company to use its cash to buy back its own shares, which reduces the
number of shares in existence and raises the value of each remaining one. Companies may announce a
buyback shortly before they report their quarterly profit, and because the managers know the profit
before the public does, the timing of the announcement can be a hint that the results will be good.
This strategy finds companies that announced a buyback in the few weeks just before their report,
buys their shares ten days before the report, and holds them for about five weeks afterwards. It
holds only as long as a qualifying report is nearby, so it is in the market only part of the time.

## Why anyone believed it

A company's managers see the accounts before anyone else. If they expect a strong quarter, they have
a reason to announce a buyback in the weeks before the report, so the news of the buyback and the
news of the profit land close together and flatter the shares. If they expect a weak quarter, they
do the opposite: they issue new shares and raise money while the price is still high. In both cases
the timing of a voluntary corporate action is information.

The counterparty is the investor who treats a buyback announcement as routine news and does not
connect it to the coming report, and the investor who sells into the announcement without waiting
for the profit figures. Managers can move a buyback announcement by a few weeks, so the pairing of a
buyback with a report is not an accident, and anyone who ignores the pairing gives the trade its
edge. The study's authors found that the market reaction to earnings after a buyback announcement is
larger than the reaction after a share sale, which is consistent with managers timing both events.

## An everyday comparison

Think of a restaurant owner who knows the health inspector is coming next month. If the kitchen is
in good shape, the owner schedules a fresh coat of paint and a public reopening just before the
inspection, so customers see the improvements and the good report together. If the kitchen is in
trouble, the owner delays the visible changes. Watching the timing of the paint job, rather than the
paint itself, is what this strategy does.

## The rules, step by step

1. Start with all shares listed on the New York, American and Nasdaq exchanges. Drop companies
   without a market value and drop the smallest quarter of companies by market value, which removes
   the very small names where prices are hardest to trust.
2. Keep a list of buyback announcements, recording for each company the day it announced and the
   number of shares it said it would buy. The strategy requires the announced buyback to be at least
   5 percent of the company's outstanding shares, a large programme rather than a token one.
3. Keep a list of the days on which each company reports its profit.
4. For each company, look at the window that starts 30 trading days before its report and ends 15
   trading days before it. If the company announced a buyback of at least 5 percent inside that
   window, it qualifies.
5. Enter the trade 10 trading days before the report. The list's implementation holds a maximum of 40
   companies at once; when more than 40 qualify, it takes the first ones it finds, which is a
   practical limit rather than part of the original rule.
6. Hold each position until 15 trading days after the report, then sell. Each position is therefore
   held for about 25 trading days.
7. Weight every holding equally, and check each day whether a new qualifying company has appeared or
   an existing one has reached its exit date.

Two things are worth separating. The original paper also describes a short leg, selling shares of
companies that raised money by issuing new shares before a report, because that timing is expected to
precede weak results. The list's implementation keeps only the long half, and this tutorial describes
that long-only version.

## The maths, with every symbol named

There is no ranking formula here; the rule is a set of dates. The only calculation is the return of
one position and the cost of the round trip.

A qualifying company's position return over the hold:

```text
R_i = Price_exit / Price_entry - 1
```

- `R_i` is the return of position `i`, written as a decimal: 0.05 is 5 percent.
- `Price_entry` is the price 10 trading days before the report.
- `Price_exit` is the price 15 trading days after the report.

The account's return over a period is the average of its positions, because they are equally
weighted:

```text
R = (R_1 + R_2 + ... + R_n) / n
```

- `R_1` to `R_n` are the returns of the `n` positions held.
- Dividing by `n` is the same as giving each position a weight of `1 / n`.

The cost of one position is paid twice, once on the way in and once on the way out:

```text
Cost_i = 2 * c
```

- `c` is the cost of one side as a fraction of the amount traded, covering the spread between the
  buying and selling price plus any commission. A realistic figure for large American shares is
  0.001, that is 0.10 percent.

Because the strategy is long-only, there is no borrow fee. The account is fully invested in shares
whenever it holds positions, so it also carries the ordinary risk of the share market.

## A worked example

Six companies qualified in one quarter. Each announced a buyback between 30 and 15 trading days
before its report. The prices are invented but plausible.

| Company | Days before report the buyback was announced | Buyback as a share of outstanding shares | Price 10 days before report | Price 15 days after report | Return |
| ------- | -------------------------------------------- | ---------------------------------------- | --------------------------- | -------------------------- | ------ |
| A       | 22 days before                               | 6 percent                                | 50.00                       | 52.50                      | 5.0%   |
| B       | 18 days before                               | 8 percent                                | 20.00                       | 20.60                      | 3.0%   |
| C       | 15 days before                               | 5 percent                                | 100.00                      | 104.00                     | 4.0%   |
| D       | 16 days before                               | 7 percent                                | 35.00                       | 35.35                      | 1.0%   |
| E       | 19 days before                               | 5 percent                                | 60.00                       | 63.00                      | 5.0%   |
| F       | 21 days before                               | 10 percent                               | 10.00                       | 10.30                      | 3.0%   |

Every one of the six has a return, so the average position return is:

```text
R = (5.0 + 3.0 + 4.0 + 1.0 + 5.0 + 3.0) / 6 = 21.0 / 6 = 3.5 percent
```

Each position pays two sides of cost, so the net return of a position is about 3.5 percent minus
2 times 0.10 percent:

```text
Cost_i = 2 * 0.001 = 0.002, that is 0.20 percent
Net average = 3.5 - 0.20 = 3.3 percent per position
```

Companies report about four times a year, so if the same kind of result repeated each quarter the
account would make roughly four times 3.3 percent, about 13.2 percent a year before compounding. That
is below the published figure below, which is what one would expect from a small made-up example; the
point of the table is to show that the cost of each round trip, 0.20 percent, is not small next to a
3.5 percent move. It also shows that the money is idle between reports, because the strategy holds
nothing unless a qualifying report is near.

## What the research actually found

| Source                                       | What it measured                                                                        | Result                                                                                                                                                                           |
| -------------------------------------------- | --------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Amini and Singal, the paper behind the rules | American shares, 1987 to 2013, the window from 10 days before to 15 days after a report | The reaction to earnings after a buyback announcement was 4.56 percent higher than after a share sale, over that window                                                          |
| Quantpedia, summarising that paper           | The long-only buyback version of the same sample                                        | An average return of 5.78 percent per announcement, which is 25.2 percent a year across four reports a year, with a reward-to-risk ratio of 2.27 and volatility of 11.11 percent |
| The list's own replication record            | Its whole catalogue of 4,843 coded papers                                               | The median strategy carries a reward-to-risk ratio of 0.37 and the median test window is 34 years; this strategy's 2.27 is far above that, but the figure is before costs        |

The effect the authors measured is real in their sample: the gap between companies that bought back
shares and companies that sold new ones is 4.56 percent over a 25-day window. The number that needs
care is the 25.2 percent a year. That figure is the arithmetic sum of four announcement periods and
is before trading costs; a strategy that trades in and out of a fresh set of names four times a year
pays the spread on every one of those round trips, and the 2.27 reward-to-risk ratio is high enough
that it is more likely to describe a measurement before costs than a result that survives them. The
list's headline table of its 61 strongest replications does not include this strategy.

## How this project relates to it

The repository's brief on execution and the order book,
[strategies/books2/25_execution_impact_and_order_book.md](../../../strategies/books2/25_execution_impact_and_order_book.md),
has a section on corporate buyback execution. It reports that a single buyback in one study paid
roughly 8.5 percent in fees on a programme worth 184 million pounds, and that many buybacks are
measured against a benchmark that can be gamed by the broker doing the buying. That is the other side
of the coin: the company doing the buyback is the one paying those costs, and this strategy is buying
shares from names that have just committed to paying them.

The completed tutorial on the same event,
[standardized unexpected earnings](../../quantconnect/standardized-unexpected-earnings/README.md),
trades on how far the reported profit beat the expectation. The buyback filter here is a different
way of guessing the same thing before the report arrives.

## Where it goes wrong

- The sample is one study. Amini and Singal used American shares from 1987 to 2013, and the 4.56
  percent gap has not been replicated here on other markets or later data.
- Costs are ignored in the headline. The 25.2 percent a year does not subtract the spread on four
  round trips a year, and for small companies that spread is wide.
- Small companies are dropped, but they are also where buybacks are most informative. Removing the
  smallest quarter makes the strategy more tradable and removes part of the signal.
- The classification is fragile. A buyback is announced on one day; whether it sits in the 30-to-15
  day window depends on the report date, which companies sometimes move by a week or two.
- Forty names is a limit, not a rule. When more than forty qualify, which ones are held is decided by
  the order they are found, not by any principle, so the result is not reproducible in a strict sense.
- Long only means market risk. When the whole market falls, the account falls with it, and the page's
  own warning is that a large part of the performance is the ordinary reward for holding shares.

## Try it yourself

You need a spreadsheet and two public lists: the dates on which large companies reported profits,
and the dates they announced buybacks.

1. Build one row per company, with columns for the report date and, for each buyback announcement,
   the date and the percentage of shares involved.
2. Add a column that marks a company as qualifying if a buyback of 5 percent or more was announced
   between 30 and 15 trading days before the report.
3. Add columns for the price 10 trading days before the report and 15 trading days after it.
4. Compute each qualifying company's return and write down the average.
5. Divide the year into four quarters and repeat for each, so you have a return per quarter.
6. Subtract 0.20 percent from every position for the round trip, and remember that the money earns
   nothing between reports.

What to notice: the number of qualifying companies jumps around from quarter to quarter, and in some
quarters there may be none. The periods when the strategy is empty matter as much as the periods when
it trades, because an account that sits in cash for part of the year cannot match a figure that
assumes it is always deployed.

## Where this came from

- [Quantpedia, earnings announcements combined with stock repurchases](https://quantpedia.com/strategies/earnings-announcements-combined-with-stock-repurchases/),
  the page the list's implementation is written from, and the source of the 5.78 percent, the 25.2
  percent, the 11.11 percent volatility and the 2.27 reward-to-risk ratio.
- Amini and Singal, [Are Earnings Predictable?](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2589966),
  the source of the buyback window, the size filter and the 4.56 percent gap.
- [strategies/books2/25_execution_impact_and_order_book.md](../../../strategies/books2/25_execution_impact_and_order_book.md),
  this repository's brief on buyback execution costs.
- The implementation the list carries:
  [earnings-announcements-combined-with-stock-repurchases.py](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/earnings-announcements-combined-with-stock-repurchases.py),
  which states the buyback window, the 40-name limit and the daily rebalancing.

## Words used in this tutorial

- buyback: a company buying its own shares, which reduces the number of shares and raises the value
  of each one left.
- earnings announcement: the scheduled quarterly report in which a company states its profit.
- long: owning something, so that a rise in its price makes money.
- market value: a company's share price multiplied by its number of shares.
- position: one holding in the account.
- spread: the gap between the price at which something can be bought and the price at which it can be
  sold.
- trading day: a day on which the exchange is open, which excludes weekends and public holidays.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
