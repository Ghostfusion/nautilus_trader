# Forecasting: guessing tomorrow's direction, and the difference between being right and being paid

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                               |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Spot gold on daily bars, and 15-minute gold bars read at a 12-hour or a 6-hour signal timeframe                                                                                                                                                     |
| How often it trades       | From 2 completed trades in four years for the daily model to 21 trades in a quarter of a year for the oscillator                                                                                                                                    |
| What you need             | Python and a data file                                                                                                                                                                                                                              |
| Where the rules come from | [The Strategy Compendium, article 30, forecasting](https://backtrader.readthedocs.io/en/latest/strategies-series/en/30-forecasting.html)                                                                                                            |
| The underlying research   | None with a paper behind these rules: the Forecast Oscillator is a MetaTrader indicator, the moving-average rule is a platform port, and the daily rule applies the textbook Box-Jenkins method to price returns without a published trading result |
| How well it held up       | Weak: three rules on one metal, two with short samples, and the only one that traded often enough to measure ended fractionally negative after 21 trades                                                                                            |
| Also appears in           | [Wavelet plus support vector machine](../../quantconnect/svm-wavelet-forecasting/README.md) and the [temporal CNN forecaster](../../quantconnect/forecasting-stock-prices-using-a-temporal-cnn-model/README.md) in this collection                  |

## The idea in one paragraph

Predicting the size of tomorrow's move is close to impossible, and every rule here knows it. What
they try instead is direction: up, or not up. One rule fits a small statistical model to recent daily
returns and buys when the model's answer is positive. One measures how far the price has drifted from
a straight line drawn through the recent past, and trades when that distance turns. One reads a fast
and a slow average on a higher timeframe and lets a faster timeframe place the trades. In each case
the model produces a yes-or-no call for the next bar, and the exit rules, not the forecast, decide
whether the year is good. None of them predicts a target price.

## Why anyone believed it

The argument starts from the observation that prices are not perfectly random. Over minutes and days
the drift is tiny compared with the noise, but it is not always zero, and a rule that is right a
little more often than not can still make money if the losses are cut and the winners are allowed to
run. The counterparty is the participant who trades on noise itself, buying after a spike and selling
after a dip, and whose predictable reaction is what a fitted model is trying to be on the other side
of.

The second part of the argument is about what a forecast is for. A person who knows tomorrow will
rise by 0.2 percent and does not know whether it might instead fall by 2 percent has not learned
enough to trade; a rule that turns the faint signal into a position and attaches a stop and a target
has converted a weak forecast into a bounded bet, which is the only usable form.

## An everyday comparison

A weather forecaster who says "it will probably rain tomorrow" is right most of the time, because
most days do not need the umbrella and most days are dry. The forecast is still useful to a farmer
deciding whether to irrigate: the cost of the wrong call is small in one direction and large in the
other, so a small edge in the odds is worth acting on. Trading on a forecast is that same arithmetic.

## The rules, step by step

The category holds three backtests, and they share one shape: turn recent prices into a number,
compare the number with zero, and take a position whose size and exit are fixed in advance.

| Strategy            | Data                               | What it does                                                                     |
| ------------------- | ---------------------------------- | -------------------------------------------------------------------------------- |
| ARIMA forecast      | Gold daily, 2022 to 2025           | Fit a small model to the last year of returns, buy when its forecast is positive |
| Forecast oscillator | Gold 15-minute, read at 12 hours   | Trade the crossing of a regression-distance measure with its own smoothed line   |
| EMA prediction      | Gold 15-minute, signals at 6 hours | A fast and a slow average cross on the higher frame; the lower frame executes    |

1. ARIMA forecast. Use daily gold. Compute the daily return, today's close divided by yesterday's
   minus one. Keep a rolling window of the last 252 returns and fit a model with one memory term for
   the last return, one difference term and one memory term for the last surprise. The fitted model
   produces a forecast for tomorrow's return. Refit the model every 20 days and keep using the last
   fit in between. If the forecast is above zero, hold 95 percent of the account long; if it is zero
   or below, hold nothing. Never sell short.
2. Forecast oscillator. Use 15-minute gold bars, resampled to 12-hour bars so that a signal covers
   half a day. Fit a straight line through the last 15 bars by least squares and extend it one bar
   forward; that extension is the regression forecast. Measure the distance from the price to that
   forecast as a percentage of the forecast. Smooth the result with a T3 average, which is a moving
   average applied several times in a row, and use that as the signal line. Buy when the raw
   oscillator crosses above its signal line, sell short when it crosses below, and reverse on the
   opposite arrow. Protect every position with a stop 1000 points away and a target 2000 points away;
   with a point of 0.01 that is 10.00 and 20.00 dollars an ounce.
3. EMA prediction. Use 15-minute gold for execution and 6-hour gold for signals. The signal line is
   an exponential moving average with a period of 1, which nearly tracks the price, against one with
   a period of 2. Buy when the fast one crosses above the slow one on a bar that closed up, and sell
   when it crosses below on a bar that closed down. Each new signal is acted on once, at the 15-minute
   bar following it, with a 1000-point stop and a 2000-point target.
4. All three act on the close and hold one position at a time. The daily rule rebalances only when its
   target changes; the two intraday rules can reverse several times a day.

## The maths, with every symbol named

The daily rule is a small model of how yesterday's return carries into today's. Written with one
memory term and one surprise term, which is what the three numbers 1, 0, 1 describe:

```text
forecast_t = c + phi * r_(t-1) + theta * e_(t-1)
```

- `r_(t-1)` is yesterday's return, as a decimal: 0.004 means plus 0.4 percent.
- `phi` is how much of yesterday's return is carried forward. A value of 0.30 says a third of
  yesterday's move is expected to repeat.
- `e_(t-1)` is yesterday's surprise, the amount the model got wrong yesterday.
- `theta` is how much of that surprise is carried forward.
- `c` is a small constant, the average return the model expects when everything else is zero.
- `forecast_t` is today's predicted return. The rule buys when it is positive and stays flat when it
  is not.

The oscillator rule draws a line through the recent past and measures the gap to it:

```text
line_i   = a + b * i          for i = 1 to n
forecast = a + b * (n + 1)
osc      = 100 * (price - forecast) / forecast
```

- `i` numbers the bars in the window, from 1 for the oldest to `n` for the newest, with `n` equal to
  15 here.
- `a` is where the line starts and `b` is how much it rises per bar. Both are chosen so that the line
  sits as close as possible to all the prices at once, which is the least-squares fit.
- `forecast` is the line extended one bar past the end of the window: the price the recent straight
  line points to.
- `price` is the newest closing price and `osc` is the gap between the price and that forecast, in
  percent of the forecast. A positive number means the price is above the line it has been following.

The oscillator is then smoothed, and the rule trades the difference between the raw line and the
smooth one. A smoothed average of the exponential kind is:

```text
EMA_t = alpha * P_t + (1 - alpha) * EMA_(t-1),  with alpha = 2 / (n + 1)
```

- `P_t` is today's value, here the oscillator rather than a price.
- `alpha` is the weight given to today: for a period `n` of 2 it is 2/3, so the line follows the
  input closely.
- `EMA_(t-1)` is yesterday's value of the same line, so the new value is a weighted blend of the two.

## A worked example

First the daily model, on eight invented returns. The fitted model in the example has `c = 0.0002`
and `phi = 0.30`, and the surprise term is left out of the table to keep the arithmetic short; the
file keeps it. The signal is read from today's return and the money is earned on tomorrow's, which is
what walk-forward testing means.

| Day | Today's return | Forecast | Signal for tomorrow | Tomorrow's return | Earned |
| --- | -------------- | -------- | ------------------- | ----------------- | ------ |
| 1   | +0.40%         | +0.14%   | long                | -0.60%            | -0.60% |
| 2   | -0.60%         | -0.16%   | flat                | +0.20%            | 0.00%  |
| 3   | +0.20%         | +0.08%   | long                | +0.80%            | +0.80% |
| 4   | +0.80%         | +0.26%   | long                | -0.10%            | -0.10% |
| 5   | -0.10%         | -0.01%   | flat                | +0.50%            | 0.00%  |
| 6   | +0.50%         | +0.17%   | long                | -0.30%            | -0.30% |
| 7   | -0.30%         | -0.07%   | flat                | +0.60%            | 0.00%  |
| 8   | +0.60%         | +0.20%   | long                | +0.40%            | +0.40% |

Check day 4. Today's return is plus 0.80 percent, written 0.0080. The forecast is
`0.0002 + 0.30 * 0.0080 = 0.0002 + 0.0024 = 0.0026`, that is plus 0.26 percent, above zero, so the
rule holds the position into the next day.

```text
Gross earned over the eight days = -0.60 + 0.80 - 0.10 - 0.30 + 0.40 = +0.20 percent
Trades completed = 4, because the position was opened and closed four times
Cost = 4 * 2 * 0.0005 = 0.0040, that is 0.40 percent
Net   = 0.20 - 0.40 = -0.20 percent
```

The file charges no commission at all, so the table above adds 0.05 percent per side, covering a small
commission and the gap between the price at which the metal can be bought and the price at which it
can be sold. That gap is the whole result: the direction was guessed correctly on enough days, but
four round trips cost twice what the forecast earned.

Now the oscillator, on eight invented 12-hour bars. The oscillator and its signal line are the
indicator's own outputs; the fit that produces them is worked through below the table.

| Day | Close   | Oscillator | Signal line | Arrow | Action                |
| --- | ------- | ---------- | ----------- | ----- | --------------------- |
| 1   | 2000.00 | +0.10      | +0.05       | none  | watch                 |
| 2   | 1996.00 | -0.25      | -0.10       | sell  | sell short at 1996.00 |
| 3   | 1990.00 | -0.40      | -0.30       | sell  | hold                  |
| 4   | 1985.00 | -0.15      | -0.28       | buy   | buy back at 1985.00   |
| 5   | 1988.00 | +0.05      | -0.10       | buy   | go long at 1988.00    |
| 6   | 1995.00 | +0.10      | +0.15       | sell  | sell at 1995.00       |
| 7   | 1992.00 | -0.05      | +0.05       | sell  | go short at 1992.00   |
| 8   | 1989.00 | -0.30      | -0.15       | sell  | hold                  |

On day 2 the oscillator has crossed below its signal line, so the rule sells short at 1996.00. On day
4 it has crossed back above, so the rule buys the short back at 1985.00 and immediately opens a long
at the same price. On day 6 it crosses below again, closing the long at 1995.00 and opening a short
at the same price, which is still open on day 8.

```text
Short 1: sold 1996.00, bought 1985.00. Gross = (1996.00 - 1985.00) / 1996.00 = 0.5511 percent
Long 2:  bought 1985.00, sold 1995.00. Gross = 1995.00 / 1985.00 - 1 = 0.5038 percent
Short 3: sold 1992.00, still open at 1989.00. Unrealised = 0.1504 percent
Cost per completed round trip = 2 * 0.0002, that is 0.04 percent
Net on the two closed trades = 0.5511 + 0.5038 - 0.04 - 0.04 = 0.975 percent
```

The two percent per side assumed above is smaller than the daily rule's because these are liquid
15-minute bars and the file charges nothing, but it is still 0.04 percent for each full turn, and the
rule turned four times in eight bars. Notice also that the target of 20.00 and the stop of 10.00 were
never touched: both closed trades ended on the opposite arrow, which is the more common exit.

The straight line behind the arrow on day 4 can be checked by hand. With the four closes 1990.00,
1988.00, 1986.00 and 1985.00 numbered 1 to 4, the average of the numbers is 2.5 and of the prices
1987.25; the line rises by minus 1.70 per bar from 1991.50, so the forecast for bar 5 is
`1991.50 - 1.70 * 5 = 1983.00`, and the oscillator at a price of 1985.00 is
`100 * (1985.00 - 1983.00) / 1983.00 = 0.101`, the positive value in the table.

## What the research actually found

The library reports one run per rule. Its own numbers are below.

| Rule                | Sample                      | Trades | Wins       | Final value | Reward for risk | Worst fall |
| ------------------- | --------------------------- | ------ | ---------- | ----------- | --------------- | ---------- |
| ARIMA forecast      | Gold daily, 2022 to 2025    | 2      | 2 (100%)   | 2,151,710   | not given       | not given  |
| Forecast oscillator | Gold 12-hour, one quarter   | 21     | 11 (52.4%) | 999,480     | -9.26           | 35.36%     |
| EMA prediction      | Gold 15-minute, one quarter | 55     | 22 (40.0%) | 1,000,476   | 0.80            | 55.85%     |

Two cautions dominate this table. The ARIMA figure comes from a contract with a multiplier of 100 and
margin of 1 percent, so a 95 percent position is a very large exposure: the doubling is leverage as
much as it is skill, and the two completed trades in four years mean the forecast was catching a
months-long drift in gold rather than daily fluctuation. The oscillator made 21 trades and ended
fractionally below where it started, described by an annualised reward-for-risk figure of minus 9.26
that is an artefact of scaling one quarter of 12-hour bars to a year. The moving-average rule made 55
trades, won 40 percent, and ended 0.05 percent higher, which is another way of saying it broke even.

The library's tests assert the final value, the reward for risk and the worst fall against numbers
captured when each strategy was migrated, and each test must also produce identical results in both
of the engine's modes. Passing those assertions proves the engine computes exactly what the file
says, to the cent and to the sixth decimal. It proves nothing about whether the forecast earns
anything, which is the point of this whole category: a rule can correctly be shown to be a coin toss.

The repository's reading of the research is in
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md).
A backtest of moving-average rules on the constituents of three American indexes found the plain rules
winning under half their trades, with win rates of 0.40 to 0.49 for the crossover version, and a
parameter search returning a different best setting for every index with the apparent improvement
driven by two trades that exceeded twenty times the initial investment (`2206.12282v1`). That is
exactly the shape of the EMA rule. A matched experiment also found the choice of training objective
moved a portfolio's reward for risk more than the choice of model did
(`2108.02283v7`, recorded in [Machine learning for finance](../../../strategies/books2/04_machine_learning_for_finance.md)).

