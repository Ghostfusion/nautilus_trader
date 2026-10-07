# Momentum combined with asset growth: running the price signal only among expanding companies

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                                                                             |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of American companies, bought and sold short in pairs                                                                                                                                                                                                                                                                                                                                                      |
| How often it trades       | Once a month, and not at all in January                                                                                                                                                                                                                                                                                                                                                                           |
| What you need             | A spreadsheet with monthly share prices and one year of balance-sheet totals for each company                                                                                                                                                                                                                                                                                                                     |
| Where the rules come from | [The list's implementation file](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/momentum-factor-combined-with-asset-growth-effect.py), which restates [the Quantpedia entry](https://quantpedia.com/strategies/momentum-factor-combined-with-asset-growth-effect/)                                                                                                  |
| The underlying research   | Nyberg and Poyry, [Firm Expansion and Stock Price Momentum](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1684767)                                                                                                                                                                                                                                                                                          |
| How well it held up       | Mixed: one long American sample from 1968 to 2006 with a strong measured result, but the profit appears in some market states and not others, there is no independent replication, and the reported worst fall is close to 89 percent                                                                                                                                                                             |
| Also appears in           | [Asset growth effect](../../../tutorials/quantconnect/asset-growth-effect/README.md), [Momentum effect in stocks](../../../tutorials/quantconnect/momentum-effect-in-stocks/README.md), [January effect in stocks](../../../tutorials/quantconnect/january-effect-in-stocks/README.md) and [Momentum and state of market filters](../../../tutorials/quantconnect/momentum-and-state-of-market-filters/README.md) |

## The idea in one paragraph

Buying shares that have been rising and selling short shares that have been falling is the oldest pattern in
share markets. This strategy adds one screen before doing that. A company's total assets are everything it
owns, added up from its yearly accounts, and some companies grow that total quickly by borrowing, issuing
shares or keeping profits. The strategy keeps only the companies in the fastest-growing tenth of that
measure, and only among those does it buy the strongest past performers and sell short the weakest. It
rebuilds once a month, but skips January, which has historically been a bad month for this kind of trade.
The bet is that price momentum is stronger and more reliable among companies that are expanding.

## Why anyone believed it

Price momentum needs a reason to persist, and one candidate is that good news takes time to spread through
the market. A company that is expanding quickly, by opening plants, buying rivals or hiring, is a company
where news is arriving. Investors who hear it late keep buying after the first move.

The asset-growth screen may also remove a particular problem. Among companies that are shrinking or standing
still, a rising share price may reflect nothing except a change of mood, and mood reverses. Among companies
that are visibly getting bigger, the same price rise is more likely to be attached to something real. The
counterparty is the slow investor who reads the accounts a quarter late, and the seller who takes profits in
a company that keeps growing, both of whom keep feeding the trend.

## An everyday comparison

Think of choosing which runners to back in a long race, using two pieces of information. The first is simply
who has been running fastest so far, which is noisy, because a runner who sprinted early often fades. The
second is who has been visibly getting stronger, for example who has been training more hours each week.
Backing the runners who are both fast so far and still building fitness is a narrower bet than backing
whoever is merely ahead, and it avoids the runner who led early on a lucky start and is now slowing down.
The asset-growth screen is the training measure; the price record is the race so far.

## The rules, step by step

1. Start with all shares listed on the New York, American and Nasdaq exchanges. Remove the smallest
   companies, those below the twentieth percentile of company size measured against the New York exchange's
   cut-off. The list's implementation takes the 500 most heavily traded American shares instead.
2. For each company, add up its total assets from its most recent annual balance sheet, and again from the
   balance sheet one year before. The assets figure is the total of everything the company owns, reported on
   the balance sheet.
3. Compute the asset growth: the newer total divided by the older total, minus one. Do this once a year,
   using the figures that would have been available in July, and use that value for the following twelve
   months. July is the cut-off because most companies report their annual accounts in the first half of the
   year.
4. Each month, rank all the remaining companies by asset growth and keep the fastest-growing tenth. This is
   the first screen.
5. For each company in that tenth, compute the momentum number: the change in its share price from twelve
   months ago to one month ago. The most recent month is left out, because over days and weeks prices tend
   to bounce back after a sharp move, which works against the signal.
6. Rank the companies in the surviving tenth by that momentum number and split them into fifths. Buy the
   strongest fifth and sell short the weakest fifth. Give every position on a side the same amount, so with
   four long positions each gets one quarter of the long side's money.
7. Rebuild at the end of each month and trade towards the new list. In January, sell everything and hold
   cash instead, because January has been recorded as a losing month for this kind of trade.
8. Pay the costs. Trading costs are roughly 0.05 to 0.15 percent of the traded amount per side for liquid
   shares, and anything held short pays a borrow fee, often around 0.5 percent a year on the value borrowed.

## The maths, with every symbol named

The asset growth of a company:

```text
G = A_new / A_old - 1
```

- `G` is the asset growth, written as a decimal: 0.35 means the company's total assets grew by 35 percent
  over the year.
- `A_new` is total assets from the most recent annual balance sheet.
- `A_old` is total assets from the balance sheet one year earlier.

The momentum number:

```text
M = P_one_month_ago / P_twelve_months_ago - 1
```

- `M` is the momentum number.
- `P_one_month_ago` is the share price one month before the rebalancing day.
- `P_twelve_months_ago` is the share price twelve months before the rebalancing day.
- Leaving out the most recent month is what makes the top of the fraction one month ago rather than today.

The two sides and their weights:

```text
Long side:  the top fifth by M among the fastest-growing tenth
Short side: the bottom fifth by M among the fastest-growing tenth
w_i = 1 / N_side
```

- `N_side` is the number of companies on that side, so every position on a side has the same weight.
- The long side holds one unit of the account and the short side holds minus one unit.

The return of the portfolio over the next month:

```text
R = average of the long positions' returns - average of the short positions' returns
```

The cost:

```text
Cost = t * c + b_short
```

- `t` is the traded fraction of the account, counting both the sale and the purchase, so replacing the whole
  long side and the whole short side gives `t` of 4.0.
- `c` is the cost per side as a fraction of the amount traded, around 0.001 for 10 basis points, where one
  basis point is one hundredth of one percent.
- `b_short` is the borrow fee over one month on the value held short, for example 0.5 percent a year divided
  by 12.

## A worked example

The illustrations use a small universe. Suppose the fastest-growing tenth of the universe contains ten
shares, which is what a universe of one hundred shares would give. The asset-growth values below are
invented but of ordinary size for fast-growing companies.

| Share | Asset growth | In the fastest-growing tenth? |
| ----- | ------------ | ----------------------------- |
| G1    | +48%         | Yes                           |
| G2    | +41%         | Yes                           |
| G3    | +35%         | Yes                           |
| G4    | +30%         | Yes                           |
| G5    | +26%         | Yes                           |
| G6    | +22%         | Yes                           |
| G7    | +19%         | Yes                           |
| G8    | +15%         | Yes                           |
| G9    | +11%         | Yes                           |
| G10   | +8%          | Yes                           |

Now the second sort, by momentum, among those ten. Splitting ten into fifths gives two shares per fifth.

| Share | Momentum | Rank | Fifth  |
| ----- | -------- | ---- | ------ |
| G1    | +30%     | 1    | Top    |
| G2    | +22%     | 2    | Top    |
| G3    | +18%     | 3    | Second |
| G4    | +14%     | 4    | Second |
| G5    | +9%      | 5    | Third  |
| G6    | +4%      | 6    | Third  |
| G7    | 0%       | 7    | Fourth |
| G8    | -5%      | 8    | Fourth |
| G9    | -12%     | 9    | Bottom |
| G10   | -20%     | 10   | Bottom |

The long side is G1 and G2, one half of the long side each. The short side is G9 and G10, one half of the
short side each. Suppose the next month brings these returns:

| Position  | Weight within its side | Next-month return | Contribution to the total |
| --------- | ---------------------- | ----------------- | ------------------------- |
| G1 long   | 0.50                   | +2.0%             | +1.00%                    |
| G2 long   | 0.50                   | -1.0%             | -0.50%                    |
| G9 short  | 0.50                   | +3.0%             | -1.50%                    |
| G10 short | 0.50                   | +4.0%             | -2.00%                    |
| Total     |                        |                   | -3.00%                    |

Both shorted shares rose, so the short side lost 3.5 percent of its unit, and the month is negative. This
is what the strategy looks like when the screen has not helped: the past losers kept rising. Now the costs.
Both positions on each side change, so the whole long book and the whole short book are replaced, giving a
traded fraction `t` of 4.0. At 10 basis points per side and a 0.5 percent annual borrow fee on the short
unit:

```text
Cost = 4.0 * 0.001 = 0.004, that is 0.40 percent
Borrow = 0.005 / 12 = 0.000417, that is 0.042 percent of the account
Net return = -3.00 - 0.40 - 0.04 = -3.44 percent
```

Over six months, a run of results could look like this. The numbers are invented; the point is the
arithmetic and the size of the cost line when both sides are replaced every month.

| Month | Long side | Short side | Gross | Trades | Cost  | Borrow | Net    |
| ----- | --------- | ---------- | ----- | ------ | ----- | ------ | ------ |
| 1     | +2.0%     | -1.0%      | +3.0% | 4.0    | 0.40% | 0.04%  | +2.56% |
| 2     | -1.0%     | +2.0%      | -3.0% | 4.0    | 0.40% | 0.04%  | -3.44% |
| 3     | +1.5%     | +0.5%      | +1.0% | 4.0    | 0.40% | 0.04%  | +0.56% |
| 4     | +3.0%     | -2.0%      | +5.0% | 4.0    | 0.40% | 0.04%  | +4.56% |
| 5     | +0.5%     | +1.5%      | -1.0% | 4.0    | 0.40% | 0.04%  | -1.44% |
| 6     | +2.5%     | -0.5%      | +3.0% | 4.0    | 0.40% | 0.04%  | +2.56% |

The six months total 5.36 percent, about 0.9 percent a month. In a real calendar one of those months would
be January, when the rules say hold cash, so the sequence would include a month of zero rather than a
trading result.

## What the research actually found

| Source                                                          | What it measured                                                                                                                                   | Result                                                                                                                                                                                                                                                                                    |
| --------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Nyberg and Poyry, Firm Expansion and Stock Price Momentum       | New York, American and Nasdaq shares, 1968 to 2006, long the strongest momentum fifth and short the weakest within the fastest-growing asset tenth | Quantpedia's page reports 16.77 percent a year, volatility 13.84 percent, a reward-to-risk of 1.21 and a statistical significance value of 5.04, with a worst fall of 88.95 percent, from the paper's Table 2                                                                             |
| The same paper's central finding                                | Momentum profits grouped by asset growth                                                                                                           | Momentum profits are large and significant among firms with large asset expansions or contractions, and small and often insignificant among firms with small changes in assets; the interaction survives controls for size, book-to-market, turnover, return volatility and credit rating |
| The awesome-systematic-trading list, its own replication record | 4,843 coded papers; aggregate statistics only                                                                                                      | The median strategy returns a reward-to-risk of 0.37 and 48 percent clear a statistical significance bar of 1.96; this is the list's own aggregate measurement, not a figure for this strategy                                                                                            |

Read together: the interaction is measured once, on a long American sample, with a large statistic, and the
paper reports that momentum is weak in the middle of the asset-growth distribution. That is exactly the
"some states and not others" pattern that makes the result hard to generalise from. The worst fall near 89
percent shows what the long side of this trade did in a sustained bear market. The list publishes no
reward-to-risk number for this strategy on its own.

## How this project relates to it

This repository has a tutorial for each half of the idea.
[Asset growth effect](../../../tutorials/quantconnect/asset-growth-effect/README.md)
measures what happens when you sort shares by how fast their total assets grow, which is the screen used
here.
[Momentum effect in stocks](../../../tutorials/quantconnect/momentum-effect-in-stocks/README.md)
measures the price signal on its own.
[January effect in stocks](../../../tutorials/quantconnect/january-effect-in-stocks/README.md)
explains why a momentum rule might sit out the first month of the year, and
[Momentum and state of market filters](../../../tutorials/quantconnect/momentum-and-state-of-market-filters/README.md)
tests the idea that momentum works in some market conditions and not others, which is the paper's finding
about asset-growth groups.

The research side is covered in
[the predictability brief](../../../strategies/books2/08_predictability_and_trading_strategies.md),
which reports both that the cross-sectional factor findings replicate broadly and that the defence of a
signal must be restated under value weighting and a factor model before it is treated as real.

## Where it goes wrong

- The two screens interact by construction. If fast-growing companies happen to be rising shares, the asset
  screen is doing the momentum work; if they are falling shares, the screen removes the very names the
  momentum leg would have bought. The result depends on the interaction, which is one measured fact, not a
  law.
- It is a small book of companies. A tenth of the universe, then a fifth of that, leaves a handful of long
  and short names, so one company can dominate a month.
- The worst fall is enormous. The reported maximum fall of 88.95 percent means the long-short spread lost
  almost nine tenths of its cumulative value at some point in the sample. A rule with that profile demands a
  strong stomach and careful sizing.
- January is switched off from the past. Excluding a month because it lost money in the sample is a choice
  made after seeing the results, and the same month may behave differently next time.
- No independent replication. The full combination is one paper's result. The two legs separately are better
  documented, but that is not the same as the combination being robust.
- Accounts are reported late. Using last year's balance sheet is unavoidable, but it means the asset screen
  is always looking at the company as it was up to a year ago, not as it is.

## Try it yourself

You need a spreadsheet, monthly share prices for about 40 companies, and their total assets for the last two
years. A free filing database such as the company filings site will give the asset totals.

1. Build a sheet with one row per company and columns `TotalAssetsNow`, `TotalAssetsYearAgo`,
   `AssetGrowth`.
2. Compute `AssetGrowth` as `TotalAssetsNow / TotalAssetsYearAgo - 1`.
3. Sort by `AssetGrowth` and mark the fastest-growing tenth. Those are the only companies you will trade.
4. Add columns `PriceToday`, `PriceOneMonthAgo`, `PriceTwelveMonthsAgo` and `Momentum`, where `Momentum` is
   `PriceOneMonthAgo / PriceTwelveMonthsAgo - 1`.
5. Among the marked companies only, sort by `Momentum` and split into fifths. The top fifth is the buy list
   and the bottom fifth is the short list.
6. Look up the following month's return for each name on both lists, average each side, and subtract the
   short side's average from the long side's average. Then subtract 0.40 percent when both sides are
   replaced.

What to notice: the fast-growing tenth and the strong-momentum fifth overlap only partly, and the overlap
changes from month to month. Count how often the buy list and the short list are the same companies as the
month before. When they are, the cost line is small; when they are not, the cost line eats a large part of a
month that looked good before costs. Also notice how often a company is in the fast-growing tenth with
strong momentum and still falls the next month.

## Where this came from

- [The awesome-systematic-trading list](https://github.com/paperswithbacktest/awesome-systematic-trading), and
  its implementation file for
  [momentum-factor-combined-with-asset-growth-effect](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/momentum-factor-combined-with-asset-growth-effect.py),
  which states the universe, the asset-growth screen, the momentum window, the January rule and the
  weights.
- [Quantpedia: Momentum Factor Combined with Asset Growth Effect](https://quantpedia.com/strategies/momentum-factor-combined-with-asset-growth-effect),
  the rules and the extracted performance figures, including the worst fall and the significance value.
- Nyberg and Poyry, [Firm Expansion and Stock Price Momentum](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1684767),
  the source paper.
- [the predictability brief](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's brief on momentum design and the replication record of cross-sectional predictors.

## Words used in this tutorial

- asset growth: the percentage change in everything a company owns, from one annual balance sheet to the
  next.
- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- borrow fee: the charge for borrowing something you do not own in order to sell it short.
- drawdown: the fall from a peak to the following low, measured in percent.
- long: owning something, so you gain if its price rises.
- momentum: the tendency of something that has been rising to keep rising for a while.
- percentile: a cut-off expressed as a position in a ranked list, so the twentieth percentile is the bottom
  fifth.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
