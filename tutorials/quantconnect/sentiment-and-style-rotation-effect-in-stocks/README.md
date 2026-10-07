# Sentiment and style rotation: switching between cheap and expensive shares according to how nervous investors are

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                             |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Two baskets of American shares: the cheapest by price to book value, and the most expensive                                                                                                                                                                                                                       |
| How often it trades       | About four times a year; the holdings are reviewed monthly but each position is kept for three months                                                                                                                                                                                                             |
| What you need             | A spreadsheet and three monthly series: the VIX, the put-call ratio, and the returns of a cheap-shares fund and an expensive-shares fund                                                                                                                                                                          |
| Where the rules come from | [QuantConnect strategy library, sentiment and style rotation effect in stocks](https://www.quantconnect.com/tutorials/strategy-library/sentiment-and-style-rotation-effect-in-stocks) and the premium [Quantpedia entry](https://quantpedia.com/Screener/Details/53) it cites                                     |
| The underlying research   | Lee and Song, [When Do Value Stocks Outperform Growth Stocks? Investor Sentiment and Equity Style Rotation Strategies](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=410185)                                                                                                                                |
| How well it held up       | Weak: the result comes from one specification on one sample from 1987 to 2001, the effect is concentrated in the smallest shares and is not significant in the largest, the paper carries a preliminary-draft banner, and no independent replication was located, so the grade rests on a single unfinished study |
| Also appears in           | Nothing else in this collection                                                                                                                                                                                                                                                                                   |

## The idea in one paragraph

Cheap shares, called value, and expensive fast-growing shares, called growth, take turns leading the
market. Research suggests the turn is partly driven by the mood of ordinary investors, which can be
read from two published numbers: the VIX, which measures how worried the market is, and the ratio of
put options to call options traded, which measures which way bets are leaning. When the mood is
nervous but the option bets are leaning the other way, the strategy holds the cheap basket; when both
point the same nervous way, it sells the cheap basket short instead. The rest of the time it holds
both baskets equally. The bet is that mood pushes the two kinds of shares apart and that the gap
closes when the mood passes.

## Why anyone believed it

A company's value comes from its profits now and its profits later. Cheap shares pay off now; growth
shares promise a lot later. Investors who are optimistic about the future weight the promise heavily
and bid those shares up, while investors who are frightened fall back on the familiar names and
companies with earnings today. If the mood swings in predictable ways, so does the gap between the
two kinds of shares.

The counterparty is the investor whose mood leads the price rather than the fundamentals. De Long,
Shleifer, Summers and Waldmann, the theoretical paper behind this line of thinking, model such
investors as noise traders: they are not fully rational, and their optimism and pessimism move prices
away from what the companies are worth. When the mood is at an extreme, the shares they have
abandoned are, on that view, cheap for a reason that has nothing to do with the companies. The
sophisticated investor who buys the abandoned basket is paid when the mood returns to normal.

## An everyday comparison

Two brands of trainers in the same shop. One is the well-known, fashionable brand and the other is
plain and cheap. When customers feel good about the future they pay up for the fashionable pair, so
that brand's price rises; when a scare comes, they switch to the ordinary pair they know and trust.
A shop that watches the shoppers' mood, and buys the neglected brand when it is at its cheapest, is
doing what this strategy does with two baskets of shares. The shop's risk is the same as the
strategy's: if the fashion never comes back, the neglected brand was not cheap, it was simply
forgotten.

## The rules, step by step

1. Start with every share on the two large American exchanges, and drop any fund or share that has no
   company accounts to measure.
2. Rank the shares by size, meaning the total market value of the company's shares, and keep the
   largest three tenths by that measure. The smallest shares are left out.
3. Inside each of those three size groups, rank the shares by price to book value: the price of a
   share divided by the accounting value of the company per share. The cheapest fifth in each group
   is the value basket; the most expensive fifth is the growth basket.
4. Collect two daily series: the VIX, which is a published measure of expected market turbulence, and
   the CBOE equity put-call ratio, which is the volume of put options traded divided by the volume of
   call options. Both are published every day.
5. Each month, compute a one-month average and a six-month average of each of the two series.
6. If the one-month VIX average is above its six-month average and the one-month put-call average is
   below its six-month average, hold the value basket, in equal amounts.
7. If the one-month VIX average is above its six-month average and the one-month put-call average is
   also above its six-month average, sell the value basket short, in equal amounts.
8. In every other case, hold both the value basket and the growth basket, in equal amounts.
9. Keep the position for three months and review it every three months, which gives at most four
   changes a year.

One label needs care. A high put-call ratio, meaning many more puts than calls, sounds bearish, and
the gauges are read in a contrary way: a very high ratio is taken as a sign that ordinary investors
expect a fall, which has historically been followed by a rise. The rule above uses the ratio only as
a relative measure, so the direction of the reading matters less than the change.

## The maths, with every symbol named

The two baskets are built from two company measures. The first is size:

```text
Size = (number of shares the company has issued) * (price of one share)
```

- `Size` is the market value of the whole company in the currency of the shares. It is used to keep
  only the largest three tenths of shares.

The second is the book-to-price ratio, used in reverse as price to book:

```text
P_B = (price of one share) / (accounting value of the company per share)
```

- `P_B` is the price of a share divided by the amount of company value it represents. A share with a
  low `P_B` is cheap relative to its accounts; the cheapest fifth is the value basket, and the most
  expensive fifth is the growth basket.

The mood is measured as a ratio of a recent average to a longer average, for each of the two gauges:

```text
PutCall_rel = (one-month average of the put-call ratio) / (six-month average of the put-call ratio)
VIX_rel     = (one-month average of the VIX)          / (six-month average of the VIX)
```

- `PutCall_rel` above 1 means option traders have been buying more puts than usual relative to calls.
- `VIX_rel` above 1 means the market expects more turbulence than it did over the past six months.
- Both are pure numbers with no units, and both compare a month with half a year, so the rule is
  asking whether the mood has shifted rather than whether it is high in absolute terms.

The selection rule is then a small table of conditions:

```text
if VIX_rel > 1 and PutCall_rel < 1 : hold value
if VIX_rel > 1 and PutCall_rel > 1 : sell value short
otherwise                          : hold value and growth
```

- Each line is read in order; the first that applies is used.
- When both gauges are above their averages, the position is a short sale of the value basket, which
  gains if the cheap shares fall.

The weights and the result. If a basket holds `N` shares, each gets the same amount:

```text
w = 1 / N for each share in the basket that is held
```

- `w` is the fraction of the account placed in one share, and the weights of a held basket add up to
  1, so the whole account is used on that basket or, in the third case, half the account on each of
  the two baskets.
- The return of a basket is the average of its shares' returns, because they are equally weighted.

The quantity the research is really about is the gap between the two baskets:

```text
Value_premium = value basket return - growth basket return
```

- `Value_premium` is how much the cheap basket beat the expensive basket over the period. A positive
  number means the cheap shares did better; a negative number means the expensive ones did.

Finally the cost. The strategy changes position at most four times a year, and each change is a round
trip through the market:

```text
Cost = (number of changes) * c
```

- `c` is the cost of one round trip as a fraction of the money traded, covering the gap between the
  buying and selling price plus commission. A round trip in a large exchange-traded fund is about
  0.5 percent in the source's own assumption, that is 0.005.
- With four changes a year, `4 * 0.005 = 0.02`, an annual drag of 2 percent of the account before any
  gain, which is the number to check every claimed result against.

## A worked example

Six three-month periods of invented but plausible numbers. For each period the table gives the two
relative gauges, the basket the rule chooses, and the three-month returns of the value and growth
baskets. The strategy's return is the value basket when it is held, the negative of the value basket
when it is sold short, and the average of the two when both are held.

| Period | VIX_rel | PutCall_rel | Rule chose  | Value return | Growth return | Strategy return |
| ------ | ------- | ----------- | ----------- | ------------ | ------------- | --------------- |
| 1      | 1.15    | 0.85        | hold value  | +6.0 percent | +1.5 percent  | +6.00 percent   |
| 2      | 1.20    | 0.80        | hold value  | +2.0 percent | +2.5 percent  | +2.00 percent   |
| 3      | 1.10    | 1.15        | short value | -2.0 percent | +1.0 percent  | +2.00 percent   |
| 4      | 0.95    | 1.05        | hold both   | +2.5 percent | +3.0 percent  | +2.75 percent   |
| 5      | 1.05    | 0.90        | hold value  | +1.0 percent | +2.0 percent  | +1.00 percent   |
| 6      | 1.30    | 0.85        | hold value  | +4.0 percent | +2.0 percent  | +4.00 percent   |

Check period 3. The VIX is above its six-month average and the put-call ratio is above its average
too, so the rule sells the value basket short. The value basket fell 2 percent, and a short position
gains when its target falls, so the strategy earned `-1 * -2.0 = +2.0` percent, even though the growth
basket rose. Check period 4: the VIX average is below its six-month average, so the rule holds both
baskets and earns the average of 2.5 and 3.0, which is 2.75 percent.

Compounding the strategy's six returns gives
`1.060 * 1.020 * 1.020 * 1.0275 * 1.010 * 1.040 = 1.1903`, about +19.03 percent over six quarters.
Now compare with simply holding both baskets in equal amounts every period, the returns of which are
3.75, 2.25, -0.50, 2.75, 1.50 and 3.00 percent. Compounding those gives
`1.0375 * 1.0225 * 0.995 * 1.0275 * 1.015 * 1.030 = 1.1339`, about +13.39 percent. The timing added
about five and a half points before costs over the eighteen months.

The costs close part of that gap. The position changed in periods 3, 4 and 5, three round trips, each
costing 0.5 percent, so multiply by `0.995` three times: `1.1903 * 0.995 * 0.995 * 0.995 = 1.1725`,
about +17.25 percent. The timing now beats the plain blend by roughly three and a half points over
eighteen months, which is larger than the published edge but the same order. The example is invented;
its purpose is to show how the rule is applied and how quickly the cost of switching eats the gain.

## What the research actually found

The rule rests on a single study, and the study's own tables are candid about how narrow the effect is.

| Source                                          | What it measured                                                                               | Result                                                                                                                                                                                                                                                                                                                                                                                    |
| ----------------------------------------------- | ---------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Lee and Song, the source paper                  | American shares, 1963 to 2001 for the long-run comparison, 1987 to 2001 for the sentiment test | The cheap basket returned 23.02 percent a year on average against 19.31 percent for the expensive basket, with a lower standard deviation, over the long sample; the cheap basket beat the expensive one in 65 of 115 quarters                                                                                                                                                            |
| The same paper, on the two sentiment gauges     | Value minus growth returns sorted by the gauges                                                | When the put-call ratio was below its six-month average, the cheap basket beat the expensive one by an annualised 10.49 percent on average over the next three months, significant at the ten percent level; when both the put-call ratio was low and the VIX was high, the figure rose to 26.11 percent on average and 23.71 percent at the median, significant at the one percent level |
| The same paper, on the other three combinations | The same sort, the remaining three cells                                                       | None was statistically significant, so the effect lives in one of the four possible combinations of the two gauges                                                                                                                                                                                                                                                                        |
| The same paper, on the rotating strategy        | Rotating between large and small value and growth indexes, July 1987 to June 2000              | The strategy beat the broad index by an annualised 2.47 percent on average, significant at the one percent level; the authors then assume a 0.5 percent cost per round trip, subtract about 1.5 points, and conclude the edge over buy and hold is about one percent a year                                                                                                               |
| The same paper, on size                         | The cheap-minus-expensive gap inside the largest shares and inside the smallest                | The gap was only about half as large among the largest shares and was not statistically significant there; the effect was strongest among the smallest shares                                                                                                                                                                                                                             |

Read together: the long-run gap between cheap and expensive shares is one of the better-known results
in finance, but the claim that it can be timed with these two gauges rests on one sample, one
specification and one combination of conditions. The remaining three combinations were not
significant, which is what a reader should weigh before treating the live one as a discovery.

## How this project relates to it

The repository's own work on measuring sentiment is directly relevant, because this strategy's whole
input is a measurement of mood. The type in
[python/nautilus_trader/decision_bridge/news.py](../../../python/nautilus_trader/decision_bridge/news.py)
carries two separate timestamps for every news or sentiment item, one for when it was published and
one for when the process received it, and refuses to be built without both. The reason is the trap
this strategy has to avoid: the VIX and the put-call ratio are published on a daily schedule, and a
monthly average must only use the days that had actually been seen by the decision time, or it is
using the future.

Two research briefs carry the measurement evidence. The first is
[Language models, news and text signals](../../../strategies/books2/11_language_models_news_and_text.md),
whose first finding is that seven independently developed language models scoring the same 1,946
earnings calls agreed only weakly with each other: the mean pairwise rank correlation was 0.52, and
the identity of the tool explained 33.4 percent of the variation in the scores (`2609.31013v1`, p.3,
p.4). A sentiment number is a measurement with a producer attached, not an objective reading, and the
same caution applies to the mood gauges used here even though they are simpler. The second brief is
[News and language-model signals](../../../strategies/books/14_news_and_language_model_signals.md),
whose summary states that every strong headline result in that area is gross of costs, and that the
published rankings track the strength of the cost and multiple-testing discipline imposed rather than
the quality of the signal. This repository does not implement a style rotation, and no file in it
trades on the VIX and the put-call ratio together.

## Where it goes wrong

- The rule is the best of four cells. Only one of the four combinations of the two gauges was
  significant, and choosing that cell after the fact is a small search, which is how noise is published.
- The effect is concentrated in small shares. The gap was not significant among the largest companies,
  so the biggest part of the effect sits where trading and short selling are hardest.
- Costs eat most of the edge. The source starts from 2.47 percent a year and ends near one percent
  after 0.5 percent per round trip, so any higher cost or more frequent switching removes it.
- The mood gauges look into the future too easily. A monthly average built without respecting the
  daily publication schedule uses days the decision had not seen; the repository's news type exists
  to prevent exactly that.
- The evidence is a single preliminary draft. The copy read here is dated January 2003, asks readers
  not to quote it, and stops in 2001, and no independent replication was located.
- Definitions are free choices. Price to book or price to earnings, the size cut, the gauge windows
  and the holding period are all choices, and another set of choices is another rule.

## Try it yourself

You need a spreadsheet and about fifteen years of public monthly data: the VIX, the CBOE equity
put-call ratio, and the returns of one cheap-shares fund and one expensive-shares fund, both tracking
large companies.

1. Put the months down the first column, then columns for the VIX, the put-call ratio, the value
   fund's return and the growth fund's return.
2. Add two columns for the one-month and six-month averages of the VIX, and two more for the same
   averages of the put-call ratio.
3. Add a column for `VIX_rel`, the one-month VIX average divided by the six-month average, and one for
   `PutCall_rel`, the same for the put-call ratio.
4. Add a column that classifies each month into one of the three rules from the rules section, and a
   column for the difference between the value fund's and the growth fund's next-three-month returns.
5. Using a pivot or a simple average, compute the mean of the difference column separately for each
   of the three classifications.
6. Finally, count how many months fall into the "hold value" classification, and repeat the whole
   exercise using price-to-earnings instead of price-to-book if your data allows.

What to notice: most of the difference sits in a small number of months, a sign that a handful of
periods do the work. Change the six-month window to twelve, or switch the accounting measure, and see
whether the pattern moves; if it does, the rule is describing those particular years.

## Where this came from

- [QuantConnect strategy library: sentiment and style rotation effect in stocks](https://www.quantconnect.com/tutorials/strategy-library/sentiment-and-style-rotation-effect-in-stocks),
  the rules as implemented: the value and growth baskets from the largest three size groups, the VIX
  and put-call rule, and the three-month holding period.
- [Quantpedia: sentiment and style rotation effect in stocks](https://quantpedia.com/Screener/Details/53),
  the premium entry the QuantConnect page points to; it is not publicly readable, so its figures
  could not be quoted.
- Lee and Song, [Investor Sentiment and Equity Style Rotation Strategies](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=410185),
  the source paper, from which the return figures, the four-cell table and the cost arithmetic above
  are taken.
- [Language models, news and text signals](../../../strategies/books2/11_language_models_news_and_text.md)
  and [News and language-model signals](../../../strategies/books/14_news_and_language_model_signals.md),
  this repository's briefs on how text and sentiment signals behave, including the cross-provider
  disagreement result from `2609.31013v1`.
- [Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  the repository's brief on why a rule chosen from among several should be treated with care.

## Words used in this tutorial

- book value: the accounting value of a company's assets minus its debts, divided among its shares.
- growth shares: shares of companies expected to grow their profits quickly, usually priced highly
  relative to their current earnings or book value.
- market neutral: holding equal amounts bought and sold, so that the overall market's direction
  matters little.
- put and call: contracts that pay out if a price falls (a put) or rises (a call).
- short selling: borrowing something you do not own, selling it, and buying it back later, so that a
  fall in its price is a gain.
- value shares: shares that are cheap relative to the company's earnings or book value.
- VIX: a published index of how much turbulence the market expects over the next thirty days.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
