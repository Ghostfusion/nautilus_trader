# Crude oil as a forecast: owning shares or cash, decided by last month's oil price

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                   |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | A fund that tracks a broad stock index, and a short-term government bill, switching between the two                                                                                                                                                                     |
| How often it trades       | Once a month, when the forecast is recomputed; it traded only nine times in nine years in one test                                                                                                                                                                      |
| What you need             | A spreadsheet and three monthly price series: oil, a stock index and a short-term interest rate                                                                                                                                                                         |
| Where the rules come from | [QuantConnect strategy library, can crude oil predict equity returns](https://www.quantconnect.com/tutorials/strategy-library/can-crude-oil-predict-equity-returns)                                                                                                     |
| The underlying research   | Driesprong, Jacobsen and Maat, [Striking Oil: Another Puzzle?](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=460500)                                                                                                                                              |
| How well it held up       | Disputed: the original study found strong predictability and a four percent yearly gain after costs, a later study on the same idea found the relationship had disappeared by 2015, and the library's own test from 2010 to 2017 matched the market but did not beat it |
| Also appears in           | The project tutorial [measuring the market's kind of weather](../../project/measuring-the-regime/README.md) builds the same habit of asking whether a relationship is real before acting on it                                                                          |

## The idea in one paragraph

Oil is one of the most important inputs to the world economy, so a rise in its price should be bad
news for company profits and a fall should be good news. The claim is that share prices do not react
to oil news immediately; they take a month or so to catch up. If that is true, the change in the oil
price last month carries a clue about which way shares will move next month. The rule turns that clue
into a single number, compares it with the interest paid by a short-term government bill, and holds
shares if the shares are expected to pay more and the bill if they are expected to pay less.

## Why anyone believed it

A company's costs rise when oil rises, and its customers have less money to spend, so a high oil price
should mean lower profits and a lower share price. That much is arithmetic. The puzzle is why the
share price should react slowly rather than at once.

The explanation offered is underreaction. Oil matters to the whole economy through many channels at
once, and it takes time for analysts to work out what a given oil move means for a particular company
or country. Different investors also read the same news at different times. If they update their
views gradually rather than together, the price only catches up over weeks, and whoever acts first on
the oil number can be paid for the wait. The counterparty is the investor who is slow to revise, or
who is trading for reasons of their own rather than on the news.

## An everyday comparison

Think of the price of flour and the price of bread in a small town. When the mill raises the price of
flour, the baker's costs go up the same afternoon, but the baker may not change the shelf price that
day, because it is awkward and customers complain. Over the following weeks the shelf price drifts up
to match the new cost. Anyone who watches the mill's price and buys bread futures from the baker
before the shelf price moves is trading on exactly the lag this strategy claims exists in shares.

## The rules, step by step

1. Get three series of monthly numbers: the price of crude oil, the level of a broad stock index, and
   the interest rate paid by a short-term government bill, which is treated as the safe alternative.
   The library page used the return of a crude oil index and a United States Treasury bill rate.
2. Turn prices into returns. For each month, divide this month's price by last month's price and
   subtract one. Do this for oil and for the stock index separately.
3. Each month, fit a straight line through the last two years of paired observations, where the
   independent variable is the oil return of the previous month and the value being explained is the
   stock return of the month after it. The library page uses about 22 or 23 monthly observations.
4. Use the fitted line and the oil return just observed to forecast the stock return for the coming
   month.
5. Convert the government bill rate from a yearly rate into a monthly rate by dividing by twelve.
6. If the forecast stock return is higher than the monthly bill rate, hold the whole account in the
   stock index. Otherwise hold the whole account in the bill, which the library page implements as
   holding cash because it could not buy the bill directly.
7. Recompute the forecast and repeat at the start of every month. Do nothing in between.

## The maths, with every symbol named

The relationship is written as a straight line, which is what a regression estimates:

```text
r_stock,t = a0 + a1 * r_oil,t-1 + e_t
```

- `r_stock,t` is the stock index return in month `t`, as a decimal: 0.02 means two percent.
- `r_oil,t-1` is the oil price return in the month before, month `t-1`.
- `a0` is the height of the line where the oil return is zero, its intercept.
- `a1` is the slope of the line: how many percent the stock return changes for each one percent of oil
  return. The study finds this number is negative, so a rise in oil goes with lower share returns.
- `e_t` is the part of the stock return that the oil number does not explain, the residual.

Fitting the line means choosing the `a0` and `a1` that make the leftover `e_t` as small as possible,
in the sense of smallest total squared size. With those two numbers in hand, the forecast for the next
month uses the latest oil return:

```text
forecast = a0 + a1 * r_oil,latest
```

- `forecast` is the expected stock return for the coming month, as a decimal.
- `r_oil,latest` is the oil return of the month just finished.

The safe alternative is the monthly bill rate:

```text
rf_monthly = rf_annual / 12
```

- `rf_annual` is the interest rate a short-term government bill pays over a year, as a decimal.
- `rf_monthly` is the same rate expressed for one month, so it can be compared with `forecast`.

The decision is then a single comparison: hold shares when `forecast > rf_monthly`, and hold the bill
otherwise. Switching from one to the other is assumed to cost 0.10 percent of the amount moved, the
figure the study uses, because an index fund can be bought and sold through a futures contract at a
small cost.

## A worked example

Eight months of made-up but plausible returns. Oil and shares are both measured in percent.

| Month | Oil return | Stock return |
| ----- | ---------- | ------------ |
| 1     | +2.0       | +1.0         |
| 2     | -3.0       | -2.0         |
| 3     | +4.0       | +2.5         |
| 4     | +1.0       | +1.5         |
| 5     | -5.0       | -3.0         |
| 6     | +3.0       | +2.0         |
| 7     | +1.0       | +1.0         |
| 8     | -4.0       | +0.5         |

The regression pairs each month's stock return with the previous month's oil return, giving seven
observations: `(+2.0, -2.0)`, `(-3.0, +2.5)`, `(+4.0, +1.5)`, `(+1.0, -3.0)`, `(-5.0, +2.0)`,
`(+3.0, +1.0)` and `(+1.0, +0.5)`, where the first number is the oil return and the second the stock
return. The average oil return is 0.4286 percent and the average stock return is 0.3571 percent.
Fitting the line gives:

```text
a1 = -0.2522   (the slope)
a0 =  0.4652   (the intercept)
```

The slope is negative, as the study reports: on this sample, a one percent rise in oil goes with a
0.25 percent fall in the next month's share return. Now forecast month 9 using the month 8 oil return
of -4.0 percent:

```text
forecast = 0.4652 + (-0.2522) * (-4.0) = 0.4652 + 1.0088 = 1.4740 percent
```

Suppose the yearly bill rate is 4.0 percent, so `rf_monthly = 4.0 / 12 = 0.3333 percent`. The forecast
of 1.4740 percent is higher than 0.3333 percent, so the rule holds shares in month 9. For contrast,
forecasting month 8 from the month 7 oil return of +1.0 percent gives
`0.4652 - 0.2522 = 0.2130 percent`, which is below the bill rate, so the rule would have held the bill
in month 8. That is the switch that matters for costs.

Suppose month 9 does produce a stock return of +2.00 percent, while the bill would have paid 0.3333
percent. Holding shares paid more, and the switch from the bill into shares cost 0.10 percent:

```text
net return for month 9 = 2.00 - 0.10 = 1.90 percent
```

Two things to notice. The forecast is an average relationship, not a statement about any single month,
so being right on average can still mean losing money in most months. And the answer depends entirely
on how many months are used to fit the line: the library page uses two years, and the study uses the
whole history so far, and the two produce different slopes from the same oil number.

## What the research actually found

Three sources disagree, and the disagreement is the finding.

| Source                                           | What it measured                                                                                                                  | Result                                                                                                                                                                                                                                                                                                                                                          |
| ------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Driesprong, Jacobsen and Maat, Striking Oil      | Monthly MSCI stock indices for 18 developed countries plus a world index, October 1973 to April 2003, oil prices lagged one month | Significant predictability in 12 of 18 developed markets and the world index, always with a negative sign; an oil shock of one standard deviation, about 10 percent, lowered world returns by about one percent; a switching strategy earned about four percent a year after 0.10 percent switching costs, on a sample that was itself used to choose the model |
| Quantpedia, summarising the same study           | A single American market-timing strategy, 1988 to 2003                                                                            | 11.9 percent a year against 10.7 percent for the benchmark, volatility 9.8 percent against 15.1 percent, a Sharpe ratio of 0.81, with the maximum drawdown figure listed as not stated                                                                                                                                                                          |
| Jiang, Skoulakis and Xue, reported by Quantpedia | The same oil-to-share relationship, using data until 2015                                                                         | Oil price changes no longer predict the equity index returns of the large developed economies; the relationship weakens once oil moves are split into supply shocks, demand shocks and oil-specific shocks                                                                                                                                                      |
| QuantConnect library page                        | A rolling two-year regression on oil and the S&P 500 fund, 2010 to 2017                                                           | A Sharpe ratio of 0.726 against 0.735 for simply holding the index, only nine trades in nine years, and monthly p-values that mostly could not reject the idea of no relationship at all                                                                                                                                                                        |

A p-value is the chance of seeing a result this extreme if there were really no relationship; the
library page reports that most of its monthly regressions failed that test, which is a plain statement
that the forecast it was acting on was not distinguishable from noise. The study's own four percent a
year is measured on the second half of a sample that was chosen because the first half showed the
effect, so it is not an independent test. Read together: the effect was strong and well documented in
data up to 2003, weaker and partly gone in data after that, and absent in the library's own recent
test.

## How this project relates to it

This repository's brief on energy markets,
[Energy and commodity markets](../../../strategies/books2/18_energy_and_commodities.md), explains that
a commodity's price is shaped by storage and by the shape of its futures curve, which is why an oil
price is not simply a cost index and why a simple regression can miss what is really moving. The brief
on macro and currencies,
[Macro, rates and foreign exchange](../../../strategies/books2/07_macro_rates_and_fx.md), makes the
comparison concrete: it reports a study that turns scheduled economic data releases into a currency
signal with a Sharpe ratio above 0.7, and it warns that the result is measured before costs and
depends on one data provider.

This repository does not implement oil-to-share market timing anywhere. The closest thing it has is
the rule that a forecast has to be validated before it is acted on, which is the subject of the
project tutorial [Telling which kind of market you are in](../../project/measuring-the-regime/README.md)
and of the engine it describes, [implementation/sector-regime-engine](../../../implementation/sector-regime-engine/README.md).
That engine measures whether a pattern is present and refuses to run a rule that depends on a pattern
it cannot find, which is exactly the test this oil forecast failed in its later samples.

## Where it goes wrong

- The relationship is not stable. The same idea measured on data to 2003 and on data to 2015 gives
  opposite answers, and the library's own 2010 to 2017 test found no usable signal. A rule that is
  estimated on one regime and traded in another is trading a different world.
- The study's own gain is in-sample. The four percent a year comes from a sample chosen after the
  effect was seen, so it cannot tell you what a trader selecting the rule in advance would have
  earned, which the authors themselves concede.
- The forecast is tiny and noisy. A monthly regression on 22 observations produces a slope with a
  wide margin of error. When the p-value is large, the entire monthly decision is being driven by a
  number indistinguishable from zero.
- Switching has a cost and a tax consequence. Each change of position pays the gap between buying and
  selling prices, and in many countries realising a gain to switch creates a tax bill, neither of
  which the four percent figure necessarily includes.
- Oil is not one number. Brent, West Texas Intermediate and other grades differ, the price depends on
  whether the market is in surplus or shortage, and a rise can mean strong demand rather than a cost
  shock, which is why the later study splits oil moves into components before testing them.
- Nobody knows the mechanism. If share prices simply reflected costs, the reaction would be immediate
  and there would be nothing to trade. The whole strategy rests on a claim about slow human updating
  that is hard to measure and easy to lose.

## Try it yourself

You need nothing but a spreadsheet and three public series: a monthly crude oil price, a monthly stock
index level, and a short-term interest rate.

1. Make a column for the month, a column for the oil price, a column for the index level and a column
   for the yearly bill rate.
2. Add an oil return column: this month's oil divided by last month's, minus one.
3. Add a stock return column in the same way.
4. Shift the oil return down by one row, so that each row shows this month's stock return beside last
   month's oil return. That pairing is what the regression uses.
5. Use your spreadsheet's slope and intercept functions on the two shifted columns, over a rolling
   window of 24 months.
6. Add a forecast column: intercept plus slope times last month's oil return. Add a column for the
   bill rate divided by twelve.
7. Add a column that says "shares" when the forecast is bigger than the monthly bill rate and "bill"
   otherwise.
8. Compute the return you would have earned each month from that choice, subtracting 0.10 percent on
   any month the choice changed.

What to notice: run the same sheet with the window set to 24 months and then to 120 months and watch
how often the choice flips, and how different the two answers are. Then split the sample into two
halves and compare: the relationship you find in the early half will often be absent in the late half,
which is the whole difficulty with this strategy.

## Where this came from

- [QuantConnect strategy library: can crude oil predict equity returns](https://www.quantconnect.com/tutorials/strategy-library/can-crude-oil-predict-equity-returns),
  the rules as implemented, the two-year rolling regression, the 2010 to 2017 result and the reference.
- [Quantpedia: crude oil predicts equity returns](https://quantpedia.com/strategies/crude-oil-predicts-equity-returns),
  the restatement of the rules, the indicative 11.9 percent annual return, the 9.8 percent volatility
  and the 0.81 Sharpe ratio, together with the abstracts of the later papers.
- Driesprong, Jacobsen and Maat, [Striking Oil: Another Puzzle?](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=460500),
  the original study: 12 of 18 developed markets, the one percent fall per one standard deviation oil
  shock, and the roughly four percent a year after 0.10 percent switching costs.
- [Energy and commodity markets](../../../strategies/books2/18_energy_and_commodities.md), this
  repository's brief on how commodity prices and curves actually behave.
- [Macro, rates and foreign exchange](../../../strategies/books2/07_macro_rates_and_fx.md), the brief
  on turning macro data into positions, including its warning about results measured before costs.

## Words used in this tutorial

- bill: a short-term loan to a government, treated here as the safe place to hold money.
- intercept: the value a fitted line takes when the input is zero.
- market timing: switching between shares and cash in an attempt to be invested only when prices rise.
- p-value: the chance of seeing a result this extreme if there were really no relationship at all.
- regression: fitting a straight line through paired observations to describe how one moves with the
  other.
- residual: the part of a value that a fitted line does not explain.
- risk-free rate: the return on something considered safe, used here as the bar the forecast must beat.
- slope: how much one quantity changes for each unit change in another.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
