# Choosing shares from company accounts rather than from prices

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                 |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of American companies, picked with numbers taken from the companies' own financial reports rather than from their share prices                                                                                                                                 |
| How often it trades       | About once a month, when the ranked list is rebuilt                                                                                                                                                                                                                   |
| What you need             | A spreadsheet and a website that publishes company accounts                                                                                                                                                                                                           |
| Where the rules come from | [QuantConnect strategy library, stock selection strategy based on fundamental factors](https://www.quantconnect.com/tutorials/strategy-library/stock-selection-strategy-based-on-fundamental-factors)                                                                 |
| The underlying research   | Ayhan Yuksel, Factor Based Stock Selection Model for Turkish Equities (2015), listed in the reference section of the [QuantConnect page](https://www.quantconnect.com/tutorials/strategy-library/stock-selection-strategy-based-on-fundamental-factors) above         |
| How well it held up       | Mixed: the underlying measures are among the best replicated in the research literature, but this particular combination was chosen after testing seven factors on one seven-year sample, and no independent out-of-sample record of the finished screen is published |
| Also appears in           | [Fundamental factor long and short](../fundamental-factor-long-short-strategy/README.md) in this collection, the same idea made symmetrical by adding short sales                                                                                                     |

## The idea in one paragraph

Every public company publishes accounts: how much it sold, what it spent, what it owns and what it
owes. Those accounts change slowly, while the share price moves every second. This strategy ignores
the price chart and reads the accounts instead. It turns each account into a few numbers, such as how
much cash the business produces compared with its price, sorts all the companies by each number, and
gives every company a score built from those sorts. Then it buys the highest-scoring companies in
equal amounts and waits a month. The bet is that companies that look cheap and profitable on paper do
better than companies that look expensive, and that the market takes months to notice.

## Why anyone believed it

The share price is an opinion, and opinions can be wrong for long stretches. The accounts are the
business itself. If two companies earn the same profit but one trades at half the price, the cheaper
one offers more for the same money; the difference is called the value premium, and it has been
documented for decades. The other side of the trade is easy to name. Investors enjoy owning exciting
companies, and a company with a rising share price attracts more buyers, which pushes the price up
further. Fund managers who are judged against their peers dislike owning an out-of-favour company,
even when it is cheap, because it makes them look wrong for months. Those buyers and avoiders are the
counterparty: they keep glamorous companies expensive and dull ones cheap.

## An everyday comparison

Think of buying a second-hand car. One way is to watch the prices other people are paying this week
and pay whatever the market asks. Another way is to open the bonnet, read the service history, count
the miles and check the age, and then compare all that with the asking price. The car with the best
engine and the lowest price is the best buy, even if nobody is talking about it. The accounts are the
service history; the share price is what the crowd happens to be paying today.

## The rules, step by step

1. Build the candidate list. Each month, take every American company whose shares are traded and
   whose accounts are available, sort them by dollar volume (the number of shares traded multiplied
   by the price, which measures how actively a share changes hands), and keep the 200 most actively
   traded.
2. For each candidate, read four numbers. The value number is book value per share. The cash number
   is free cash flow yield. The momentum number is the change in the share price over the past month.
   The growth number is the change in revenue over the past year.
3. Throw away any company for which one of those numbers is missing or exactly zero, because a zero
   is usually a missing value in disguise.
4. Sort the whole list by each number in turn. For this tutorial we treat a high number as good for
   all four, which is the common-sense direction; QuantConnect instead decides the direction of each
   factor from a test described below, and its own description of that step is ambiguous.
5. Split each sorted list into five equal groups, from the best fifth to the worst fifth.
6. Give a company 5 points if it is in the best fifth of a list, 4 points for the next fifth, down to
   1 point for the worst fifth. Do this separately for all four lists, so every company ends with
   four point scores.
7. Average the four point scores. That average is the company's composite score.
8. Buy the highest-scoring fifth of the list, in equal amounts, at the start of the month. In
   QuantConnect's version that is about 20 companies.
9. Hold for one month. Do not look at the prices in between. At the start of the next month, repeat
   from step 1, selling anything that has dropped out and buying anything that has entered.

## The maths, with every symbol named

The strategy is four sorts, four scores and one average. The point score for one factor is:

```text
s = 6 - k
```

- `s` is the points a company earned on that factor, between 5 and 1.
- `k` is the number of the fifth it fell into, counting from the best: `k = 1` for the best fifth,
  `k = 5` for the worst fifth.

The composite score is the plain average of the four factor scores:

```text
S = (s_value + s_cash + s_momentum + s_growth) / 4
```

- `S` is the company's composite score, between 5 and 1.
- `s_value`, `s_cash`, `s_momentum` and `s_growth` are its four point scores, each between 5 and 1.

Two of the four numbers need defining. Free cash flow yield is the free cash flow divided by the
company's total market value:

```text
free cash flow yield = free cash flow / market value
```

- `free cash flow` is the cash the business produced after paying to run and maintain itself.
- `market value` is the number of shares multiplied by the share price.

Book value per share is the accounting value of the company divided by its number of shares:

```text
book value per share = (assets - liabilities) / number of shares
```

- `assets` is everything the company owns, valued as the accounts record it.
- `liabilities` is everything it owes.
- `number of shares` is the count of shares in issue.

Finally, the cost of rebuilding the list each month:

```text
Cost = t * c
```

- `t` is the traded fraction of the account: 2.0 if every holding is sold and replaced, because
  selling the old and buying the new counts twice, and less when some holdings are kept.
- `c` is the cost per unit traded, covering the gap between the buying and selling price plus
  commission. A realistic figure for large American shares is 0.0005 to 0.001, that is five to ten
  basis points, where one basis point is one hundredth of one percent.

## A worked example

Ten invented companies, but the numbers are of the size real companies show. The five columns are the
four account numbers plus the price change.

| Company | Book value per share ($) | Free cash flow yield (percent) | One-month price change (percent) | Revenue growth (percent) |
| ------- | ------------------------ | ------------------------------ | -------------------------------- | ------------------------ |
| A       | 40                       | 8                              | 2                                | 10                       |
| B       | 25                       | 5                              | 1                                | 6                        |
| C       | 50                       | 9                              | 3                                | 12                       |
| D       | 10                       | 2                              | -1                               | 2                        |
| E       | 30                       | 6                              | 0                                | 8                        |
| F       | 15                       | 3                              | -2                               | 4                        |
| G       | 60                       | 7                              | 1                                | 5                        |
| H       | 20                       | 4                              | -3                               | 3                        |
| I       | 35                       | 5                              | 4                                | 7                        |
| J       | 5                        | 1                              | 5                                | 1                        |

Ten companies split into fifths gives two companies per fifth, so the best two on each factor score 5
points and the worst two score 1 point. Applying the rule to each column produces these scores.

| Company | Value points | Cash points | Momentum points | Growth points | Composite score |
| ------- | ------------ | ----------- | --------------- | ------------- | --------------- |
| A       | 4            | 5           | 4               | 5             | 4.50            |
| B       | 3            | 3           | 3               | 3             | 3.00            |
| C       | 5            | 5           | 4               | 5             | 4.75            |
| D       | 1            | 1           | 2               | 1             | 1.25            |
| E       | 3            | 4           | 2               | 4             | 3.25            |
| F       | 2            | 2           | 1               | 2             | 1.75            |
| G       | 5            | 4           | 3               | 3             | 3.75            |
| H       | 2            | 2           | 1               | 2             | 1.75            |
| I       | 4            | 3           | 5               | 4             | 4.00            |
| J       | 1            | 1           | 5               | 1             | 2.00            |

The highest composite scores are C at 4.75 and A at 4.50, so those two are the holdings. Suppose last
month the holdings were A and I. Then I is sold and C is bought, one of two positions changed. Now
suppose next month C returns 3 percent and A returns 1 percent.

| Company | Weight | Next-month return | Contribution    |
| ------- | ------ | ----------------- | --------------- |
| A       | 0.5000 | +1.0 percent      | +0.5000 percent |
| C       | 0.5000 | +3.0 percent      | +1.5000 percent |
| Total   | 1.0000 |                   | +2.0000 percent |

The portfolio gained 2.0000 percent before costs. Half the money moved, counting both sides, so:

```text
t = 2 * (1 / 2) = 1.0
Cost = 1.0 * 0.001 = 0.001, that is 0.10 percent
Net return for the month = 2.0000 - 0.10 = 1.90 percent
```

Two things are worth noticing. First, in a ten-company universe with a two-for-one split the top fifth
is only two companies, while the real rule buys about twenty out of two hundred; the arithmetic is the
same, only the table is smaller. Second, the worked example shows how to apply the rules. It says
nothing about whether the screen earns anything, because the numbers were invented to be easy to
follow.

## What the research actually found

The QuantConnect page does two things. First it tests seven factors one at a time on American data
from January 2005 to March 2012. Each factor is used to sort 200 actively traded companies into five
groups of 40, and the groups' monthly returns are compared with each other and with an index fund.
The page then applies three pass conditions: the correlation between a group's rank and its return
should exceed 0.8 in absolute value, the best and worst groups should each beat or lag the index more
than 40 percent of the months, and the best group's annual excess return should be above 0.25 while
the worst group's is below 0.05. The table below is the page's own result table.

| Factor               | FCFYield | BuyBackYield | PriceChange1M | TrailingDividendYield | EVToEBITDA | RevenueGrowth | BookValuePerShare |
| -------------------- | -------- | ------------ | ------------- | --------------------- | ---------- | ------------- | ----------------- |
| The correlation      | -0.936   | -0.987       | 0.918         | -0.981                | 0.939      | 0.89          | -0.92             |
| Win probability      | 0.630    | 0.639        | 1             | 0.667                 | 0.722      | 0.69          | 0.69              |
| Loss probability     | 0.426    | 0.472        | 1             | 0.518                 | 0.472      | 0.42          | 0.40              |
| Excess return (win)  | 0.324    | 0.212        | 0.303         | 0.225                 | 0.414      | 0.23          | 0.27              |
| Excess return (loss) | 0.060    | 0.037        | -1.67         | 0.043                 | 0.042      | 0.07          | 0.06              |

Four factors are kept: free cash flow yield, one-month price change, book value per share and revenue
growth. The pages states that the finished screen buys about 20 companies, equally weighted, rebuilt
monthly. It does not publish a return, volatility or drawdown for that finished screen, so the honest
summary is that the page reports factor evidence, not a performance figure for the complete rule.

The same page also states that the underlying study used Turkish shares, and that QuantConnect's test
used American ones, so the two samples are not the same market. The copy of the Turkish study linked
from the page is no longer reachable, so its own numbers could not be checked here.

The wider literature is friendlier than the folklore. This repository's brief
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md)
reads a 2022 review which bounds the share of false findings in the cross-section of stock returns at
8.5 to 25 percent and finds that at least 91.5 percent of equally weighted findings are probably
real. A second brief,
[Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
records that sorting on 240 accounting variables produces 18,113 candidate strategies, of which 30.17
percent clear the common significance bar by chance alone rather than by skill. Both facts matter
here: the ingredients are plausible, but the act of choosing four out of seven after seeing the
results is exactly the search that inflates apparent success.

## How this project relates to it

This repository does not implement a fundamental stock screen. There is no file here that reads book
value or free cash flow and returns a list of shares, and this tutorial will not pretend otherwise.

The closest things are three research briefs. The predictability brief above is the direct one: it is
where the claim that cheap shares beat expensive ones is audited against replication and multiple
testing. The
[portfolio construction brief](../../../strategies/books2/10_portfolio_and_allocation.md)
adds the two corrections that matter for a screen built this way: the false-discovery bound rises to
41.7 percent once returns are value-weighted and adjusted for known factors, and publication-bias
corrections shrink published cross-sectional returns by only 10 to 15 percent rather than wiping them
out. The overfitting brief is where the accounting-variable mining experiment lives, and its lesson is
the one to carry into step 4 above: a screen chosen from many candidates must report how many
candidates it tried.

## Where it goes wrong

- Accounts arrive late. A company's report for a quarter is published weeks after the quarter ends,
  and that is the day the information became usable. A backtest that reads a quarter's accounts on the
  last day of the quarter is using numbers no investor had yet, and it will look better than reality.
- Survivorship. The list of companies that exist today is not the list that existed ten years ago.
  Companies that failed, were bought out or were delisted have vanished from most databases, and they
  are exactly the ones a cheap-looking screen would have owned.
- The direction was decided after the fact. Whether a high number is good or bad for each factor was
  read off the same seven years used to report the result. A direction fitted to the sample is a
  choice, not a finding.
- Value traps. A share can be cheap on the accounts and stay cheap, or get cheaper, because the
  business is genuinely fading. Book value is a historical cost, not a promise about the future.
- Illiquid and odd companies. Small, thinly traded names dominate the extremes of any accounting
  screen. The dollar-volume filter is there to remove them, but it also removes the companies where
  the effect is often reported to be largest.
- It is not free. Rebuilding monthly pays a spread on every name that changes, and a screen that
  churns its holdings pays more than one that does not.

## Try it yourself

You need a spreadsheet and any finance website that lists company accounts.

1. Choose eight large, well-known companies and make one row per company.
2. Add a column for book value per share, a column for free cash flow yield, a column for the
   one-month price change and a column for the revenue growth.
3. For each of the four columns, sort the companies and write their order in a new column, 1 for the
   best down to 8 for the worst.
4. Convert each order into points: with eight companies you can use two groups of four, giving 4, 4,
   3, 3, 2, 2, 1 and 1 points.
5. Average the four point columns to get a composite score and sort by it.
6. Repeat for a second month using a second set of numbers, and compare the two lists.

What to notice: the list changes a lot for small differences in the accounts, and the company that
tops the list is often the one whose price fell, not the one whose business improved. That is the
screen doing what it is told, and it is also why the direction of each factor should be a deliberate
choice rather than a coincidence.

## Where this came from

- [QuantConnect strategy library: stock selection strategy based on fundamental factors](https://www.quantconnect.com/tutorials/strategy-library/stock-selection-strategy-based-on-fundamental-factors),
  the rules, the factor test table, the pass conditions and the four chosen factors.
- Ayhan Yuksel, Factor Based Stock Selection Model for Turkish Equities (2015), the study the
  QuantConnect page says the factor-selection method came from; the copy linked from that page is no
  longer reachable, so its own numbers were not used here.

## Words used in this tutorial

- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- book value per share: the accounting value of a company, assets minus liabilities, divided by its
  number of shares.
- composite score: a single number built by averaging a company's points on several factors.
- dollar volume: the number of shares traded multiplied by their price, a measure of how actively a
  share changes hands.
- factor: a measurable characteristic of a company, such as how cheap or how profitable it is, used to
  rank shares.
- fundamental data: numbers taken from a company's own accounts rather than from its share price.
- quintile: one of five equal groups after a list has been sorted, so the top quintile is the best
  fifth.
- survivorship bias: the error that comes from studying only the companies that still exist today.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
