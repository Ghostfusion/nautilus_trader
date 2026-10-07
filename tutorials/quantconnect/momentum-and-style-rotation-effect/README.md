# Style momentum: rotating between the kinds of shares that have been winning

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                               |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Six exchange-traded funds, each tracking one combination of company size (small, mid or large) and valuation style (value or growth)                                                                                                                                                                                                |
| How often it trades       | About once a month, when the pair of funds is rebuilt                                                                                                                                                                                                                                                                               |
| What you need             | A spreadsheet and twelve months of prices for six style funds                                                                                                                                                                                                                                                                       |
| Where the rules come from | [QuantConnect strategy library, momentum and style rotation effect](https://www.quantconnect.com/tutorials/strategy-library/momentum-and-style-rotation-effect) and the [Quantpedia entry](https://quantpedia.com/strategies/momentum-factor-and-style-rotation-effect) it cites                                                    |
| The underlying research   | Tibbs, Eakins and DeShurko, [Using Style Index Momentum to Generate Alpha](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1276815)                                                                                                                                                                                             |
| How well it held up       | Mixed: the long-short version was profitable over a long American sample and in several replications, but later tests across more index families found the long-short profit statistically indistinguishable from zero while only the long-only form earned a reliable excess, so the grade rests on which construction is measured |
| Also appears in           | [Sector momentum](../sector-momentum/README.md) in this collection, the same cross-sectional momentum idea applied to industry funds instead of style funds                                                                                                                                                                         |

## The idea in one paragraph

Shares can be sorted in two ways at once: by how big the company is (small, medium or large) and
by whether the shares look cheap or expensive relative to the company's earnings (the value and
growth styles). Crossing the two gives six groups, and a fund exists for each. This strategy
measures how each of the six groups performed over the past year, buys the best-performing one and
bets against the worst-performing one, holding both for a month. It does this by buying the style
fund that has been strongest and selling short the style fund that has been weakest, placing equal
amounts on each of the two sides. The bet is that the style in favour stays in favour for a while
and the style out of favour stays out of favour, so the gap between the two keeps moving the way it
has been moving.

## Why anyone believed it

Style is partly a fashion and partly an economic fact. The economic part is real: value companies,
such as banks and energy producers, respond differently to interest rates and to the business cycle
than growth companies, such as software firms, and those differences play out over many months
rather than in a day. The fashion part is the reason the move continues after the news is known.
Money moves toward whatever category has been doing well, because funds are judged against their
peers and against a style benchmark; a manager who is underweight the winning style risks losing
clients even if the choice is defensible.

The counterparty is therefore the investor who is slow to move, and the investor who is forced to
move for reasons unrelated to the outlook. A value manager whose growth holdings have grown far
beyond their intended size trims them, which is selling the winner; a fund that is losing clients
because it holds the unpopular style must sell that style, which is selling the loser. If those
flows keep arriving, the winner keeps winning and the loser keeps losing, which is exactly what the
strategy needs.

## An everyday comparison

Think of a small shop that sells one soft drink in six flavours: cola, lemon, orange, ginger and so
on. The flavour that outsold the others over the past year tends to keep outselling them for a few
more months. Shoppers buy what they bought last time and recommend it to friends, the shop gives
the best-selling flavour the front of the shelf and orders more of it, and the weak flavour is
pushed toward the back where fewer people find it. Neither change has anything to do with how the
drink tastes; both changes make the winner sell better and the loser sell worse. The strategy here
is to place a bet that the best-selling flavour keeps selling and the worst-selling flavour keeps
failing to sell.

## The rules, step by step

1. Choose six style funds. In the American market these are commonly the iShares funds for the six
   combinations of size and style: small-cap value, small-cap growth, mid-cap value, mid-cap growth,
   large-cap value and large-cap growth. Each fund tracks an index of shares in that group.
2. For each fund, compute its return over the past twelve months: take the price twelve months ago,
   take today's price, and divide. A fund that went from 100.00 to 116.00 has a twelve-month return
   of 16 percent.
3. Rank the six funds by that return, best first.
4. Buy the top fund, placing half of the account in it, and sell short the bottom fund, placing the
   other half of the account in that short position. The long side gains when its fund rises; the
   short side gains when its fund falls.
