# The same month a year later: buying the shares that won this month last year

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                  |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of American companies listed on the New York and American exchanges, held in a basket of winners and a basket of losers                                                                                                                                                                                         |
| How often it trades       | About once a month, when the two baskets are rebuilt                                                                                                                                                                                                                                                                   |
| What you need             | A spreadsheet, a year of share prices, and a rough measure of company size                                                                                                                                                                                                                                             |
| Where the rules come from | [QuantConnect strategy library, 12 month cycle in cross section of stocks returns](https://www.quantconnect.com/tutorials/strategy-library/12-month-cycle-in-cross-section-of-stocks-returns) and the [Quantpedia entry](https://quantpedia.com/strategies/12-month-cycle-in-cross-section-of-stocks-returns) it cites |
| The underlying research   | Heston and Sadka, [Seasonality in the Cross-Section of Expected Stock Returns](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=687022)                                                                                                                                                                             |
| How well it held up       | Mixed: the pattern is measured on a long American sample with strong statistics and repeated in an international follow-up, but the long-short figures are quoted before the cost of rebuilding a large portfolio every month and the quoted worst fall is 92 percent                                                  |
| Also appears in           | nothing else in this collection                                                                                                                                                                                                                                                                                        |

## The idea in one paragraph

Every calendar month leaves its own mark on the share market. Shares that did well in, say, last
January have tended to do well again this January, and the same holds for May, September and the rest.
So each month, look at how each share performed in that same month one year earlier, buy the best
tenth of the market (a long holding, meaning you own the shares) and bet against the worst tenth (a
short holding, meaning you borrow the shares, sell them, and hope to buy them back cheaper). Hold the
two baskets for one month, then look again. The library page puts the accent on January, the strongest
month, while the general rule uses whichever month you are in.

## Why anyone believed it

The story is that the same people buy and sell on the same schedule every year. At the end of the
year, investors sell shares that have fallen to set the losses against tax, which pushes prices down
in December and lets them bounce in January. Fund managers tidy their holdings before reporting
dates, selling the shares they are least proud of and buying them back afterwards. Dividends,
bonuses, holidays and the school year arrive at fixed times, so spare cash enters the market at fixed
times.

Who is on the other side of the trade? A seller who is not acting on the outlook at all: someone
taking a tax loss, a manager tidying up, a saver who pays in every January. If those sellers keep
returning on the same dates, the same shares keep being pushed down on the same dates and recovering
afterwards, which is what the strategy tries to collect.

## An everyday comparison

Think of a branch-line train each weekday morning. The same faces board at the same stops, because
the same jobs, schools and shifts run on the same timetable. The train does not cause the commuters to
appear; the timetable underneath does. A rule that buys whenever the platform was crowded one year
ago and expects it to be crowded again is trading the timetable, not a coincidence. It stops paying
when the timetable changes, as happened to city commuter routes when working from home emptied the
early trains.

## The rules, step by step

1. Build the list to choose from: every company listed on the New York Stock Exchange (NYSE) and the
   American Stock Exchange (AMEX). Drop any share that has no company accounts attached.
2. Keep the largest 30 percent of those companies by market capitalisation, which is the share price
   multiplied by the number of shares the company has issued. The library works this size out from the
   company's average share count, its earnings per share and its price-to-earnings ratio.
3. Find the one-month window that sits about twelve months ago and matches the current calendar month.
   For each remaining share, take its price at the start of that window and its price at the end, and
   divide.
4. Rank every share from the highest such return to the lowest, then cut the ranked list into ten
   equal groups (deciles) of the same size.
5. Buy the top group (the winners) and short the bottom group (the losers). Each share gets the same
   amount of money, and the two groups together use the whole account: half the money owned, half
   borrowed and sold.
6. Hold for one month. Do not act on prices in between.
7. At the start of the next month, recompute steps 2 to 4 and rebuild both baskets. Close whatever
   has left the top and bottom groups and open whatever has entered.

One note on the calendar. The library page describes the signal as the January return and calls the
idea the January effect. Its code instead finds the same calendar month one year before, whatever the
month. Quantpedia states the general rule the same way, the return of the same month one year
earlier, and reports that the pattern appears in every calendar month, not only January. This
tutorial follows the general rule.

## The maths, with every symbol named

The whole strategy is one division repeated for every share, one sort, and one subtraction.

First the signal, the return of the share in the matching calendar month one year earlier:

```text
S_i = P_i(end) / P_i(start) - 1
```

- `S_i` is the cycle score of share `i`, written as a decimal: 0.12 means 12 percent.
- `P_i(start)` is the share's price at the start of the calendar month twelve months ago.
- `P_i(end)` is the share's price at the end of that same month.

Then rank the shares by `S_i`, from largest to smallest, and cut the list into ten equal groups. Call
the size of one group `N`. The portfolio weights come from which group a share falls in:

```text
w_i = +1 / (2N)   for each share in the top group (winners)
w_i = -1 / (2N)   for each share in the bottom group (losers)
w_i = 0           for every other share
```

- `w_i` is the fraction of the account placed in share `i`. A plus sign means owned, a minus sign
  means borrowed and sold.
- `N` is the number of shares in one group.
- Summing the weights over one group gives `N` times `1 / (2N)`, which is 1/2; so each side uses half
  the money and the account is fully deployed and split evenly between owning and shorting.

The return on the whole account over the following month is then:

```text
R = ( mean return of the winners - mean return of the losers ) / 2
```

- `R` is the account's return for the month.
- `mean return of the winners` is the average next-month return of the shares in the top group.
- The division by two is because only half the money sits on each side.

Finally the cost of rebuilding the book:

```text
Cost = t * c
```

- `t` is the traded fraction, counting a sale and a purchase as two trades; `t = 2.0` means the whole
  account was sold and replaced.
- `c` is the cost of one trade as a fraction of the amount traded, covering the gap between the buying
  and selling price plus commission. A realistic figure for large American shares is 0.0005, that is
  five basis points, where one basis point is one hundredth of one percent.

## A worked example

Start with the signal. Ten invented shares, and the prices they had in the same calendar month one
year ago:

| Share | Price at start | Price at end | Score S_i | Rank |
| ----- | -------------- | ------------ | --------- | ---- |
| A     | 100.00         | 112.00       | +12.0%    | 1    |
| B     | 50.00          | 55.00        | +10.0%    | 2    |
| C     | 80.00          | 86.00        | +7.5%     | 3    |
| D     | 40.00          | 42.00        | +5.0%     | 4    |
| E     | 200.00         | 208.00       | +4.0%     | 5    |
| F     | 60.00          | 61.80        | +3.0%     | 6    |
| G     | 25.00          | 25.50        | +2.0%     | 7    |
| H     | 90.00          | 90.90        | +1.0%     | 8    |
| I     | 70.00          | 69.30        | -1.0%     | 9    |
| J     | 30.00          | 29.10        | -3.0%     | 10   |

A real group would hold hundreds of shares. To keep the arithmetic on one page, here the top two (A
and B) stand in for the winner group and the bottom two (I and J) for the loser group, so `N = 2` and
each share gets `1 / (2 * 2) = 0.25` of the account on the long side, or `-0.25` on the short side.

Now six holding months. The two baskets are re-formed each month; the columns show what each basket
returned over the coming month, and the long-short result follows from the formula above.

| Month | Winners' return | Losers' return | R = (W - L) / 2 | Traded fraction t | Cost = t x c | Net return | Cumulative net |
| ----- | --------------- | -------------- | --------------- | ----------------- | ------------ | ---------- | -------------- |
| 1     | +3.5%           | -1.0%          | +2.25%          | 2.0               | 0.10%        | +2.15%     | 2.15%          |
| 2     | +1.0%           | +2.0%          | -0.50%          | 2.0               | 0.10%        | -0.60%     | 1.54%          |
| 3     | -0.5%           | -3.0%          | +1.25%          | 2.0               | 0.10%        | +1.15%     | 2.70%          |
| 4     | +2.0%           | +0.5%          | +0.75%          | 2.0               | 0.10%        | +0.65%     | 3.37%          |
| 5     | +0.5%           | -1.5%          | +1.00%          | 2.0               | 0.10%        | +0.90%     | 4.30%          |
| 6     | -2.0%           | -1.0%          | -0.50%          | 2.0               | 0.10%        | -0.60%     | 3.68%          |

Check row 1: (3.5 - (-1.0)) / 2 = 2.25, and 2.25 - 0.10 = 2.15. Check row 3: (-0.5 - (-3.0)) / 2 =
1.25, and 1.25 - 0.10 = 1.15. The cost uses `c = 0.0005` and `t = 2.0`, so it is 0.10 percent each
month.

Compounding the six net returns gives 1.0368, so the account gained about 3.68 percent over the six
months, roughly 7.5 percent a year at that pace. The cost line removed 0.60 percent over the six
months, a large part of the 3.68.

Two things to notice. First, the strategy made money in four months of six and lost in two, which is
what a small, real edge looks like: a little edge collected often, interrupted by reversals. Second,
the costs are not decoration. At ten basis points a month they come to about 1.2 percent a year,
which is a sixth of the 8.6 percent a year the source reports.

## What the research actually found

| Source                                             | What it measured                                                   | Result                                                                                                                                                                                                     |
| -------------------------------------------------- | ------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Quantpedia, summarising Heston and Sadka           | Long-short groups by same-month return, American shares, 1965-2002 | 8.6 percent a year before costs, volatility 12.2 percent, Sharpe ratio 0.38, worst fall 91.68 percent, about 1,000 instruments                                                                             |
| Heston and Sadka, seasonality in the cross-section | Expected returns for each calendar month, American shares          | The annualised standard deviation of the seasonal effect is 13.8 percent, there are distinct expected returns in all twelve months, and the seasonal variation is largest in October, December and January |
| Heston and Sadka, international follow-up          | Canada, Japan and twelve European countries                        | The same same-month pattern appears, lasts up to ten years, and is independent of country, currency and company size                                                                                       |
| Hirshleifer, Jiang and Meng, cited by Quantpedia   | Investor mood by month                                             | A "mood beta" explains part of the pattern: shares that do well in good-mood months continue in later good-mood months and reverse in the opposite ones                                                    |

The honest reading is that this is better documented than most calendar patterns. There is a long
American sample with strong statistics, and the same authors find the pattern again in fourteen other
markets, which is more than a single study. Against that, the 8.6 percent is the long-short spread
before the cost of running a thousand-name book every month, the quoted worst fall is 92 percent, and
the reward for the risk, a Sharpe ratio of 0.38, is below what simply owning the market has often
delivered. Nothing here says the strategy works; it says the pattern has been measured.

## How this project relates to it

The repository does not implement a calendar-month seasonality rule. The closest material is the
cross-sectional evidence in
[the predictability brief](../../../strategies/books2/08_predictability_and_trading_strategies.md),
which is about ranking rules built on past returns and how much of their edge survives value
weighting and a factor adjustment, and
[Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), which asks the
same question for industries: whether a leader stays a leader long enough to trade. A reader who
wants the ranking arithmetic itself can look at the rate-of-change indicator in
[crates/indicators/src/momentum/roc.rs](../../../crates/indicators/src/momentum/roc.rs), which is the
same division, today's price over an earlier price, that step 3 performs by hand.

## Where it goes wrong

- The worst fall. Quantpedia quotes a 91.68 percent fall for the long-short book. A pattern can be
  statistically strong and still be ruined by a single bad run, particularly when both baskets move
  against the position at the same time.
- Costs. A thousand shares in two baskets, rebuilt every month, is a great deal of trading. The 8.6
  percent figure is before that bill, and the worked example shows the bill is large.
- Survivorship. A backtest that uses only the shares still listed today quietly deletes the companies
  that went to zero, which are exactly the ones a loser basket would have held.
- Size and accounts data. The universe step uses today's earnings and share counts to decide which
  companies existed and how big they were. A figure that was not knowable at the time is a way of
  seeing the future.
- Choosing the month. Twelve months are on offer, and January is the strongest. Picking the best
  month after seeing the results is how a small real edge gets reported as a large one.
- Trading the pattern down. If enough money runs a published seasonal rule, the buying moves earlier,
  the prices adjust sooner, and the edge shrinks toward the cost of trading.

## Try it yourself

No money and no code, just a spreadsheet and any public source of monthly share prices.

1. Write twelve columns, one for each calendar month.
2. Pick twenty large shares you recognise, and for each fill in the return it earned in that month in
   each of the last ten years.
3. In a thirteenth column, for each month and year, average the returns of the ten shares that did
   best in that month a year earlier, and put the average of the ten worst in a fourteenth column.
4. Subtract the worst from the best and halve the result. That is the strategy's return for the month.
5. Add one more column that subtracts 0.10 percent of cost each month.
6. Total each year, then chain the yearly figures into one long series.

What to notice: the strategy has many small winning months and a few large losing ones, so the final
number depends heavily on whether a bad month happened to fall inside your ten years. Notice too how
much the answer moves when you add or drop a single year. That sensitivity, not the average, is the
honest finding.

## Where this came from

- [QuantConnect strategy library: 12 month cycle in cross section of stocks returns](https://www.quantconnect.com/tutorials/strategy-library/12-month-cycle-in-cross-section-of-stocks-returns),
  the rules as implemented: NYSE and AMEX shares, the largest 30 percent by size, ten groups by the
  same month's return a year earlier, long the top group and short the bottom group.
- [Quantpedia: 12 month cycle in cross-section of stocks returns](https://quantpedia.com/strategies/12-month-cycle-in-cross-section-of-stocks-returns),
  the performance figures, the instrument count and the underlying papers.
- Heston and Sadka, [Seasonality in the Cross-Section of Expected Stock Returns](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=687022),
  the original monthly-seasonality study.
- Heston and Sadka, [Common Patterns of Predictability in the Cross-Section of International Stock Returns](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=971141),
  the international repetition.
- [The predictability brief](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's own reading of the cross-sectional predictability literature.

## Words used in this tutorial

- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- decile: one of ten equal groups, made by sorting a list and cutting it into tenths.
- long: owning a share, so the position gains when its price rises.
- market capitalisation: a company's size, its share price multiplied by the number of shares issued.
- seasonality: a pattern that repeats on the calendar, such as doing well in the same month each year.
- Sharpe ratio: the reward for the risk taken, the average return divided by how much the return moves
  around.
- short: borrowing a share, selling it, and buying it back later, so the position gains when its price
  falls.
- volatility: how much a price or return moves around its average, usually given as a percentage a
  year.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
