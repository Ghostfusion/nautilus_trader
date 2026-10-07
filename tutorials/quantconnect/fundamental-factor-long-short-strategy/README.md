# Buying the companies with the best accounts and selling short the ones with the worst

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                            |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | Shares of American companies: the best twenty by company accounts are bought, the worst twenty are sold short                                                                                                                                                                                    |
| How often it trades       | About once a month, when both lists are rebuilt                                                                                                                                                                                                                                                  |
| What you need             | A spreadsheet and a source of company accounts                                                                                                                                                                                                                                                   |
| Where the rules come from | [QuantConnect strategy library, fundamental factor long short strategy](https://www.quantconnect.com/tutorials/strategy-library/fundamental-factor-long-short-strategy)                                                                                                                          |
| The underlying research   | Frazzini, Israel, Moskowitz and Novy-Marx, [A New Core Equity Paradigm: Using Value, Momentum, and Quality to Outperform Markets](https://www.anderson.ucla.edu/documents/sites/about/clubs/asam/4-AQR-A-New-Core-Equity-Paradigm.pdf) (AQR, March 2013)                                         |
| How well it held up       | Mixed: the three ingredients are among the most replicated findings in finance and the AQR study reports long-run rewards net of trading costs, but that study is hypothetical, it is long-only rather than long and short, and no independent out-of-sample record of this short version exists |
| Also appears in           | [Choosing shares from company accounts](../stock-selection-strategy-based-on-fundamental-factors/README.md) in this collection, the long-only ancestor of this rule                                                                                                                              |

## The idea in one paragraph

Rank the most heavily traded American companies using three numbers from their accounts and their
prices: how cheap they are, how profitable they are, and how they have moved recently. Buy the twenty
companies that come out best, in equal amounts. Sell short the twenty that come out worst, also in
equal amounts. Hold both lists for a month, then rebuild. Because half the money is bet on shares
going up and half on shares going down, the market's own rise or fall largely cancels out, and what is
left is the gap between the good companies and the bad ones. This is called a long and short
portfolio: long means bought, short means borrowed and sold.

## Why anyone believed it

Three separate habits of investors are being harvested at once. Cheap companies beat expensive ones,
because investors overpay for exciting stories; profitable companies beat unprofitable ones, because
good businesses compound and the market is slow to appreciate it; and recent winners beat recent
losers over the medium term, because news reaches investors gradually. Each of these is a documented
pattern with a name: value, quality and momentum.

Combining them helps for a reason that is easy to check. Value and momentum tend to move in opposite
directions: when one is having a bad year the other often has a good one. In the AQR study the
correlation between the value and momentum returns was minus 0.48 for American large companies, so
holding both smooths the ride without giving up the average return. The counterparty is any investor
who consistently overpays for glamour, or who sells a good but unfashionable company to buy a popular
one, and any fund that is forced to sell because clients are withdrawing money.

## An everyday comparison

Think of a football league table. Every team is ranked by goals scored, goals conceded and recent
form, and a combined ranking is produced. A spectator who believes the top of the table is
underrated, or that the bottom is overrated, has a natural bet: back the top teams, and bet against
the bottom ones. The long and short portfolio does both at once. If the whole league gets better or
worse, the two sides of the bet move together and cancel; what survives is whether the top teams pull
away from the bottom or the gap closes.

## The rules, step by step

1. Build the universe. Each month, take all American shares, keep those with accounts available and
   a price above 5 dollars, sort by dollar volume (shares traded multiplied by price), and keep the
   250 most heavily traded.
2. Compute three numbers for each company. Value is book value per share. Quality is operating margin,
   the share of revenue left after the costs of running the business. Momentum is the share's price
   change over the past month.
3. Sort the whole list three times, once by each factor, with the best company first each time. For
   this tutorial a high number is best, which is the natural direction for all three.
4. Write down each company's position in each sorted list, counting from 0 for the best.
5. Combine the three positions into one final score, using weights of 40 percent for value, 40
   percent for quality and 20 percent for momentum. The QuantConnect page states this 40/40/20 split,
   but the code shown on the same page assigns the 40 percent to momentum and only 20 percent to
   quality, so treat the weighting as a choice the author made rather than a fact.
6. Sort by the final score, smallest first. A small score means a high position on all three lists,
   so the smallest scores are the best companies.
7. Buy the 20 smallest scores, putting 4.5 percent of the account into each, so the long side uses 90
   percent of the money. Sell short the 20 largest scores, 4.5 percent each, so the short side also
   uses 90 percent. Hold the remaining 10 percent in cash, which the page keeps aside to avoid being
   forced to sell when a short position moves against it.
8. Hold for one month. At the start of the next month, recompute everything and rebuild both lists.

## The maths, with every symbol named

The final score of one company:

```text
Score = 0.4 * v + 0.4 * q + 0.2 * m
```

- `Score` is the company's final score; smaller is better.
- `v` is the company's position in the list sorted by value, counting from 0 for the cheapest.
- `q` is its position in the list sorted by quality, counting from 0 for the most profitable.
- `m` is its position in the list sorted by momentum, counting from 0 for the strongest.

The score is a weighted average of positions, so it is measured in list places rather than in dollars
or percent. A company that is first on all three lists scores 0. A company that is last on all three
scores 0.4 times the list length, which for 250 companies is about 100.

The portfolio's monthly return has two halves. The long half is the average return of the twenty
bought companies, and the short half is the average return of the twenty sold short:

```text
R_long  = (R_1 + R_2 + ... + R_20) / 20
R_short = (S_1 + S_2 + ... + S_20) / 20
R_strategy = 0.9 * (R_long - R_short)
```

- `R_long` is the average return of the twenty best-scoring companies.
- `R_short` is the average return of the twenty worst-scoring companies.
- `R_1` to `R_20` are the returns of the long companies, and `S_1` to `S_20` the returns of the short
  ones.
- `0.9` is the fraction of the account actually invested, since 10 percent is held in cash.
- The subtraction is the point of the structure: a market-wide move up adds the same amount to both
  averages, so it disappears from the difference.

The cost of rebuilding the book each month is the traded fraction times the cost per trade, plus a
borrowing fee on the short side:

```text
Cost = t * c + b
```

- `t` is the traded fraction: 2.0 if every position is closed and reopened, because selling the old
  and buying the new counts twice.
- `c` is the cost per unit traded, covering the gap between buying and selling prices plus commission;
  a realistic figure for large American shares is about 0.0005, that is five basis points, where one
  basis point is one hundredth of one percent.
- `b` is the monthly fee for borrowing the shares sold short, paid on the value of the short book.

## A worked example

Ten invented companies and three positions each, so every company already has its three positions
written down. Position 0 is the best on that factor and position 9 the worst.

| Company | Value position | Quality position | Momentum position | Score |
| ------- | -------------- | ---------------- | ----------------- | ----- |
| A       | 1              | 2                | 3                 | 1.8   |
| B       | 5              | 4                | 5                 | 4.6   |
| C       | 0              | 0                | 2                 | 0.4   |
| D       | 8              | 8                | 8                 | 8.0   |
| E       | 4              | 3                | 6                 | 4.0   |
| F       | 7              | 6                | 9                 | 7.0   |
| G       | 2              | 1                | 4                 | 2.0   |
| H       | 6              | 7                | 1                 | 5.4   |
| I       | 3              | 5                | 7                 | 4.6   |
| J       | 9              | 9                | 0                 | 7.2   |

The scores come from the rule directly. Company C, for instance, is first on value and quality and
third on momentum, so its score is 0.4 times 0 plus 0.4 times 0 plus 0.2 times 2, which is 0.4.
Company A is second on value, third on quality and fourth on momentum, so its score is 0.4 times 1
plus 0.4 times 2 plus 0.2 times 3, which is 1.8.

Sorted smallest first, the list runs C (0.4), A (1.8), G (2.0), E (4.0), B (4.6), I (4.6), H (5.4),
F (7.0), J (7.2), D (8.0). The tie between B and I is broken arbitrarily. The two largest scores are
D and J, so with ten companies the two-and-two example buys C and A and sells short D and J.

| Company | Position   | Weight | Next-month return | Contribution   |
| ------- | ---------- | ------ | ----------------- | -------------- |
| C       | bought     | +0.45  | +2.0 percent      | +0.900 percent |
| A       | bought     | +0.45  | +1.0 percent      | +0.450 percent |
| D       | sold short | -0.45  | -3.0 percent      | +1.350 percent |
| J       | sold short | -0.45  | -2.0 percent      | +0.900 percent |
| Total   |            | 0.00   |                   | +3.600 percent |

The two bought companies rose, and the two sold short fell, which a short position turns into a gain.
The total before costs is 3.600 percent. Now suppose the whole book is rebuilt, so every position is
closed and reopened:

```text
t = 2.0
Trading cost = 2.0 * 0.0005 = 0.0010, that is 0.10 percent
Borrowing fee = 0.9 * 0.003 / 12 = 0.000225, that is 0.0225 percent
Net return for the month = 3.600 - 0.10 - 0.0225 = 3.4775 percent
```

The borrowing fee above assumes a fee of 0.3 percent a year on the borrowed shares, applied for one
month to the short book. As with every worked example in this collection, the numbers are invented to
be easy to follow; they show how the arithmetic behaves, not what the rule is claimed to earn.

## What the research actually found

The AQR study behind this rule is a long-only study. Its headline portfolio holds the best shares and
does not short the worst, so the QuantConnect version is a variant, not a reproduction. The AQR
portfolio combines value, momentum and profitability, measured with several numbers each rather than
one, and rebalances gradually. Its published figures for American large companies over 1980 to 2012
are: a return of 17.3 percent a year, volatility of 16.6 percent, a reward-to-risk ratio of 0.74, and
an excess return over the market of 5.3 percent a year. The same table reports that the return net of
estimated trading costs was 15.6 percent with a net reward-to-risk ratio of 0.64, and that the
one-sided turnover was 136 percent a year, meaning about two-thirds of the portfolio was replaced
each year. The study also reports that the strategy underperformed the market over some rolling
three-year windows, 4.7 percent of the time in the American large-company case, with a worst
cumulative shortfall of 11 percent.

A later table in the same study separates the costs. For American large companies the gross excess
return of 4.1 percent a year falls to 3.6 percent after trading costs and to 2.1 percent after taxes,
with turnover of 64 percent a year. That is the honest shape of the result: a real reward, roughly
halved by the cost of collecting it, on a hypothetical portfolio that pays no management fee.

QuantConnect's own version claims a backtest over a 60-year span and states that the strategy
"consistently beats the market and has solid economic intuition". Those are the page author's words
about his own backtest. The page publishes no performance table for the finished rule that this
tutorial could check, so it is reported here as a claim rather than as a measurement. The page does
note that its version is "very different from the paper one", which is the right way to read it.

The ingredients themselves are the strongest part of the case. This repository's brief
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md)
reads a 2022 review that bounds the false-discovery rate in the cross-section of stock returns at 8.5
to 25 percent. The
[portfolio construction brief](../../../strategies/books2/10_portfolio_and_allocation.md) adds that a
second review finds publication-bias corrections shrink published cross-sectional returns by only 10
to 15 percent. Value, quality and momentum are not fringe ideas; the open question is whether this
particular combination, weighted 40/40/20 and shorted at both ends, adds anything on top of them.

## How this project relates to it

This repository does not implement a long and short fundamental strategy. There is no file here that
ranks companies on accounts and takes both sides, and it would be dishonest to imply otherwise.

What exists is the measurement apparatus. The predictability brief above is where the value, quality
and momentum evidence is assembled and where the momentum horizon is worked out. The overfitting brief,
[Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
is the one to read before choosing the weights: it records that mining 240 accounting variables
produces 18,113 strategies, of which 30.17 percent clear the common significance bar by chance alone,
so a 40/40/20 weighting picked after seeing results is a choice that needs a trial count beside it.
The
[portfolio construction brief](../../../strategies/books2/10_portfolio_and_allocation.md) supplies the
rest: how a factor ranking changes with the way the test portfolios are built, and how the
false-discovery bound rises to 41.7 percent once returns are value-weighted and adjusted for known
factors. The
[execution brief](../../../strategies/books/15_portfolio_construction_under_frictions.md) is where the
cost side lives, and it is blunt that ignoring the cost of trading does not round the answer, it can
reverse it.

## Where it goes wrong

- The short side is not a mirror image of the long side. Borrowing shares costs money, the loan can be
  recalled at the worst moment, and the worst companies are often the ones whose shares are hardest to
  borrow. A long-only version and a long and short version of the same ranking are different products
  with different risks.
- The weighting was chosen by the authors. The 40/40/20 split comes from the study, and the
  QuantConnect page's own text and code disagree about which factor gets the 40 percent. A reader
  cannot verify that split against anything except the sample it was chosen on.
- Value, quality and momentum can all fail at once. Value can stay cheap for years, a profitable
  company can be expensive, and momentum suffers sudden crashes when a market turns, because the
  shares it holds are the ones that had run up. Combining them smooths the average, it does not remove
  the bad years.
- Costs are larger than for a long-only rule. The book trades twice as many positions, pays two
  spreads on every name that changes, and pays a borrow fee on half its money every month. The AQR
  table shows a real reward falling from 4.1 percent to 2.1 percent once costs and taxes are counted.
- Look-ahead and survivorship. Accounts are published weeks after a period ends, and databases tend to
  drop companies that failed. Both errors flatter a strategy that ranks on cheapness and profitability.
- The market exposure does not fully cancel. The two sides are not the same size in volatility, and a
  sharp rebound can lift the shorted companies more than the bought ones, which is exactly the
  environment that hurts this kind of book.

## Try it yourself

You need nothing but a spreadsheet and a website that publishes company accounts.

1. Choose ten large companies and make one row per company.
2. Add a column for book value per share, a column for operating margin and a column for the
   one-month price change.
3. Sort the companies by each column in turn and write each company's position, 0 for the best down
   to 9 for the worst, in a new column.
4. Compute the score: 0.4 times the value position, plus 0.4 times the quality position, plus 0.2
   times the momentum position.
5. Sort by the score and mark the two smallest as buys and the two largest as shorts.
6. Look up what each of those four did over the following month, then repeat the exercise a month
   later.

What to notice: the companies on the buy list are usually the ones whose prices have just fallen, so
the value and momentum columns often pull in opposite directions. Also notice how few of the ten
companies change sides between two months, and how many trades a monthly rebuild would generate in a
list of 250.

## Where this came from

- [QuantConnect strategy library: fundamental factor long short strategy](https://www.quantconnect.com/tutorials/strategy-library/fundamental-factor-long-short-strategy),
  the three factors, the scoring rule, the 40/40/20 weighting, the 250-company universe and the
  long and short construction.
- Frazzini, Israel, Moskowitz and Novy-Marx, [A New Core Equity Paradigm](https://www.anderson.ucla.edu/documents/sites/about/clubs/asam/4-AQR-A-New-Core-Equity-Paradigm.pdf),
  AQR Capital Management, March 2013, the value, momentum and profitability measures and the
  performance, cost and correlation tables.
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's reading of the cross-sectional predictability literature.
- [Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md)
  and [Portfolio construction and allocation](../../../strategies/books2/10_portfolio_and_allocation.md),
  the multiple-testing and publication-bias numbers quoted above.

## Words used in this tutorial

- book value per share: the accounting value of a company, assets minus liabilities, divided by its
  number of shares.
- borrow fee: the charge for borrowing shares to sell them short.
- dollar volume: the number of shares traded multiplied by their price, a measure of how actively a
  share changes hands.
- factor: a measurable characteristic of a company or share, such as cheapness or profitability, used
  to rank investments.
- long: bought and owned, hoping the price rises.
- operating margin: the share of revenue left after the costs of running the business, as a percent.
- short: borrowed, sold, and hopefully bought back cheaper, gaining if the price falls.
- turnover: how much of the portfolio is replaced over a period, usually expressed per year.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