The other relevant brief is
[Stylized facts, tails and scaling](../../../strategies/books2/13_stylized_facts_and_scaling.md). It
records that over 150 years and a dozen markets, the autocorrelation of traditional-market returns
falls inside the noise band within about twenty minutes (`2504.08611v1`). An ARIMA model fitted to
daily returns is looking for structure in a quantity whose memory is negligible at anything but the
shortest horizons, so there is very little for it to find.

## How this project relates to it

The closest material in this repository is the brief
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
which documents both the sub-fifty-percent win rates of indicator rules and the way a parameter
search overfits them, and the brief
[Stylized facts, tails and scaling](../../../strategies/books2/13_stylized_facts_and_scaling.md),
whose twenty-minute autocorrelation result says what a forecaster is up against. Together they are the
honest frame: the forecasting step is cheap, and the cost and the exit discipline decide the money.

Two finished tutorials forecast direction with heavier machinery: the
[wavelet plus support vector machine](../../quantconnect/svm-wavelet-forecasting/README.md) page
splits a price into slow and fast pieces and fits each one, and the
[temporal CNN forecaster](../../quantconnect/forecasting-stock-prices-using-a-temporal-cnn-model/README.md)
page trains a deep network for the same one-day question. Both reach this category's conclusion.

## Where it goes wrong

