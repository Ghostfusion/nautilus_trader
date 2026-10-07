# The price to earnings ratio: buying shares that are cheap relative to their yearly profits

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                         |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of American companies, ten at a time, chosen because their price is low relative to their yearly profit                                                                                                                                                |
| How often it trades       | Once a year, when the whole list is rebuilt                                                                                                                                                                                                                   |
| What you need             | A spreadsheet, a list of share prices, and each company's yearly profit per share                                                                                                                                                                             |
| Where the rules come from | [QuantConnect strategy library, price earnings anomaly](https://www.quantconnect.com/tutorials/strategy-library/price-earnings-anomaly)                                                                                                                       |
| The underlying research   | Persson and Stahlberg, [P/E and EV/EBITDA Investment Strategies vs. the Market](https://www.diva-portal.org/smash/get/diva2:23073/FULLTEXT01.pdf), and Fama and French, [Multifactor Explanations of Asset Pricing Anomalies](https://ssrn.com/abstract=7365) |
| How well it held up       | Mixed: cheap shares have beaten expensive ones across long published samples, but the record is entangled with company size and with risk, and it is the value factor under another name                                                                      |
| Also appears in           | [Fama-French five factors](../fama-french-five-factors/README.md) in this collection, where the same premium is one of the five systematic influences                                                                                                         |

## The idea in one paragraph

Every company earns a profit. Divide the price of one share by the profit that share earned over the
past year, and you get a number called the price to earnings ratio. A share that costs 20.00 and
earned 2.00 per share has a ratio of 10. This strategy ranks a universe of shares by that number,
buys the ten with the lowest ratios in equal amounts, and holds them for a year. At the start of the
next year it rebuilds the list, sells whatever has become more expensive relative to its profits,
and buys the new cheapest names. The bet is that boring, unloved, cheap shares tend to be priced too
low and slowly drift up, while exciting shares with high ratios tend to be priced too high.

## Why anyone believed it

A low ratio can mean the market is paying little attention to a company, or is afraid of it. Some of
that fear is justified and some is not, and the argument behind this strategy is that the market
over-reacts to bad news. A company has one bad quarter, its profit falls, its ratio jumps, and
investors abandon it. If the trouble is temporary, the shares are cheap for no good reason and later
recover.

The counterparty is the investor who dislikes uncertainty. A company in trouble is unpleasant to
own: the news is bad, the price wobbles, and a fund manager is embarrassed to hold it. That
discomfort is a reason to sell even when no money is needed, which pushes the price below what the
profits alone would justify. If enough investors behave this way, cheap shares keep earning a little
more than expensive ones, and the extra is the reward for holding something unpopular.

## An everyday comparison

Think of two shops on the same street selling identical apples. One is a smart new shop with bright
lights and a queue; the other is a tired old shop with a hand-written sign. The apples are the same,
so the smart shop charges more per apple. Over a year, the fuss around the new shop fades and it has
to cut prices, while the old shop is discovered by a few regulars and raises its prices slightly.
The person who bought from the tired shop did not get better apples. They got the same apple at a
lower price, and that lower price was the whole edge. The risk is that the apples at the tired shop
are cheap because they are about to spoil, and a low price on its own does not tell you which case
you are in.

## The rules, step by step

1. Start from a universe of American shares that have published financial reports and trade above
   5.00 per share. The library page removes very cheap shares because they are awkward and expensive
   to trade. The universe is then cut down to the most heavily traded names in the library's data.
2. For each share, keep only those whose profit over the past year is positive. A company that lost
   money has no meaningful price to earnings ratio, because dividing by a negative number gives a
   negative ratio that sorts wrongly.
3. Compute each share's price to earnings ratio: today's share price divided by the profit per share
   over the past year.
4. Rank the shares from the lowest ratio to the highest.
5. Buy the ten lowest, giving each the same amount of money, so one tenth of the account goes into
   each.
6. Hold for one year. Do not trade in between.
7. At the start of the next year, recompute every ratio and rebuild the list from step 4. Sell the
   shares that have left the ten and buy the ones that have entered.

One point worth stating plainly: this is not a rule about companies being good. It is a rule about
price, and nothing in it checks whether the profit is likely to continue. That gap is where most of
the risk lives, and it is the subject of the last section of this tutorial.

## The maths, with every symbol named

The whole strategy is one division, repeated for each share, and one sort.

The price to earnings ratio:

```text
P/E = price / EPS
```

- `P/E` is the price to earnings ratio, a plain number of years.
- `price` is the market price of one share today.
- `EPS` is earnings per share: the company's profit over the past year divided by the number of
  shares it has issued.

It helps to read the result as a period of time. If profits stayed exactly the same every year, a
ratio of 10 would mean ten years of profits add up to the current price. That is why the number is
sometimes called the payback period.

The reverse of the ratio is the earnings yield, which is easier to compare with an interest rate:

```text
earnings yield = EPS / price = 1 / (P/E)
```

- A ratio of 10 gives an earnings yield of 0.10, that is 10 percent. A ratio of 40 gives 2.5 percent.

When a high ratio is expected to be justified by fast profit growth, a second formula is used:

```text
PEG = (P/E) / growth
```

- `growth` is the expected yearly growth of profits, written as a percent number, for example 20 for
  20 percent.

A ratio of 20 with growth of 20 percent gives a PEG of 1. A ratio of 20 with growth of 5 percent
gives a PEG of 4. The rule of thumb, popularised by the investor Peter Lynch, is that a fairly
priced share has a PEG near 1. This formula is a heuristic, not a measurement, and the source paper
for this tutorial spends several pages on its weaknesses.

Finally the cost of the yearly rebuild:

```text
Cost = t * c
```

- `t` is the traded fraction: 2.0 if the entire list is sold and replaced at once, because selling
  the old holdings and buying the new ones counts twice, and less when some holdings are kept.
- `c` is the cost per trade as a fraction of the amount traded, covering the gap between the buying
  and the selling price plus commission. For large American shares a realistic figure is 0.0005 to
  0.001, that is five to ten basis points, where one basis point is one hundredth of one percent.

## A worked example

A universe of twelve shares. The library page buys ten of the cheapest; here the example buys the
five cheapest so the table fits on one page, which changes nothing about the arithmetic. The prices
and profits are invented, but they are of the sizes that real share prices and profits take.

| Company | Price | Profit per share, past year | P/E   | Kept?                      |
| ------- | ----- | --------------------------- | ----- | -------------------------- |
| Alpha   | 20.00 | 2.50                        | 8.0   | Yes                        |
| Beta    | 30.00 | 3.00                        | 10.0  | Yes                        |
| Gamma   | 12.00 | 1.00                        | 12.0  | Yes                        |
| Delta   | 45.00 | 3.00                        | 15.0  | Yes                        |
| Epsilon | 25.00 | 1.50                        | 16.7  | Yes                        |
| Zeta    | 60.00 | 3.00                        | 20.0  | No                         |
| Eta     | 18.00 | 0.50                        | 36.0  | No                         |
| Theta   | 8.00  | 0.20                        | 40.0  | No                         |
| Iota    | 50.00 | 0.50                        | 100.0 | No                         |
| Kappa   | 10.00 | 0.05                        | 200.0 | No                         |
| Lambda  | 15.00 | -0.20                       | none  | Excluded, loss             |
| Mu      | 3.00  | 0.30                        | 10.0  | Excluded, price below 5.00 |

The five kept shares are Alpha, Beta, Gamma, Delta and Epsilon, one fifth of the money each. Now
suppose the following year produces these returns:

| Share held | Weight | Next-year return | Contribution  |
| ---------- | ------ | ---------------- | ------------- |
| Alpha      | 0.20   | +12 percent      | +2.40 percent |
| Beta       | 0.20   | +6 percent       | +1.20 percent |
| Gamma      | 0.20   | -4 percent       | -0.80 percent |
| Delta      | 0.20   | +9 percent       | +1.80 percent |
| Epsilon    | 0.20   | +2 percent       | +0.40 percent |
| Total      | 1.00   |                  | +5.00 percent |

So the portfolio gained 5.00 percent before costs, against 3 percent for the market index. Suppose
the rebuild replaces two of the five shares, so the traded fraction is:

```text
t = 2 * (2 / 5) = 0.8
Cost = 0.8 * 0.001 = 0.0008, that is 0.08 percent
Net return for the year = 5.00 - 0.08 = 4.92 percent
```

Two things are worth noticing. First, the cost is small when the list changes slowly, and a yearly
rule trades much less than a monthly one, which is why this version is comparatively cheap to run.
Second, the worked example says nothing about whether the strategy works. It only shows how to apply
the rules and how the arithmetic behaves. A single year of five shares is noise, not evidence.

## What the research actually found

The published record for cheap shares is long, and it does not point one way.

| Source                                                                              | What it measured                                                 | Result                                                                                                                                                                                                                       |
| ----------------------------------------------------------------------------------- | ---------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Quantpedia, value (book to market) factor                                           | Long cheap shares, short expensive ones, American data           | 3.6 percent a year over 1926 to 2014, volatility 12.02 percent, worst fall 55.99 percent, reward-to-risk 0.3; the site grades its own confidence in the effect as strong                                                     |
| Persson and Stahlberg, master's thesis cited by the library page                    | Price to earnings and related valuation rules against the market | A price to earnings rule beat the market in their sample; the thesis is the direct source the library page names                                                                                                             |
| Fama and French, Multifactor Explanations of Asset Pricing Anomalies                | Whether the market alone explains cheap-share returns            | A simple market model does not explain them, which is the finding that pushed value into multi-factor models                                                                                                                 |
| Baird, Dodd and Middleton, growth adjusted price to earnings (`2001.08240v1`, p.11) | Five groups by an earnings and growth measure, 1990 to 2015      | The cheapest group returned 19.82 percent a year against 15.80 percent for the most expensive group; a plain price to earnings sort gave 18.89 percent for the cheapest group, so the plain ratio already carried most of it |
| The library page's own backtest                                                     | Ten cheapest shares, yearly rebuild, 2016 to July 2019           | The portfolio beat the S&P 500 over three and a half years; the page itself notes that most chosen shares were small companies, so a size effect was mixed in with the value effect                                          |
| Lev and Srivastava, cited by Quantpedia                                             | The value premium over recent decades                            | Argues the strategy has been unprofitable for almost thirty years barring a brief revival after the dot-com fall, so any claim of a permanent edge is disputed                                                               |

Read together, the picture is this. There is a long and replicated tendency for cheap shares to
outperform expensive ones over many years, and it is one of the best documented patterns in finance.
But the prize is modest, the strategy falls further than the market in crashes, a large part of the
gain is compensation for holding companies that might fail, and the pattern has been weak for a long
stretch of recent history. The library page's own sample is short and tilted towards small companies
by its filters, so it is better read as a demonstration than as proof.

## How this project relates to it

This repository contains research briefs that cover the same question from the evidence side. In
[the predictability and trading strategies brief](../../../strategies/books2/08_predictability_and_trading_strategies.md),
the "factor zoo" section reports that a large study of published cross-sectional predictors found
most of them are probably real rather than the result of trying many rules, but that the measured
power drops sharply once the shares are weighted by company size and adjusted for a handful of
factors. That is exactly the caution this tutorial applies to the price to earnings ratio: the raw
sort looks strong, and part of it is company size and risk in disguise.

A second related file is
[the portfolio and allocation brief](../../../strategies/books2/10_portfolio_and_allocation.md),
whose cross-section section reports the same result, and whose caveats note that the evidence comes
from United States equity data. If you want to see the mechanism measured rather than asserted, those
two briefs are the closest thing this repository has.

## Where it goes wrong

- A low ratio can be a warning, not a discount. If the market expects next year's profits to fall,
  a low ratio on last year's profits is not cheap. The strategy reads the past and the price is about
  the future, so the rule systematically buys companies whose profits may not continue.
- The profit figure itself is soft. Reported profit depends on accounting choices, one-off charges
  and write-downs, and two companies with identical economics can report different profits. The
  source paper for this tutorial shows that using a cleaned-up profit measure that removes one-off
  items changes the result.
- The extra return may be a risk premium rather than a mistake. Companies with low ratios are often
  in trouble, and one reason they earn more is that they can fail. For the whole idea to be false, it
  is enough that the extra return is fair payment for that risk, in which case there is nothing to
  exploit, only risk to be paid for.
- Company size is tangled with the ratio. Cheap shares tend to be small ones, and small shares have
  their own long-run premium. The library page's own note that its chosen shares were mostly small
  companies is this problem in the open.
- Costs punish the small and the unloved. The shares that top the ranking are often thinly traded,
  where the gap between the buying and the selling price is wide. A test that ignores that gap has
  not measured what a real account would earn.
- The rule was chosen after the fact. Value, size, growth and profitability all overlap, and picking
  the price to earnings ratio because it worked in a particular sample is a form of looking back.

## Try it yourself

You need nothing but a spreadsheet, a list of share prices, and each company's profit per share over
the past year; any finance website will supply all three for a list of well-known companies.

1. Build a sheet with one row per company and columns for price, profit per share, and price divided
   by profit.
2. Sort the rows by that last column, lowest first. Write down the five companies at the top.
3. Write down the five companies at the bottom.
4. On a second sheet, record what each of those ten companies did over the following year. Use a
   public price history.
5. Average the returns of the cheap five and the expensive five separately.

What to notice: the cheap group usually contains companies you would recognise as troubled, and the
expensive group usually contains fashionable ones. In many single years the cheap group loses. The
pattern, if it shows up at all, appears over many years, and it comes with years in which the cheap
group falls much further than the market. If your exercise shows the cheap group winning by a wide
margin in one year, the honest conclusion is that one year is not enough to say anything.

## Where this came from

- [QuantConnect strategy library: price earnings anomaly](https://www.quantconnect.com/tutorials/strategy-library/price-earnings-anomaly),
  the rules as implemented: a filtered universe, the ten lowest ratios, equal weights, rebuilt each
  year.
- Persson and Stahlberg, [P/E and EV/EBITDA Investment Strategies vs. the Market](https://www.diva-portal.org/smash/get/diva2:23073/FULLTEXT01.pdf),
  the thesis the library page names as its source.
- Fama and French, [Multifactor Explanations of Asset Pricing Anomalies](https://ssrn.com/abstract=7365),
  the paper that showed a market-only model does not explain the cheap-share premium.
- Baird, Dodd and Middleton, [A growth adjusted price-earnings ratio](https://arxiv.org/abs/2001.08240),
  the source of the 19.82 percent and 15.80 percent figures and of the discussion of how profit
  measurement changes the result.
- [Quantpedia: value (book to market) factor](https://quantpedia.com/strategies/value-book-to-market-factor),
  the performance figures, volatility, worst fall and instrument count for the value premium.
- [The predictability and trading strategies brief](../../../strategies/books2/08_predictability_and_trading_strategies.md)
  and [the portfolio and allocation brief](../../../strategies/books2/10_portfolio_and_allocation.md),
  this repository's own studies of the value and factor evidence.

## Words used in this tutorial

- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- earnings per share: a company's yearly profit divided by the number of shares it has issued.
- market index: a single number that tracks the average price of a chosen list of shares.
- risk premium: extra average return that investors demand for holding something that might fall.
- small companies: companies whose total market value is low compared with the largest listed firms.
- value factor: the tendency of shares that are cheap relative to fundamentals to beat expensive ones.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
