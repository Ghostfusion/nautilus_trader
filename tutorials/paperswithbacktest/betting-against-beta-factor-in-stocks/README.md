# Betting against beta: buying the calmest shares with borrowed money and shorting the jumpiest

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                               |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of American companies in both directions: the least market-sensitive bought with borrowed money, the most market-sensitive sold short                                                                                                                                                        |
| How often it trades       | Once a calendar month, when both baskets are rebuilt                                                                                                                                                                                                                                                |
| What you need             | A spreadsheet and a year of daily prices for the shares and for a broad market index                                                                                                                                                                                                                |
| Where the rules come from | [The list's implementation file](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/betting-against-beta-factor-in-stocks.py) and the [Quantpedia entry](https://quantpedia.com/strategies/betting-against-beta-factor-in-stocks/) it repeats             |
| The underlying research   | Frazzini and Pedersen, [Betting Against Beta](http://pages.stern.nyu.edu/~lpederse/papers/BettingAgainstBeta.pdf)                                                                                                                                                                                   |
| How well it held up       | Disputed: the paper and several follow-ups find the effect, but a direct critique argues its size comes from how the baskets are built, a second line of work attributes it to demand for lottery-like shares, and the list's own run returned a Sharpe ratio of 0.57, so credible sources disagree |
| Also appears in           | [Beta factors in stocks](../../quantconnect/beta-factors-in-stocks/README.md), the beta tutorial in this collection, and its sibling on [country funds](../../quantconnect/beta-factor-in-country-equity-indexes/README.md)                                                                         |

## The idea in one paragraph

Beta is a number for how strongly one share moves when the whole market moves; a beta of one moves
about as much as the market, a beta of 1.5 moves more and a beta of 0.6 moves less. Many investors
are not allowed to borrow money to buy more of the market, so to make their money swing harder they
buy the jumpiest shares instead, which pushes those shares up and their later returns down. This
strategy does the opposite. It buys a basket of the calmest shares and borrows so that the basket
swings as much as the market, and it sells short a basket of the jumpiest shares and trims it so
that the shorted basket also swings just as much. The bet is that the calm shares are underpriced
and the jumpy ones overpriced.

## Why anyone believed it

The story is about who is allowed to borrow. A pension fund or a mutual fund is usually forbidden
from using borrowed money, and an ordinary investor faces limits on it too. If such an investor
wants more return than the market gives, the only lever available is to hold shares that move more
than the market. That demand bids up the jumpy, high-beta shares, so they become expensive and the
return they deliver for the risk they carry is disappointing. Meanwhile the calm, low-beta shares
are left cheap.

The counterparty, then, is the constrained investor who overpays for high-beta shares because
borrowing is closed to them, and the winner is the investor who is free to borrow. When funding
gets scarce, the paper predicts that the calm basket stops winning, because even the free investors
have to pull back. That prediction is the part of the story that makes it a theory rather than a
pattern.

## An everyday comparison

Imagine two neighbours who each want to own the same house. One has a mortgage and can simply buy
the larger, safer house on the quiet street. The other cannot borrow, so to get the same potential
gain they buy a run-down house on a flood plain, because that is the only purchase that might
double. Because many buyers face the same restriction, run-down flood-plain houses are bid up and
end up poor value, while the quiet-street houses sell at a discount. The strategy buys the quiet
houses and, in effect, bets against the flood-plain ones.

## The rules, step by step

1. Assemble a list of shares. The paper uses every share in a research database; the code uses the
   1000 most heavily traded American shares priced above five dollars.
2. Choose a market index to represent the market. The paper uses a broad American equity index; the
   code uses the SPY fund, which tracks the largest American index.
3. At the end of each month, estimate each share's beta against that index over the past year of
   daily prices. The method, the covariance of the share's daily returns with the index's divided by
   the variance of the index's, is given below.
4. Sort the shares by beta, smallest first. Split them into a low-beta group and a high-beta group.
   The paper splits the list in two and weights shares by their rank; the code takes the bottom
   tenth and the top tenth in equal amounts. Either way the two groups are the calm and the jumpy.
5. Rescale the low-beta group so its beta is one: if its members' average beta is 0.60, buy 1.67
   units for every unit of your own money, borrowing the extra 0.67.
6. Rescale the high-beta group so its beta is one: if its members' average beta is 1.40, short only
   0.71 units for every unit of your own money.
7. Hold both baskets for one month, then go back to step 3 and rebuild. Pay trading costs, interest
   on the money borrowed, and a borrow fee on everything shorted.
8. Size the whole thing so that the combined bet has a beta of zero, which it does almost by
   construction: the long side swings like the market and the short side swings like the market, so
   the two cancel.

## The maths, with every symbol named

The beta of one share is its fitted sensitivity to the market:

```text
beta_i = Cov(r_i, r_m) / Var(r_m)
```

- `beta_i` is the beta of share `i`, a plain number with no units.
- `r_i` is the daily return of share `i` over the past year.
- `r_m` is the daily return of the market index over the same days.
- `Cov(r_i, r_m)` is the covariance, a measure of how much the two return series move together.
- `Var(r_m)` is the variance of the index returns, a measure of how much the index itself moves.

The two baskets are then levered so that each has a beta of one:

```text
L_low  = 1 / (average beta of the low-beta basket)
L_high = 1 / (average beta of the high-beta basket)
```

- `L_low` is the multiplier applied to the calm basket. A value of 1.67 means 1.67 of the basket is
  bought with one unit of your own money and 0.67 of borrowed money.
- `L_high` is the multiplier applied to the jumpy basket. A value of 0.71 means only 0.71 is shorted
  per unit of your own money.
- Both multipliers are usually capped, at two in the code and five in the country version, because
  a very low beta would otherwise demand enormous borrowing.

The monthly return of the whole bet is one line:

```text
R = L_low * R_low - L_high * R_high
```

- `R_low` is the return of the calm basket over the month, equally weighted across its shares.
- `R_high` is the return of the jumpy basket, equally weighted.
- `R` is the strategy's return, stated per unit of your own money.

The monthly cost has three parts:

```text
Cost_month = (L_low + L_high) * turn * c + (L_low - 1) * i + L_high * s
```

- `turn` is the fraction of the two baskets replaced at the rebuild, about 0.2 if a fifth of the
  names change.
- `c` is the cost of one trade as a fraction of its value, about 0.001, ten basis points, counting
  the gap between the buying and selling price. One basis point is one hundredth of one percent.
- `i` is the interest paid per month on the borrowed money, the yearly rate divided by twelve.
- `s` is the borrow fee per month on the shorted shares, again the yearly rate divided by twelve.
- `(L_low + L_high) * turn` is the notional traded; `(L_low - 1)` is the borrowed fraction of the
  long basket; `L_high` is the shorted fraction.

## A worked example

Six shares, three calm and three jumpy, with invented but plausible betas and monthly returns.

| Basket    | Share | Beta | Average beta | Leverage |
| --------- | ----- | ---- | ------------ | -------- |
| Low beta  | L1    | 0.40 |              |          |
| Low beta  | L2    | 0.60 |              |          |
| Low beta  | L3    | 0.80 | 0.60         | 1.6667   |
| High beta | H1    | 1.20 |              |          |
| High beta | H2    | 1.40 |              |          |
| High beta | H3    | 1.60 | 1.40         | 0.7143   |

The calm basket's average beta is `(0.40 + 0.60 + 0.80) / 3 = 0.60`, so its leverage is
`1 / 0.60 = 1.6667`. The jumpy basket's average beta is `(1.20 + 1.40 + 1.60) / 3 = 1.40`, so its
leverage is `1 / 1.40 = 0.7143`. The bet's beta is then `1.6667 * 0.60 - 0.7143 * 1.40 = 1.00 -
1.00 = 0.00`, which is the goal.

| Month | Calm basket | Jumpy basket | Gross    | Cost    | Net      | Cumulative |
| ----- | ----------- | ------------ | -------- | ------- | -------- | ---------- |
| 1     | +0.90%      | +1.30%       | +0.5714% | 0.3235% | +0.2480% | +0.2480%   |
| 2     | -0.30%      | -1.00%       | +0.2143% | 0.1329% | +0.0813% | +0.3296%   |
| 3     | +1.20%      | +2.00%       | +0.5714% | 0.1329% | +0.4385% | +0.7695%   |
| 4     | +0.20%      | -0.20%       | +0.4762% | 0.1329% | +0.3433% | +1.1154%   |
| 5     | +1.00%      | +1.50%       | +0.5952% | 0.1329% | +0.4623% | +1.5829%   |

The gross column uses `1.6667 * calm - 0.7143 * jumpy`. Month one, for example:
`1.6667 * 0.90 - 0.7143 * 1.30 = 1.5000 - 0.9286 = 0.5714` percent. The cost column in month one
includes opening the whole book, and from month two only a fifth of the names turn over. Interest is
one percent a year on the borrowed 0.6667, which is `0.6667 * 0.01 / 12 = 0.0556` percent a month;
the borrow fee is half a percent a year on the shorted 0.7143, which is `0.7143 * 0.005 / 12 =
0.0298` percent a month.

The result is about plus 1.6 percent over five months, gross plus 2.4 percent, so costs eat a third
of the gross gain. That is the honest shape of this strategy: the raw effect is small, and leverage,
interest and the borrow fee are a large part of the story. If the calm basket had fallen ten percent
in a month, the borrowed 0.6667 would have turned that into a loss of about sixteen percent on your
own money in that same month.

## What the research actually found

| Source and what it measured                                             | Result                                                                                                                                                                                                                   |
| ----------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Frazzini and Pedersen, American shares, bonds and futures, 1926 to 2009 | High beta goes with low risk-adjusted return in US shares, 20 international equity markets, government and corporate bonds, and futures; the levered long-short factor earns a significant positive risk-adjusted return |
| Quantpedia's summary of the same paper                                  | 8.86 percent a year at 11.5 percent volatility, worst fall 61.5 percent, a Sharpe ratio of 0.42 over 1926 to 2009, graded Moderate, with a note that its out-of-sample test was slightly negative                        |
| Novy-Marx and Velikov, rebuilding the factor differently                | The measured performance comes largely from non-standard construction choices, not from beta itself; the factor quietly equal-weights shares and tilts toward small, profitable companies                                |
| Bali, Brown, Murray and Tang, controlling for lottery-like shares       | Once the baskets are made neutral to demand for lottery-like shares, the abnormal return disappears, so the effect may be that demand rather than a funding story                                                        |
| The list's own measurement, on its own data                             | A Sharpe ratio of 0.57, annual return 10.95 percent, volatility 22.87 percent, worst fall 60.0 percent, 1990 to 2026 (the vendor's own measurement)                                                                      |

The disagreement is the point, and it is unresolved. The original paper finds the effect across
many markets and asset classes. A careful critique says the headline number is an artefact of how
the baskets are assembled, and a separate line of research says the whole thing is a disguised bet
against popular speculative shares. The list's own run sits between them: a Sharpe ratio of 0.57
over 36 years, with a worst fall of 60 percent. For context, the list reports that across thousands
of papers the median replication has a Sharpe ratio of 0.37 and 48 percent clear a t-statistic of
1.96, so a single published figure of this size is not, by itself, strong evidence.

## How this project relates to it

The finished tutorial [Beta factors in stocks](../../quantconnect/beta-factors-in-stocks/README.md)
in this collection works the plain version of the same idea, sorting shares by how much they move
with the market rather than levering two baskets against each other. Its sibling,
[Beta factor in country equity indexes](../../quantconnect/beta-factor-in-country-equity-indexes/README.md),
applies the same sort to national funds.

Two briefs in this repository cover the machinery the strategy depends on.
[Portfolio construction and allocation](../../../strategies/books2/10_portfolio_and_allocation.md)
reads the recent evidence on how fragile a backtest Sharpe ratio is and how much of a factor's
measured return depends on weighting choices, which is exactly the ground on which this factor is
disputed. [Banking, credit and funding liquidity](../../../strategies/books2/02_banking_credit_and_funding.md)
covers how leverage rules and margin amplify a cycle, which is the mechanism the paper says makes
betting against beta earn anything at all.

## Where it goes wrong

- Leverage turns a bad month into a very bad month. The long side is bought with borrowed money, so
  a fall in the calm basket is multiplied, and the published worst fall of about 60 percent is the
  cost of that.
- Funding tightens exactly when the strategy loses. The paper itself predicts this, and the list's
  own chart shows a fall of 60 percent, so the worst losses arrive together with the highest
  borrowing costs.
- The construction is doing part of the work. The critique above shows that equal-weighting and a
  tilt to small, profitable shares account for much of the measured return, and those choices are
  easy to make unconsciously.
- Shorting is expensive for the shares that matter most. The jumpy, high-beta names are often the
  ones with the highest borrow fees and the most crowded shorts, so the short leg costs more than
  the backtest assumes.
- A single measured Sharpe ratio is weak evidence. A rule tried after the fact and measured over one
  history can look good by luck, and the list's own median of 0.37 across thousands of papers is the
  clearest warning.
- The theory is falsifiable. If high-beta shares were not systematically overpriced by constrained
  investors, the calm basket would earn no premium once its beta is restored to one; the tournament
  is whether the effect survives after costs and after neutralising the lottery-demand explanation.

## Try it yourself

You need a spreadsheet and daily closing prices for a handful of shares and one index fund.

1. List eight shares in one column and, beside each, paste the last 252 daily closing prices in a
   row.
2. Add a row that computes each day's return: today's price divided by yesterday's, minus one.
3. Do the same for the index fund. Now for one share, compute the covariance of its returns with the
   index returns, and divide by the variance of the index returns. That is its beta. Repeat for all
   eight.
4. Sort the eight by beta. Add up the betas of the four lowest and divide by four; do the same for
   the four highest.
5. Write down the leverage of each basket: one divided by its average beta.
6. Take next month's returns, average them inside each basket, and compute
   `leverage_low * low - leverage_high * high`.
7. Repeat for a year, rebuilding each month, and subtract about 0.13 percent a month for costs.

What to notice: the two baskets are almost never equally sized in money, because the leverage
formula deliberately makes the calm side bigger; and in a month when the market falls hard, the
shorts usually save the day while the levered longs hurt. Over a few shares and one year the result
is noise, which is the honest limit of this exercise.

## Where this came from

- [The list's implementation of this strategy](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/betting-against-beta-factor-in-stocks.py),
  which states the rolling beta, the two baskets and the rescaling to beta one.
- [Quantpedia: betting against beta factor in stocks](https://quantpedia.com/strategies/betting-against-beta-factor-in-stocks/),
  the performance figures, the confidence grade and the source-paper link.
- Frazzini and Pedersen, [Betting Against Beta](http://pages.stern.nyu.edu/~lpederse/papers/BettingAgainstBeta.pdf),
  the paper the implementation follows.
- Novy-Marx and Velikov, [Betting Against Betting Against Beta](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=3300965),
  the construction critique.
- Bali, Brown, Murray and Tang, [Betting Against Beta or Demand for Lottery](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2408146),
  the lottery-demand explanation.
- The list's own measurement page at
  [paperswithbacktest.com/strategies/betting-against-beta](https://paperswithbacktest.com/strategies/betting-against-beta).
- [Portfolio construction and allocation](../../../strategies/books2/10_portfolio_and_allocation.md),
  this repository's brief on how weighting and measurement choices decide a factor's reported return.

## Words used in this tutorial

- beta: a number for how strongly one price moves when the market moves; one means it moves about as much.
- borrow fee: the rent paid for the shares you have shorted, charged for as long as the position is open.
- covariance: a measure of how much two series of numbers move together.
- leverage: using borrowed money so that a given price move produces a larger gain or loss.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- variance: a measure of how widely a series of numbers is spread around its average.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
