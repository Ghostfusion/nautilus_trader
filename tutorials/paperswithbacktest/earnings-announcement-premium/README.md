# The earnings announcement premium: buying shares just before they report

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                 |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of large American companies, plus borrowed shares sold in the hope they fall                                                                                   |
| How often it trades       | Once a month, when the whole portfolio is rebuilt                                                                                                                     |
| What you need             | A spreadsheet, a list of announcement dates and four years of daily trading volume                                                                                    |
| Where the rules come from | [Quantpedia, earnings announcement premium](https://quantpedia.com/strategies/earnings-announcement-premium/), the page the list's implementation is written from     |
| The underlying research   | Frazzini and Lamont, [The Earnings Announcement Premium and Trading Volume](https://www.nber.org/papers/w13090)                                                       |
| How well it held up       | Mixed: many independent samples and countries show a premium, but the published returns are before costs and the short leg has to borrow shares that are hard to find |
| Also appears in           | [Standardized unexpected earnings](../../quantconnect/standardized-unexpected-earnings/README.md), which trades a different part of the same quarterly report         |

## The idea in one paragraph

Every three months a company must tell the public how much profit it made, and this report is called
an earnings announcement. The days around the report are unusual: much more of the company's shares
change hands than on a normal day, and on average the price drifts up a little. This strategy buys
shares during the month they are about to report and sells short shares that are not about to
report, so it collects that small upward drift. It sharpens the bet by first finding the companies
whose past announcements attracted the heaviest trading, because those are the ones where the effect
has been largest. It rebuilds the whole list once a month.

## Why anyone believed it

A company's report is a scheduled event that draws attention. When it arrives, the company is in the
news, and investors who normally ignore it look again. The paper behind this strategy argues that
part of the buying comes from small, individual investors who buy shares that have grabbed their
attention. Those investors rarely sell short, so their buying pressure is one-sided and can push the
price up regardless of whether the news was good.

The counterparty, then, is the investor who does not read every report, or who over-reacts to the
first line of it. Against them sit professional traders who do read everything, but the paper argues
those traders cannot fully correct the effect, because the price swings around a single report are
hard to hedge and the number of announcements happening at once is large. When the sellers cannot
step in, the drift persists.

## An everyday comparison

Think of a village market held on the same Saturday every month. Most weekends the footfall is
ordinary, but on the market day people who never otherwise visit still turn up, and stallholders who
are known for drawing a crowd on that day can charge a little more. The extra price is not because
the goods are better that day; it is because a predictable wave of buyers arrives at a predictable
time. This strategy is the stallholder who raises the price on market day, and who also bets against
the produce sold on the quiet days.

## The rules, step by step

1. Start with the largest, most heavily traded stocks on the American exchanges, around a thousand
   of them. The original research used every share in the CRSP database, which records almost all
   American listed stocks.
2. For each stock, add up its daily trading volume over the past 48 months. Then add up its volume
   over only those 16 months that contained an announcement. Divide the announcement total by the
   48-month total. This number is the volume concentration ratio: a stock that trades heavily only
   around its reports has a high value, and a stock that trades evenly has a low one.
3. Rank all stocks by that ratio from highest to lowest and keep the top fifth. These are the
   companies whose reports historically move the most shares.
4. For each stock in that top fifth, ask whether it reported in this same calendar month one year
   ago. Companies report on a roughly yearly cycle, so if it reported last year in this month it is
   expected to announce again now. Call it an expected announcer; everything else in the group is an
   expected non-announcer.
5. Buy the expected announcers. Sell short the expected non-announcers, that is, borrow their shares,
   sell them now, and buy them back later, profiting if the price falls.
6. Within each leg, give every stock a share of that leg proportional to the company's market value,
   which is its share price times the number of its shares. This is called value weighting.
7. Do this at the start of every month and hold until the next month. Rebuild the whole portfolio
   each month, so positions that leave the ranking are sold and positions that enter are bought.

The long and short halves are put together as one account, so the money made from short selling
helps pay for the shares bought. Costs come from two places: the gap between the price at which
something can be bought and the price at which it can be sold, which is called the spread, and the
fee charged for borrowing the shares that are sold short.

## The maths, with every symbol named

The volume concentration ratio, one number per stock:

```text
VCR = V_announce / V_total
```

- `V_announce` is the stock's total number of shares traded during the 16 months in the past four
  years that contained an earnings announcement.
- `V_total` is the stock's total number of shares traded during all 48 months.

A higher `VCR` means more of the stock's activity is concentrated in its report months, which
historically goes with a larger price drift.

Inside one leg, the weight of a stock is its share of the leg's combined market value:

```text
w_i = MarketValue_i / (sum of MarketValue over all stocks in the leg)
```

- `w_i` is the fraction of that leg's money placed in stock `i`.
- `MarketValue_i` is stock `i`'s share price times its number of shares.

The weights in a leg add up to 1, so each leg is fully invested.

The return of the long-short account over a month is the long leg's weighted return minus the short
leg's weighted return:

```text
R = (sum of w_i * R_i over the long leg) - (sum of w_j * R_j over the short leg)
```

- `R_i` is the return of long stock `i`, written as a decimal, so 0.02 is 2 percent.
- `R_j` is the return of short stock `j`; subtracting it means a fall in a shorted share adds to the
  account.

The monthly cost has two parts:

```text
Cost = t * c + b
```

- `t` is the traded fraction of the account: 2.0 if the entire book is sold and replaced, because
  the sale and the purchase both count, and less when some holdings are kept.
- `c` is the cost of one side as a fraction of the amount traded, covering the spread and any
  commission. A realistic figure for large American shares is 0.001, that is 0.10 percent.
- `b` is the fee for borrowing the shorted shares for the month, as a fraction of the shorted
  amount. For shares that are easy to borrow this is a few tenths of a percent a year, which is
  under 0.05 percent for one month.

## A worked example

Ten large companies, with four years of history behind them. The volumes are invented but of the
size real companies trade.

| Stock | Volume in the 16 announcement months | Total volume over 48 months | VCR  | Expected to announce now? |
| ----- | ------------------------------------ | --------------------------- | ---- | ------------------------- |
| A     | 640 million                          | 1,000 million               | 0.64 | yes                       |
| B     | 500 million                          | 1,000 million               | 0.50 | no                        |
| C     | 300 million                          | 1,000 million               | 0.30 | yes                       |
| D     | 280 million                          | 1,000 million               | 0.28 | no                        |
| E     | 450 million                          | 1,000 million               | 0.45 | yes                       |
| F     | 200 million                          | 1,000 million               | 0.20 | no                        |
| G     | 260 million                          | 1,000 million               | 0.26 | yes                       |
| H     | 360 million                          | 1,000 million               | 0.36 | no                        |
| I     | 410 million                          | 1,000 million               | 0.41 | yes                       |
| J     | 150 million                          | 1,000 million               | 0.15 | no                        |

Ranked by `VCR`, the top fifth of ten stocks is the top two: A (0.64) and B (0.50). A is an
expected announcer, so it goes in the long leg. B is an expected non-announcer, so it goes in the
short leg. In a real portfolio the legs would hold many stocks; here they hold one each, which makes
the arithmetic visible.

Suppose the account holds 1,000 dollars of A and 1,000 dollars of borrowed B, giving 2,000 dollars
of exposure on 1,000 dollars of capital. Over the month A rises 1.8 percent and B rises 0.4 percent.

| Position           | Amount   | Return this month | Profit |
| ------------------ | -------- | ----------------- | ------ |
| Long A             | 1,000.00 | +1.8 percent      | +18.00 |
| Short B (borrowed) | 1,000.00 | +0.4 percent      | -4.00  |
| Total before costs |          |                   | +14.00 |

So the account made 14.00 dollars, or 1.4 percent of its 1,000 dollars of capital. Now the costs.
Suppose the whole book is replaced, so 4,000 dollars is traded in total (sell A, buy a new long,
close the B short, open a new short). At 0.10 percent per side:

```text
t * c = 4000 * 0.001 = 4.00 dollars
b     = 1000 * 0.0005 = 0.50 dollars (borrow fee, half a basis point for the month)
Net   = 14.00 - 4.00 - 0.50 = 9.50 dollars, that is 0.95 percent of capital
```

One basis point is one hundredth of one percent. Twelve months at 0.95 percent each is about 11.4
percent a year before compounding, which sits inside the range the sources report below. Two things
are worth noticing. First, the cost line is a large part of the result: four times the capital is
traded every month. Second, the worked example shows only how the rules are applied; it says nothing
about whether the premium will appear next month.

## What the research actually found

| Source                                                  | What it measured                                                        | Result                                                                                                                                                 |
| ------------------------------------------------------- | ----------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Frazzini and Lamont, the paper behind the rules         | All American common stocks, January 1972 to December 2004, monthly      | Buying every share expected to announce within the coming month and shorting every share not expected to announce returned over 0.60 percent per month |
| Frazzini and Lamont, the improvement the rules describe | The same data, restricted to the high-volume group defined in the rules | The long-short portfolio in their Table VIII returned 1.53 percent per month, about 18.36 percent a year, with a reward-to-risk ratio of 0.89          |
| Quantpedia, summarising that paper                      | The same long-short portfolio                                           | Volatility of 16.12 percent a year and a worst fall of 57.25 percent, and it rates the anomaly's reliability as strong                                 |
| Barber, De George, Lehavy and Trueman                   | 46 countries                                                            | The average return during announcement months beat the other months by over 11 percent a year, after allowing for common factors                       |
| Quantpedia's description of the range across studies    | Monthly excess returns of the announcement-month strategy               | Between 7 percent and 18 percent a year, depending on how the portfolio is defined                                                                     |

Read together, the premium is one of the better-replicated patterns in the equity literature: it
shows up in America back to 1927, in 46 countries, and over long windows. What the published numbers
do not do is subtract the cost of running the trade. The measured returns are before the spread and
before the borrow fee on the short leg, and the worst fall of 57 percent is a reminder that the
long-short account is not a gentle one. The list's own headline table of its 61 strongest
replications does not include this strategy, and the list reports that the median strategy in its
whole catalogue carries a reward-to-risk ratio of 0.37; this one is above that, at 0.89, but that
figure is gross of costs.

## How this project relates to it

The repository's brief on text and earnings signals,
[strategies/books2/11_language_models_news_and_text.md](../../../strategies/books2/11_language_models_news_and_text.md),
studies what the words of an earnings release and an earnings call carry, and finds that the choice
of model used to score the text changes the result as much as the text itself. It is the same
quarterly event seen through a different instrument, and it is a useful reminder that anything
measured around an announcement sits on top of an event whose interpretation is itself uncertain.

The second link is the completed tutorial on the same report,
[standardized unexpected earnings](../../quantconnect/standardized-unexpected-earnings/README.md).
That tutorial trades the surprise in the profit figure rather than the act of announcing, so the two
are close cousins: one buys the fact of the event, the other buys the direction of the news.

## Where it goes wrong

- Borrowing costs. The short leg is the half that pays the premium, and borrowing the shares can cost
  far more than the few tenths of a percent assumed here. A hard-to-borrow share can cost several
  percent a year, which eats the whole effect.
- Crowding. Once a rule is written down and easy to run, more money runs it. The buying arrives
  earlier, the premium arrives smaller, and the latecomers pay for the reversal.
- The window matters. The effect is not only in the three days around the announcement; it spills
  across the whole month, which is why the strategy uses a monthly calendar rather than the report
  date alone. A version that trades only the report day measures something narrower.
- The signal is a proxy. "Expected to announce" is guessed from last year's month. Companies move
  their reporting dates, and a company that reports early or late is misclassified.
- The size of the effect is contested at the margin. The base figure of about 0.60 percent a month is
  a broad average; the strong version depends on first finding the high-volume group, and that group
  is defined using past data that may not repeat.
- Being right about the average is not being right about any month. A long-short account with a worst
  fall of 57 percent can lose heavily even if the long-run average is positive.

## Try it yourself

You need a spreadsheet, a public list of the companies that report in the next month, and four years
of daily trading volume. Both the announcement calendar and the volume are available from any major
finance website.

1. Build one row per large company, with columns for the last 16 announcement-month volumes and the
   total 48-month volume. If the exact announcement months are hard to collect, use the four
   calendar months in which the company reported in each of the last four years.
2. Add a column for the ratio: announcement volume divided by total volume.
3. Sort by that ratio and mark the top fifth. This is the group the strategy cares about.
4. Add a column that says whether the company reported in this calendar month last year.
5. Pick the marked companies that are about to report, and note their last month's return; then pick
   the marked companies that are not, and note theirs.
6. Subtract the about-0.10 percent cost per side from each position, and remember to subtract a
   borrow fee from the shorted names.

What to notice: the top fifth by ratio is often a small, stable group, while the bottom fifth churns.
If your sheet shows the long leg rising and the short leg rising even faster, the premium did not
appear that month; that happens and the sources say the average over many months is what they
measure, not any single one.

## Where this came from

- [Quantpedia, earnings announcement premium](https://quantpedia.com/strategies/earnings-announcement-premium/),
  the page the list's implementation is written from, and the source of the 18.36 percent, the 16.12
  percent volatility, the 57.25 percent fall and the range of 7 to 18 percent.
- Frazzini and Lamont, [The Earnings Announcement Premium and Trading Volume](https://www.nber.org/papers/w13090),
  Working Paper 13090, the source of the volume concentration ratio, the 1972 to 2004 sample and the
  0.60 percent monthly base figure.
- [Stocks Rise Around Earnings Announcements](https://www.nber.org/digest/mar08/stocks-rise-around-earnings-announcements),
  the NBER's own plain-words summary of that paper.
- The implementation the list carries:
  [earnings-announcement-premium.py](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/earnings-announcement-premium.py),
  which states the universe, the ranking, the value weights and the monthly rebuild.
- [strategies/books2/11_language_models_news_and_text.md](../../../strategies/books2/11_language_models_news_and_text.md),
  this repository's brief on earnings text and attention.

## Words used in this tutorial

- earnings announcement: the scheduled quarterly report in which a company states its profit.
- long: owning something, so that a rise in its price makes money.
- short selling: borrowing something you do not own, selling it now, and buying it back later, which
  makes money if the price falls.
- spread: the gap between the price at which something can be bought and the price at which it can be
  sold.
- value weighting: sizing each holding by the company's market value rather than equally.
- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- market value: a company's share price multiplied by its number of shares.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
