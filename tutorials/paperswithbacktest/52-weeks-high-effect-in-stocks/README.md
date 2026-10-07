# The 52-week high: buying shares close to their highest price of the past year

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | Shares of American companies in both directions: bought when their industry sits close to its own 52-week high, sold short when it sits far from it                                                                                                                                  |
| How often it trades       | Once a month, but each selection is held for three months, so a third of the book is replaced at each monthly rebuild                                                                                                                                                                |
| What you need             | A spreadsheet, twelve months of daily prices for a list of shares, and an industry label for each share                                                                                                                                                                              |
| Where the rules come from | [The list's implementation file](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/52-weeks-high-effect-in-stocks.py) and the [Quantpedia entry](https://quantpedia.com/strategies/52-weeks-high-effect-in-stocks/) it repeats            |
| The underlying research   | Hong, Jordan and Liu, [Industry Information and the 52-Week High Effect](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1787378), building on George and Hwang (2004)                                                                                                           |
| How well it held up       | Weak: the list's own replication of this industry version returned a Sharpe ratio of -0.10 over 1990 to 2026, and Quantpedia states that its out-of-sample test was slightly negative, so the grade rests on those two negative measurements rather than on the paper's positive one |
| Also appears in           | [Momentum in stocks](../../quantconnect/momentum-effect-in-stocks/README.md), the plain momentum tutorial in this collection, and the related [12-month cycle in the cross-section of stocks](../../quantconnect/12-month-cycle-in-cross-section-of-stocks-returns/README.md)        |

## The idea in one paragraph

Investors remember the highest price a share reached over the past year. When a share trades
just below that high, the argument is that the good news which pushed it there has not been fully
absorbed into the price, because buyers hesitate to pay more than a familiar record. This strategy
measures how close each share is to its own 52-week high, groups the shares by industry, and ranks
the industries by that closeness. It buys the shares of the six industries closest to their highs
and sells short the shares of the six industries furthest from theirs, holding each selection for
three months.

## Why anyone believed it

The story is a mental bias called anchoring. Asked to estimate a number, people start from
something they already know and adjust too little. For a share, the past year's highest price is
an easy anchor: it is printed beside the price in almost every newspaper and finance page. When
good news lifts a share near that high, buyers treat the old high as a ceiling and are slow to pay
more, so the price lags the news and drifts up over the following months.

The counterparty is the investor who under-reacts to the news, and the researchers report that
large institutions suffer less from the bias and are the ones buying shares near their highs. The
strategy measures closeness to the high industry by industry because the study found that most of
the under-reaction is to news about a whole industry, not to news about one company. That is why
the rule ranks industries first and only then buys the shares inside them.

## An everyday comparison

Think of a village where everyone knows the highest price ever paid for a house on the main
street. When a house there sells close to that record, buyers hesitate, because the number feels
like a ceiling; they offer a little less than the news about the area really justifies. The seller
accepts, and the price catches up only after a few more sales at the new level make the old record
look ordinary. A share price near its 52-week high behaves like that village record: an anchor
that slows the adjustment rather than a wall that stops it.

## The rules, step by step

1. Assemble a list of shares traded on the large American exchanges. The paper uses every share in
   a research database; the code uses the 500 most heavily traded. Give each share an industry
   label, using the same set of industries throughout, so two shares are compared only inside their
   own group.
2. At the end of each month, for each share divide its current price by the highest price it
   reached at any point over the past twelve months (about 252 trading days). A share trading at
   96.00 when its high was 100.00 scores 0.96.
3. Inside each industry, take the average of those scores, weighting each share by its market
   value, which is its price multiplied by the number of its shares. A large company counts for
   more than a small one in the same industry.
4. Rank the industries by that weighted average, best first. The published rule takes six winners
   from the top of twenty industries and six losers from the bottom.
5. Buy the shares inside the winner industries and sell short the shares inside the loser
   industries. Give every share in a leg the same money, and give the long leg and the short leg
   the same total, so the two sides cancel in value and the bet is on the gap between them.
6. Hold each selection for three months. At the end of every month, rebuild one third of the book,
   so that three overlapping selections are alive at once and only a third of the money moves each
   month.
7. On every trade pay the gap between the buying and selling price plus any commission, and pay a
   borrow fee for as long as a share is shorted.

## The maths, with every symbol named

The score of one share:

```text
S_i,t = P_i,t / H_i,t
```

- `S_i,t` is the score of share `i` at the end of month `t`, a decimal between 0 and 1, where 1.00
  means the share is exactly at its high.
- `P_i,t` is the closing price of share `i` at that moment.
- `H_i,t` is the highest price of share `i` over the previous twelve months.

The score of one industry is the market-value-weighted average of its members' scores:

```text
A_j,t = sum over i in industry j of ( S_i,t * M_i,t / M_j,t )
```

- `A_j,t` is the score of industry `j`.
- `M_i,t` is the market value of share `i`, its price times its number of shares.
- `M_j,t` is the total market value of all shares in industry `j`.
- Each share's weight `M_i,t / M_j,t` is its slice of the industry, and the weights add to one.

Rank the industries by `A_j,t`. The strategy's return over the next holding period is the average
return of the winner shares minus the average return of the loser shares:

```text
R_long  = (1 / n_L) * sum over winners of r_i
R_short = (1 / n_S) * sum over losers  of r_i
R_strategy = R_long - R_short
```

- `r_i` is the return of share `i` over the holding period, as a decimal.
- `n_L` and `n_S` are the numbers of winner and loser shares.
- Dividing by the count gives each share equal weight; subtracting the loser average is the payoff
  of the short leg, and `R_strategy` is stated per unit of one leg, not of the whole book.

The monthly cost of keeping the book alive:

```text
Cost_month = 4 * f * c + b
```

- `f` is the fraction of each leg replaced at a monthly rebuild, one third in the published rule.
- `c` is the cost of one trade as a fraction of its value, covering the gap between the buying and
  selling price and any commission; about 0.001, that is ten basis points, is realistic for large
  American shares. One basis point is one hundredth of one percent.
- The factor four counts the four trades of a replacement: selling the old slice and buying the new
  slice on the long leg, and the same two trades on the short leg.
- `b` is the borrow fee on the short leg for one month.

## A worked example

Five industries, two shares each, with invented but plausible numbers. The published rule takes six
winners and six losers from twenty industries; here the table takes the best one and worst one from
five so that it fits on a page, and the arithmetic is otherwise identical.

| Industry   | Share | Price  | 52-week high | Score  | Market value | Industry score |
| ---------- | ----- | ------ | ------------ | ------ | ------------ | -------------- |
| Technology | T1    | 120.00 | 125.00       | 0.9600 | 100          | 0.9600         |
| Technology | T2    | 48.00  | 50.00        | 0.9600 | 25           |                |
| Health     | HC1   | 90.00  | 95.00        | 0.9474 | 60           | 0.9355         |
| Health     | HC2   | 18.00  | 20.00        | 0.9000 | 20           |                |
| Energy     | E1    | 50.00  | 60.00        | 0.8333 | 40           | 0.8167         |
| Energy     | E2    | 30.00  | 40.00        | 0.7500 | 10           |                |
| Retail     | R1    | 70.00  | 100.00       | 0.7000 | 30           | 0.7000         |
| Retail     | R2    | 14.00  | 20.00        | 0.7000 | 15           |                |
| Utilities  | U1    | 40.00  | 60.00        | 0.6667 | 50           | 0.6597         |
| Utilities  | U2    | 25.00  | 40.00        | 0.6250 | 10           |                |

The industry score is the market-value-weighted average, for example for health:
`(0.9474 * 60 + 0.9000 * 20) / 80 = 0.9355`. Ranking the five scores gives technology first,
health second, energy third, retail fourth and utilities last. The winner is technology, the loser
is utilities, and to match the paper's six-of-twenty shape the example also takes health as a
second winner and retail as a second loser.

Now the four chosen shares are held while the ranking is unchanged, so each monthly rebuild reselects
them and the book's monthly return is simply the chosen basket's monthly return.

| Month | Long basket | Short basket | Gross    | Cost   | Net      | Cumulative |
| ----- | ----------- | ------------ | -------- | ------ | -------- | ---------- |
| 1     | +1.2500%    | -1.2500%     | +2.5000% | 0.175% | +2.3250% | +2.3250%   |
| 2     | +1.0000%    | +0.1250%     | +0.8750% | 0.175% | +0.7000% | +3.0413%   |
| 3     | +1.5000%    | +0.5000%     | +1.0000% | 0.175% | +0.8250% | +3.8914%   |
| 4     | +0.6250%    | +0.1250%     | +0.5000% | 0.175% | +0.3250% | +4.2290%   |
| 5     | -1.3750%    | +2.2500%     | -3.6250% | 0.175% | -3.8000% | +0.2683%   |
| 6     | +1.6250%    | -0.7500%     | +2.3750% | 0.175% | +2.2000% | +2.4742%   |

The long and short columns are the equal-weighted averages of the chosen shares' returns. The gross
column is long minus short. The cost uses one third of the book rolled each month, ten basis points
per trade and a borrow fee of half a percent a year:

```text
Cost_month = 4 * (1/3) * 0.001 + 0.005/12 = 0.001333 + 0.000417 = 0.00175, that is 0.175 percent
```

The result is about plus 2.5 percent over six months. Two things stand out. Month five is a
momentum crash: the shares that had been falling hardest, which the strategy was short, bounced,
and the loss in one month is larger than the gains of the other five. And the cost line is modest
only because a third of the book moves each month; a rule that replaced the whole book every month
would pay about 0.4 percent a month, near five percent a year.

## What the research actually found

| Source and what it measured                                                          | Result                                                                                                                                                                              |
| ------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| George and Hwang (2004), nearness to the 52-week high on American shares             | The single ratio of price to 52-week high explains much of the profit from ordinary momentum and beats past returns as a predictor; the profits do not reverse                      |
| Hong, Jordan and Liu, the industry version on the same database, 1963 to 2009        | Buying industries near their highs and shorting industries far from theirs earned 0.60 percent a month, about half again the share-level version                                    |
| Liu, Liu and Ma, the same idea in twenty international markets                       | Profits in 18 of 20 markets, significant in 10, but no longer significant in most markets once trading costs are counted                                                            |
| Quantpedia's own summary                                                             | 11.75 percent a year at 11 percent volatility, worst fall 53.9 percent over 1963 to 2009; confidence graded Moderate, with a note that its out-of-sample test was slightly negative |
| The list's own measurement of this industry version, on its own data                 | A Sharpe ratio of -0.10, annual return -0.73 percent, volatility 5.77 percent, worst fall 48.4 percent, 1990 to 2026 (the vendor's own measurement)                                 |
| The list's own measurement of the original George and Hwang version, on its own data | A Sharpe ratio of 0.29, annual return 1.64 percent, volatility 6.19 percent, worst fall 20.0 percent, 1990 to 2026 (the vendor's own measurement)                                   |

