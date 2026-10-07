# ESG momentum: buying the companies whose environmental, social and governance rating has just improved

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                              |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | Shares of large companies from around the developed world, held in two baskets: one bought, one sold short                                                                                                                                                         |
| How often it trades       | About once a month, when the portfolio is rebuilt                                                                                                                                                                                                                  |
| What you need             | A spreadsheet and a table of company ratings                                                                                                                                                                                                                       |
| Where the rules come from | [The list's implementation file](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/esg-factor-momentum-strategy.py) and the [Quantpedia entry](https://quantpedia.com/strategies/esg-factor-momentum-strategy) it cites |
| The underlying research   | Nagy, Kassam and Lee, [Can ESG Add Alpha? An Analysis of ESG Tilt and Momentum Strategies](https://www.semanticscholar.org/paper/Can-ESG-Add-Alpha-An-Analysis-of-ESG-Tilt-and-Nagy-Kassam/64f77da4f8ce5906a73ffe4e9eec7c49c0960acc)                               |
| How well it held up       | Weak: one eight-year study of model portfolios before trading costs, and the vendor's own out-of-sample check came out slightly negative, so the grade rests on the absence of a test that survives costs                                                          |
| Also appears in           | Nothing else in this collection                                                                                                                                                                                                                                    |

## The idea in one paragraph

Outside agencies score every large company on environmental, social and governance matters, and
publish a rating. This strategy does not buy the best-rated companies. It buys the companies whose
rating has improved the most over the past year, and it sells short the companies whose rating has
fallen the most. It holds both baskets for a month, then rebuilds them from fresh ratings. The bet is
that a change in a rating is news the market absorbs slowly, so improvers keep drifting up and
fallers keep drifting down.

## Why anyone believed it

The obvious way to use a rating is to buy the companies that have the best one. The counterparty, on
this story, is the investor who does exactly that and ignores the change. A bigger, more forced
counterparty is the fund that is required to hold only companies above some rating: when a company
is downgraded it must be sold, whatever the price, and when it is upgraded the fund may buy it later,
after the initial move. Rating changes also arrive on a schedule from a small number of publishers,
so the news is public and arrives in steps rather than all at once.

There is a slower story too. A company that improves its environmental or governance practices may,
over years, face fewer fines, cheaper borrowing and fewer sudden losses. If that is true, the first
year after an upgrade is where the market begins to price it. Neither story is proven by the fact
that improvers have done better; both are reasons the effect might be real rather than a fluke.

## An everyday comparison

Think of a restaurant guide that re-grades restaurants once a year. A restaurant just lifted from two
stars to three is not yet on the shortlist that visitors carry around, because that list is written
from last year's guide. But the people who read the new guide first start booking, and the place
fills up before the wider crowd arrives. Buying the improvers is joining that early queue. The guide
itself is the rating agency: its grade is an opinion, and two different guides can disagree about the
same restaurant.

## The rules, step by step

1. Choose the universe: the large companies from developed markets that make up the MSCI World index
   and that carry a published rating.
2. For each company, write down its rating two months ago and its rating fourteen months ago.
3. Work out the rating momentum: divide the later number by the earlier one and subtract one.
4. Rank every company from the largest momentum to the smallest.
5. Buy the top tenth, the companies whose rating improved most, and sell short the bottom tenth, the
   companies whose rating fell most. Place equal money on each side.
6. Hold both baskets for one month, then recompute from step 2 and rebuild.
7. The implementation measures the change from two months ago to fourteen months ago rather than from
   one month ago to thirteen, so the most recent month, which may not yet have been published when
   the trade is made, is skipped.

A note on what the rating is. Agencies publish both a level and a set of letter grades. The common
scale runs from the best, often written AAA, down to the worst, often written CCC. The implementation
turns each rating into a number and works with the number, so a rise from one grade to the next is a
fixed step.

## The maths, with every symbol named

The momentum of one company's rating, over one year:

```text
MOM_i = S_i(t-2) / S_i(t-14) - 1
```

- `MOM_i` is the rating momentum of company `i`, written as a decimal: 0.20 means the rating rose by
  20 percent.
- `S_i(t-2)` is company `i`'s rating two months before the rebuild date.
- `S_i(t-14)` is company `i`'s rating fourteen months before the rebuild date.

Rank all companies by `MOM_i`, keep the top tenth as the bought basket and the bottom tenth as the
sold basket. If the money is weighted by company size, the weight of a company inside its basket is:

```text
w_i = C_i / sum(C over the basket)
```

- `w_i` is the fraction of that basket's money placed in company `i`.
- `C_i` is company `i`'s market value: the share price times the number of shares.
- `sum(C over the basket)` is the total market value of all companies in that basket.

The return of the whole bet over the following month is the bought basket minus the sold basket:

```text
R_strategy = sum(w_i * r_i for i in the bought basket) - sum(w_i * r_i for i in the sold basket)
```

- `r_i` is the return of company `i` over the month ahead, in decimal form.
- The first sum is what the bought basket earned. The second is what the sold basket earned, which
  the strategy keeps because a price that falls on a short position is a gain.

Finally the cost of trading and of borrowing:

```text
Cost = t * c + b * s
```

- `t` is the fraction of the account traded, counting both sides: selling 20 percent of the old
  holdings and buying 20 percent of new ones is `t = 0.40`.
- `c` is the cost per trade as a fraction of the amount traded, covering the gap between the buying
  and the selling price plus commission. For large shares a realistic number is 0.001, that is ten
  basis points, where one basis point is one hundredth of one percent.
- `b` is the fraction of the account held short.
- `s` is the borrow fee for one month, the rent paid to borrow the shares that were sold. A large
  share with a liquid lending market costs about 0.005 to 0.03 a year, so about 0.0004 to 0.0025 for
  one month.

## A worked example

Ten companies, with ratings on a scale from 0 to 100. The numbers are invented but of a plausible
size. Momentum is the later rating divided by the earlier one, minus one.

| Company | Rating 14 months ago | Rating 2 months ago | MOM   | Rank |
| ------- | -------------------- | ------------------- | ----- | ---- |
| A       | 60.0                 | 72.0                | 0.20  | 1    |
| B       | 50.0                 | 57.5                | 0.15  | 2    |
| C       | 80.0                 | 90.4                | 0.13  | 3    |
| D       | 40.0                 | 44.4                | 0.11  | 4    |
| E       | 70.0                 | 75.6                | 0.08  | 5    |
| F       | 55.0                 | 58.3                | 0.06  | 6    |
| G       | 65.0                 | 67.6                | 0.04  | 7    |
| H       | 45.0                 | 45.9                | 0.02  | 8    |
| I       | 75.0                 | 75.0                | 0.00  | 9    |
| J       | 30.0                 | 27.0                | -0.10 | 10   |

With ten companies a tenth is one company, so the example buys A and sells short J, the same amount
on each. Real implementations use a universe of hundreds, so each basket holds dozens of names; a
single name here would be far too concentrated.

Now suppose six months pass, with the best-improved company bought and the worst-deteriorated sold
in each month. The return of each side, and the cost of rebuilding the whole book each month, are
shown below. The monthly cost is `t * c + b * s = 2.0 * 0.001 + 1.0 * 0.0025 = 0.002 + 0.0025 =
0.0045`, that is 0.45 percent, because every position is replaced and the whole short side is
borrowed.

| Month | Bought side | Sold side | Gross (bought minus sold) | Cost  | Net    |
| ----- | ----------- | --------- | ------------------------- | ----- | ------ |
| 1     | +0.8%       | -0.2%     | +1.00%                    | 0.45% | +0.55% |
| 2     | -0.3%       | -0.1%     | -0.20%                    | 0.45% | -0.65% |
| 3     | +0.5%       | -0.4%     | +0.90%                    | 0.45% | +0.45% |
| 4     | -0.6%       | 0.0%      | -0.60%                    | 0.45% | -1.05% |
| 5     | +0.3%       | -0.5%     | +0.80%                    | 0.45% | +0.35% |
| 6     | +0.1%       | -0.2%     | +0.30%                    | 0.45% | -0.15% |
| Total |             |           | +2.20%                    | 2.70% | -0.50% |

The gross gain is 2.20 percent over six months, which would be about 4.4 percent a year, but the
trading and borrowing cost is 2.70 percent over the same six months, so the example ends slightly
negative. Two things are worth noticing. First, the cost line is not a detail: for a monthly
long-short rebuild, the two sides of every trade plus the borrow fee are larger than the edge in this
made-up run. Second, the worked example says nothing about whether the strategy works. It shows only
how to apply the rules and how the arithmetic behaves.

## What the research actually found

| Source                                                           | What it measured                                            | Result                                                                                                                                                                                                      |
| ---------------------------------------------------------------- | ----------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Nagy, Kassam and Lee, Can ESG Add Alpha?                         | Two global model portfolios built from ratings, eight years | Both a rating-tilt portfolio and a rating-momentum portfolio beat the MSCI World index while holding a better-rated set of companies; the portfolios are modelled, before any trading costs                 |
| Quantpedia, summarising that paper                               | The momentum portfolio's active return, 2007 to 2015        | About 2.23 percent a year above the index, volatility 2.5 percent, reward-to-risk 0.89, taken from the paper's Exhibit 8; the same page records that the out-of-sample back-test came out slightly negative |
| Hanicova and Vojtko, Backtesting ESG Factor Investing Strategies | Rating level and rating momentum on an independent dataset  | The momentum strategy earned a small positive return and the level strategy did better; the authors caution that different rating providers give different scores for the same company                      |
| The list's replication record                                    | 4,843 coded papers, each over its own full history          | The median replication has a reward-to-risk of 0.37, 48 percent clear a t-statistic of 1.96, the median test window is 34 years, and the median carries a market exposure of +0.17                          |

Read together, the picture is this. Rating changes are followed by returns in the direction of the
change, and the published effect is small. The measurement is fragile because the rating itself is
not one number: an independent study in this repository's own briefs finds that different agencies
often correlate below 0.60 with one another on the same companies, against roughly 0.99 for credit
ratings (`2606.31469v1`, p.2). A strategy that is long the improvers scored by one agency and short
the fallers scored by another may not be measuring the same thing at all.

## How this project relates to it

This repository's closest piece is the brief
[ESG, climate and sustainability: the strongest signal is disagreement between raters](../../../strategies/books2/15_esg_climate_and_sustainability.md).
It collects the measurement problem directly: the same 200-firm European sample gives different
answers when the environmental score is taken from a different provider, and simple best-in-class
filtering on the MSCI World universe produced small negative excess returns over 2009 to 2018, while
a learned screen on the raw variables earned about 2.76 percent a year from 2013 to 2018 and then
decayed when it was not retrained (`2002.07477v2`, p.4, p.14, p.16). Anyone thinking of using a
rating as a signal should read that brief first, because it is where the rating-disagreement numbers
are recorded.

## Where it goes wrong

- The rating is an opinion, not a measurement. Two agencies can score the same company very
  differently, and the brief above records correlations below 0.60 between publishers against about
  0.99 for credit ratings. A result that holds for one provider's data may vanish with another's.
- The evidence is model portfolios before costs. The eight-year study builds portfolios with a risk
  model and reports active returns without the gap between buying and selling prices, the borrow fee,
  or the cost of holding a short book, and the vendor's own out-of-sample run was
  slightly negative.
- A long-short book needs shares to borrow. Popular short candidates may be expensive or impossible
  to borrow, and the borrow fee can exceed the expected edge for exactly the names the rule wants to
  sell.
- Corporate behaviour and the rules around it change. Disclosure standards, fund mandates and the
  rating scales themselves have moved repeatedly, so the population being ranked in 2007 is not the
  population being ranked today.
- Monthly rebuilding is expensive. The worked example shows a gross gain of 2.20 percent over six
  months wiped out by 2.70 percent of trading and borrow cost. The more often the ranking changes,
  the more of the edge the frictions take.
- The whole idea would be false if rating changes only republished information the market had already
  priced from the underlying news, so that the improvers drifted up before the rating was published
  and not after. Testing the improvement against the date the market learned the underlying facts,
  and not the date the agency published, is what would settle it.

## Try it yourself

You need a spreadsheet, a list of companies, and two years of rating history from a public source.
You do not need any money.

1. Make one row per company with two columns: the rating two months ago and the rating fourteen
   months ago. Both are available on any company's rating history page.
2. Add a column `MOM` that divides the newer rating by the older one and subtracts one.
3. Sort the sheet by `MOM`, best first.
4. Mark the top tenth as "buy" and the bottom tenth as "sell".
5. Add a column with each company's price change over the month after the older of the two ratings
   was published.
6. Average the price change of the top tenth, subtract the average of the bottom tenth, and subtract
   0.45 percent for the cost of one rebuild.

What to notice: the ranking changes little from month to month for most companies, so a rule that
only trades when a name enters or leaves a basket pays far less than 0.45 percent a month. Notice
too how few companies move between the top and bottom tenth in a given year. If the sheet shows the
strategy winning by a wide margin, check whether the newer rating was actually published before the
price you are measuring, because using a rating that was released after the return is a form of
looking into the future.

## Where this came from

- [The list's implementation file](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/esg-factor-momentum-strategy.py),
  the rules as coded: monthly rebuild, the momentum measured from two to fourteen months back, the
  top tenth bought and the bottom tenth sold.
- [Quantpedia: ESG factor momentum strategy](https://quantpedia.com/strategies/esg-factor-momentum-strategy),
  the indicative performance figures, the instrument count and the underlying papers.
- Nagy, Kassam and Lee, [Can ESG Add Alpha?](https://www.semanticscholar.org/paper/Can-ESG-Add-Alpha-An-Analysis-of-ESG-Tilt-and-Nagy-Kassam/64f77da4f8ce5906a73ffe4e9eec7c49c0960acc)
  the original study of rating tilt and rating momentum.
- [ESG, climate and sustainability](../../../strategies/books2/15_esg_climate_and_sustainability.md),
  this repository's brief, which is where the rater-disagreement and best-in-class numbers come from,
  including `2606.31469v1` and `2002.07477v2`.

## Words used in this tutorial

- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- borrow fee: the rent paid to borrow shares that have been sold short, usually quoted as a rate per
  year.
- decile: a tenth of a ranked list, so the top decile is the best tenth.
- momentum: the tendency of something that has been rising to keep rising for a while.
- rating: an outside agency's score of a company, here combining environmental, social and governance
  matters.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