- Direction is not money. The first worked example guesses the direction of most days and still ends
  down, because the position changed four times in eight days and every change paid the gap between
  buying and selling prices and a commission.
- The horizon is the hardest one. Over a single day or a half day, the drift a model is trying to
  catch is tiny next to the noise, so the forecast sits close to a coin toss even when everything in
  the code is correct.
- The samples are too short. Two completed trades in four years, 21 trades in three months, 55 trades
  in three months. None of those is enough to tell a small edge from luck, and the extreme reward-for-
  risk figures are annualisation artefacts rather than measurements.
- Leverage flatters the daily rule. A multiplier of 100 with 1 percent margin turns a modest price
  move into a large account move in both directions, and the test reports the account, not the risk.
- The parameters were chosen after the result was seen. The training window of 252 days, the refit
  every 20 days, the zero threshold and the 15-bar regression window are all free choices.
- Fitting on the whole file leaks the future. A model whose coefficients were fitted using data that
  includes the days it is trading has already seen the answer; the daily rule refits as it goes,
  which is the correct pattern, and any reimplementation has to keep it.
- The oscillator's smoothing lags. A T3 average is deliberately slow, so the arrow arrives after the
  move has begun and the four turns in eight bars are partly the strategy chasing its own smoothing.

## Try it yourself

