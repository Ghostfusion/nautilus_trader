# Commodity currencies and macro factors: why the Australian dollar follows iron ore

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                              |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | The currencies of commodity-producing countries against the dollar - the Australian, New Zealand and Canadian dollars - plus gold, and, as signals only, a share index fund and a government bond fund             |
| How often it trades       | Every twenty-one trading days, when the factor scores are recomputed                                                                                                                                               |
| What you need             | Python and a data file, and for one file a downloaded positioning report                                                                                                                                           |
| Where the rules come from | [The compendium's commodity and currency article](https://backtrader.readthedocs.io/en/latest/strategies-series/en/13-commodity-currency.html)                                                                     |
| The underlying research   | Chen, Rogoff and Rossi, [Can Exchange Rates Forecast Commodity Prices?](https://www.nber.org/papers/w13901), and Erb and Harvey, [The Golden Dilemma](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2078535) |
| How well it held up       | Weak: the three representative files produced flat or modest results over fifteen to twenty years, one losing money, each on a single narrow dataset                                                               |
| Also appears in           | [Risk premia in forex markets](../../quantconnect/risk-premia-in-forex-markets/README.md) and [Can crude oil predict equity returns](../../quantconnect/can-crude-oil-predict-equity-returns/README.md)            |

## The idea in one paragraph

Some countries sell a lot of one raw material to the world. Australia sells iron ore and coal, New
Zealand sells dairy and wool, Canada sells oil. When the world price of that material rises, those
countries earn more, their economies strengthen, and their currencies tend to rise against the
dollar. Commodity-currency trading turns that chain into a rule: watch the things that drive the
raw-material price - economic growth, interest rates, who is already positioned in the market - and
buy the currency of the country that stands to benefit. Gold gets the same treatment from the other
side, because gold pays no interest, so the cost of holding it is the interest given up, and that
cost moves with real rates. These rules look at the outside world more than at the currency's own
chart.

## Why anyone believed it

A currency's value is tied to what its country sells. If the world wants more iron ore, Australia's
exporters earn more foreign money, which they convert into Australian dollars to pay wages and taxes,
and the currency rises. The counterparty is anyone trading for a real reason rather than a forecast:
an exporter converting its receipts, an importer paying a bill, a central bank managing reserves, a
producer hedging next year's crop. Those flows are large, they have a direction, and they can arrive
with a lag after the raw-material price moves, which gives an observer of the chain a small head
start. Add the interest-rate channel and the story is one line: rates decide carry, carry decides
flows, flows decide prices.

## An everyday comparison

Think of a village whose livelihood depends on the price of the wool it sells to a distant town.
Nobody in the village sets that price; it depends on the town's demand, the weather, and how much
wool is already in the town's warehouses. But the village learns the price within a day or two, and
the shopkeepers and money-changers adjust over the following weeks. A trader watching the town's
warehouse reports and interest rates can guess which way the village will move before it has finished
adjusting.

The positioning report deserves its own explanation. Every Friday the American regulator, the
Commodity Futures Trading Commission, publishes a report saying how much of each futures market each
group of participants holds: the commercial hedgers who make or use the thing, the financial
speculators betting on the price, and the small traders. A futures contract is an agreement to buy or
sell something at a fixed price on a future date. The report exists because the regulator has
collected it since the early twentieth century to see who is concentrated in a market and to
discourage corners and manipulation, and publishing it gives everyone the same picture, including
you. It answers what the price chart cannot: how much of this market is already held by the large
participants, so how much room is left for them to keep buying.

## The rules, step by step

The shared mechanism is: find the outside variable that should drive what you trade, turn it into a
number comparable across time, combine the numbers, and let the total set the position size.

1. Pick the driver. Three appear here: world economic growth, real interest rates, and who is already
   positioned in the market.
2. Turn each driver into a z-score, the number of standard deviations a reading sits from its own
   recent average, so that a growth measure and a rate measure become comparable.
3. Combine the z-scores with fixed weights, and scale the result by how sensitive the asset is to the
   driver, its beta.
4. Clip the combined number to a fixed range so one extreme reading cannot demand an enormous
   position, and map it to a target share of the pot.
5. Recompute every twenty-one trading days and trade to the new target. Some variants rank several
   assets instead and hold the leaders.
6. For the positioning files, compare current holdings with their own history: buy when the hedgers
   are unusually long and the speculators unusually short, and stand down as both return to normal.

The strategies in this category. Twenty-one files are grouped here; these are the ones a reader meets
first.

| File                                  | Driver               | What it does in one clause                                                                  |
| ------------------------------------- | -------------------- | ------------------------------------------------------------------------------------------- |
| `test_0016_macro_fx_strategy`         | Growth, rates, trend | Scores four currencies on share momentum, bond momentum and their own trend, scaled by beta |
| `test_0004_gold_cot`                  | Positioning report   | Buys gold when commercial hedgers are extremely long and speculators extremely short        |
| `test_0010_gold_real_rate_signal`     | Real interest rates  | Raises gold exposure when a bond-to-inflation-fund ratio is falling and below its trend     |
| `test_0001_gold_change_point_trading` | Regime change        | Detects a shift in gold's average and volatility, then trades the new regime                |
| `test_0002_gold_walk_forward`         | Rolling refitting    | Fits parameters inside each window and trades them in the next, to fight overfitting        |
| `test_0003_gold_factor_timing`        | Value and momentum   | Sets gold exposure from a blend of cheapness and recent strength                            |
| `test_0009_gold_ranking_system`       | Risk-adjusted return | Ranks five gold-linked funds and holds the leaders                                          |
| `test_0017_metal_inventory_strategy`  | Physical inventories | Allocates across four precious metals from reported stock levels                            |

Three files carry almost all the category's ideas.

The macro FX file (`test_0016_macro_fx_strategy.py`). It trades four currencies - the euro, the
Australian dollar, the New Zealand dollar and the pound - but every number that drives the trade
comes from two things it never trades: a share index fund as a stand-in for world growth, and a
government bond fund as a stand-in for interest rates. The growth factor is the share fund's change
over 63 days as a z-score; the rates factor is minus the bond fund's change over the same window as a
z-score, so rising bond prices score positive; each pair adds its own 63-day trend as a z-score. The
three are weighted 0.4, 0.35 and 0.25, the growth and rates terms are multiplied by the pair's beta
(one for the Australian and New Zealand dollars, 0.8 for the pound, 0.6 for the euro), and the total
is clipped to plus or minus 0.5 before being mapped to a cap of twenty-five percent per pair. Over
4,331 daily bars from 2008 to 2025 it made 259 trades and ended at 1,040,485.14 from 1,000,000, up
4.05 percent, with a reward-to-risk ratio of 1.037 and a worst fall of 34.82 percent.

The gold positioning file (`test_0004_gold_cot.py`). It downloads the weekly report and turns each
group's net position into a z-score over 156 weeks. A group's net position is its long contracts
minus its short contracts. When the hedgers are extremely long (z at or above plus two) while the
speculators are extremely short (at or below minus two), the file buys gold, on the theory that the
hedgers are the informed side. It exits when both have reverted within one standard deviation.
Position size scales with the extremity, from a base of three percent up to five, with a three
percent stop and a four-week pause after three consecutive losses. Over 888 usable weekly bars it
made 22 trades, about 36 percent of them winners, ending at 997,205.05 from 1,000,000 - a small loss.

The real-rate file (`test_0010_gold_real_rate_signal.py`). Real rates are interest rates after
subtracting expected inflation, and they are the first thing gold is compared with, because holding
gold means giving up whatever a safe bond would pay. The file avoids the macro database by using a
ratio of two funds: a government bond fund divided by a fund of inflation-protected bonds. When that
ratio (as a logarithm) has fallen over 63 days and sits below its 126-day trend, gold is treated as
supported, and exposure scales between fifty and one hundred percent by how strong the reading is,
halved whenever gold's own volatility is above twenty-five percent a year. Over 2011 to 2025 it made
only 10 trades, ending at 1,064,691.53 from 1,000,000, up 6.47 percent, with a reward-to-risk ratio
of 1.284 and a worst fall of 25.10 percent.

## The maths, with every symbol named

The z-score, which makes different drivers comparable:

```text
z = (x - mean(x over the window)) / std(x over the window)
```

- `x` is the raw reading, such as a 63-day change in the share fund.
- `mean(...)` is the average of that reading over the look-back window, such as 126 days.
- `std(...)` is the standard deviation of those readings, a measure of how widely they scatter.
- `z` says how many standard deviations today's reading sits from its own average.

The composite signal and how it becomes a position, as in the macro FX file:

```text
raw = w_growth * z_growth * beta + w_rates * z_rates * beta + w_trend * z_trend
signal = clip(raw, -t, +t)
target_share = signal / t * max_pair_weight
```

- `w_growth`, `w_rates`, `w_trend` are the factor weights, 0.4, 0.35 and 0.25, which add to one.
- `z_growth`, `z_rates`, `z_trend` are the three z-scores.
- `beta` is the pair's sensitivity, one for the Australian dollar, 0.6 for the euro.
- `clip(raw, -t, +t)` chops the signal to lie between minus `t` and plus `t`; `t` is 0.5 here.
- `max_pair_weight` is the most the rule will put in one pair, 0.25 here.
- `target_share` is the signed share of the pot: positive is bought, negative is sold.

The real-rate proxy, built from two funds:

```text
real_rate_proxy = log(bond_fund_price / inflation_fund_price)
```

- `bond_fund_price` is the price of a government bond fund.
- `inflation_fund_price` is the price of a fund of inflation-protected bonds.
- The ratio rises when ordinary bonds beat inflation-protected ones, standing in for a rise in real
  rates; falling real rates are treated as supportive of gold.

The positioning report, as two numbers and their z-scores:

```text
net_position = long_contracts - short_contracts
z_net = (net_position - mean over the window) / std over the window
```

- `long_contracts` and `short_contracts` are the numbers a group holds to buy and to sell.
- `net_position` is the difference, positive when the group is on balance betting on a rise.
- The z-score is computed separately for the commercial group and the speculator group over the same
  window, here 156 weeks.

## A worked example

First, the macro FX rule for the Australian dollar, whose beta is one, over five rebalances. The
factor z-scores are given; the raw signal is `0.4 * growth + 0.35 * rates + 0.25 * trend`, clipped to
plus or minus 0.5, then divided by 0.5 and multiplied by twenty-five percent.

| Rebalance | Growth z | Rates z | Trend z | Raw    | Clipped | Target share | Realised return | Contribution |
| --------- | -------- | ------- | ------- | ------ | ------- | ------------ | --------------- | ------------ |
| 1         | +1.2     | +0.5    | +0.8    | +0.855 | +0.500  | +25.00%      | +1.0%           | +0.250%      |
| 2         | +0.6     | -0.4    | +0.3    | +0.175 | +0.175  | +8.75%       | -0.5%           | -0.044%      |
| 3         | -0.9     | -1.1    | -0.5    | -0.870 | -0.500  | -25.00%      | -0.8%           | +0.200%      |
| 4         | -0.3     | +0.2    | -0.1    | -0.075 | -0.075  | -3.75%       | +0.2%           | -0.008%      |
| 5         | +1.0     | +0.8    | +0.6    | +0.830 | +0.500  | +25.00%      | +0.6%           | +0.150%      |

Read the first row: all three drivers favour the currency, so the rule wants a large position, clips
it to the maximum, and puts a quarter of the pot in. The third row turns the position negative, so
the book goes short. The contribution column is the target share times the currency's return. Start
with 100,000, pay 0.0002 on the amount traded each time the share changes, and let the returns
compound: the pot runs 100,244.99, 100,197.87, 100,391.49, 100,379.70 and finally 100,524.49, a gain
of 0.52 percent over five periods. Over eighteen years the file ended only about four percent ahead,
which shows how thin the signal is against its costs and its wide swings.

Second, the positioning rule on five weeks of made-up but plausible numbers. Suppose each group's net
position is measured against a window whose average is zero and whose standard deviation is 100,000
contracts, so the z-score is the net position divided by 100,000.

| Week | Commercial net | Commercial z | Speculator net | Speculator z | Rule reading                            |
| ---- | -------------- | ------------ | -------------- | ------------ | --------------------------------------- |
| 1    | +50,000        | +0.50        | -50,000        | -0.50        | nothing, neither is extreme             |
| 2    | +120,000       | +1.20        | -130,000       | -1.30        | nothing, still short of two             |
| 3    | +220,000       | +2.20        | -240,000       | -2.40        | both extreme: buy gold, scaled position |
| 4    | +180,000       | +1.80        | -200,000       | -2.00        | hold, the exit has not triggered        |
| 5    | +90,000        | +0.90        | -95,000        | -0.95        | both within one: sell the position      |

At week 3 the extremity is the larger of 2.20 and 2.40, so the size scaler is 2.40 divided by 2.0,
which is 1.2. The base size of three percent becomes 3.6 percent of the pot, under the five percent
cap. On a 100,000 pot that is 3,600. If gold rises four percent between week 3 and week 5, the gross
gain is 3,600 times 0.04, which is 144. The round trip trades about 7,200, so at 0.0005 the cost is
3.60 and the net is about 140.40, or 0.14 percent of the pot.

## What the research actually found

The economic case starts with Chen, Rogoff and Rossi, who showed that commodity-exporting countries'
exchange rates carry information about future commodity prices - the currencies move ahead of the
commodities, not only after them - on a sample running from the 1980s to the 2000s. The study is
about predictability, not a profitable rule, and says so. The gold side rests on Erb and Harvey's
"The Golden Dilemma", whose finding is that gold's link to inflation and real rates is real but
unstable over long horizons, which is why the real-rate file uses a falling-window signal rather than
a fixed relationship.

The results on the compendium's data are modest and are quoted from the category article: macro FX
up 4.05 percent over eighteen years with a 34.82 percent worst fall; the positioning file down 0.28
percent over twenty years; the real-rate file up 6.47 percent over fifteen years with a 25.10 percent
worst fall. The article's own summary is that this is the physique of a macro signal strategy: low
frequency, low turnover and transparent logic, on one narrow dataset.

One thing must be said plainly, because it applies to every file in this category and to every other
backtest in this compendium. Each backtest asserts its final value, its reward-to-risk ratio and its
worst fall against a stored baseline, and it must produce identical numbers in the engine's two
calculation modes. The reward-to-risk ratio is the average return divided by how much the pot swung,
and the worst fall is the largest drop from a peak to the following low. Passing those assertions
proves the engine computes exactly what the file says it computes, on the stored data. It does not
prove the strategy earns anything. The assertion is a test of the software, not a claim about the
market. A file can pass every assertion and still describe a plan that makes nothing; most of this
category does exactly that.

## How this project relates to it

The repository's own brief on macro, rates and foreign exchange,
[strategies/books2/07_macro_rates_and_fx.md](../../../strategies/books2/07_macro_rates_and_fx.md),
is the closest research here. Its central finding is that the tradable macro information is
concentrated in scheduled announcement windows: on US Treasury and European bond futures from 2007 to
2017, co-jumps play about twice as large a role in the United States as in Europe, and roughly thirty
percent of central-bank announcements are accompanied by a shift of the whole interest-rate curve
(`1905.01541v1`, p.25). Since this category's signals are built from bond and share prices, a macro
currency rule is exposed to exactly those windows. The brief also reports an AI-built currency signal
with an annualised reward-to-risk ratio above 0.7 (`2608.00761v1`, p.2).

The brief on energy and commodities,
[strategies/books2/18_energy_and_commodities.md](../../../strategies/books2/18_energy_and_commodities.md),
adds the commodity-finance half: a commodity's futures curve - its level, slope and curvature -
explains 96.5 percent of the curve's variation, and the slope carrying forward is the profitable part
(`2308.00383v1`, p.11, p.13). Two finished tutorials cover related ground:
[Risk premia in forex markets](../../quantconnect/risk-premia-in-forex-markets/README.md) and
[Can crude oil predict equity returns](../../quantconnect/can-crude-oil-predict-equity-returns/README.md).

## Where it goes wrong

- The proxy is not the driver. A share fund's price is a noisy stand-in for world growth, and a bond
  fund for interest rates. The rule can be right about the story and still follow the wrong number.
- The lag can close. The idea needs the currency to respond after the driver, but the more people
  watch the same public reports, the faster the response, until there is nothing left to capture.
- Positioning reports are backward-looking and popular. They say what large participants held several
  days before publication, and once a report is widely followed, that position is already partly
  unwound, which is the well-known failure of "following the smart money" rules.
- Real relationships drift. The link between gold and real rates is real on average and unstable at
  the horizon a rule can trade, which is the golden-dilemma finding.
- Wide swings for thin gains. The macro FX file's 34.82 percent worst fall on a rule that made about
  four percent over eighteen years is the worst possible shape: nearly as much risk as a share
  portfolio, with none of the reward so far.

## Try it yourself

You need a spreadsheet, monthly prices for a commodity-exporting country's currency pair (such as the
Australian dollar against the US dollar), and monthly prices for a broad commodity fund.

1. One row per month, with a column for the currency and a column for the commodity fund.
2. Add a column for each one's three-month return.
3. Add a column for the currency's z-score: its return minus the 24-month average return, divided by
   the 24-month standard deviation.
4. Add a column that says "buy the currency" when the z-score is above one and "sell" when below
   minus one.
5. On the next row, apply the currency's actual next-month return to that decision.
6. Beside it, repeat the exercise using the commodity fund's return as the signal instead.

What to notice: the two signals are often the same and sometimes opposite. Where they differ, look at
which moved first; the claim that currencies lead commodities is a claim about those lags. Then run
the whole thing without costs and again with 0.001 subtracted for each change, and see whether the
sign of the result survives.

## Where this came from

- [The compendium's commodity and currency article](https://backtrader.readthedocs.io/en/latest/strategies-series/en/13-commodity-currency.html),
  the rules, the inventory and the file-level results quoted above.
- Chen, Rogoff and Rossi, [Can Exchange Rates Forecast Commodity Prices?](https://www.nber.org/papers/w13901),
  the study that currencies carry information about future commodity prices.
- Erb and Harvey, [The Golden Dilemma](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2078535),
  the long-horizon study of gold, inflation and real rates.
- [macro rates and FX brief](../../../strategies/books2/07_macro_rates_and_fx.md), this repository's
  own, on rates and currencies.
- [energy and commodities brief](../../../strategies/books2/18_energy_and_commodities.md), this
  repository's own, including the futures-curve finding.

## Words used in this tutorial

- beta: how much one thing moves when another moves by one unit.
- carry: the payment for holding one thing financed by another, such as an interest difference.
- futures: an agreement to buy or sell something at a fixed price on a future date.
- hedger: a trader using a market to reduce a risk from their real business, not to bet.
- real rate: an interest rate after subtracting expected inflation.
- speculator: a trader taking a position to profit from a price move.
- z-score: how many standard deviations a reading sits from its own recent average.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
