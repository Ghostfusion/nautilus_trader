# Fund momentum: trading the companies that manage the funds

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                 |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of listed companies that manage mutual funds, in a long basket and a short basket                                                                                                                                                                              |
| How often it trades       | It ranks monthly, and each bet is intended to be held for about six months                                                                                                                                                                                            |
| What you need             | A spreadsheet and six to twelve months of price history for the asset-management shares                                                                                                                                                                               |
| Where the rules come from | [QuantConnect strategy library, momentum in mutual fund returns](https://www.quantconnect.com/tutorials/strategy-library/momentum-in-mutual-fund-returns) and the [Quantpedia entry](https://quantpedia.com/strategies/momentum-in-mutual-fund-returns) it cites      |
| The underlying research   | Sapp, [The 52-Week High, Momentum, and Predicting Mutual Fund Returns](http://papers.ssrn.com/sol3/papers.cfm?abstract_id=1462408)                                                                                                                                    |
| How well it held up       | Weak: the underlying fund-level pattern is well documented in the research, but this tradable version, which buys the managers' own shares instead of the funds, lagged a simple market fund in the implementation's own five-year test with no help from lower costs |
| Also appears in           | [Sector momentum](../sector-momentum/README.md) and [momentum with a market filter](../momentum-and-state-of-market-filters/README.md) in this collection, the same momentum idea on other things                                                                     |

## The idea in one paragraph

A mutual fund is a pool of money that many people pay into, run by a professional manager who buys
shares on their behalf. Research found that funds which did well over the past few months tend to do
well for a while longer, and that the funds sitting closest to their highest value of the past year
also tend to do well. Those patterns are useful, but a reader cannot always buy the funds
themselves. This strategy instead buys the shares of the listed companies that manage the funds,
such as the firms whose names are on the fund prospectus. It ranks those companies by two numbers:
how much their share price changed over six months, and how close the price is to its highest level
of the past year. It buys the strongest quarter and bets against the weakest quarter, and holds for
about six months.

## Why anyone believed it

A fund's past performance attracts new money. Investors pile into funds with strong recent returns,
which brings the manager more fees and pushes the manager's own share price up. If the pattern is
real, then the managers' shares move with the money flow, so the shares are a way to ride the same
wave without being able to buy the funds.

The second idea comes from behaviour. Investors treat a price near its old high as a sign of quality
and are slow to sell, while a price far below its high feels damaged. That stickiness makes a share
near its high keep rising a little, and a share far below its high keep sagging. The counterparty is
the investor who chases the best-performing fund late, and the one who anchors on the old high price.

## An everyday comparison

Think of a city where restaurants are ranked by the newspaper each month. Diners crowd the ones near
the top of the list, so those places sell more and can raise their prices. Now suppose you cannot get
a table at the popular restaurants, but you can buy shares in the companies that own them. The
owners of the top-ranked restaurants get richer as the crowds arrive. That is the trade here: not the
meal, but the company that serves it. It works while the crowds keep coming and stops when the
ranking changes and the queues move elsewhere.

## The rules, step by step

1. Start with every company classified as being in the asset-management industry. This is the line of
   business that runs funds and manages money for others.
2. For each company, compute the rate of change over the past six months: the latest share price
   divided by the price six months ago, minus one.
3. For each company, compute the nearness: the latest share price divided by the highest price the
   share reached at any point over the past twelve months.
4. Give every company two ranks. Rank by rate of change, so the weakest is rank 1 and the strongest
   is rank N, where N is the number of companies. Do the same for nearness. Then add the two ranks.
5. Sort the companies by that sum, highest first.
6. Buy the top quarter of the list. Sell short the bottom quarter. Short selling means borrowing
   shares you do not own, selling them now and buying them back later; if the price falls you keep
   the difference, and if it rises you lose.
7. Spread each side equally, so the long side is fifty percent of the money and the short side is
   fifty percent sold short, and the account has no net money invested in shares.
8. Rank again at the start of each month, but let each bet live for about six months, so that the
   holdings change gradually rather than all at once.
9. A company that carries both a buy and a sell instruction at the same time is left alone for that
   month.

## The maths, with every symbol named

Two scores per company, then a ranking.

The rate of change over the past six months:

```text
ROC = P_now / P_6m_ago - 1
```

- `ROC` is the rate of change of the share, a decimal: 0.20 means 20 percent.
- `P_now` is the latest share price.
- `P_6m_ago` is the price six months earlier.

The nearness to the trailing high:

```text
Nearness = P_now / High_12m
```

- `Nearness` is how close the share is to its best level of the past year, a decimal at or below 1.
  A value of 0.95 means the share is five percent below its one-year high.
- `P_now` is the latest share price.
- `High_12m` is the highest price the share reached over the past twelve months.

Rank the companies twice over the same list, once by `ROC` and once by `Nearness`, from 1 for the
lowest value up to `N` for the highest. Add the two ranks for each company:

```text
Score_i = Rank_ROC_i + Rank_Nearness_i
```

- `Score_i` is the combined ranking score of company `i`.
- `Rank_ROC_i` is its position in the rate-of-change ranking.
- `Rank_Nearness_i` is its position in the nearness ranking.
- `N` is the number of companies in the list, so the highest possible score is `2 * N` and the
  lowest is 2.

Sort by `Score_i` from largest to smallest. Buy the top quarter and short the bottom quarter. The
two sides are equal and opposite, so the return over the holding period is:

```text
R = 0.5 * (R_long - R_short)
```

- `R` is the return on the account over the period, a decimal.
- `R_long` is the average return of the companies bought.
- `R_short` is the average return of the companies sold short.
- The 0.5 appears because the account is half long and half short.

The cost of trading:

```text
Cost = T * c
```

- `T` is the two-way turnover in units of the whole account. Replacing half of each basket means
  selling 0.5 and buying 0.5 on each side, so `T = 2.0`.
- `c` is the cost of one trade as a fraction of the amount traded. The managers' shares are liquid
  and trade in narrow gaps, so 0.001, or 10 basis points, is a reasonable working figure; one basis
  point is one hundredth of one percent. The short side also pays a small fee to borrow.

The return the account keeps is `R - Cost`.

## A worked example

Eight invented asset-management companies. The price columns are in one currency, and the twelve-
month high is the best price of the past year.

| Company | Price six months ago | Price now | Rate of change | One-year high | Nearness |
| ------- | -------------------- | --------- | -------------- | ------------- | -------- |
| A       | 100                  | 130       | +0.30          | 132           | 0.985    |
| B       | 50                   | 60        | +0.20          | 65            | 0.923    |
| C       | 80                   | 92        | +0.15          | 100           | 0.920    |
| D       | 200                  | 220       | +0.10          | 230           | 0.957    |
| E       | 40                   | 42        | +0.05          | 50            | 0.840    |
| F       | 90                   | 90        | +0.00          | 95            | 0.947    |
| G       | 60                   | 54        | -0.10          | 70            | 0.771    |
| H       | 30                   | 25.5      | -0.15          | 40            | 0.638    |

Rank each column from 1 for the lowest to 8 for the highest, then add the ranks.

| Company | Rate of change | Rank | Nearness | Rank | Sum |
| ------- | -------------- | ---- | -------- | ---- | --- |
| A       | +0.30          | 8    | 0.985    | 8    | 16  |
| B       | +0.20          | 7    | 0.923    | 5    | 12  |
| C       | +0.15          | 6    | 0.920    | 4    | 10  |
| D       | +0.10          | 5    | 0.957    | 7    | 12  |
| E       | +0.05          | 4    | 0.840    | 3    | 7   |
| F       | +0.00          | 3    | 0.947    | 6    | 9   |
| G       | -0.10          | 2    | 0.771    | 2    | 4   |
| H       | -0.15          | 1    | 0.638    | 1    | 2   |

The top quarter of eight is two companies, so the long basket is A and, on the tie between B and D at
12, the one with the higher rate of change, which is B. The bottom two are H and G. Now suppose the
next six periods bring these returns, with the ranking refreshed and positions rolled gradually.

| Period | Long side return | Short side return | Spread | Half of spread | Turnover | Cost  | Net    |
| ------ | ---------------- | ----------------- | ------ | -------------- | -------- | ----- | ------ |
| 1      | +2.5%            | -3.5%             | +6.0%  | +3.00%         | 2.0      | 0.20% | +2.80% |
| 2      | +1.0%            | -1.0%             | +2.0%  | +1.00%         | 4.0      | 0.40% | +0.60% |
| 3      | -2.0%            | -0.5%             | -1.5%  | -0.75%         | 4.0      | 0.40% | -1.15% |
| 4      | +4.0%            | +1.0%             | +3.0%  | +1.50%         | 2.0      | 0.20% | +1.30% |
| 5      | +3.0%            | -2.0%             | +5.0%  | +2.50%         | 4.0      | 0.40% | +2.10% |
| 6      | -1.0%            | +2.0%             | -3.0%  | -1.50%         | 2.0      | 0.20% | -1.70% |

The arithmetic for period 1: the spread is 2.5 minus (-3.5), which is 6.0 percent, and half of that
is 3.00 percent. Turnover of 2.0 units at 10 basis points is 2.0 times 0.001, which is 0.0020, or
0.20 percent. Net is 3.00 minus 0.20, which is 2.80 percent. A turnover of 4.0 means the baskets were
almost entirely replaced that period, which is unavoidable when each side holds only two companies.

The net values add to 3.95, an average of 0.66 percent a period. Compounding the six factors (1.0280
times 1.0060 times 0.9885 times 1.0130 times 1.0210 times 0.9830) gives 1.0393, so the account grew
by 3.93 percent over six months, which is about 8 percent a year if it kept that pace. Note that this
is the return on the paired position, not a guaranteed outcome, and that a small basket like this one
turns over heavily and pays for it.

## What the research actually found

| Source                            | What it measured                                                                                                                      | Result                                                                                                                                                                                                                                                           |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Sapp, on fund net asset value     | Funds sorted by nearness to the one-year high of their value, by recent extreme returns, and by sensitivity to momentum, 1973 to 2004 | All three measures predicted later fund returns, and the three carried largely separate information, with correlations near zero; nearness to the high and recent extreme returns also predicted which funds attracted money, while momentum sensitivity did not |
| Quantpedia, from the same work    | The top decile of funds by six-month return, held three months                                                                        | An indicative 19 percent a year, or 1.46 percent a month, with volatility 19.5 percent and reward-to-risk 0.77, over 1973 to 2004, and a confidence grade of Strong for the fund-level pattern                                                                   |
| Sapp and Tiwari                   | A quarterly rule buying the top decile of funds by momentum exposure                                                                  | An annualised three-factor excess return of 3.72 percent over 1973 to 2000                                                                                                                                                                                       |
| Friesen and Nguyen                | Fund investor behaviour, 1992 to 2016                                                                                                 | The tendency of investors to chase recent returns almost disappeared from 2011, as flows became more sensitive to costs and risk instead of to past returns                                                                                                      |
| QuantConnect's own implementation | The tradable version, buying the managers' shares, 2015 to 2020                                                                       | A reward-to-risk of 0.192 against 0.709 for a plain market fund; it beat the market fund sharply during the 2020 crash, at 10.386 against minus 1.243, but lost to it in every other window tested; removing all trading costs did not close the gap             |

Read together: the fund-level pattern is one of the better documented results in fund research. The
tradable proxy is a different thing, and in this implementation's own sample it did not match a
simple market fund. The two findings can both be true because the shares of a manager move with the
whole market and with the manager's own business, not just with the performance of its funds.

## How this project relates to it

This repository's study of the momentum family at the sector level is in
[Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), which sets out
the condition momentum needs and collects the tests, including the 1,022-rule experiment whose
average rule failed to beat simply holding the market.

The reading of momentum as a design problem, rather than a forecast, is in
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md).
It reports that a momentum rule's reward depends on how the ranking is defined and how the position
is scaled, not only on the market, which is why the ranksum design used here matters.

The third brief is
[Portfolio construction and allocation](../../../strategies/books2/10_portfolio_and_allocation.md),
whose evidence from replicated predictors is that the average cross-sectional signal keeps only a
part of its in-sample edge once it is published and traded. That is the likely fate of a rule built
on a public fund-ranking effect.

## Where it goes wrong

- The proxy is not the thing. Buying the manager's shares is not the same as buying the managed
  funds, because the shares also carry the manager's other business, its costs and the market's own
  mood, which can drown the fund effect.
- The universe is tiny. There are only a few dozen listed asset managers, so the top and bottom
  quarters are small baskets and a single company can swing the result.
- The borrowing fee. The short side must borrow shares, and the weakest managers can be the most
  expensive to borrow, so part of the measured spread can be eaten before it is captured.
- Chasing a published effect. Investor return-chasing itself appears to have faded after 2011, which
  removes the money flow that the managers' shares were supposed to ride.
- Costs on a monthly re-rank. Refreshing the ranking each month while holding each bet for six
  months keeps turnover moderate, but a small universe still trades a large fraction of itself.
- What would have to be true for the idea to be false: that fund returns do not persist, that the
  flow of money into past winners is not strong enough to move the managers' shares, or that the
  manager's share price is driven by things unrelated to its funds.

## Try it yourself

You need a spreadsheet and a public source of prices for a handful of listed asset managers.

1. List six to ten asset-management companies and collect their monthly prices for the past two
   years.
2. Add a column for the six-month rate of change: this month's price divided by the price six rows
   up, minus one.
3. Add a column for the highest price over the previous twelve rows.
4. Add a column for nearness: this month's price divided by that high.
5. Add two rank columns, one for rate of change and one for nearness, ranking each row across the
   companies that month.
6. Add a column for the sum of the two ranks. The top two sums are the buys; the bottom two are the
   sells.
7. In the next row, record what those four shares actually did, and average the buys and the sells
   separately.

What to notice: the two scores will often disagree, placing a strong six-month performer low on
nearness and vice versa. Watch what happens to the buys in the month after a sharp fall in the market
as a whole. If the whole basket falls together, the pattern being measured is the market rather than
the managers, and the rule is mostly a market bet in disguise.

## Where this came from

- [QuantConnect strategy library: momentum in mutual fund returns](https://www.quantconnect.com/tutorials/strategy-library/momentum-in-mutual-fund-returns),
  the rules as implemented: the asset-management universe, the six-month rate of change, the
  twelve-month nearness, the top and bottom quarters, and the six-month holding period. The library
  page now redirects, and the strategy text was taken from the published tutorial and its companion
  [forum announcement](https://www.quantconnect.com/forum/discussion/9074/strategy-library-addition-momentum-in-mutual-fund-returns/).
- [Quantpedia: momentum in mutual fund returns](https://quantpedia.com/strategies/momentum-in-mutual-fund-returns),
  the fund-level figures, the confidence grade and the underlying papers.
- Sapp, [The 52-Week High, Momentum, and Predicting Mutual Fund Returns](http://papers.ssrn.com/sol3/papers.cfm?abstract_id=1462408),
  the study of the nearness-to-high measure for funds.
- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md),
  [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md)
  and [Portfolio construction and allocation](../../../strategies/books2/10_portfolio_and_allocation.md),
  this repository's own reading of momentum and of how cross-sectional signals decay.

## Words used in this tutorial

- asset-management company: a listed company whose business is running funds and managing money for
  others.
- long basket: the group of holdings you buy because you expect them to rise.
- momentum: the tendency of something that has been rising to keep rising for a while.
- mutual fund: a pool of money from many investors, run by a manager who buys and sells on their
  behalf.
- net asset value: the value of one share of a fund, being the fund's holdings divided by the number
  of shares.
- short basket: the group of holdings you sell short because you expect them to fall.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- turnover: how much of a portfolio is sold and replaced over a period.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