The two pictures do not agree, and that disagreement is the finding. The papers, written on data
ending in 2009, report a real and sizeable in-sample profit. The list's own run of the same industry
rule over 1990 to 2026, which includes the later years, returns a negative Sharpe ratio, and
Quantpedia's own note says its out-of-sample result is slightly negative too. The list also reports
an aggregate record across thousands of papers: the median replication has a Sharpe ratio of 0.37,
48 percent of them clear a t-statistic of 1.96, the median window is 34 years, and removing each
strategy's market exposure takes the median information ratio down to 0.21. Read alongside those
aggregates, a single published profit is weak evidence.

## How this project relates to it

This repository contains its own study of industry leadership, which is exactly the mechanism this
strategy leans on: [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md).
Its section 3.2 states the condition the idea needs, that leaders keep leading for several months,
and its section 7 collects the industry-momentum evidence, including the finding that the residual
effect after removing industry is not statistically significant.

The broader question of whether cross-sectional predictors like this survive out of sample is
covered in [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
a brief that reads the recent research and reports that the cross-section is mostly real but that
value weighting and factor adjustments raise the share of findings that look false. The finished
tutorial [Momentum in stocks](../../quantconnect/momentum-effect-in-stocks/README.md) in this
collection works the plain version of the same bet, on past returns rather than on nearness to the
high.

## Where it goes wrong

- The measured result turns negative. On the list's own data the industry version lost money over
  1990 to 2026, and Quantpedia reports a slightly negative out-of-sample test, so the positive
  paper numbers should be treated as a description of an earlier period, not a current edge.
- A single anchor can be wrong for a share. A company that splits its shares, or one whose high was
  set during a mania, carries a misleading 52-week high, and the score then says nothing useful.
- The signal is crowded and easy. Anyone with a newspaper can compute price divided by 52-week
  high, so once the rule is known the buying happens earlier and the profit shrinks.
- Costs bite on a monthly rebuild. A third of the book moves each month, and every short position
  also pays a borrow fee; the international evidence specifically shows the profit disappearing
  once costs are counted.
- It is largely a momentum bet. The high and the recent return are correlated, so the strategy
  inherits momentum's crashes, as month five of the example shows, and its fate is tied to whether
  momentum itself keeps working.
- Measurement traps. A run that uses only shares that still exist, or that labels industries with
  knowledge of how they later performed, will look better than a rule applied in real time.

## Try it yourself

You need a spreadsheet and a source of daily closing prices; any finance website will give you a
year of history for a handful of shares.

1. Pick ten shares from two industries, five each. Write the industry in one column and the ticker
   in the next.
2. Add a column for the current price and a column for the highest price over the past 252 trading
   days.
3. Add a column that divides the price by the high. That is the score.
4. Sort the sheet by industry, and for each industry compute the average of the scores, weighting
   by a market value you type in from a public source.
5. Rank the two industries by that average, and write down which one is closer to its high.
6. Add the next month's returns for all ten shares and average them inside each industry. Subtract
   the loser's average from the winner's. That is the strategy's return for the month, before the
   cost of about 0.17 percent.
7. Repeat for twelve months, rolling a third of the position each month.

What to notice: the score changes slowly, so most months the same industry is the winner and little
trades, but a share that reaches a new high can jump the ranking, and one bad month for the short
leg can erase several good months. The result over a year of ten shares says almost nothing about
whether the idea works, which is precisely why the papers use thousands of shares and decades.

## Where this came from

- [The list's implementation of this strategy](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/52-weeks-high-effect-in-stocks.py),
  which states the universe, the industry weighting and the three-month holding.
- [Quantpedia: 52-weeks high effect in stocks](https://quantpedia.com/strategies/52-weeks-high-effect-in-stocks/),
  the performance figures, the confidence grade and the source-paper link.
- Hong, Jordan and Liu, [Industry Information and the 52-Week High Effect](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1787378),
  the paper the implementation follows.
- George and Hwang, [The 52-Week High and Momentum Investing](http://www.bauer.uh.edu/tgeorge/papers/gh4-paper.pdf),
  the original study of nearness to the high.
- Liu, Liu and Ma, [The 52-Week High Momentum Strategy in International Stock Markets](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1364566),
  the international test that fails once costs are counted.
- The list's own measurement pages at
  [paperswithbacktest.com/strategies/industry-information-and-the-52-week-high-effect](https://paperswithbacktest.com/strategies/industry-information-and-the-52-week-high-effect)
  and [paperswithbacktest.com/strategies/the-52-week-high-and-momentum-investing](https://paperswithbacktest.com/strategies/the-52-week-high-and-momentum-investing).
- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), this
  repository's own study of the industry-leadership mechanism.

## Words used in this tutorial

- anchoring: judging a number by starting from an unrelated reference point and adjusting too little.
- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- borrow fee: the rent paid for the shares you have shorted, charged for as long as the position is open.
- market value: the total worth of a company's shares, its price multiplied by the number of shares.
- momentum: the tendency of something that has been rising to keep rising for a while.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