5. Hold for one month. Do not look at the prices in between.
6. At the start of the next month, recompute step 2 for all six funds and repeat from step 3. Buy
   the fund that has entered the top spot and sell the one that has entered the bottom spot;
   close any position that is no longer wanted.
7. If you prefer the long-only variant, buy the top fund with the whole account and hold it, which
   both sources also allow. That version is much closer to simply owning the stock market.

One refinement worth knowing: many researchers skip the most recent month when measuring momentum,
using the return from twelve months ago to one month ago, because prices tend to bounce back over
days and weeks after a sharp move. That convention is written as 12-1 momentum. This particular
implementation uses the full twelve months, which the QuantConnect page states as a formation
period of twelve months.

## The maths, with every symbol named

The whole strategy is one calculation repeated six times, one sort, and one subtraction.

The twelve-month return of a style fund:

```text
M_i = P_i_today / P_i_twelve_months_ago - 1
```

- `M_i` is the momentum score of style fund `i`, written as a decimal: 0.16 means 16 percent.
- `P_i_today` is that fund's price today.
- `P_i_twelve_months_ago` is that fund's price on the same day twelve months earlier.

Rank the six funds by `M_i` from largest to smallest. Give the winner weight `+0.5` and the loser
weight `-0.5`:

```text
w_winner = +0.5        w_loser = -0.5        all other weights = 0
```

- `w` is the fraction of the account placed in a fund. A positive weight is a purchase; a negative
  weight is a short sale, meaning you borrow the fund, sell it, and buy it back later.
- Because `+0.5` and `-0.5` cancel, the account owns half its value in one fund and owes half its
  value in another, so it is not pushed around by the market as a whole nearly as much as a
  long-only portfolio would be.

The return of that month is then half the winner's return minus half the loser's return:

```text
R_month = 0.5 * R_winner - 0.5 * R_loser
```

- `R_winner` and `R_loser` are the returns of the two chosen funds over the following month.
- The minus sign is there because a short position gains when its fund falls: if the loser's return
  is `-0.03`, meaning it fell by 3 percent, the term contributes `+0.015`, or 1.5 percent.

Finally the cost of rebuilding. If a fraction `t` of the account is traded at the rebuild, the cost
in that month is:

```text
Cost_month = t * c
```

- `t` is the traded fraction, counting both the sold side and the bought side. Replacing the long
  fund alone means selling half the account and buying half the account, so `t = 1.0`. Replacing
  both funds means `t = 2.0`. Replacing neither means `t = 0`.
- `c` is the cost per trade as a fraction of the amount traded, covering the gap between the buying
  and selling price plus commission. A realistic figure for large style funds is 0.0005 to 0.001,
  that is five to ten basis points, where one basis point is one hundredth of one percent. A short
  sale also pays a borrow fee to whoever lends the shares, which is small for big index funds but
  is not zero.

## A worked example

Six style funds, ranked by their return over the past twelve months. The prices are invented, but
they are of the size these funds actually move.

| Style fund       | Price a year ago | Price today | M      | Rank |
| ---------------- | ---------------- | ----------- | ------ | ---- |
| Small-cap value  | 100.00           | 121.00      | +0.210 | 1    |
| Small-cap growth | 100.00           | 116.00      | +0.160 | 2    |
| Mid-cap value    | 100.00           | 112.00      | +0.120 | 3    |
| Large-cap growth | 100.00           | 108.00      | +0.080 | 4    |
| Large-cap value  | 100.00           | 103.00      | +0.030 | 5    |
| Mid-cap growth   | 100.00           | 98.00       | -0.020 | 6    |

The chosen trade is long small-cap value and short mid-cap growth, half the account on each. Now
suppose six months pass and the ranking changes slowly, as it does in reality. Each row below is a
month: the pair the rules would hold, the two funds' returns over that month, the gross result of
the formula `0.5 * R_winner - 0.5 * R_loser`, then the traded fraction, the cost at ten basis points
per trade, and the net result.

