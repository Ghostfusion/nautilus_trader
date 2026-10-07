# Gold market timing: holding gold only when shares look cheap against government bonds

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                              |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | Gold, held through its price, and cash when the rule says to be out                                                                                                                                                                                                |
| How often it trades       | About once a month, when the rule is checked again                                                                                                                                                                                                                 |
| What you need             | A spreadsheet, the S&P 500 index level and yearly earnings, and the 10-year bond yield                                                                                                                                                                             |
| Where the rules come from | [QuantConnect strategy library, gold market timing](https://www.quantconnect.com/tutorials/strategy-library/gold-market-timing)                                                                                                                                    |
| The underlying research   | Ryan Daly, [Tactical Asset Allocation to Gold](https://ssrn.com/abstract=783187), built on the Fed model; the library cites a Quantpedia entry behind a paid login                                                                                                 |
| How well it held up       | Weak: one practitioner paper, a factor of two that no measurement justifies, a library backtest that could not trade because its data was withdrawn, and a broad test of gold timing rules in which this family did not survive a correction for trying many rules |
| Also appears in           | Nothing else in this collection, which describes no other gold strategy                                                                                                                                                                                            |

## The idea in one paragraph

Gold pays no interest and no dividend; it is only worth what someone will pay for it later. Shares
and government bonds both do pay something, so the choice of whether to hold gold can be framed as a
comparison between them. The rule here compares the yearly profit the stock market earns for each
dollar of its price with the yearly interest the government pays for each dollar lent to it. When the
stock market's profit is at least twice the bond interest, the rule says shares are so cheap that
gold is the better shelter, and it holds gold. Otherwise it holds cash. It checks the comparison once
a month.

## Why anyone believed it

The comparison is called the Fed model, though it was not written by the Federal Reserve. The idea is
that shares and bonds compete for the same money: if you can earn much more from a company's profits
per dollar than from a government's interest per dollar, shares are cheap and bond yields are low,
and something unusual is happening in the economy. Historically the stock market's earnings yield and
the bond yield have moved together, and the theory says that when the first is far above the second,
shares are undervalued.

Gold enters because it is thought to move against share valuations. When inflation expectations rise,
bond yields rise, the stock market's earnings yield has to rise to match, and share prices fall
relative to profits; in that environment gold has often risen. The person on the other side is an
investor who is comfortable holding shares or bonds and does not see the inflation that the yield gap
is signalling, and the rule takes the other side by holding gold while they hold paper assets.

## An everyday comparison

Think of a saver choosing between lending money to the government at a fixed rate and buying a share
of a shop's profits. If the shop's yearly profit is small relative to what its share costs, the
government looks the better deal. But if the shop's profit is twice what the government pays, the shop
looks cheap, and a careful saver starts to wonder whether the government's promise is the risky one,
because its interest will not keep up with rising prices. In that situation the saver might hold gold
instead of either, because gold has no promise to break, which is both its weakness and its appeal. The
rule follows exactly that instinct, and only when the gap is very wide.

## The rules, step by step

1. Each month, find the stock market's earnings yield: take the yearly profit earned by all the
   companies in the S&P 500 index, divide it by the level of the index, and write the answer as a
   percentage. The library uses the profit over the past twelve months.
2. Find the 10-year government bond yield: the interest rate, in percent per year, that the United
   States government pays when it borrows money for ten years.
3. Divide the earnings yield by the bond yield. This ratio is the whole signal. If the earnings yield
   is 6 percent and the bond yield is 3 percent, the ratio is 2.
4. Hold gold only when two things are true at once: the earnings yield is higher than the bond yield,
   and the ratio of the two is at least 2. In the library the position is 90 percent of the account in
   gold. When either condition fails, sell the gold and hold cash.
5. Check the two yields and the ratio again at the start of the next month and repeat from step 3.

The number 2 is not a rounded convenience; it is written into the rule. The library page says the
market is treated as undervalued when the earnings yield is higher than the bond yield and their ratio
is at least 2, and the code applies the test as the earnings yield being greater than twice the bond
yield. So the rule is asking for the stock market's profit-per-dollar to be double the government's
interest-per-dollar before it will hold gold. That is a high bar: it is met only when bond yields are
very low relative to profits, which in the published history happens in a few episodes rather than
most of the time.

## The maths, with every symbol named

The earnings yield of the stock market:

```text
E = ( yearly profit of the index / index level ) * 100
```

- `E` is the earnings yield in percent per year, so 6 means six cents of profit for each dollar of
  index value.
- `yearly profit of the index` is the total profit the index's companies earned over the past twelve
  months.
- `index level` is the price of the whole index.
- Dividing profit by price gives a percentage; this is the inverse of the familiar price-to-earnings
  ratio, so a low price-to-earnings ratio is a high earnings yield.

The comparison, sometimes called the Fed model ratio:

```text
ratio = E / B
hold gold if E > B and ratio >= 2
```

- `B` is the 10-year government bond yield in percent per year, so 3 means three cents of interest for
  each dollar lent.
- `ratio` is how many times the stock market's earnings yield exceeds the bond yield.
- The condition `E > B and ratio >= 2` is the same as saying `E >= 2 * B`: the earnings yield must be
  at least twice the bond yield.

The portfolio's return over a month:

```text
R_portfolio = w_gold * r_gold
```

- `w_gold` is the fraction held in gold: 0.9 when the rule is on, otherwise 0, with the rest in cash.
- `r_gold` is gold's return over the month, its price at the end divided by its price at the start
  minus one.
- When the rule is off, the return is near zero because the money sits in cash.

The cost of switching, which is low because the rule changes state rarely:

```text
Cost = t * c
```

- `t` is the traded fraction of the account, about 1.0 when the whole position moves into or out of
  gold.
- `c` is the cost of one trade as a fraction of the amount traded, covering the gap between the buying
  and selling prices plus commission. For gold held through a large fund, 0.0005 to 0.001 is a
  realistic range, that is five to ten basis points, where one basis point is one hundredth of one
  percent.

## A worked example

Ten months, with invented but plausible numbers, chosen to show how rarely the rule is on. The index
profit is the S&P 500's yearly earnings in points, so the earnings yield is profit divided by level.

| Month | Index level | Index profit | Earnings yield E | Bond yield B | Ratio | Hold gold? | Gold return |
| ----- | ----------- | ------------ | ---------------- | ------------ | ----- | ---------- | ----------- |
| M1    | 100.00      | 8.00         | 8.00 percent     | 9.50 percent | 0.84  | No         | +2 percent  |
| M2    | 110.00      | 8.80         | 8.00 percent     | 9.00 percent | 0.89  | No         | +3 percent  |
| M3    | 120.00      | 10.80        | 9.00 percent     | 7.00 percent | 1.29  | No         | +4 percent  |
| M4    | 130.00      | 13.00        | 10.00 percent    | 6.00 percent | 1.67  | No         | +1 percent  |
| M5    | 140.00      | 16.80        | 12.00 percent    | 5.50 percent | 2.18  | Yes        | +3 percent  |
| M6    | 150.00      | 18.00        | 12.00 percent    | 5.00 percent | 2.40  | Yes        | +4 percent  |
| M7    | 145.00      | 14.50        | 10.00 percent    | 5.50 percent | 1.82  | No         | -2 percent  |
| M8    | 135.00      | 12.20        | 9.04 percent     | 6.50 percent | 1.39  | No         | 0 percent   |
| M9    | 125.00      | 11.30        | 9.04 percent     | 7.50 percent | 1.21  | No         | -3 percent  |
| M10   | 118.00      | 11.20        | 9.49 percent     | 8.00 percent | 1.19  | No         | +2 percent  |

The rule is on in M5 and M6 only. In those months the account holds 90 percent gold, so the monthly
return is 0.9 times the gold return:

| Month            | Gold return | Position         | Portfolio return |
| ---------------- | ----------- | ---------------- | ---------------- |
| M5               | +3 percent  | 90 percent gold  | +2.7000 percent  |
| M6               | +4 percent  | 90 percent gold  | +3.6000 percent  |
| All other months | any         | 100 percent cash | about 0 percent  |

Compounding the two on months and charging 0.10 percent for each switch into and out of gold, the
account gained about 6.19 percent over the ten months while holding a position for only two of them.
The remaining months are the point of the example: the rule spends most of its time in cash, so its
record is decided by a handful of episodes. Notice also that M3 and M4 have earnings yields well
above the bond yield, yet the ratio is still below 2, so the rule stays out. That is what the factor
of two does.

## What the research actually found

| Source                                        | What it measured                                                                                     | Result                                                                                                                                                                                                                                                      |
| --------------------------------------------- | ---------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Daly, Tactical Asset Allocation to Gold, 2005 | Going long gold when the Fed model shows the market undervalued, compared with a risk-free rate      | An excess return of about 8.48 percent when the model showed undervaluation, rising to an average near 27 percent when the market was undervalued by 15 percent or more, and about 39 percent from 1978, with the DJIA-to-gold ratio used as a second check |
| Quantpedia, cited by the library page         | The strategy behind a paid login                                                                     | The entry could not be read for this tutorial; its numbers sit behind the login, so none are quoted here                                                                                                                                                    |
| Bartsch, Baur, Dichtl and Drobetz, 2018       | More than four thousand seasonal, technical and fundamental gold timing strategies from 1990 to 2015 | Large gains appeared for several groups, but after correcting for the fact that so many rules were tried, only selected technical trend rules survived; the fundamental family, of which the Fed model is a member, did not                                 |
| Asness, 2003, and Ritter and Warr, 2002       | The Fed model itself                                                                                 | They argue it is flawed because it compares a real quantity, the earnings yield, with a nominal one, the bond yield, so part of the apparent signal is an inflation illusion rather than an opportunity                                                     |
| The library page                              | Its own backtest                                                                                     | The page states its data feeds were discontinued, and a reader reports that the backtest placed no orders, so the published run does not demonstrate the rule trading                                                                                       |

Read together: the idea has a clear economic story and one favourable practitioner study, but the
specific rule is not backed by a measurement of its own, the Fed model it rests on is disputed, and
the one broad, careful test of gold timing rules found that the fundamental group, the group this rule
belongs to, did not stand up once the sheer number of rules tried was accounted for.

## How this project relates to it

This repository contains no gold timing strategy, and this tutorial will not pretend that it does. The
nearest pieces are two research briefs built from a harvest of academic papers. The first is
[Macro rates and FX](../../../strategies/books2/07_macro_rates_and_fx.md), whose subject is
government bond yields, the very quantity the rule compares against; it reports that tradable
information around scheduled announcements co-moves the whole yield curve, which is the mechanism by
which a bond yield could carry a signal about other assets at all. The second is
[Bubbles, crashes and criticality](../../../strategies/books2/05_bubbles_crashes_and_criticality.md),
which contains a dated, falsifiable forecast of the 2011 gold bubble and notes that its fitted
parameter sat outside the range its own authors cite, an example of how easily a confident gold story
outruns its evidence.

## Where it goes wrong

- The factor of two is arbitrary. Nothing in the sources derives the threshold from a measurement; it
  is a round number chosen by a practitioner, and a rule of 1.5 or 2.5 would hold gold in different
  months and give a different record.
- Comparing a real number with a nominal one. The earnings yield is a real profit against a price,
  while the bond yield is a nominal interest rate, so the two are not measured in the same units and
  part of the gap the rule reads is an inflation illusion.
- Almost no data. The rule is on in a handful of episodes, so its whole record rests on two or three
  stretches of history, and any one of them can carry or destroy the result.
- Data that is no longer available. The library page itself reports that the feeds it used were
  discontinued, and a reader could not get the algorithm to place a trade, so the published backtest
  is not evidence that the rule was ever exercised.
- Overfitting by rule count. Testing more than four thousand variants is the clearest warning here;
  when so many rules are tried, the best-looking one is likely to be luck, which is exactly what the
  2018 study found for the fundamental group.
- Crowding and regime change. Even if the rule once described the inflation of the 1970s, the
  relationship between bond yields, share valuations and gold can change, and a rule that must wait
  for a ratio of two may simply never trade again.

## Try it yourself

You need nothing but published figures: an index's yearly earnings, the index level, the 10-year
government bond yield, and gold prices. All are available from public finance pages. Build a monthly
table for the last twenty years.

1. Add columns: index level, index yearly earnings, earnings yield as profit divided by level times
   100, bond yield in percent, and the ratio of earnings yield to bond yield.
2. Add a column that says "gold" when the earnings yield is greater than the bond yield and the ratio
   is at least 2, and "cash" otherwise.
3. Add a column of gold's monthly return, and a second column that is that return where the rule said
   "gold" and zero otherwise.
4. Chain the on returns together by multiplying one plus each monthly figure, and do the same for a
   column that holds gold every month.
5. Count the months in which the rule said "gold".

What to notice: the rule says "gold" in only a few isolated months, often clustered around a single
crisis, so the comparison with holding gold throughout rests on those few months. If the rule looks
dramatically better, check whether the improvement comes from the rule or simply from the particular
months you chose to start and end.

## Where this came from

- [QuantConnect strategy library: gold market timing](https://www.quantconnect.com/tutorials/strategy-library/gold-market-timing),
  the rules as implemented, including the earnings yield over the trailing twelve months, the 90
  percent gold position, the monthly check, and the requirement that the ratio be at least 2.
- Ryan Daly, [Tactical Asset Allocation to Gold](https://ssrn.com/abstract=783187), the practitioner
  paper behind the idea, and the source of the 8.48, 27 and 39 percent figures quoted above.
- Bartsch, Baur, Dichtl and Drobetz, [Investing in the Gold Market: Market Timing or
  Buy-and-Hold?](https://ssrn.com/abstract=3202658), the test of more than four thousand gold timing
  rules.
- [Macro rates and FX](../../../strategies/books2/07_macro_rates_and_fx.md) and [Bubbles, crashes and
  criticality](../../../strategies/books2/05_bubbles_crashes_and_criticality.md), this repository's
  briefs closest to the question.

## Words used in this tutorial

- earnings yield: a company's or an index's yearly profit divided by its price, written as a
  percentage; the inverse of the price-to-earnings ratio.
- bond: a loan to a government or company that pays a fixed rate of interest and is repaid on a set
  date.
- bond yield: the interest a bond pays per year, as a percentage of its price.
- Fed model: a comparison of the stock market's earnings yield with the government bond yield, used to
  judge whether shares look cheap or expensive.
- cash: money held without being invested, which earns little but does not fall in price.
- gold: a metal held as a store of value that pays no interest or dividend.
- position: the amount of something you hold, where a positive number means you own it and a negative
  number means you owe it.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
