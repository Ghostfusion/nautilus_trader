# The rebalancing premium in cryptocurrencies: the gain from resetting a basket back to equal weights

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                        |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | A basket of twenty-seven cryptocurrencies held in equal dollar amounts and reset every day, plus a short position in the same basket left untouched to drift with the market                                                                                                 |
| How often it trades       | The equal-weight basket is topped up daily; the drifting basket is sold short once and left alone                                                                                                                                                                            |
| What you need             | Python and a data file of daily prices for the twenty-seven coins                                                                                                                                                                                                            |
| Where the rules come from | [Quantpedia, rebalancing premium in cryptocurrencies](https://quantpedia.com/strategies/rebalancing-premium-in-cryptocurrencies/)                                                                                                                                            |
| The underlying research   | Hanicova and Vojtko, [Rebalancing Premium in Cryptocurrencies](https://ssrn.com/abstract=3982120), building on Willenbrock, [Diversification Return, Portfolio Rebalancing, and the Commodity Return Puzzle](https://ssrn.com/abstract=1898864)                              |
| How well it held up       | Weak: rests on one study over 2018 to 2021 that reported 7.65 percent a year with a Sharpe ratio of 2.93, on a three-year sample that ended at the top of a crypto bull market, and on the same table showing a worst-case fall of 99.99 percent, which is the whole account |
| Also appears in           | [Portfolio and allocation](../../../strategies/books2/10_portfolio_and_allocation.md) in this collection                                                                                                                                                                     |

## The idea in one paragraph

Suppose you spread your money equally across twenty-seven cryptocurrencies and then never touch the portfolio.
After a year the coins that rose are a much larger share of the money, and the ones that fell are a much smaller
share, so the portfolio has quietly become a bet on last year's winners. If instead you reset the portfolio back
to equal amounts every day, you sell a little of whatever rose and buy a little of whatever fell, every day. Over
time the reset portfolio and the untouched portfolio hold the same coins but the reset one grows a little faster,
as long as the coins move around a lot and do not all move together. This strategy holds the reset basket and
sells short seventy percent of the untouched basket, in an attempt to collect that difference.

## Why anyone believed it

Mathematics gives the reason. A portfolio's growth is reduced by its own wobble, and a basket of many coins that do
not move together wobbles less than the average member. Resetting the weights keeps the basket at the low-wobble
mix; leaving it alone lets the highest-wobble coin take over and raises the portfolio's wobble. The gap between the
two is sometimes called the diversification return, or the rebalancing premium.

The counterparty is the buy-and-hold investor who lets winners run. In a market that rises for years, that investor
looks smarter, and the resetting portfolio looks timid because it keeps trimming the best performers. The resetting
investor is effectively selling the chance of a very large gain in exchange for a steadier, compounding result, and
the difference is the premium. It is only a premium at all when the assets are volatile and not tightly correlated,
because those are the conditions under which the untouched basket's wobble rises the most.

## An everyday comparison

A gardener plants four crops in equal-sized beds and never moves them. One crop thrives, so by autumn three
quarters of the garden is that one crop, and the following year the whole harvest depends on whether that one crop
does well again. A second gardener walks the same four crops back to equal beds each spring, digging out some of
the thriving crop and planting more of the struggling one. If the crops' fortunes vary independently from year to
year, the second gardener's total harvest is steadier and slightly larger over time. If instead one crop improves
every single year forever, the first gardener wins, and the second gardener's digging looks like a mistake.

## The rules, step by step

1. Choose the basket. The study uses twenty-seven cryptocurrencies; the implementation trades twenty-six because one
   coin's data were broken.
2. Buy the same dollar amount of each coin, so that each is an equal fraction of the basket. With twenty-seven
   coins each gets about 3.7 percent of the money.
3. Leave that basket alone. Do not top it up. This is the drifting portfolio.
4. Separately, sell short an amount equal to seventy percent of the value of the drifting basket, spread across the
   same coins in the same proportions. Selling short means borrowing something you do not own, selling it now, and
   buying it back later.
5. Once a day, at a fixed clock time, sell enough of every holding in the long basket that rose and buy enough of
   every holding that fell to bring the long basket back to equal dollar amounts.
6. Hold the short position as a fixed quantity; do not resell it to keep its weight constant. The implementation
   opens it once and adjusts only the long basket.
7. Repeat the daily reset until you choose to stop. There is no monthly or annual decision; the only clock is
   daily.

Two details matter. First, the daily reset is what earns the premium; a reset every year earns much less, because
the weights have time to drift far from equal. Second, in the implementation the daily reset is done with small
orders, and each order is skipped if it is below the exchange's minimum size, which for a small account means the
basket may not be reset exactly.

## The maths, with every symbol named

A portfolio's growth per period is approximately its average holding return minus half its variance:

```text
growth = average_return - (variance / 2)
```

- `growth` is the compounded growth rate per period.
- `average_return` is the arithmetic average of the period returns.
- `variance` is the square of the standard deviation, so it is a measure of wobble in squared units.
- The half of the variance that is subtracted is the cost of the wobble, and it is why a wilder portfolio needs a
  higher average return just to keep up.

For a basket of `N` coins, each with the same volatility and an average correlation `rho`, the basket's own
volatility is much smaller than a single coin's:

```text
basket_volatility = coin_volatility * square root of ( 1/N + (1 - 1/N) * rho )
```

- `basket_volatility` is the wobble of the equally weighted basket.
- `coin_volatility` is the wobble of one coin.
- `N` is the number of coins, twenty-seven here.
- `rho` is the average correlation between coins, a number from -1 to 1.

The rebalancing premium follows directly, as half the amount by which the basket's variance is smaller than the
average coin's variance:

```text
premium_per_period = ( coin_volatility squared / 2 ) * (1 - 1/N) * (1 - rho)
```

- `premium_per_period` is the extra growth the reset basket earns over the untouched one, per period.
- The term `(1 - 1/N)` is close to 1 when there are many coins.
- The term `(1 - rho)` is close to 1 when the coins are nearly unrelated and close to 0 when they move together.
- The formula says the premium is larger when the coins are volatile and when their moves are unrelated, and it
  shrinks to nothing when the coins are either calm or perfectly correlated.

## A worked example

Three coins, called A, B and C, each starting at 100 dollars, so the basket begins at 300. One basket is reset back
to equal amounts at the end of every month; the other is left alone. The returns are invented but of the size that
crypto returns actually take.

| Month | Coin A return | Coin B return | Coin C return | Reset basket value | Reset: each coin | Drifting basket value |
| ----- | ------------- | ------------- | ------------- | ------------------ | ---------------- | --------------------- |
| 0     |               |               |               | 300.00             | 100.00           | 300.00                |
| 1     | +60 percent   | -20 percent   | +10 percent   | 350.00             | 116.67           | 350.00                |
| 2     | -30 percent   | +40 percent   | 0 percent     | 361.67             | 120.56           | 334.00                |
| 3     | +50 percent   | -10 percent   | +20 percent   | 434.00             | 144.67           | 400.80                |
| 4     | -40 percent   | +60 percent   | +30 percent   | 506.35             | 168.78           | 433.68                |
| 5     | +20 percent   | +20 percent   | +20 percent   | 607.62             | 202.54           | 520.42                |

After five months the reset basket is worth 607.62 dollars and the untouched basket 520.42, both from the same 300
dollars and the same five sets of returns. The reset basket gained 102.5 percent, the untouched basket 73.5
percent, and the difference is the premium in this example. Now the strategy itself holds the reset basket long and
sells short seventy percent of the untouched basket. Its profit each month is the change in the reset basket minus
0.7 times the change in the drifting basket:

| Month | Change in reset basket | Change in drifting basket | Short leg, 0.7 times | Strategy profit |
| ----- | ---------------------- | ------------------------- | -------------------- | --------------- |
| 1     | +50.00                 | +50.00                    | -35.00               | +15.00          |
| 2     | +11.67                 | -16.00                    | +11.20               | +22.87          |
| 3     | +72.33                 | +66.80                    | -46.76               | +25.57          |
| 4     | +72.35                 | +32.88                    | -23.02               | +49.33          |
| 5     | +101.27                | +86.74                    | -60.72               | +40.55          |
| Total | +307.62                | +220.42                   | -154.30              | +153.32         |

The strategy profit is +153.32 dollars on a starting account of 300, which is 51 percent over five months, and the
point to notice is the second column and the fourth column: in every month of a rising market the short leg lost
money, and only the fact that the reset basket rose faster kept the whole thing profitable. The costs are the daily
reset, which traded about 0.97 times the value of the long basket across the five resets, and the fee for
borrowing the coins that were sold short. At ten basis points a side, the reset costs under 0.1 percent of the
starting account, and a monthly borrow or funding fee of 0.035 percent on seventy percent of the basket adds about
0.18 percent over the five months. Together that is under 0.3 percent, small against this example's gain, but
perpetual funding fees in crypto can be far higher than 0.035 percent a month, and they are charged on the short
leg every day.

Now the opposite case, in one line of arithmetic. Keep the same three coins but let A rise 50 percent every month
while B and C stay flat. After four months the untouched basket is worth 706.25 dollars and the reset basket only
555.80, so the short leg loses and the strategy makes 555.80 minus 300, minus 0.7 times 706.25 minus 300, which is
a loss of 28.20 dollars. A single coin that keeps winning turns the whole idea upside down.

## What the research actually found

| Source                                                       | What it measured                                                                                                                   | Result                                                                                                                                                                                     |
| ------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Hanicova and Vojtko, Rebalancing Premium in Cryptocurrencies | A daily-reset basket of twenty-seven coins against an untouched one, crypto data around 2018 to 2021, short leg of seventy percent | 7.65 percent a year, an annualized volatility of 2.62 percent, a Sharpe ratio of 2.93, and a worst-case fall of 99.99 percent, all assuming that only one tenth of the account is invested |
| The same study, simulation section                           | Thirty simulated coin series, each with a zero average daily return and a 7.5 percent daily volatility                             | The reset basket beat the untouched one on cumulative return, wobble and worst fall, which is the cleanest demonstration because the average coin return was set to zero on purpose        |
| The same study, conclusion                                   | Whether the premium can be relied on                                                                                               | It depends on no single coin consistently outperforming the others; if one does, resetting is not profitable, which is exactly the failure case shown above                                |
| Willenbrock, Diversification Return                          | The same premium in commodity and other portfolios                                                                                 | A portfolio that is periodically reset to equal weights earns an extra return equal to half its variance reduction, and a buy-and-hold portfolio earns none of it                          |

The list that carries this strategy does not publish its own Sharpe ratio for it. Its general replication record,
which is the vendor's own measurement, is a median Sharpe ratio of 0.37 across the papers it has coded, with 48
percent clearing a t-statistic of 1.96 and a median test window of 34 years. Against that, a Sharpe ratio of 2.93
from a three-year sample is not comparable: the sample is short, it covers a period when the coins moved a great
deal, and the leverage in the short leg turns the numbers extreme in both directions.

## How this project relates to it

The repository's survey of this area is
[Portfolio and allocation](../../../strategies/books2/10_portfolio_and_allocation.md). It reports a directly
relevant experiment: holding the same shares and the same starting weights but changing only the rule that keeps
the weights equal changed which factor model looked best, and in some cells the pricing errors reached 490 basis
points. The lesson is the one this strategy turns on: the rebalancing rule is not a detail, it is part of the
strategy, and it changes the answer.

The same brief also carries a warning about the measurement. It reports that a covariance matrix which is merely
badly behaved can rotate the portfolio weights away from what the returns forecast, and that regime changes mean
ninety-four of one hundred assets are needed to capture ninety-five percent of the variance. A basket of
twenty-seven highly volatile coins is small by that standard, so the "many assets" assumption behind the premium
is weaker than the formula suggests.

## Where it goes wrong

- A trending coin destroys it. The premium exists only while no single coin permanently leads. The moment one does,
  the reset basket keeps trimming its best performer and the short leg is short exactly the thing that is rising.
  That is where a 99.99 percent fall comes from.
- The short leg is the dangerous half. Selling short seventy percent of a market that can rise several hundred
  percent in a year means the loss is not bounded, and the borrow or funding fee is charged every day. The paper
  itself notes that funding exceeding the premium reverses the trade.
- The sample is three years. The published window ends in 2021, after a spectacular crypto bull market. A premium
  measured in a period when all the coins rose may be nothing more than the effect of that rise.
- Costs scale with the coin count. Resetting twenty-seven coins daily means many small orders, each with a
  bid-offer gap, and the exchange minimum order sizes can prevent small accounts from resetting at all.
- The formula assumes the coins are identical in volatility and equally correlated. Real coins differ wildly, and
  a basket dominated by a handful of volatile ones behaves differently from the clean formula.
- It is not free money. The reset basket and the untouched basket hold the same coins; the difference is a
  reallocation, and it is paid for by selling the chance of a large gain.

## Try it yourself

You need a spreadsheet and daily prices for four or five cryptocurrencies over the last two years; any finance
website will provide them.

1. Build a sheet with one column per coin and one row per day, starting with the same dollar amount in each.
2. Add a second set of columns for a reset basket, and at the end of each row reset all of them back to the same
   dollar amount.
3. Leave the first set of columns untouched, so that its weights drift.
4. Carry both baskets down for two years and, in the final row, compare their values and their worst falls from a
   peak.
5. Add a column computing, for each day, how much of the reset basket had to be traded to bring it back to equal
   weights.
6. Multiply that daily traded amount by a cost of about 0.1 percent to see the total cost.

What to notice: in a year when every coin rose together, the untouched basket often wins, because the strongest
coin kept compounding. The reset basket wins more often when the coins take turns leading. If your two-year test
shows the reset basket winning by a wide margin, check whether you reset it using the day's closing price while
buying at that same closing price, which is not possible in practice and flatters the result.

## Where this came from

- [Quantpedia, rebalancing premium in cryptocurrencies](https://quantpedia.com/strategies/rebalancing-premium-in-cryptocurrencies/),
  the rules, the performance figures and the reference to Willenbrock.
- The implementation file the list carries,
  `static/strategies/rebalancing-premium-in-cryptocurrencies.py`, whose header states the twenty-seven coins, the
  daily reset, the seventy percent short side and the exchange used.
- Hanicova and Vojtko, [Rebalancing Premium in Cryptocurrencies](https://ssrn.com/abstract=3982120), the study
  behind the rules.
- Willenbrock, [Diversification Return, Portfolio Rebalancing, and the Commodity Return Puzzle](https://ssrn.com/abstract=1898864),
  the earlier paper that defined the premium this strategy tries to collect.
- [Portfolio and allocation](../../../strategies/books2/10_portfolio_and_allocation.md), this repository's survey
  of how weights are built and how fragile the inputs are.

## Words used in this tutorial

- borrow fee: the charge for borrowing something you have sold short, paid while the position is open.
- correlation: a number between -1 and 1 saying how much two things move together; 1 is perfect company, 0 is no
  relationship.
- diversification return: the extra compounded growth a portfolio earns from holding many imperfectly related
  assets and resetting them to equal weights.
- geometric return: the compounded growth rate, which is lower than the simple average when returns wobble.
- short selling: borrowing something you do not own, selling it now, and buying it back later.
- variance: the square of the standard deviation, a measure of wobble in squared units.
- volatility: the size of a price's typical swing around its average, usually quoted as a yearly percentage.
- weight: the share of your money placed in one holding.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