| Month | Long fund        | Short fund       | Winner | Loser | Gross  | t   | Cost  | Net    |
| ----- | ---------------- | ---------------- | ------ | ----- | ------ | --- | ----- | ------ |
| 1     | Small-cap value  | Mid-cap growth   | +3.0%  | +0.5% | +1.25% | 1.0 | 0.10% | +1.15% |
| 2     | Small-cap value  | Mid-cap growth   | +0.5%  | -1.5% | +1.00% | 0.0 | 0.00% | +1.00% |
| 3     | Small-cap value  | Large-cap value  | +2.0%  | -0.5% | +1.25% | 1.0 | 0.10% | +1.15% |
| 4     | Small-cap value  | Large-cap value  | -1.0%  | -2.0% | +0.50% | 0.0 | 0.00% | +0.50% |
| 5     | Small-cap growth | Large-cap value  | +1.5%  | +0.5% | +0.50% | 1.0 | 0.10% | +0.40% |
| 6     | Small-cap growth | Large-cap growth | +0.5%  | -1.0% | +0.75% | 1.0 | 0.10% | +0.65% |

Read month 3 as an example: the short fund changed from mid-cap growth to large-cap value, so the
old short was closed and a new one opened, which trades half the account twice, so `t = 1.0`; the
gross `+1.25%` becomes `+1.15%` after `1.0 * 0.001 = 0.001`, that is 0.10 percent.

Compounding the six net figures: `1.0115 * 1.0100 * 1.0115 * 1.0050 * 1.0040 * 1.0065 = 1.0495`,
so the account gained about 4.95 percent over half a year, or a little over 10 percent a year if
the same pace continued. The published long-short figure is 9.25 percent a year, so the arithmetic
here is the right order of magnitude. Two things are worth noticing. First, the cost line is small
in the months when nothing changed and doubles in the months when the pair changed, which is the
whole reason a slow-moving rule survives costs and a jumpy one does not. Second, the example says
nothing about whether the strategy works; it only shows how to apply the rules and how the numbers
behave.

## What the research actually found

| Source                                 | What it measured                                                        | Result                                                                                                                                                                                             |
| -------------------------------------- | ----------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Quantpedia, summarising the literature | Six Russell style indexes, monthly long winner and short loser, US data | 9.25 percent a year for the long-short pair over 1972 to 2005, volatility 16.01 percent, worst fall 52.49 percent, reward-to-risk 0.33, six instruments, rebuilt monthly                           |
| Tibbs, Eakins and DeShurko             | Russell style indexes over a 34-year period                             | The long-minus-short portfolio averaged 9.25 percent a year; the returns were reported as robust through time, and the portfolio needed rebuilding on average only every six months                |
| Wang and Brooks                        | Nine S&P style indexes, June 1995 to March 2009                         | Buying the best past style and short-selling the worst earned 0.8 percent a month, and the profit stayed economically plausible after risk adjustment, short-sale costs and transaction costs      |
| Liu and Wang                           | Russell, Fama-French and MSCI style indexes                             | Long-only style momentum produced significant positive abnormal returns, while the long-short versions did not; the result also varied by index family and shrank as the holding period lengthened |
| Gokani and Todorovic                   | UK size and value/growth style indexes                                  | Value against growth rotation was not profitable under any method tested; small against large rotation was profitable at achievable cost levels, but only long-only or modestly long-short         |

Read together, the picture is this. There is a documented tendency for a style that has been strong
to stay strong over months, and it is one of the better replicated patterns in the momentum family.
But the long-only version is really a way of owning the stock market, because it rises and falls
with shares; the long-short version, which is what this page describes, is where the sources
disagree most, with one careful study finding the profit statistically indistinguishable from zero
across three families of indexes. The disagreement is about construction, not about whether style
returns move around.

## How this project relates to it

This repository has no style-rotation strategy of its own. The closest things are two research
documents.

The first is [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md),
this repository's own study of a rotation rule over a group of equity portfolios. Its Section 3.2
states the condition such a rule needs, that the leaders stay leaders for several months, and its
Section 3.3 collects the tests of that condition, including an experiment over 1,022 rotation rules
in American sectors whose average rule returned 0.86 percent a month against 0.89 percent for simply
holding the market. The style universe is smaller than the sector universe, but the arithmetic of
trying many rules and keeping the best applies here too.

The second is the harvest of the momentum literature in
[the predictability brief](../../../strategies/books2/08_predictability_and_trading_strategies.md).
Two of its findings bear directly on this page. It records that a momentum strategy's return is
positively skewed by construction and that the skewness depends on the holding horizon (`2101.01006v2`,
p.3), which means the reward here is lumpy: many small losses and a few large gains, not a steady
edge. And it records a model in which a published signal's half-life falls to about eighteen months
as adoption rises (`2605.23905v1`, p.1, p.5), which is the reason a rule this simple and this easy
to trade is a rule whose reward should be expected to shrink.

