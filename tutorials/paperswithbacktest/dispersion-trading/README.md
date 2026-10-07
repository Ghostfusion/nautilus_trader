# Dispersion trading: betting on how far apart share prices move, not which way they move

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                           |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Options (contracts that give the right to buy or sell at a set price) on the shares of one hundred large American companies, and options on the index that bundles those same companies                                         |
| How often it trades       | About once a month, each time the options held are close to expiring                                                                                                                                                            |
| What you need             | Python and a data file of option prices                                                                                                                                                                                         |
| Where the rules come from | [Quantpedia, dispersion trading](https://quantpedia.com/strategies/dispersion-trading/)                                                                                                                                         |
| The underlying research   | Buraschi, Trojani and Vedolin, [Equilibrium Index and Single-Stock Volatility Risk Premia](https://www.academia.edu/16327015/EQUILIBRIUM_INDEX_AND_SINGLE_STOCK_VOLATILITY_RISK_PREMIA)                                         |
| How well it held up       | Mixed: rests on one American sample, 1996 to 2007, which reported 15.4 percent a year after costs and a Sharpe ratio of 0.82 but also a 43 percent fall, and on the trade losing heavily whenever shares suddenly move together |
| Also appears in           | [Options and derivative instruments](../../../strategies/books/16_options_and_derivative_instruments.md) in this collection                                                                                                     |

## The idea in one paragraph

An index is simply a bundle of shares, and a bundle moves less than its average member, because on any day some
members rise while others fall and the two partly cancel. The price of an option on the index therefore contains a
hidden forecast of how strongly the members will move together, a quantity called correlation. This strategy sells
index options and buys options on the individual members, in roughly equal amounts. If the members turn out to move
independently, the index stays calm and the member options pay off, so the strategy gains. If the members all fall
together, both legs move the wrong way at once and the strategy loses. The trade is about the spread between
members, not about whether the market rises or falls.

## Why anyone believed it

Selling an index option is selling insurance against a market-wide fall. Many institutions want that insurance, and
they pay for it, so the seller is paid a premium that is larger than the losses suggest on an ordinary day. The same
is not equally true for options on a single company: those are bought and sold by people with particular views on
that company, and the insurance premium is thinner. The difference is the prize.

Buraschi, Trojani and Vedolin model the gap and trace it to a separate price for correlation itself. The market pays
a premium for protection against everyone falling at once, and correlation rises exactly then. The counterparty is
the pension fund or insurer that must hold index protection, and the trader who buys single-company options because
they want a view on one company. Both keep doing it, one because rules require the hedge, the other because that is
their natural habitat.

## An everyday comparison

A fruit shop sells a mixed box of a dozen different fruits, and it also sells each fruit loose. The box is priced
from a forecast of how much fruit prices will move together: in a good harvest everything is cheap, so the box is
calm, but in a bad year everything is dear and the box swings. If you believe the weather will be mixed, so that
some fruit is cheap while other fruit is dear, you would sell insurance on the box and buy insurance on the loose
fruit. You profit when the fruits disagree, and you are badly hurt by the one bad year in which every fruit fails
together.

## The rules, step by step

1. Choose the index and its members. The implementation uses the one hundred largest and most heavily traded
   American companies, and the index that holds those same companies.
2. On one day a month, for every member company, buy one put option, which is a contract that gains value when that
   company's share price falls.
3. Buy the put whose strike price is closest to today's share price. The strike price is the level at which the
   contract pays off.
4. Pick a contract that expires in twenty to sixty days from today.
5. At the same time, sell one put option on the index itself, also at the strike closest to today's index level and
   with twenty to sixty days to expiry. Selling means you receive money now and owe a payment later if the index
   falls.
6. Give each member company's option the same amount of money, and give the single index option an amount equal to
   the whole book. In the implementation the index leg and the member leg are sized so that the money at risk on
   each side is comparable.
7. Only use companies whose price is above five dollars and for which the data are complete, because a company with
   a broken price series cannot be hedged.
8. Hold the book until two days before the index option expires, then close everything and rebuild from step 2. In
   practice that is once a month.
9. The academic version adds a filter: rank the member companies by how far apart professional analysts' earnings
   forecasts are, and buy the options of the companies with the widest disagreement. The implementation drops this
   step because it does not have the forecast data, and simply uses the hundred most liquid companies.

The word for the whole trade is delta hedging: the index option is sold, so its value falls as the index falls, and
the position must be adjusted along the way so that the book does not simply become a bet on the market's direction.

## The maths, with every symbol named

The index's volatility, written as a yearly percentage, is roughly the average member's volatility multiplied by the
square root of the average correlation between members:

```text
sigma_index = sigma_average * sqrt(rho)
```

- `sigma_index` is the volatility of the index, meaning the size of its typical yearly swing around its average.
- `sigma_average` is the average volatility of the member companies.
- `rho` is the average correlation, a number between -1 and 1 that says how much two members tend to move together.
  A value of 1 means they move in perfect step; 0 means they are unrelated.
- `sqrt(rho)` is the square root of that number.

Rearranged, this is how the option market's hidden correlation forecast is read off:

```text
rho_implied = (sigma_index_implied / sigma_member_implied) squared
```

- `rho_implied` is the correlation the option prices imply.
- `sigma_index_implied` is the volatility that index option prices imply.
- `sigma_member_implied` is the average volatility that member option prices imply.
- The right-hand side is that ratio multiplied by itself.

If option prices imply a correlation of 0.55 and the shares actually move with a correlation of 0.48, the trade of
selling the index option and buying the member options gains, because the index was priced as calmer or wilder than
reality delivered. Sizing the book so that each one-point (0.01) miss is worth a fixed number of dollars turns that
into a profit:

```text
profit = k * (rho_implied - rho_realized) - cost
```

- `profit` is the gain in dollars over the month.
- `k` is the dollars gained or lost for each one-point miss between implied and realized correlation.
- `rho_implied` and `rho_realized` are the correlation the options implied and the correlation that happened.
- `cost` is the money lost to the gap between buying and selling prices on both legs, plus any commission.

## A worked example

The account holds 200,000 dollars. The book is sized so that one point of correlation (0.01) is worth 1,000
dollars, and rebuilding the options each month costs 400 dollars. The correlations below are invented but of the
size that equity correlations actually take: calm in normal months, and shooting above 0.80 in a panic.

| Month | Implied correlation | Realized correlation | Gap, in points | Gross profit | Rebuild cost | Net profit | Running total |
| ----- | ------------------- | -------------------- | -------------- | ------------ | ------------ | ---------- | ------------- |
| 1     | 0.55                | 0.48                 | +7             | +7,000       | 400          | +6,600     | +6,600        |
| 2     | 0.50                | 0.46                 | +4             | +4,000       | 400          | +3,600     | +10,200       |
| 3     | 0.60                | 0.58                 | +2             | +2,000       | 400          | +1,600     | +11,800       |
| 4     | 0.45                | 0.30                 | +15            | +15,000      | 400          | +14,600    | +26,400       |
| 5     | 0.52                | 0.85                 | -33            | -33,000      | 400          | -33,400    | -7,000        |
| 6     | 0.80                | 0.62                 | +18            | +18,000      | 400          | +17,600    | +10,600       |
| 7     | 0.62                | 0.55                 | +7             | +7,000       | 400          | +6,600     | +17,200       |
| 8     | 0.58                | 0.51                 | +7             | +7,000       | 400          | +6,600     | +23,800       |

After eight months the account is up 23,800 dollars, which is 11.9 percent of the 200,000 dollars it started with.
That pace is close to the published figure of about 15 percent a year. The important row is month 5. One month in
which every share fell together wiped out almost half of the gain built over the previous four months, and the fall
from the peak of 26,400 dollars to minus 7,000 dollars is a drop of 12.6 percent of the account at that moment. The
published worst fall for the full sample is 43 percent, because worse episodes occurred. Note also that the costs
inside this example are small; a real book pays the gap between buying and selling prices on dozens of options, and
those gaps are wider for options than for shares.

## What the research actually found

The source paper is a model, not a backtest: Buraschi, Trojani and Vedolin build an economy with many assets and
show that the insurance premium on the index exceeds the premium on single companies, and that the gap is mainly
the price of correlation risk. The measurement of the trade itself comes from Quantpedia's summary of the paper and
its implementation.

| Source                                                | What it measured                                                       | Result                                                                                                                                             |
| ----------------------------------------------------- | ---------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| Quantpedia, dispersion trading                        | Selling index puts and buying member puts, American data, 1996 to 2007 | 15.39 percent a year after transaction costs, an annualized volatility of 13.86 percent, a worst fall of 43.49 percent, and a Sharpe ratio of 0.82 |
| The same page, quoted description                     | Whether the trade hedges a falling market                              | No: it is a fat-tail strategy that loses heavily in crises, when correlations jump                                                                 |
| Buraschi, Trojani and Vedolin                         | A structural model of index and single-stock option premia             | Index option writers earn high returns and single-stock writers earn lower ones; the wedge is a correlation risk premium, and it is large          |
| Driessen, Maenhout and Vilkov, cited by the same page | Option-implied correlation and the price of correlation risk           | The risk of correlation changes is priced, so correlation itself is a separate thing to buy and sell                                               |

The list that carries this strategy does not publish its own Sharpe ratio for it in the current table of results;
its general replication record, which is the vendor's own measurement, is a median Sharpe ratio of 0.37 across the
papers it has coded, with 48 percent clearing a t-statistic of 1.96. Against that backdrop the 0.82 here is
respectable, but it rests on a single sample and a decade that contained two enormous correlation spikes.

## How this project relates to it

The repository's own research on options is collected in
[Options and derivative instruments](../../../strategies/books/16_options_and_derivative_instruments.md), which
surveys index option portfolios and shows that how often a hedged option book is readjusted is a first-order lever
on its results. That is the same practical point this strategy turns on: a dispersion book that is left alone is a
direction bet, and one that is adjusted often pays more in trading.

The other relevant brief is
[Options and derivatives in the arXiv harvest](../../../strategies/books2/12_options_and_derivatives.md). It
documents that index option implied volatility reacts to jumps and then keeps drifting for about an hour, and that
an option model can match prices while still disagreeing about the variance the market expects. Both are warnings
that the clean correlation arithmetic above is only an approximation of what an option book actually does.

## Where it goes wrong

- Correlation spikes in a crash. The whole gain depends on members disagreeing, and in a panic they stop
  disagreeing. Every share falls at once, the index option sold moves against the book, and the loss arrives exactly
  when other holdings are also falling. The published 43 percent fall is this risk.
- It is not a hedge. The quoted description is explicit: this is not protection against a falling market, and it is
  a fat-tail strategy that loses heavily in crises.
- Costs are large and hard to measure. The book holds dozens of options, each with a gap between buying and selling
  price. The strategy is classified as very complex to implement, and small errors in the model used to size the
  legs turn it into a directional bet.
- The evidence is one sample. The American figures cover 1996 to 2007, before the 2008 crisis, and no independent
  replication appears in the sources used here.
- A model-dependent reading. The correlation read off the option prices depends on the model used to convert prices
  into volatilities. If that model is wrong, the measured gap is an artefact.
- Analyst-disagreement filter. The academic rule sorts by how far apart analysts' forecasts are, and the
  implementation cannot, because the data are not available. The two are therefore not the same trade.

## Try it yourself

You need a spreadsheet and a public source of index option prices, such as the CBOE's published volatility index,
which is itself an average of implied volatilities.

1. Build a sheet with one row per month for the last five years, and columns named Month, Index implied volatility,
   Average member implied volatility, Implied correlation, Index realized volatility, Average member realized
   volatility, Realized correlation.
2. In the Implied correlation cell, put the index implied volatility divided by the average member implied
   volatility, then square the result.
3. For realized volatility, compute the standard deviation of that month's daily returns and multiply by the square
   root of the number of trading days in the year (about 252). Do it once for the index and once for a basket of
   members.
4. In the Realized correlation cell, repeat the same squaring of the ratio.
5. Add a column that subtracts the realized correlation from the implied correlation.
6. Sort the sheet by that final column, worst first.

What to notice: the misses are mostly small, but the largest negative values cluster in a handful of months when
the market fell hard. Those months are exactly the ones that decide whether the strategy keeps its gains. If your
sheet shows the implied correlation permanently above the realized one by a large amount, check whether your sample
starts after a crash, which biases the answer.

## Where this came from

- [Quantpedia, dispersion trading](https://quantpedia.com/strategies/dispersion-trading/), the rules, the
  performance figures and the description of where the trade loses.
- The implementation file the list carries, `static/strategies/dispersion-trading.py`, whose header states the
  universe, the monthly rebuild and the analyst-disagreement step.
- Buraschi, Trojani and Vedolin, [Equilibrium Index and Single-Stock Volatility Risk Premia](https://www.academia.edu/16327015/EQUILIBRIUM_INDEX_AND_SINGLE_STOCK_VOLATILITY_RISK_PREMIA),
  the paper that explains why index and single-stock option premia differ.
- Driessen, Maenhout and Vilkov, [Option-Implied Correlations and the Price of Correlation Risk](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2359380),
  the companion work on correlation as a priced risk.
- [Options and derivative instruments](../../../strategies/books/16_options_and_derivative_instruments.md) and
  [Options and derivatives](../../../strategies/books2/12_options_and_derivatives.md), this repository's two
  surveys of the option literature.

## Words used in this tutorial

- correlation: a number between -1 and 1 saying how much two things move together; 1 is perfect company, 0 is no
  relationship, -1 is perfect opposition.
- delta hedge: adjusting a sold option position as the price moves so the book is not simply a bet on direction.
- implied volatility: the size of price swing that an option's price implies, as a yearly percentage.
- option: a contract giving the right, but not the obligation, to buy or sell at a set price before a set date.
- premium: the price paid or received for an option.
- put option: an option that gains value when the thing it refers to falls in price.
- Sharpe ratio: a reward-per-risk measure; the average return divided by how much the return wobbles.
- strike price: the level at which an option starts to pay off.
- volatility: the size of a price's typical swing around its average, usually quoted as a yearly percentage.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