You need nothing but a spreadsheet and about 250 daily closing prices of any share or metal.

1. Put the dates down one column and the closes in the next.
2. Add a column for the daily return: today's close divided by yesterday's, minus one.
3. Add a column for a one-term forecast: a constant you choose, say 0.0002, plus 0.30 times
   yesterday's return.
4. Add a column that says long when that forecast is positive and flat when it is not.
5. Add a column for what the rule earns: tomorrow's return when the signal says long, and zero when
   it says flat.
6. Total that column. Now count how many times the signal changed, multiply by 0.10 percent, and
   subtract it.

What to notice: the total before costs is usually a small positive number, because the market drifts
up over long samples, and the count of changes is usually twenty or thirty a year, which costs three
percent. Then change the 0.30 to 0.10 and to 0.60: if the sign of the result flips with a coefficient
you chose, you have chosen the answer rather than found it.

## Where this came from

- [The Strategy Compendium, article 30, forecasting](https://backtrader.readthedocs.io/en/latest/strategies-series/en/30-forecasting.html),
  the category inventory, the three deep dives and every performance figure quoted above.
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's brief, including the moving-average win rates and parameter-search result
  (`2206.12282v1`).
- [Stylized facts, tails and scaling](../../../strategies/books2/13_stylized_facts_and_scaling.md),
  this repository's brief, including the twenty-minute autocorrelation result (`2504.08611v1`).
- [Machine learning for finance](../../../strategies/books2/04_machine_learning_for_finance.md),
  the brief that records the loss-function experiment (`2108.02283v7`).
- [Wavelet plus support vector machine](../../quantconnect/svm-wavelet-forecasting/README.md) and the
  [temporal CNN forecaster](../../quantconnect/forecasting-stock-prices-using-a-temporal-cnn-model/README.md),
  the finished tutorials in this collection that forecast direction.

## Words used in this tutorial

- autoregressive: describing a series in which today's value depends on its own recent values.
- exponential moving average: an average that gives more weight to recent values, so it turns faster
  than a simple average of the same length.
- leverage: using borrowed money so that a given price move produces a larger move in the account.
- reward for risk (Sharpe ratio): the average return divided by how much it wobbled, usually written
  per year; zero means no reward once the wobble is counted.
- walk-forward: fitting the model on past data only, then trading the next piece, then refitting, so
  the model never sees the future.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