## Where it goes wrong

- The long-only version is just the market with extra steps. It is invested in shares at all times,
  so it falls when shares fall, and the measured improvement over the index is a few points a year
  before costs.
- The long-short version has a short leg with no ceiling on losses. A fund that is sold short can
  rise without limit, and a short squeeze in one style fund can wipe out months of the small edge
  the strategy collects.
- The pairs are highly correlated. Small-cap value and large-cap growth move together most of the
  time because both are shares, so the gap between them is small and noisy, and the profit has to
  be big enough to clear costs that are incurred every time the ranking changes.
- Later samples are less kind. The study that tested three index families found the long-short
  profit insignificant, and the UK test found value against growth rotation unprofitable at every
  setting; a rule that worked on Russell indexes in one sample may not be a property of style
  returns in general.
- The number of ways to define the rule is large: which twelve months, whether to skip the most
  recent month, how many styles to hold, how often to rebuild, whether to demand the winner also be
  above its own average. The 1,022-rule experiment in this repository's sector study shows how a
  best-of-many choice can look like skill when it is arithmetic.
- Costs and shorting frictions. Every rebuild pays the gap between buying and selling prices on both
  legs, a short sale pays a borrow fee, and a style fund that is thinly traded has a wider gap than
  the large ones. A rule that changes its mind every month pays roughly 2.4 percent a year at ten
  basis points per trade.

## Try it yourself

You need nothing but a spreadsheet and a public source of fund prices; a finance website will give
you monthly closing prices for the six iShares style funds.

1. Build a sheet with one column per style fund and one row per month for the last five years.
2. Add a column that computes the twelve-month return: today's price divided by the price twelve
   rows up, minus one.
3. Add two columns, for each month, naming the fund with the highest value in that row and the fund
   with the lowest. These are what the rules would have bought and sold short.
4. In the next row down, compute `0.5 * (winner's return) - 0.5 * (loser's return)`. That is the
   strategy's gross return for the month.
5. Add a column for the traded fraction: count how many of the two names changed from the previous
   month, and write 1.0 for each name that changed.
6. Subtract `t * 0.001` from the gross return, and multiply the monthly net figures together to get
   a running total.

What to notice: the long-short line rises and falls far less than either fund on its own, because
the two sides cancel the market's drift, and the months where the pair does not change are the
months that pay almost nothing in cost. If your sheet shows the strategy winning by a wide margin,
the likely cause is that you picked the six funds after seeing which ones had done best, or that
you ignored the borrow cost on the short side and the years when one style fund did not yet exist.

## Where this came from

- QuantConnect strategy library, momentum and style rotation effect,
  [the page](https://www.quantconnect.com/tutorials/strategy-library/momentum-and-style-rotation-effect):
  the rules as implemented, six style funds, twelve-month formation, long the best and short the
  worst at half the account each, rebuilt monthly.
- Quantpedia, momentum factor and style rotation effect,
  [the entry](https://quantpedia.com/strategies/momentum-factor-and-style-rotation-effect):
  the performance figures, the instrument count, the monthly rebalancing and the source paper.
- Tibbs, Eakins and DeShurko, Using Style Index Momentum to Generate Alpha,
  [the paper](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1276815): the original study,
  9.25 percent a year for the long-minus-short portfolio over 34 years.
- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), this
  repository's own study of rotation rules, and specifically Sections 3.2 and 3.3, which are where
  the 1,022-rule experiment and the condition a rotation rule needs come from.
- [the predictability brief](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  the repository's harvest of the momentum literature, which supplies `2101.01006v2` on momentum
  skewness and `2605.23905v1` on the decay of a published signal.

## Words used in this tutorial

- ETF: an exchange-traded fund, a single listed thing that holds a basket of other assets and trades
  like a share.
- growth: a style of company whose sales and earnings are expected to rise faster than average, so
  its shares usually look expensive against current earnings.
- long: owning something, so that you gain when its price rises.
- momentum: the tendency of something that has been rising to keep rising for a while.
- short: selling something you do not own, so that you gain when its price falls.
- style: a group of shares defined by a shared characteristic such as company size or cheapness,
  rather than by industry.
- value: a style of company whose shares look cheap against its earnings or assets.
- weight: the share of your money placed in one holding.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
