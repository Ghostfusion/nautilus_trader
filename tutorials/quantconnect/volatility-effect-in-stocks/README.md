# The volatility effect: buying the calmest shares

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                     |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of large American companies, bought and held in equal amounts                                                                                                                                                                                                      |
| How often it trades       | About once a month, when the calmest list is recomputed                                                                                                                                                                                                                   |
| What you need             | A spreadsheet and one year of daily prices for the shares you want to rank                                                                                                                                                                                                |
| Where the rules come from | [QuantConnect strategy library, volatility effect in stocks](https://www.quantconnect.com/tutorials/strategy-library/volatility-effect-in-stocks) and the [Quantpedia long-only entry](https://quantpedia.com/strategies/low-volatility-factor-effect-in-stocks) it cites |
| The underlying research   | Baker, Bradley and Wurgler, [Benchmarks as Limits to Arbitrage: Understanding the Low-Volatility Anomaly](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1585031), with Clarke, de Silva and Thorley on minimum-variance portfolios                                  |
| How well it held up       | Disputed: the effect is measured repeatedly over long samples and survives costs, but the credible academic accounts give three different and unreconciled reasons for it, and one strong reading says it is not a separate effect at all                                 |
| Also appears in           | Nothing else in this collection describes the low-volatility anomaly                                                                                                                                                                                                      |

## The idea in one paragraph

Every share price moves, and some move far more violently than others. That movement is called
volatility, and it can be measured: for each share, look at how its price changed each day for the
past year, and work out how widely those daily changes were scattered. This strategy does that
measurement for a list of large American companies, lines the shares up from calmest to wildest,
and buys the calmest handful in equal amounts. Once a month it looks again, sells any share that has
become too jumpy for the list and buys whatever calm share has taken its place. The surprising part,
first noticed in the 1970s, is that the calm shares have often earned more for the risk taken than
the wild ones, which is the opposite of the usual idea that more risk brings more reward.

## Why anyone believed it

If calm shares really do better, someone has to be on the other side, happily selling calm shares and
buying wild ones. Two kinds of buyer are usually named.

The first is an investor who wants a high return but is not allowed, or is unwilling, to borrow money
to get it. The textbook answer to "I want more return" is to borrow and buy more of a safe thing; if
borrowing is forbidden, that investor instead buys the wild shares, which move a lot on their own.
That demand pushes the wild shares up and the calm ones down. The people running a fund judged against
an index face a related temptation: owning a share that moves more than the index is a cheap way to
look like a winner when it works, so they crowd into the bouncy names.

The second is an investor who treats a share like a lottery ticket. A share that occasionally leaps
is exciting to own, and buyers will pay a bit over the odds for the thrill, the same way people pay
more for a lottery ticket than the average payout is worth. Nobody gets excited about the calm share,
so nobody overpays for it.

## An everyday comparison

Think of two fundraising raffles at a village fair. The first sells tickets for a small, steady
prize: everyone who buys gets roughly their money back in useful goods. The second sells tickets for
one enormous prize, with almost no chance of winning. It raises far more money per prize paid out,
because buyers are paying partly for the daydream, not the odds. The second raffle is selling hope at
a markup, and the calm shares in this strategy are the first raffle: fairly priced, and quietly the
better purchase for anyone counting the money rather than the daydream.

## The rules, step by step

1. Choose the shares to rank. The QuantConnect page uses large American companies: it screens out
   shares with no published financial data and any share priced below 5 dollars, keeps the 100 with
   the most dollars traded, and from those keeps the 50 with the largest total market value.
2. For each share, collect its closing price on each of the past 252 trading days, which is about one
   year.
3. Turn the prices into daily changes. For each day, take today's price divided by yesterday's price,
   and subtract one. Write the answer as a small decimal: 0.01 means one percent.
4. Work out the volatility score of the share. This is how widely those daily changes were scattered
   around their own average. A calm share has a small score; a jumpy share has a large score.
5. Rank the shares by that score, smallest first.
6. Buy the calmest ones, in equal amounts. The QuantConnect page buys the 5 calmest shares. The
   Quantpedia long-only entry divides the whole list into tenths and buys only the calmest tenth,
   which is about 50 shares when the list is the 500 companies of the main index.
7. Hold for one month. Do not look at the prices in between.
8. On the first trading day of the next month, recompute the scores and repeat from step 5. Sell any
   share that has dropped off the calmest list and buy whatever has joined it.
9. You may also sell short the wildest shares and hold the money from the sale, which is the
   long-and-short version. This tutorial describes the long-only version, which is the one the
   Quantpedia page names and the one the QuantConnect algorithm runs.

## The maths, with every symbol named

The daily change of a share:

```text
r_t = ( P_t / P_t-1 ) - 1
```

- `r_t` is the share's change on day `t`, as a decimal.
- `P_t` is the share's closing price on day `t`.
- `P_t-1` is its closing price on the day before.

The volatility score is the standard deviation of those daily changes:

```text
sigma_daily = sqrt( ( 1 / ( n - 1 ) ) * sum over t of ( r_t - r_bar ) ^ 2 )
```

- `sigma_daily` is the daily volatility score, a decimal.
- `n` is the number of daily changes used, here 252.
- `r_bar` is the average of all the daily changes.
- `sum over t` means add up the squared gaps for every day.
- `sqrt` means take the square root.

Dividing by `n - 1` rather than `n` corrects for measuring around an average computed from the same
data; with 252 days the difference is tiny, but it is the usual convention. The score can be turned
into an annual figure, because a year has about 252 trading days:

```text
sigma_year = sigma_daily * sqrt( 252 )
```

- `sigma_year` is the annualised volatility, the number usually quoted, as a decimal.
- `sqrt( 252 )` is about 15.87.

Each chosen share gets the same fraction of the money, `w_i = 1 / N`, so the portfolio's return over
the next month is the plain average of the chosen shares' returns:

```text
R_portfolio = ( R_1 + R_2 + ... + R_N ) / N
```

- `w_i` is the fraction of the account placed in chosen share `i`.
- `N` is how many shares are chosen, 5 in the QuantConnect version and about 50 in the Quantpedia
  version.
- `R_i` is the next-month return of chosen share `i`; dividing by `N` is the same as giving each share
  a weight of `1 / N` and adding.

Finally the cost of rebuilding. If a fraction `t` of the account is traded this month:

```text
Cost = t * c
```

- `t` is the traded fraction. Selling something and buying something else counts twice, so replacing
  the whole account is `t = 2.0`, and replacing one third of it is `t = 0.67`.
- `c` is the cost per trade as a fraction traded, covering the bid-ask gap plus any commission. For
  large American shares a realistic modern figure is 0.0005, that is five basis points.

## A worked example

First the score for one share, from five daily changes: +1.0 percent, -0.5 percent, +0.5 percent,
0.0 percent, -1.0 percent. As decimals these are 0.010, -0.005, 0.005, 0.000, -0.010.

```text
average = ( 0.010 - 0.005 + 0.005 + 0.000 - 0.010 ) / 5 = 0.000
squared gaps = 0.000100, 0.000025, 0.000025, 0.000000, 0.000100; their sum is 0.000250
sigma_daily = sqrt( 0.000250 / 4 ) = sqrt( 0.0000625 ) = 0.007906, that is 0.79 percent a day
sigma_year = 0.007906 * 15.87 = 0.1255, that is 12.6 percent a year
```

Now eight shares, each already scored over a year. The scores are invented but of a size that real
shares take. The calmest three are bought.

| Share | Annual volatility | Rank | Chosen |
| ----- | ----------------- | ---- | ------ |
| A     | 12.6 percent      | 1    | yes    |
| B     | 14.2 percent      | 2    | yes    |
| C     | 15.1 percent      | 3    | yes    |
| D     | 18.4 percent      | 4    | no     |
| E     | 21.0 percent      | 5    | no     |
| F     | 25.3 percent      | 6    | no     |
| G     | 31.7 percent      | 7    | no     |
| H     | 40.2 percent      | 8    | no     |

A, B and C are bought with one third of the money each. Suppose the next month brings these returns:

| Share held | Weight | Next-month return | Contribution    |
| ---------- | ------ | ----------------- | --------------- |
| A          | 0.3333 | +0.5 percent      | +0.1667 percent |
| B          | 0.3333 | +1.0 percent      | +0.3333 percent |
| C          | 0.3333 | +1.2 percent      | +0.4000 percent |
| Total      | 1.0000 |                   | +0.9000 percent |

The portfolio gained 0.9000 percent before costs. Now suppose that at the rebuild the list changes by
one name: D, which was held a month ago, has become too jumpy and is replaced by C. Then one third of
the money is sold and one third is bought, counting both sides:

```text
t = 2 * ( 1 / 3 ) = 0.667
Cost = 0.667 * 0.0005 = 0.000333, that is 0.0333 percent
Net return for the month = 0.9000 - 0.0333 = 0.8667 percent
```

Twelve months like that, compounded, is about 11.3 percent a year before costs and 10.9 percent after.
That sits close to the published figure below. Two things are worth noticing. The calm shares move
little, so the monthly numbers are small and the cost is a visible slice of them. And the worked
example says nothing about whether the strategy works; it only shows how to apply the rules and how
the arithmetic behaves.

## What the research actually found

The record is long, and the argument is about the reason, not the size.

| Source                                                           | What it measured                                                                                        | Result                                                                                                                                                                                                                          |
| ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Quantpedia, long-only low-volatility entry                       | Lowest-volatility tenth of global or US large caps, weekly volatility over three years, monthly rebuild | 11.3 percent a year, volatility 10.1 percent, reward-to-risk 0.72, over 1986 to 2006; about 50 shares for the S&P 500, confidence rated strong                                                                                  |
| QuantConnect algorithm                                           | 50 largest US companies, daily volatility over 252 days                                                 | Long the 5 calmest, equal weight, rebuilt monthly, long only                                                                                                                                                                    |
| Clarke, de Silva and Thorley                                     | Minimum-variance portfolios on the 1000 largest US shares, 1968 to 2005                                 | About a 25 percent cut in volatility while delivering comparable or higher average returns than the market index                                                                                                                |
| Baker, Bradley and Wurgler                                       | High-volatility versus low-volatility shares in the US                                                  | Over the 41 years they study, the high-volatility and high-beta shares substantially underperformed the low-volatility and low-beta shares                                                                                      |
| Stefano, Lamperiere, Bevaratos, Simon, Laloux, Potters, Bouchaud | Deconstructing the low-volatility result across nine countries                                          | Low-volatility and low-beta profits move together (correlation about 0.9, one effect); a large share of the return comes from dividends, and once value and profitability are controlled the leftover return is not significant |

The disagreement is real and it is about the cause. One camp, associated with Frazzini and Pedersen
and with the leverage-constraint argument, says the low-volatility effect is compensation for a hidden
risk that most investors cannot reach because they cannot borrow: the effect is a risk premium, so it
should persist. A second camp says it is mispricing, driven by lottery-seeking buyers and by funds
hugging their benchmark, so it should shrink as more money learns about it. A third says it is not a
separate effect at all: the deconstruction above finds the return concentrated in dividends, correlated
with value and earnings measures, and largely explained by profitability, so a low-volatility strategy
is mostly a repackaged value strategy. On top of that, the deconstruction finds the skew slightly
positive, which argues against the risk-premium reading, because a risk premium should come with a
downside. Nobody has settled this.

## How this project relates to it

This repository implements no low-volatility strategy and no minimum-variance optimiser, so there is
nothing here that trades the idea directly. The closest things are the briefs.

[Volatility, jumps and microstructure noise](../../../strategies/books2/14_volatility_and_microstructure_noise.md)
is the brief on how volatility is measured and forecast; it is where a reader would look for why a
calm-share score is a stable statistic at all.

[Portfolio construction and allocation](../../../strategies/books2/10_portfolio_and_allocation.md)
covers the minimum-variance and diversification machinery a low-volatility book needs, and carries two
numbers that bear on the figures above: bookkeeping choices can inflate the S&P 500 Sharpe ratio by
nearly 30 percent (`2405.10920v1`, p.19), and a hundred-asset universe needs 94 components for 95
percent of its variance under a regime-switching model, far more than the handful of names a
concentrated calm-share portfolio holds (`2204.13398v1`, pp.15-16).

[Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md)
is the brief on how much of a published return survives. Its central number is that replicated
predictors lose about 26 percent of their in-sample return once the sample ends (`2209.13623v3`,
reported at p.7 of the brief), which is the correction any reader should apply to the 11.3 percent
above.

## Where it goes wrong

- The reason may not be a reason. If the effect is a repackaged value or dividend tilt, then owning it
  is not a separate bet and it will behave like a value book, including the long stretches when value
  does badly.
- Crowding changes the price of calm shares. Low-risk funds became popular, and the popularity moves
  the very valuation that made the shares cheap. The Quantpedia page itself warns that a high price
  for low-volatility stocks can hurt them during market stress.
- Costs on a monthly rebuild are the wrong shape of cost. The calmest list is fairly stable, so
  turnover is low, which helps. But the strategy holds shares, and every share has a bid-ask gap, so a
  monthly rebuild still pays a real toll on a small monthly return.
- The measurement can be gamed. Volatility is estimated from history, and a share that had a quiet
  year can become wild the next month. A calm score is a forecast, and forecasts of risk are wrong
  exactly when it matters, in a crisis.
- The horizon is a choice. Weekly volatility over three years, used by Quantpedia, and daily
  volatility over one year, used by QuantConnect, are different measurements and will pick partly
  different shares. Choosing between them after seeing the result is the classic way a backtest is
  made to look better than it is.

## Try it yourself

You need a spreadsheet and a public source of daily closes for any eight large shares.

1. Make one column per share and one row per trading day, for the last 252 trading days.
2. Add a return row for each day after the first: today's price divided by yesterday's price, minus
   one.
3. Below each return column, compute the average, then each return's squared gap from that average,
   then the sum of those squared gaps divided by 251, then the square root. That is the daily
   volatility.
4. Multiply the daily volatility by 15.87 to get the annual figure. Write the eight annual figures in
   one column.
5. Sort the eight shares by that column, smallest first. Write down the calmest three. That is the
   list the rules would have bought on that day.
6. Next month, repeat and compare the two lists. Mark which names changed.

What to notice: the calmest three are usually the dull, well-known businesses, and they change slowly,
so the monthly cost is small. Also notice how narrowly the shares are separated: two shares scored 12.6
and 14.2 percent are not really different kinds of thing, and which one lands in the top three on a
given day is close to a coin toss. That fragility is what the strategy is built on.

## Where this came from

- [QuantConnect strategy library: volatility effect in stocks](https://www.quantconnect.com/tutorials/strategy-library/volatility-effect-in-stocks),
  the rules as implemented: 50 largest US companies, daily volatility over 252 days, long the 5
  calmest, rebuilt monthly.
- [Quantpedia: low volatility factor effect in stocks](https://quantpedia.com/strategies/low-volatility-factor-effect-in-stocks),
  the long-only version the QuantConnect page names, with the decile construction, the instrument
  count and the 1986 to 2006 performance figures.
- [Quantpedia: Deconstructing the Low-Volatility Anomaly](https://quantpedia.com/deconstructing-the-low-volatility-anomaly/),
  the summary of Stefano and others, which is where the dividend, value and profitability findings
  come from.
- Baker, Bradley and Wurgler, [Benchmarks as Limits to Arbitrage: Understanding the Low-Volatility Anomaly](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1585031),
  the benchmark-hugging and leverage-constraint account.
- [Volatility, jumps and microstructure noise](../../../strategies/books2/14_volatility_and_microstructure_noise.md),
  [Portfolio construction and allocation](../../../strategies/books2/10_portfolio_and_allocation.md)
  and [Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's own briefs, which is where the net-return and replication corrections above live.
- The arXiv identifiers cited above, `2405.10920v1`, `2204.13398v1` and `2209.13623v3`, are held in
  the local corpus and were read through those briefs.

## Words used in this tutorial

- basis point: one hundredth of one percent, so five basis points is 0.05 percent.
- minimum-variance portfolio: the mix of shares, given their history of moving together, that has the
  smallest possible overall bounce.
- risk premium: the extra return an investor is paid for accepting a risk they would rather avoid.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- skew: whether the surprisingly large outcomes tend to be on the winning side or the losing side.
- standard deviation: a measure of how widely a set of numbers is scattered around its own average.
- volatility: how much a price moves around its average, usually quoted as a percentage a year.
- weight: the share of your money placed in one holding.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
