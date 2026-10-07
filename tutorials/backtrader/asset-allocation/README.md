# Asset allocation: how much of the money goes into each holding

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                           |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | A handful of broad holdings - shares of large companies, government bonds, gold and cash, and sometimes commodities or a Bitcoin fund - held in the proportions a rule sets                                                     |
| How often it trades       | Rarely. Some schemes review the weights once a year, some once a month, and some adjust only when a holding has drifted far from its target                                                                                     |
| What you need             | A spreadsheet and a year or two of monthly prices for a few funds                                                                                                                                                               |
| Where the rules come from | [The compendium's asset allocation article](https://backtrader.readthedocs.io/en/latest/strategies-series/en/10-asset-allocation.html)                                                                                          |
| The underlying research   | Markowitz, [Portfolio Selection](https://www.jstor.org/stable/2975974) (1952), and Lopez de Prado, [Building Diversified Portfolios that Outperform Out-of-Sample](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2708678) |
| How well it held up       | Mixed: the diversification benefit is one of the best replicated facts in finance, but each scheme's extra return over a simple split rests on a sample whose falling interest rates will not repeat                            |
| Also appears in           | [Rebalancing premium](../../project/rebalancing-premium/README.md) and [Position sizing](../../project/position-sizing/README.md) in this collection                                                                            |

## The idea in one paragraph

Every strategy in this category answers one question: how much of the money goes into each holding.
A holding is one thing you own, such as an index fund, a bond fund, a gold fund or a pile of cash. A
timing strategy asks "when should I buy?"; an allocation strategy asks "how much of each, and why
that amount?". The schemes here give different answers. One splits the money into fixed percentages
drawn from long experience. One gives every holding an equal share of the risk rather than an equal
share of the money. One groups holdings that move together and splits risk between groups. One holds
a floor the pot must not fall below and lets the rest ride. None forecasts direction; each bets on
the shape of how the holdings move together.

## Why anyone believed it

Own one thing and your money lives or dies with that one thing. Own several that do not move in step,
and a bad day for one is partly paid for by a good day for another, so the whole pot swings less.
This is the oldest idea in portfolio theory, and it needs no forecast. The person on the other side
is not making a mistake: they may simply want to sell today - a fund meeting withdrawals, an estate
settling - and accept a small discount to your price for the convenience. A fixed-weight rule keeps
collecting that small discount because it is always buying whatever has just fallen relative to the
target and selling whatever has just risen.

## An everyday comparison

Think of packing for a trip when you cannot see the weather. You take a coat, a rain jacket, shorts
and sandals in set proportions, because you do not know which you will need and want to be wrong by
only a little whichever way it turns out. If the weather turns out hot, you have too many coats, and
the rule says to swap one for shorts before the next trip. That swap feels wrong each time, because
you are selling the thing that just proved useful and buying the thing that did not. Do it for years
and a pattern can emerge, either because the seasons really do rotate or because nothing else was
stable enough to beat the simple fixed split.

## The rules, step by step

The shared skeleton is the same everywhere: choose the holdings, choose a target share for each one,
check the shares on a schedule, and move money back toward the targets when they have drifted.

1. Choose the holdings. In this category these are broad funds: a share index fund, a government bond
   fund, a gold fund, a broad commodity fund, and cash. A Bitcoin fund appears in one variant.
2. Write down a target share for every holding, as a fraction of the whole pot. The fractions must add
   up to one, because every pound or dollar has to be somewhere.
3. On the chosen schedule - once a year, once a month, or every sixty-three trading days - measure
   what each holding is worth now. Divide by the total pot to get its current share.
4. Compare the current share with the target share. The difference is the holding's drift. If every
   drift is smaller than the scheme's threshold, do nothing at all this time.
5. When a drift is larger than the threshold, trade back to the target shares: sell the holdings that
   have grown too large, buy the ones that have fallen too small. Count the amount traded on both
   sides when you estimate the cost.
6. The schemes differ only in the targets and the trigger. The 60/40 scheme fixes one split for the
   whole life of the pot. The permanent portfolio fixes four equal quarters and accepts a rebalance
   when a quarter strays past a band. The risk-parity scheme recomputes the targets from how much
   each holding moves. The hierarchical scheme recomputes them from which holdings move together. The
   insurance scheme lets the risky target change with a cushion above a floor.

The strategies in this category. Twenty-three files are grouped here; these are the ones a reader
meets first. Each clause says in one line what the file does.

| File                                 | Scheme                    | What it does in one clause                                                                   |
| ------------------------------------ | ------------------------- | -------------------------------------------------------------------------------------------- |
| `test_0011_sixty_forty_portfolio`    | 60/40 with a trend filter | Holds 60% when price is above its 200-day average, 30% when below                            |
| `test_0007_permanent_portfolio`      | Permanent portfolio       | Holds four equal quarters - gold, shares, bonds, cash - and rebalances yearly or past a band |
| `test_0008_taa_risk_parity_trend`    | Risk parity with a gate   | Weights four assets by one over their volatility, only if each is above its ten-month mean   |
| `test_0012_hierarchical_risk_parity` | Volatility-tier sizing    | Cuts a holding's size when its recent volatility is high and restores it when low            |
| `test_0017_cppi_portfolio_insurance` | CPPI portfolio insurance  | Keeps a floor at 80% of the peak and sets the risky share from the cushion above it          |
| `test_0002_gold_60_40_enhancement`   | 60/40 plus gold           | The classic split with a third leg in gold                                                   |
| `test_0005_trinity_portfolio_gold`   | Trinity withdrawal design | The four-percent-a-year withdrawal portfolio, with gold added                                |
| `test_0018_optimal_gold_allocation`  | Weight search for gold    | Searches for the gold share that suited a four-asset mix                                     |

Two representative schemes are worth reading closely, because most of the others vary them.

The 60/40 scheme (`test_0011_sixty_forty_portfolio.py`). The target is 60% of the pot in the risky
holding and the rest in the safer one, with the risky target cut to 30% whenever the price is below
its own 200-day simple moving average. The rule checks the drift every sixty-three trading days and
trades only when the current share differs from the target by more than a tenth of the target (ten
percent of 60%, so six percentage points). There is no "go long" or "go flat", only "hold sixty",
"hold thirty" or "hold nothing".

The permanent portfolio (`test_0007_permanent_portfolio.py`). Four equal quarters: gold, an equity
index fund, a government bond fund, and cash. The rule rebalances on the first trading day of each
calendar year, and also at any time when a holding drifts past a band - five percent for the equity,
bond and cash legs, and a tighter two percent for gold, because gold is the jumpiest of the four. If
several holdings have drifted, they are all moved back at once. There is no entry or exit condition;
a position simply stays at its target share.

## The maths, with every symbol named

The share of the money in each holding, and the fact that the shares add to one:

```text
w_1 + w_2 + ... + w_n = 1
```

- `w_i` is the fraction of the pot in holding `i`, written as a decimal: 0.25 means a quarter.
- `n` is the number of holdings.

This says the target shares must spend the whole pot, no more and no less. If the target for cash is
zero, the risky shares must add to one exactly.

How much a mix swings, which is what almost every scheme here controls. The standard measure of swing
is volatility (the size of the typical move, measured as the standard deviation of returns, where a
return is the percentage change in value over a period):

```text
sigma_portfolio = sqrt( sum over i, sum over j of  w_i * w_j * rho_ij * sigma_i * sigma_j )
```

- `sigma_portfolio` is the volatility of the whole mix, as a fraction per year.
- `sigma_i` is the volatility of holding `i` on its own.
- `rho_ij` is the correlation (a number from -1 to +1 saying how much two things move together; +1
  means in lockstep, 0 means unrelated, -1 means opposite) between holdings `i` and `j`.
- When `i` equals `j`, `rho_ij` is 1 and the term is `w_i` squared times `sigma_i` squared.

The important part is that a mix's swing is not the average of its parts' swings. It is lower whenever
the correlations are below one, and collapses toward zero as they fall. That is diversification,
written as a formula.

The risk-parity answer. Give each holding a share inversely proportional to its own volatility:

```text
w_i = (1 / sigma_i) / ( 1/sigma_1 + 1/sigma_2 + ... + 1/sigma_n )
```

- The right-hand side divides the inverted volatility of holding `i` by the sum of all inverted
  volatilities, so the shares still add to one.
- The calm holdings get large shares and the jumpy ones small shares, so each contributes roughly the
  same swing. When correlations are zero, the products `w_i * sigma_i` come out equal.

The portfolio-insurance answer, called CPPI. Keep a floor the pot must not fall below, and let the
amount above it drive the risky share:

```text
Cushion = Value - Floor
Exposure = multiplier * Cushion / Value
Floor = floor_fraction * Peak
```

- `Value` is the current value of the whole pot, and `Peak` the highest value it has reached.
- `floor_fraction` is the promise kept to yourself, such as 0.80 for 80 percent.
- `Cushion` is the amount above the floor, and `multiplier` how aggressively it is invested, such as 3.
- `Exposure` is the fraction held in the risky holding, capped between 0 and 1; when the pot rises
  the exposure grows, and when it falls the exposure shrinks.

A band keeps the scheme from trading on every small wobble: a holding is only moved back when its
share has drifted past the scheme's threshold.

## A worked example

The numbers are invented but of the size these holdings really take. Costs are counted as the cost
per trade times the amount traded, counting both the sale and the purchase, at 0.0005 of the amount
(five basis points, where one basis point is one hundredth of a percent).

First, the risk-parity arithmetic on two holdings. Shares move about 20 percent a year and bonds
about 5 percent.

```text
w_shares = (1 / 0.20) / (1/0.20 + 1/0.05) = 5.00 / 25.00 = 0.20
w_bonds  = (1 / 0.05) / (1/0.20 + 1/0.05) = 20.00 / 25.00 = 0.80
```

Risk parity gives 20 percent to the jumpy holding and 80 percent to the calm one. Check the swing
each contributes, ignoring correlation: shares contribute 0.20 times 0.20, which is 0.04; bonds
contribute 0.80 times 0.05, which is 0.04. Equal, as intended.

Second, the 60/40 scheme run for five years with a yearly return to target. Start with 1,000.00,
split 600.00 in shares and 400.00 in bonds. Each year the holdings move, then the rule restores the
60/40 split and pays the cost.

| Year | Share return | Bond return | Shares | Bonds  | Total   | Traded | Cost   | Net total |
| ---- | ------------ | ----------- | ------ | ------ | ------- | ------ | ------ | --------- |
| 1    | +10%         | +3%         | 660.00 | 412.00 | 1072.00 | 33.60  | 0.0168 | 1071.98   |
| 2    | -5%          | +2%         | 611.03 | 437.37 | 1048.40 | 36.02  | 0.0180 | 1048.38   |
| 3    | +15%         | -1%         | 723.38 | 415.16 | 1138.54 | 80.52  | 0.0403 | 1138.50   |
| 4    | +5%          | +4%         | 717.26 | 473.62 | 1190.87 | 5.46   | 0.0027 | 1190.87   |
| 5    | -12%         | +6%         | 628.78 | 504.93 | 1133.71 | 102.89 | 0.0514 | 1133.66   |

"Traded" is the amount bought plus the amount sold to return to 60/40, and "Cost" is that amount
times 0.0005. After five years the pot is 1,133.66, up 13.4 percent. The trading is small in money
terms - the largest year moves about ten percent of the pot - so at five basis points the cost never
exceeds six pence per thousand pounds. At a wider gap between buying and selling prices, say 0.002,
the same column would be four times larger and would start to matter.

Third, CPPI, with a floor at 80 percent of the peak, a multiplier of 3, and the same 0.0005 cost. The
single holding is volatile on purpose, so the cushion moves.

| Period | Return | Value before cost | Peak   | Floor | Cushion | Exposure | Risky before | Risky after | Traded  | Cost   | Value after |
| ------ | ------ | ----------------- | ------ | ----- | ------- | -------- | ------------ | ----------- | ------- | ------ | ----------- |
| start  | -      | 1000.00           | 1000.0 | 800.0 | 200.00  | 0.6000   | -            | 600.00      | -       | -      | 1000.00     |
| 1      | +10%   | 1060.00           | 1060.0 | 848.0 | 212.00  | 0.6000   | 660.00       | 636.00      | -24.00  | 0.0120 | 1059.99     |
| 2      | -20%   | 932.79            | 1060.0 | 848.0 | 84.79   | 0.2727   | 508.80       | 254.36      | -254.44 | 0.1272 | 932.66      |
| 3      | +15%   | 970.82            | 1060.0 | 848.0 | 122.82  | 0.3795   | 292.52       | 368.45      | +75.93  | 0.0380 | 970.78      |
| 4      | -25%   | 878.67            | 1060.0 | 848.0 | 30.67   | 0.1047   | 276.33       | 92.00       | -184.34 | 0.0922 | 878.57      |
| 5      | -15%   | 864.77            | 1060.0 | 848.0 | 16.77   | 0.0582   | 78.20        | 50.32       | -27.88  | 0.0139 | 864.76      |
| 6      | +30%   | 879.86            | 1060.0 | 848.0 | 31.86   | 0.1086   | 65.42        | 95.57       | +30.15  | 0.0151 | 879.84      |

The pot ends at 879.84. Holding the volatile thing alone through the same returns would have given
1000 times 1.10 times 0.80 times 1.15 times 0.75 times 0.85 times 1.30, which is about 838.70. So
CPPI gave up some of the upside and kept the value above the 800 floor, exactly as designed. It also
shows the design's weakness: a single jump of more than about a fifth in a day would pass through the
floor before any rule could act.

## What the research actually found

Markowitz's 1952 paper is the source of the idea that a mix can swing less than its parts, and that
there is a set of best mixes for a given level of swing. The paper proves the arithmetic; it does not
claim that any particular split is right, and it treats each holding's expected return, swing and
correlations as known. Almost all the later difficulty comes from those inputs not being known.

The concrete claims reported by the schemes themselves, all on the compendium's own data, are these.
The permanent portfolio, on gold, an equity fund and a bond fund from 2008 to 2025, ended at
4,268,547 from 1,000,000, a rise of 326.9 percent, with a worst fall of 32.3 percent and a
reward-to-risk ratio of 0.659; the article notes that both gold and shares had a powerful run in that
window, which flatters the design. The trend-filtered 60/40 scheme, on gold alone from 2008 to 2025,
ended at 2,542,114, a worst fall of 11.9 percent and a reward-to-risk ratio of 0.770. The CPPI scheme
ended at 1,533,999 from 1,000,000 with a reward-to-risk ratio of 0.447. These are results on one
narrow set of funds over one window, not a general claim that the schemes make money.

One thing must be said plainly, because it applies to every file in this category and to every other
backtest in this compendium. Each backtest asserts its final value, its reward-to-risk ratio and its
worst fall against a stored baseline, and it must produce identical numbers in the engine's two
calculation modes. The reward-to-risk ratio is the average return divided by how much the pot swung,
and the worst fall is the largest drop from a peak to the following low. Passing those assertions
proves the engine computes exactly what the file says it computes, on the stored data. It does not
prove the strategy earns anything. The assertion is a test of the software, not a claim about the
market. A file can pass every assertion and still describe a plan that loses money.

## How this project relates to it

The most useful document here is the repository's own brief on portfolio construction,
[strategies/books2/10_portfolio_and_allocation.md](../../../strategies/books2/10_portfolio_and_allocation.md).
It collects the evidence that the inputs to any allocation rule are themselves noisy. Its sharpest
point is geometric: if you build weights by inverting a covariance matrix (a table of how every pair
of holdings moves together), a poorly conditioned matrix rotates your bets away from your own
forecast, and the size of that rotation is bounded only by the matrix's condition number -
`2107.06194v5` (p.4). The practical reading is that risk-parity and hierarchical schemes exist partly
to avoid inverting that matrix at all. The same brief reports that in a regime-switching model
calibrated to the Nasdaq 100, 94 holdings are needed to explain 95 percent of a hundred-asset
universe's variation (`2204.13398v1`, p.15-16).

Two finished tutorials here cover the parts that touch allocation: the
[rebalancing premium](../../project/rebalancing-premium/README.md) tutorial works through what the
periodic move back to target does and does not earn, and the
[position sizing](../../project/position-sizing/README.md) tutorial covers how much of the pot goes
into a single holding.

## Where it goes wrong

- Correlations rise in a crisis. The formula rewards holdings that move apart, but in a crash most
  risky holdings fall together, so diversification is weakest exactly when needed. The 60/40
  scheme's soft spot in 2008 is the standard example.
- The inputs are estimated, not known. Risk parity needs each holding's volatility and the
  hierarchical version a full table of correlations; both are measured from a past window and both
  change, and a small error in the estimate moves many weights at once.
- One sample, one interest-rate path. Government bonds earned well for four decades as their yields
  fell from high levels to low ones. A bond fund cannot repeat that from today's low starting point,
  so any scheme that leaned on bonds in a backtest leaned on history that will not recur.
- Portfolio insurance has gap risk. CPPI assumes the rule can always sell before the floor is reached,
  but a price that jumps straight through the floor in one step, as it can overnight, breaks the
  promise.
- Costs can be mistaken for skill. The more often a scheme checks and trades, the more it pays in the
  gap between buying and selling prices; a scheme that appears to add value can lose it to those
  costs.

## Try it yourself

You need a spreadsheet and the monthly prices of two funds - a share index fund and a bond fund - for
the last five years.

1. Make one row per month, with a column for the share fund's price and a column for the bond fund's
   price.
2. Add a column that computes each fund's monthly return: this month's price divided by last month's
   price, minus one.
3. Set the first row's holding values to 600 in shares and 400 in bonds.
4. For each later row, grow the holdings by the two returns, then add a column that restores the 60/40
   split and subtracts 0.0005 times the amount traded.
5. Beside that, add a second path that never rebalances: the holdings just grow.
6. Finally, add a column for each path's running worst fall: the largest drop from a peak to the
   following low.

What to notice: the unrebalanced path's shares usually drift above 60 percent after a strong run of
the share market, so the two paths differ more than the small cost suggests. Over most five-year
windows they end close together, sometimes one ahead and sometimes the other. If the rebalanced path
wins by a wide margin, check that you have not used today's fund names on prices from before those
funds existed, which is a way of looking into the future.

## Where this came from

- [The compendium's asset allocation article](https://backtrader.readthedocs.io/en/latest/strategies-series/en/10-asset-allocation.html),
  the rules, the inventories and the scheme-level results quoted above.
- Markowitz, [Portfolio Selection](https://www.jstor.org/stable/2975974), the 1952 paper that made the
  arithmetic of diversification explicit.
- Lopez de Prado, [Building Diversified Portfolios that Outperform Out-of-Sample](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2708678),
  the hierarchical risk parity method.
- [strategies/books2/10_portfolio_and_allocation.md](../../../strategies/books2/10_portfolio_and_allocation.md),
  this repository's brief on how fragile the inputs to an allocation rule are.

## Words used in this tutorial

- basis point: one hundredth of one percent, so five basis points is 0.05 percent.
- correlation: a number from minus one to one saying how much two things move together.
- covariance: a table saying how every pair of holdings moves together.
- drawdown: the fall from a peak to the following low, measured in percent.
- position: the amount of one holding you own, measured in money or units.
- rebalance: to trade holdings back toward their target shares.
- volatility: how much a price moves around its average, measured as a percentage per year.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
