# Buying shares that are both rising in price and growing their profit

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                      |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | Shares of the hundred most heavily traded American companies priced above five dollars                                                                                                                                                     |
| How often it trades       | Once a quarter, when the whole list is rebuilt                                                                                                                                                                                             |
| What you need             | A spreadsheet and a few years of quarterly prices and profit figures                                                                                                                                                                       |
| Where the rules come from | [QuantConnect strategy library, price and earnings momentum](https://www.quantconnect.com/tutorials/strategy-library/price-and-earnings-momentum)                                                                                          |
| The underlying research   | Jegadeesh and Titman, [Momentum](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=299107)                                                                                                                                               |
| How well it held up       | Mixed: the grade rests on momentum and its profit-based cousin being among the most replicated patterns in finance, set against an implementation on the page that lost money over a decade and a published worst fall of about 87 percent |
| Also appears in           | [Standardized unexpected earnings](../standardized-unexpected-earnings/README.md), which uses the profit-growth half on its own, and [sector momentum](../sector-momentum/README.md), which uses the price half on industry baskets        |

## The idea in one paragraph

Two things are said to keep going: a share price that has been rising, and a company's profit that
has been growing. This strategy measures both. It looks at how much each share rose over the past
quarter, and at how much the company's profit per share grew over the same period, ranks every share
on each measure, adds the two ranks, and buys the shares with the best combined score. It sells short
the shares with the worst. The bet is that the two signals are the same story told twice, one by the
market and one by the accounts, so a share that scores well on both is the one to hold for the coming
quarter.

## Why anyone believed it

The counterparty is an investor who reacts slowly to good news, or who sells a rising share simply
because it has risen and the gain feels too large. When a company's profit beats expectations, the
share price does not jump to its new level at once. Analysts revise their forecasts over weeks, other
investors notice the trend over months, and each new buyer pushes the price a little further. That is
the momentum in price, and it is the visible half of the same slow adjustment that the profit growth
measures directly.

Adding the profit measure is meant to separate two kinds of rising share. One has risen because the
company is genuinely earning more, and its price and its profit are moving together. The other has
risen for no fundamental reason and may be a bubble. A share that scores well on both price and
profit is more likely to be the first kind. The published work also finds that profit momentum and
price momentum each predict returns even after allowing for the other, which is why combining them is
supposed to be better than either alone.

## An everyday comparison

Think of choosing a football team for the coming season by looking at two records at once: how many
matches the team won last season, and how much its training fitness improved. A team that won a lot
may have been lucky; a team whose fitness improved may not yet be winning. A team that both won and
got fitter is the safer bet, because two independent signs point the same way. The strategy does not
choose the team with the single best record on either scale; it chooses the ones near the top of
both lists, which is why it adds the two rankings rather than using either one alone.

## The rules, step by step

1. Build a list of the hundred most heavily traded American shares priced above five dollars that
   have quarterly profit data available. "Most heavily traded" means the largest dollar volume,
   which is the number of shares traded multiplied by the price.
2. For each share, compute its return over the past quarter: the price at the end of the quarter
   divided by the price at the start, minus one. A share that went from 50.00 to 56.00 returned 0.12,
   that is 12 percent.
3. Rank the shares by that return, best first. The best gets rank 1, the next rank 2, and so on.
4. For each share, compute its profit growth over the quarter: this quarter's earnings per share
   minus last quarter's, divided by last quarter's. A company that went from 1.00 to 1.20 per share
   grew 0.20, that is 20 percent.
5. Rank the shares by profit growth in the same way.
6. For each share, add its two ranks together and divide by two. A share that ranks third on price
   and fifth on profit has a combined score of four.
7. Buy the ten shares with the smallest combined score. Sell short the ten with the largest. Buying
   the smallest score means buying the shares that did best on both measures.
8. Give every selected share the same weight: one divided by twenty, with the bought names positive
   and the sold names negative. The QuantConnect code fixes the number at ten.
9. Hold for a quarter. At the end of the quarter, sell whatever has left the two lists, buy whatever
   has entered, and repeat from step 2.

## The maths, with every symbol named

Four small calculations per share, then two sorts and one average.

The two measures:

```text
r_i = P_end / P_start - 1
g_i = (E_now - E_prev) / E_prev
```

- `r_i` is the price momentum of share `i` over the quarter, as a decimal.
- `P_end` is its price at the end of the quarter and `P_start` its price at the start.
- `g_i` is its profit growth: the change in earnings per share divided by the previous quarter's
  earnings per share.
- `E_now` is this quarter's earnings per share and `E_prev` the previous quarter's.

The two ranks and the combined score:

```text
rank_price(i) = the position of share i when the shares are sorted by r_i, best first
rank_profit(i) = the position of share i when the shares are sorted by g_i, best first
C_i = ( rank_price(i) + rank_profit(i) ) / 2
```

- `rank_price(i)` is 1 for the strongest price momentum and rises as the momentum weakens.
- `rank_profit(i)` is 1 for the fastest profit growth and rises as the growth slows.
- `C_i` is the combined score. The smallest combined score belongs to the share that did best on
  both measures, so the strategy buys the smallest scores, not the largest.

The weights, the return of the book and the cost:

```text
w_i = +1/20 for each of the ten best scores, -1/20 for each of the ten worst
R_quarter = SUM( w_i * r_next_i )
Cost = t * c
```

- `w_i` is the fraction of the money placed in share `i`, with a positive sign for a purchase and a
  negative one for a sale.
- The positive weights add to +0.5 and the negative weights to -0.5, so the book is half bought and
  half sold, with no net exposure to the general market.
- `r_next_i` is the return of share `i` over the following quarter.
- `R_quarter` is the return of the whole book for the quarter.
- `t` is the traded fraction. Selling the old holdings and buying the new ones counts twice, so a
  full rebuild costs about 2.0.
- `c` is the cost per side as a fraction of the amount traded, covering the bid-ask gap and any
  commission. For shares of this size a realistic figure is 0.001, that is ten basis points, where
  one basis point is one hundredth of one percent.

## A worked example

Eight shares in the universe, with their price return and profit growth over one quarter. The
numbers are invented but of the size these measures actually take.

| Share | Quarter price return (%) | Profit growth (%) | Price rank | Profit rank | Combined score | Action     |
| ----- | ------------------------ | ----------------- | ---------- | ----------- | -------------- | ---------- |
| S1    | +18                      | +40               | 1          | 1           | 1.0            | buy        |
| S2    | +12                      | +25               | 2          | 2           | 2.0            | buy        |
| S3    | +9                       | +10               | 3          | 4           | 3.5            | none       |
| S4    | +6                       | +15               | 4          | 3           | 3.5            | none       |
| S5    | +3                       | -5                | 5          | 6           | 5.5            | none       |
| S6    | 0                        | +8                | 6          | 5           | 5.5            | none       |
| S7    | -4                       | -10               | 7          | 7           | 7.0            | sell short |
| S8    | -9                       | -30               | 8          | 8           | 8.0            | sell short |

S1 and S2 top both lists and are bought; S7 and S8 are at the bottom of both and are sold, each with
a weight of 0.25. Suppose the next quarter produced these returns.

| Share | Weight | Direction | Quarter return | Contribution    |
| ----- | ------ | --------- | -------------- | --------------- |
| S1    | +0.25  | bought    | +5.0 percent   | +1.2500 percent |
| S2    | +0.25  | bought    | +4.0 percent   | +1.0000 percent |
| S7    | -0.25  | sold      | -5.0 percent   | +1.2500 percent |
| S8    | -0.25  | sold      | -6.0 percent   | +1.5000 percent |
| Total | 0.00   |           |                | +5.0000 percent |

The book gained 5.00 percent before costs, a quarter in which the winners kept winning and the losers
kept falling. Suppose three of the four names change at the next rebuild, so the traded fraction is
`t = 2 * (3 / 4) = 1.5`, and the cost at ten basis points per side is 1.5 * 0.001 = 0.0015, that is
0.15 percent. The net return for the quarter is 5.00 - 0.15 = 4.85 percent. Four quarters at that
rate is about 21 percent a year, far above the published averages below, which is exactly why the
example is only a check on the arithmetic and not a forecast. Momentum has quarters this good and
quarters that give much of it back.

## What the research actually found

| Source                                               | What it measured                                                              | Result                                                                                                                                                                                                          |
| ---------------------------------------------------- | ----------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Jegadeesh and Titman, the study the page cites       | Shares ranked on their returns over the past three to twelve months, US data  | Shares that did best over three to twelve months tended to keep doing well over the following three to twelve months; stocks with high profit momentum also beat those with low profit momentum (paper summary) |
| Chan, Jegadeesh and Lakonishok, cited by Quantpedia  | Price momentum and profit momentum used together                              | Past returns and past profit surprises each predict future returns even after allowing for the other, with little sign of reversal afterwards (paper abstract)                                                  |
| Quantpedia, the momentum factor entry                | The one-year price momentum factor, US shares, 1927 to 2013                   | About 8.3 percent a year, volatility about 16.6 percent, reward-for-risk about 0.5, and a worst fall of about 87.4 percent (Quantpedia entry)                                                                   |
| Quantpedia, on the same factor in 2009               | The momentum factor's worst episode                                           | The pure long-short momentum portfolio fell more than 80 percent in 2009, which is the crash the page's caution is about                                                                                        |
| QuantConnect, the implementation on the library page | A hundred liquid names, best ten bought and worst ten sold, quarterly rebuild | A reward-for-risk figure of -0.268 against 0.758 for the index it compared with; the page names a too-small universe, equal weighting and quarterly rebuilding as likely causes                                 |

The picture is a sharp contrast between the underlying idea and this particular version of it. Price
momentum and profit momentum are two of the most replicated patterns in the published record, with
decades of evidence across many markets. The implementation on the page, however, lost money over a
decade, and the page itself lists the reasons it might have: a hundred shares is a small universe, so
one or two names dominate; equal weighting means a weak signal has as much money as a strong one; and
rebuilding every quarter may be too often for a signal whose measurement noise is a whole quarter of
prices. Momentum is also not a smooth reward: the published worst fall of about 87 percent shows what
happens when the market reverses sharply and the shares that fell furthest were the ones the rule had
been buying.

## How this project relates to it

The idea behind this strategy is measured directly in two places in this repository, and the
mechanics of one half are already built.

The brief [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md)
is the closest match. It reports that momentum strategies are positively skewed by construction, even
when the underlying returns are symmetric, and that the skewness depends on the holding horizon,
rising to a peak and then shrinking, citing `2101.01006v2`. That is the honest shape of this
strategy: it wins often and loses hugely occasionally, and it should be judged at the horizon it
trades. The brief also reports that cross-sectional claims weaken when restated under value weighting,
which matters here because this rule weights every name equally regardless of size.

Two sibling tutorials cover the halves separately. [Sector momentum](../sector-momentum/README.md)
runs the price half on industry baskets, and [Standardized unexpected earnings](../standardized-unexpected-earnings/README.md)
runs the profit half on its own. Reading all three together shows what the page's combination is
trying to do: two views of the same slow adjustment to news, one from prices and one from the
accounts.

## Where it goes wrong

- Momentum crashes. After a market-wide fall, the shares that had risen most are often the ones that
  fall hardest when the market rebounds, because the rule has been buying the losers of the fall and
  they reverse. The published 87 percent fall is a real outcome of the rule, not a data error.
- Two measurement windows, one quarter. A quarter of prices is a noisy number, and a company's
  profit can move because of a one-off item. Both measures are crude, which is why the published work
  usually averages several past periods rather than using one quarter.
- Costs on the whole list every quarter. Twenty positions rebuilt every three months is a lot of
  trading, and the shares at the bottom of the list, which must be borrowed and sold, are the ones
  with the widest bid-ask gaps and the highest borrowing fees.
- Equal weighting and a small list. With only twenty positions, each at the same weight, a single
  share can dominate the result, and three bad shares can erase a year of gains. The page's own
  explanation of its weak result starts with the hundred-share universe.
- The number of choices. One quarter or one year, ten names or fifty, sum the ranks or average them,
  equal or weighted. Trying many versions and keeping the one that looks best after seeing the
  results is how a weak rule is made to look strong.
- What would have to be true for the idea to be false: that a share's recent price strength and its
  recent profit growth contain no information about the next few months. The published record argues
  the opposite, but it also argues that the reward is paid in bursts and can be taken back quickly.

## Try it yourself

You need a spreadsheet, quarterly closing prices for a few dozen well-known shares, and their
quarterly earnings per share. Any finance website publishes both.

1. Build one row per share and one block holding each share's closing price at the end of the last
   four quarters.
2. Add a column computing each share's return over the last quarter: this quarter's price divided by
   last quarter's, minus one.
3. Add a column computing profit growth: this quarter's earnings per share minus last quarter's,
   divided by last quarter's.
4. Add two rank columns, one for the price return and one for profit growth, using the spreadsheet's
   rank function so that the best value gets rank 1.
5. Add a combined score column: the average of the two ranks.
6. Sort by the combined score and write down the top three and the bottom three.
7. In the following quarter's columns, average the returns of the top three and subtract the average
   of the bottom three. That is the strategy's return for the quarter, before costs.
8. Subtract 0.20 percent per quarter for a full rebuild.

What to notice: the two rank columns disagree often, and the shares that top both are usually a short
list, sometimes empty. Where they disagree, the combined score puts a share in the middle and it is
neither bought nor sold, so the strategy is mostly a way of finding the few shares that two noisy
signals agree on. Compare the combined score's result over a year with the price rank alone and with
the profit rank alone; the paper's claim is that the combination is more reliable, not that it wins
every quarter.

## Where this came from

- [QuantConnect strategy library: price and earnings momentum](https://www.quantconnect.com/tutorials/strategy-library/price-and-earnings-momentum),
  the hundred-share universe, the two measures, the combined rank and the reported result.
- Jegadeesh and Titman, [Momentum](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=299107),
  the original evidence for price momentum and profit momentum together.
- [Quantpedia: momentum factor effect in stocks](https://quantpedia.com/strategies/momentum-factor-effect-in-stocks),
  the restatement of price momentum, the performance figures and the 2009 crash.
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's brief on momentum's shape, its horizon and the value-weighting test.

## Words used in this tutorial

- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- earnings per share: a company's quarterly profit divided by the number of shares it has issued.
- momentum: the tendency of something that has been rising to keep rising for a while.
- profit growth: the change in earnings per share from one quarter to the next, as a fraction of the
  earlier figure.
- rank: the position of a value in a sorted list, where the best value is rank 1.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- universe: the list of things a strategy is allowed to choose from.
- weight: the share of your money placed in one holding.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
