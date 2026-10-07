# The January effect: small companies in January, large ones for the rest of the year

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                       |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of small American companies during January and shares of large American companies for the other eleven months                                                                                                                        |
| How often it trades       | Once a month, switching into small companies at the start of January and back to large ones at the start of February                                                                                                                        |
| What you need             | A spreadsheet and monthly share prices with company size                                                                                                                                                                                    |
| Where the rules come from | [QuantConnect strategy library, January effect in stocks](https://www.quantconnect.com/tutorials/strategy-library/january-effect-in-stocks) and the [Quantpedia entry](https://quantpedia.com/strategies/january-effect-in-stocks) it cites |
| The underlying research   | Easterday, Sen and Stephan, [The Persistence of the Small Firm/January Effect](http://papers.ssrn.com/sol3/papers.cfm?abstract_id=1166149)                                                                                                  |
| How well it held up       | Disputed: credible studies reach opposite conclusions, one finding the effect alive and consistent and another finding it declining and almost gone, while the index that collects them reports recent returns too small to cover the costs |
| Also appears in           | nothing else in this collection                                                                                                                                                                                                             |

## The idea in one paragraph

Shares of small companies, the ones with the least value on the market, have tended to do unusually
well in January compared with the rest of the year. The usual explanation is tax. Many small shares
are owned by ordinary people, who sell their losers in December to set the loss against tax, then put
money back to work in January. If that selling presses prices down in December and the buying lifts
them in January, then a trader can buy the small shares at the start of January and sell them at the
end of the month. The rule in this tutorial does exactly that, and then holds large, well-known
companies for the other eleven months because they are calmer and more liquid.

## Why anyone believed it

The people on the other side of the trade have a reason that has nothing to do with the company's
value. An investor who has a loss in a small share can reduce their tax bill by selling it before the
year ends, and much of the selling is concentrated in the last weeks of December. That selling is not
driven by news about the business; it is driven by the calendar. When January arrives, the tax motive
disappears, the sellers have finished, and the money returns.

The counterparty is therefore the tax-sensitive seller, plus the investor who does not want to hold a
small share over the year end for reasons of reporting or risk. If the same people repeat the pattern
every year, and if their selling is large relative to how much those shares normally trade, the price
is pushed down in December and recovers in January. The effect should be strongest in the smallest,
least liquid shares, which is exactly where the original studies found it.

## An everyday comparison

In a town where everyone gets paid at the end of the month, a bicycle repair man notices the same
thing every year. In late December people sell their spare bikes cheaply, because they need cash for
the holidays and because a bike in the shed is easy to part with. By the second week of January the
same bikes sell for more, because the buyers are back. Nobody has learned anything about bicycles;
the calendar moved the crowd. This strategy is the person who buys in the last days of December and
sells in January, betting that the annual crowd returns.

## The rules, step by step

1. Start with every company whose shares are listed in the United States. Keep only those whose
   share price is above ten dollars, because very cheap shares behave oddly and are hard to trade.
2. From what is left, keep the one thousand with the highest dollar volume, which is the number of
   shares traded in a day multiplied by the price. This filters out shares that are too thinly traded
   to buy and sell reliably.
3. Keep only companies that have a reported share count and a positive profit, so that the size and
   price measures below make sense.
4. Compute each company's market value, the share price multiplied by the number of shares. Sort all
   the companies from largest to smallest.
5. Call the ten largest the large group and the ten smallest the small group.
6. On the first trading day of January, sell everything and buy the ten small companies, giving each
   one tenth of the money. Hold them through January.
7. On the first trading day of February, sell the small companies and buy the ten large companies,
   again equally weighted. Hold those for the rest of the year, checking each month that they are
   still the ten largest and replacing any that have dropped out.
8. Repeat every year. Each January the portfolio jumps back into the smallest names.

The whole idea lives in one comparison: the return of small shares during January against the return
of large shares during the same month. Everything else is a background holding.

## The maths, with every symbol named

The size of a company:

```text
Market_value = P * N
```

- `P` is the share price.
- `N` is the number of shares the company has issued.

The dollar volume used for the liquidity filter:

```text
Dollar_volume = P * V
```

- `V` is the number of shares traded in the day.

The return of a group over a month is the average of its members' returns:

```text
R_month = (P_end / P_start) - 1
```

- `P_start` and `P_end` are the price at the start and the end of the month.
- Averaging `R_month` across the ten shares in a group gives that group's return for the month.

The strategy's return for a year is the January return of the small group compounded with the return
of the large group over the other eleven months:

```text
R_year = (1 + R_small_Jan) * (1 + R_large_Feb_Dec) - 1
```

- `R_small_Jan` is the small group's return during January.
- `R_large_Feb_Dec` is the large group's return from February to December.

The return of simply holding the large group all year, the yardstick:

```text
R_large_year = (1 + R_large_Jan) * (1 + R_large_Feb_Dec) - 1
```

- `R_large_Jan` is the large group's return during January.

The difference between the two is the payoff to being in small shares for one month instead of large
ones. Finally the cost of the two full switches per year:

```text
Cost = t * c
```

- `t` is the fraction of the account traded. Selling the whole portfolio and buying another trades
  twice the account, so `t` is 2.0 for each complete switch, and 4.0 for the two switches a year.
- `c` is the cost per trade as a fraction of the amount traded. Small shares have a wider gap between
  the buying and selling price, so a realistic figure is 0.002, that is twenty basis points, where one
  basis point is one hundredth of one percent. Large shares are cheaper, nearer 0.001.

## A worked example

Six invented years, of a size that large and small shares actually produce. The small group is held
only in January; the large group is held for the other eleven months. The last column is the strategy
minus the return of holding large shares all year, which isolates the value of the January switch.

| Year | Small caps in January | Large caps in January | Large caps February-December | Strategy | Large only | Difference |
| ---- | --------------------- | --------------------- | ---------------------------- | -------- | ---------- | ---------- |
| 1    | +7.0%                 | +1.0%                 | +9.0%                        | +16.63%  | +10.09%    | +6.54%     |
| 2    | -2.0%                 | +2.0%                 | +12.0%                       | +9.76%   | +14.24%    | -4.48%     |
| 3    | +5.0%                 | +3.0%                 | +14.0%                       | +19.70%  | +17.42%    | +2.28%     |
| 4    | +1.0%                 | +4.0%                 | +6.0%                        | +7.06%   | +10.24%    | -3.18%     |
| 5    | +8.0%                 | -1.0%                 | +18.0%                       | +27.44%  | +16.82%    | +10.62%    |
| 6    | -3.0%                 | 0.0%                  | +10.0%                       | +6.70%   | +10.00%    | -3.30%     |

The strategy return for year 1 is 1.07 times 1.09 minus one, which is 0.1663, or 16.63 percent. The
large-only figure is 1.01 times 1.09 minus one, which is 10.09 percent. The difference is 6.54
percentage points, which is what the January switch added that year.

The six differences add up to 8.48 percentage points, so the average January switch was worth about
1.41 percentage points a year. Costs are not small here. Two complete switches trade four times the
account, and the small shares carry a wider gap between buying and selling prices, so at an average of
about fifteen basis points per side the cost is roughly 4.0 times 0.0015, that is 0.60 percent a year,
before adding anything for the monthly checks. Call it 0.70 percent a year in total. Net of that, the
average switch was worth about 0.71 percentage points a year, less than the size of one bad month.
The example is invented and shows only how the arithmetic works.

## What the research actually found

| Source                                                    | What it measured                                                                | Result                                                                                                                                                                                                                                                                                    |
| --------------------------------------------------------- | ------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Quantpedia, summarising the January-effect literature     | Small shares in January against large shares the rest of the year, 1947 to 2007 | 12.7 percent a year, built from a January premium of 2.7 percent for middle-sized companies rising to 6.7 percent for the smallest, plus roughly 10 percent a year from large shares; worst fall 54.98 percent                                                                            |
| Keim, Size-related anomalies and stock return seasonality | American shares, 1963 to 1979                                                   | The original study that put small shares' January returns on the map, and the sample that defined the effect                                                                                                                                                                              |
| Easterday, Sen and Stephan                                | A wider sample and a longer period                                              | January returns were smaller after 1963 to 1979 but had simply returned to the levels seen before that stretch, the effect also appeared on the new Nasdaq exchange, and trading volume in December and January was no different from other months, which argues against active arbitrage |
| Haug and Hirschey, The January Effect                     | Broad samples of value- and equally-weighted returns                            | The January effect in small shares was remarkably consistent over time and was not changed by the 1986 tax reform, which they read as evidence for behavioural rather than tax explanations                                                                                               |
| Gu, The Declining January Effect                          | American indices from 1988                                                      | The effect showed a clear downward trend, and was disappearing for the Russell indices, more so for those holding small shares                                                                                                                                                            |
| Zhang and Jacobsen, Are Monthly Seasonals Real?           | Three centuries of British returns                                              | Monthly seasonal patterns are sample-specific; the January effect only appeared around 1830, coinciding with Christmas becoming a public holiday                                                                                                                                          |

The disagreement is the story. Haug and Hirschey conclude the effect was alive and well; Gu concludes
it was declining and nearly gone; the index that collects both notes that in the recent period the
January effect was so small that transaction costs made it impossible to trade. The one thing the
sources agree on is that the effect was largest in the smallest, least liquid shares, which are also
the most expensive to trade.

## How this project relates to it

This repository does not study the January effect, but its survey of research integrity is the right
place to think about it:
[strategies/books2/28_overfitting_and_research_integrity.md](../../../strategies/books2/28_overfitting_and_research_integrity.md).
That brief reports that publication-bias corrections shrink published cross-sectional returns by only
10 to 15 percent and that false-discovery rates sit under 10 percent, so a single anomaly is not
automatically a data-mining artefact. It also reports that the multiple-testing hurdle matters: a
t-statistic of 1.96 implies a false-discovery rate of 8.8 percent, while a hurdle of 3.0 leaves 81
percent of accepted findings true. The January effect is exactly the kind of calendar pattern that
must clear that higher hurdle, because it was found by looking at the same twelve months of returns in
many ways.

## Where it goes wrong

- The effect may simply have faded. The strong numbers come from the years around the original 1963
  to 1979 study; later samples show it shrinking, and the index that collects the studies says recent
  returns are too small to cover costs.
- Costs fall hardest on the trade that is supposed to work. The effect lives in the smallest shares,
  whose spread is at its widest, so the very shares that carry the signal are the ones that are most
  expensive to buy and sell.
- It is one bet on one month. A single January that goes the wrong way dominates the year, and the
  published worst fall of about 55 percent shows how much damage the background holding can do in a
  bad market.
- The tax story does not fit all the evidence. The effect kept appearing after the 1986 tax reform
  that should have changed the tax motive, and British data show it arriving with a public holiday
  rather than a tax rule, which points to mood and habit as much as taxes.
- Small samples make it look stronger. Ten shares is a very short list, so one company with news in
  January moves the whole month. A rule tested on a handful of names and years can find an effect that
  is not there.
- The calendar can move. When a year ends on a weekend, or when the tax rules change, the December
  selling and January buying shift, and a rule fixed to "the first trading day" may be trading the
  wrong days.

## Try it yourself

You need only a spreadsheet and a public source of monthly share prices, such as any finance website,
plus a list of small and large companies.

1. Make a sheet with one column per company and one row per month, holding the month's closing price.
2. Pick ten large and ten small companies and label the two groups; do not change the list afterwards.
3. Add a row that computes the January return for every company: the price at the end of January
   divided by the price at the end of the previous December, minus one.
4. Average the January returns across the ten small and across the ten large, for every year you have.
5. Subtract the large average from the small average for each year. That difference is the January
   effect in your data.

What to notice: the difference will be positive in some years and negative in others, and the average
will usually be a fraction of the biggest single year. If you then subtract the cost of two switches a
year, the remainder is often close to zero. That is the same conclusion the sources above reached,
and it is why this rule is graded disputed rather than working.

## Where this came from

- [QuantConnect strategy library: January effect in stocks](https://www.quantconnect.com/tutorials/strategy-library/january-effect-in-stocks),
  the rules as implemented: a price filter, the thousand most-traded shares, the ten largest against
  the ten smallest, small shares in January and large shares the rest of the year.
- [Quantpedia: January effect in stocks](https://quantpedia.com/strategies/january-effect-in-stocks),
  the performance figures, the sample, the instrument count and the papers below.
- Easterday, Sen and Stephan, [The Persistence of the Small Firm/January Effect](http://papers.ssrn.com/sol3/papers.cfm?abstract_id=1166149),
  the source paper.
- Haug and Hirschey, [The January Effect](http://papers.ssrn.com/sol3/papers.cfm?abstract_id=831985),
  the study arguing the effect persists.
- Gu, [The Declining January Effect](https://www.researchgate.net/publication/222788436_The_declining_January_effect_Evidences_from_the_US_equity_markets),
  the study arguing the effect is fading.
- Zhang and Jacobsen, [Are Monthly Seasonals Real? A Three Century Perspective](http://papers.ssrn.com/sol3/papers.cfm?abstract_id=1697861),
  the long British sample that shows how sample-specific monthly patterns can be.
- [strategies/books2/28_overfitting_and_research_integrity.md](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's survey of multiple testing and replication for exactly this kind of claim.

## Words used in this tutorial

- anomaly: a pattern in prices that the usual theory of efficient markets does not explain.
- market capitalisation: the total value of a company's shares, the share price times the number of
  shares.
- seasonality: a pattern that tends to appear at the same time of year, month or day.
- small cap: a company with a small market capitalisation, as opposed to a large cap.
- spread: the gap between the best buying and best selling price, paid on every round trip.
- tax-loss selling: selling a share that has fallen in order to use the loss against tax.
- transaction cost: everything paid to trade, including commission and the spread.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
