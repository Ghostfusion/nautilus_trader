# Momentum in stocks: buying the shares that have already been rising

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                 |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of the largest American companies, chosen by how much their price rose over the past twelve months                                                                                                                                             |
| How often it trades       | About once a month, when the holding list is rebuilt                                                                                                                                                                                                  |
| What you need             | A spreadsheet and a year of share prices for a list of large companies                                                                                                                                                                                |
| Where the rules come from | [QuantConnect strategy library, momentum effect in stocks](https://www.quantconnect.com/tutorials/strategy-library/momentum-effect-in-stocks) and the [Quantpedia entry](https://quantpedia.com/strategies/momentum-factor-effect-in-stocks) it cites |
| The underlying research   | Asness, Frazzini, Israel and Moskowitz, [Fact, Fiction and Momentum Investing](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2435323)                                                                                                           |
| How well it held up       | Strong: the same tendency has been measured across many countries and about a century of data, but the plain long-and-short version has suffered a fall of more than 80 percent in 2009 and its profit shrinks once trading costs are paid            |
| Also appears in           | [Sector momentum](../sector-momentum/README.md), which applies it to whole industries, and [Momentum effect in country equity indexes](../momentum-effect-in-country-equity-indexes/README.md), which applies it to national stock markets            |

## The idea in one paragraph

Some companies' shares rise a great deal over a year, others barely move, and others fall. This
strategy picks out the ones that rose the most and buys them, in equal amounts, then holds them for
a month. At the start of the next month it looks again at the full list, sells whatever has dropped
out of the winners, buys whatever has climbed in, and repeats. The bet is that a share that has
been rising tends to keep rising for a while, so the money sits in the parts of the market that are
already moving up. This tutorial describes the plainest, long-only form of the idea: buy the
winners, do not sell anything short.

## Why anyone believed it

News does not reach everyone at once. When a company reports several good quarters, some investors
buy on the first headline, others wait for the next confirmation, and pension funds and index funds
buy slowly as money arrives. Each of those buyers pushes the price up a little more, so a rise that
began for a real reason can continue for months.

The people on the other side have their own reasons to sell. A fund facing withdrawals must raise
cash and sells whatever it can, not what it would prefer. A manager trims a holding that has grown
too large after a big run. An investor takes a profit simply because the price has risen. If those
sellers keep appearing, a winner can keep winning even though there is nothing new in the news.

## An everyday comparison

Think of a song climbing a music chart. A radio station plays a new track a few times and it starts
to move up. Other stations see it moving up and add it to their playlists, which pushes it higher,
which persuades yet more stations. For a few weeks the song keeps climbing because each new player
is reacting to the climb rather than to the song. The same thing can happen to a share price: the
rise itself becomes the reason more people buy. The strategy here is to back the songs that are
already near the top of the chart, on the bet that the climb is not finished.

## The rules, step by step

1. Build a list of the fifty largest American companies by market value, where market value is the
   share price multiplied by the number of shares the company has issued. Refresh this list at the
   start of each month. The library page calls this the large-company universe.
2. For each company, work out its return over the past twelve months: take the price twelve months
   ago, take today's price, and divide. A share that went from 200.00 to 260.00 has a twelve-month
   return of 30 percent.
3. Put the fifty companies in order, best twelve-month return first.
4. Buy the best ten of them, in equal amounts: one tenth of the money in each. The library page
   does not state a fixed number to hold; this tutorial uses ten of fifty, which is the top fifth,
   and the worked example below uses the same one-in-five fraction on a shorter list.
5. Hold for one month. Do not buy or sell in between.
6. At the start of the next month, repeat steps 2 to 4 on the refreshed list. Sell anything that
   has dropped out of the best ten and buy whatever has taken its place.
7. Optional refinement, used by many researchers and written as 12-1 momentum: measure the return
   from twelve months ago to one month ago, skipping the most recent month. Over days and weeks
   prices tend to bounce back after a sharp move, which works against the signal, so the most recent
   month is dropped.

## The maths, with every symbol named

The whole strategy is one calculation repeated for each company, one sort, and a cost line.

The twelve-month return, called the momentum score:

```text
M = P_today / P_twelve_months_ago - 1
```

- `M` is the momentum score of the company, as a decimal: 0.30 means 30 percent.
- `P_today` is the company's share price today.
- `P_twelve_months_ago` is its share price on the same day twelve months earlier.

Rank the companies by `M`, largest first, and keep the top ten. Give each of the ten the same
weight:

```text
w_i = 1 / 10 for each of the ten selected companies, and 0 for the other forty
```

- `w_i` is the fraction of the money placed in company `i`.
- The ten weights add up to 1, so the whole account is invested, in ten equal slices.

The portfolio's return over the following month is the average of the ten returns, because the
slices are equal:

```text
R_portfolio = (R_1 + R_2 + ... + R_10) / 10
```

- `R_1` to `R_10` are the next-month returns of the ten chosen companies.
- Dividing by ten is the same as multiplying each return by one tenth and adding them up.

The cost of rebuilding the list each month:

```text
Cost = t * c
```

- `t` is the traded fraction: 2.0 when the entire portfolio is sold and replaced at once, because
  selling the old holdings and buying the new ones counts twice, and less than 2.0 when some
  holdings are kept.
- `c` is the cost per trade as a fraction of the amount traded, covering the gap between the buying
  and selling price plus commission. For large American shares a realistic figure is 0.0005 to
  0.001, that is five to ten basis points, where one basis point is one hundredth of one percent.

## A worked example

The table below uses a made-up list of ten large companies, standing in for the real fifty. The
rule is to hold the best fifth, which here means the best two. The returns are invented but are of
the size large-company returns actually take over a year.

| Company | Price a year ago | Price today | M              | Rank |
| ------- | ---------------- | ----------- | -------------- | ---- |
| Apex    | 200.00           | 260.00      | +0.30 (30 pct) | 1    |
| Beacon  | 50.00            | 62.00       | +0.24 (24 pct) | 2    |
| Cobalt  | 100.00           | 118.00      | +0.18 (18 pct) | 3    |
| Delta   | 80.00            | 92.00       | +0.15 (15 pct) | 4    |
| Echo    | 120.00           | 132.00      | +0.10 (10 pct) | 5    |
| Foxtrot | 60.00            | 63.00       | +0.05 (5 pct)  | 6    |
| Golf    | 150.00           | 150.00      | 0.00 (0 pct)   | 7    |
| Hotel   | 90.00            | 87.30       | -0.03 (3 pct)  | 8    |
| India   | 40.00            | 36.80       | -0.08 (8 pct)  | 9    |
| Juliet  | 70.00            | 63.00       | -0.10 (10 pct) | 10   |

The two chosen companies are Apex and Beacon, half the money in each. Now suppose the next month
produces these returns:

| Company held | Weight | Next-month return | Contribution  |
| ------------ | ------ | ----------------- | ------------- |
| Apex         | 0.50   | +2.0 percent      | +1.00 percent |
| Beacon       | 0.50   | -1.0 percent      | -0.50 percent |
| Total        | 1.00   |                   | +0.50 percent |

So the portfolio gained 0.50 percent before costs. Now suppose that when the list is rebuilt, Apex
stays in the top two but Beacon drops out and a new company enters. One of the two holdings is sold
and one is bought, which is half the money traded on each side:

```text
t = 2 * (1 / 2) = 1.0
Cost = 1.0 * 0.001 = 0.001, that is 0.10 percent
Net return for the month = 0.50 - 0.10 = 0.40 percent
```

If that same figure repeated every month for a year it would compound to about 6.2 percent before
costs and 4.9 percent after. Two things are worth noticing. First, the cost line is not small: a
list that changes half its holdings every month pays roughly 1.2 percent a year at ten basis points
per trade. Second, the example says nothing about whether the strategy works. It only shows how to
apply the rules and how the arithmetic behaves.

## What the research actually found

Momentum is one of the most studied effects in finance, and the record is long.

| Source                                          | What it measured                                                          | Result                                                                                                                                                                          |
| ----------------------------------------------- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Quantpedia, summarising the momentum literature | A portfolio long the strongest past-year performers and short the weakest | 8.3 percent a year, volatility 16.6 percent, worst fall 87.41 percent, reward-to-risk 0.5, over 1927 to 2013                                                                    |
| Jegadeesh and Titman                            | American shares ranked over 3 to 12 months                                | Shares that did best over 3 to 12 months tended to keep doing well over the next 3 to 12 months, and such rules were profitable in the United States and most developed markets |
| Quantpedia's own notes on the literature        | The long-and-short version and its crashes                                | The pure version fell more than 80 percent in 2009; the long side has been more profitable than the short side, and managing the risk nearly doubles the reward-to-risk ratio   |
| Choi, Choi and Kang, on Korean large companies  | Monthly momentum on the KOSPI 200, 2000 to 2011, with costs               | The best rules returned about 2 percent a month even after 35 basis points per basket, but the very largest companies hindered the result (p.6, p.9)                            |
| Baltussen, Dom, Van Vliet and Vidojevic         | Momentum across markets and up to about 150 years                         | The premium is sizable and robust to many design choices; the weakness that remains is crash risk                                                                               |

Read together, the picture is this. There is a real, replicated tendency for strong shares to stay
strong over months, and it is far better documented than most patterns. But the size of the prize
depends sharply on how the rule is written, the plain long-and-short form has lost most of a year's
gains in a single bad year, and trading costs take a visible slice. A long-only momentum list is, in
the end, still a way of being invested in the stock market.

## How this project relates to it

This repository contains a study of the question in [Profiting from sector
rotation](../../../strategies/sector_rotation_strategies.md). Section 7 of that document collects the
finding that momentum in individual shares is largely momentum in their industries: raw (6,6)
momentum on individual shares returned 0.43 percent a month, but after subtracting the industry
return only 0.13 percent a month remained, and after adjusting for size and value only 0.08 percent
a month remained, with a t-statistic of 0.91, which is not distinguishable from chance.

The second related piece is the research brief
[the momentum design brief](../../../strategies/books2/08_predictability_and_trading_strategies.md).
It reads the momentum design literature and records that a momentum strategy's returns are
positively skewed by construction, that a binary plus-or-minus-one version of the signal is not
optimal, and that turnover and cost decide whether the paper profit survives.

## Where it goes wrong

- Momentum crashes. After a market-wide fall, the shares that fell furthest are often the ones
  that led the previous ranking, and they rebound hardest. A rule that bought them just before the
  fall takes the loss twice, which is where the published worst fall of more than 80 percent comes
  from.
- Crowding and decay. Once a rule is published and easy to trade, more money runs it. The buying
  happens earlier, the reward arrives sooner and smaller, and the latecomers pay for the reversal.
- It is mostly market exposure. The long-only version is invested in shares at all times, so it
  falls when the market falls. The measured improvement over simply holding the market is a few
  points a year, and part of that is the market's own rise.
- The number of rules tried. There are many ways to define momentum: which months to use, whether
  to skip the most recent one, how many shares to hold, how often to rebuild. Choosing the best of
  these after seeing the results is how a dozen small decisions become one large illusion.
- Costs on a monthly rebuild. Every rebuild pays the gap between buying and selling prices plus
  commission. The more the list changes, the more the strategy pays, and the short side is both
  expensive to borrow and hard to keep open.
- The signal can invert over short horizons. Over days to weeks prices tend to reverse rather than
  continue, which is why the 12-1 convention exists and why rules built on very short windows can
  lose money even though the twelve-month version earned it.

## Try it yourself

You need nothing but a spreadsheet and a public source of share prices; any finance website will
give you monthly closing prices for a list of large companies.

1. Build a sheet with one column per company and one row per month for the last five years.
2. Add a column that computes the twelve-month return: today's price divided by the price twelve
   rows up, minus one.
3. Add a column, for each month, naming the company or companies with the highest value in that
   row. This is what the rules would have bought at that moment.
4. In the next row down, average the next month's returns of the chosen companies. That is the
   strategy's return for the month.
5. Do the same for a market index on its own, so you have two columns to compare.
6. Finally, add a column that subtracts the cost: about 0.10 percent for each side of every position
   that changed from the previous month.

What to notice: on some rows the chosen shares are almost the same as the previous month, so little
trades and the cost is near zero, while on others the whole list changes. Over five years the result
will usually sit close to the index, sometimes above it and sometimes below. If your sheet shows
the strategy winning by a wide margin, the likely cause is that the company list changed over time
(the large companies of ten years ago are not the large companies of today), which is a form of
looking into the future.

## Where this came from

- [QuantConnect strategy library: momentum effect in stocks](https://www.quantconnect.com/tutorials/strategy-library/momentum-effect-in-stocks),
  the rules as implemented: a large-company universe, the twelve-month momentum score, equal
  weights and a monthly rebuild.
- [Quantpedia: momentum factor effect in stocks](https://quantpedia.com/strategies/momentum-factor-effect-in-stocks),
  the indicative performance, the instrument count and the list of underlying papers.
- Asness, Frazzini, Israel and Moskowitz, [Fact, Fiction and Momentum Investing](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2435323),
  the review that answers the common objections to the momentum idea.
- Choi, Choi and Kang, momentum universe shrinkage in price momentum, arXiv `1211.6517v1`, the
  large-company momentum returns, the transaction-cost figure and the liquidity section used here.
- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), this
  repository's own study, whose Section 7 gives the industry-share decomposition of individual
  stock momentum.
- [the momentum design brief](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's brief on momentum design and skewness.

## Words used in this tutorial

- drawdown: the fall from a peak in value to a later low, measured as a percentage of the peak.
- long: owning something, so that you gain when its price rises.
- market capitalisation: the total value of a company's shares, found by multiplying the share
  price by the number of shares.
- momentum: the tendency of something that has been rising to keep rising for a while.
- rebalance: adjusting a portfolio back to its intended weights by buying and selling.
- short: selling something you do not own, so that you gain when its price falls.
- universe: the full set of things a rule is allowed to choose from.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
