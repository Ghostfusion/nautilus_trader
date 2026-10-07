# Wavelet plus support vector machine: splitting a price line into its slow drift and its fast wiggles

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                            |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | One currency pair, the euro against the Japanese yen, held as a bet on the next day's direction                                                                                                  |
| How often it trades       | A fresh forecast and a fresh bet every day                                                                                                                                                       |
| What you need             | Python and a data file                                                                                                                                                                           |
| Where the rules come from | [QuantConnect strategy library, SVM wavelet forecasting](https://www.quantconnect.com/tutorials/strategy-library/svm-wavelet-forecasting)                                                        |
| The underlying research   | Raimundo and Okamoto, [SVR-wavelet adaptive model for forecasting financial time series](https://doi.org/10.1109/INFOCT.2018.8356851), ICICT 2018                                                |
| How well it held up       | Weak: a single currency pair over a single period, backtested once by the library page, with a reward for risk (0.553) only slightly above buying and holding (0.463) and no cost figure printed |
| Also appears in           | nothing else in this collection                                                                                                                                                                  |

## The idea in one paragraph

Take a long record of a price, such as 152 daily closes of the euro against the yen. Split that line
into two kinds of piece: a smooth, slowly changing part that carries the drift, and several fine,
fast-changing parts that carry the day-to-day wiggles. The small wiggles are noise, so set the tiniest
ones to zero. Now ask a curve-fitting method, a support vector machine, to look at each piece and guess
one step beyond the end of it. Put the pieces back together, shifted forward by one day with the guesses
at the new end, and read off a forecast price for tomorrow. If the forecast is above today's price the
rules buy; if below, they sell. The size of the bet is set to the size of the forecast move, so a big
predicted move means a big position.

## Why anyone believed it

A price is a mixture of a real, persistent drift and a great deal of random noise, and the argument is
that a method which separates the two should learn faster from the drift and stop chasing the noise.
Much of the noise in prices is known to reverse: a sudden jump up is often followed by a small drop
back, which is exactly the kind of short-lived wiggle that the detail parts hold and the smoothing part
drops. The counterparty is the impatient trader who reacts to the noise itself, buying after a spike
and selling after a dip, and whose predictable reaction the smoothed forecast is trying to be on the
other side of. If that impatient flow keeps appearing, the smooth part stays the better guide.

## An everyday comparison

Think of a year of daily outdoor temperatures. There are two things going on at once: the slow swing of
the seasons, which you can predict months ahead, and the day-to-day weather, which is much harder. If
someone asked you for tomorrow's temperature, you would not look only at today's gust of wind; you would
look at the season, and then add a small correction for the weather. Splitting a price into a smooth
part and a detail part is the same move. The smooth part is the season, the detail parts are the weather,
and the method throws away the weather that is too small to matter and then extends both.

## The rules, step by step

1. Collect at least 152 consecutive daily closing prices of the euro against the yen. The page derives
   152 from its choice of a three-level split, because each level halves the number of points.
2. Feed the last 152 closes into the splitter. The page uses a shape called Symlets 10 and three levels,
   which produces four parts: one smooth part and three detail parts.
3. For each of the three detail parts, set to zero every value whose size is less than half the largest
   value in that same part. This is the page's threshold of 0.5, and it is not applied to the smooth
   part, which is kept whole.
4. For each of the four parts, fit a support vector machine to its recent values and ask the machine for
   one value beyond the end. The machine is the page's forecasting step; it draws the smoothest curve
   through the part and extends that curve by one step.
5. Shift every part forward by one step and place the machine's guess at the newly empty end.
6. Add the four shifted parts back together. The final value of the sum is the forecast price for
   tomorrow.
7. Divide the forecast price by today's close and subtract one. This is the forecast percentage change,
   and its sign is the forecast direction.
8. Emit the bet in that direction, with the size of the bet equal to the absolute value of the forecast
   percentage change. A forecast of plus 0.2 percent means a bet worth 0.2 percent of the account, and a
   forecast of minus 0.4 percent means a short bet worth 0.4 percent.
9. Repeat from step 1 the next day, using the newest 152 closes.

## The maths, with every symbol named

Splitting a line into a smooth part and a detail part, shown on two neighbouring values, is only adding
and subtracting:

```text
a = (x1 + x2) / 2      the smooth value, the average of the pair
d = (x1 - x2) / 2      the detail value, half the gap between them
```

- `x1` and `x2` are two consecutive prices.
- `a` is the smooth value for the pair, an average.
- `d` is the detail value for the pair; it is zero when the two prices are equal, and its sign tells you
  which of the two was the higher.

Putting them back together is the reverse:

```text
x1 = a + d
x2 = a - d
```

- Adding the two pieces returns the higher price; subtracting them returns the lower one. Nothing is
  lost, which is why this is a lossless split rather than an approximation.

Applying the same two formulas again to the list of smooth values gives the next level, so three levels
turn 152 prices into one smooth part and three detail parts, each shorter than the last. The denoising
step keeps only the large details:

```text
d_kept = d if |d| > 0.5 * largest |d| in this part, otherwise 0
```

- `|d|` is the size of the detail value, ignoring its sign.
- `0.5` is the page's threshold, chosen there as a strength between 0 and 1.
- Setting the small details to zero is what removes the wiggles that are too small to matter.

The forecast, the bet size and the result are then:

```text
f = P_forecast / P_today - 1
size = |f|
net return = size * sign(f) * R_actual - cost
```

- `P_forecast` is the reassembled forecast price from the last step.
- `P_today` is today's closing price.
- `f` is the forecast change as a decimal; its sign is the direction and its size, ignoring the sign, is
  `size`, the fraction of the account placed on the bet.
- `R_actual` is the change that actually happened over the next day, as a decimal.
- `cost` is what the two sides of the bet cost; the page uses the library's weighting model, which turns
  the size into the allocation, but prints no cost figure, so the figure used below is an assumption.

## A worked example

Eight daily closes of the euro against the yen are invented below; they are of the size that pair
actually takes. First, one level of the split, pairing the days two at a time.

| Pair | Prices         | Smooth value | Detail value | Kept?                                          |
| ---- | -------------- | ------------ | ------------ | ---------------------------------------------- |
| 1    | 160.00, 160.40 | 160.20       | -0.20        | kept                                           |
| 2    | 160.20, 160.80 | 160.50       | -0.30        | kept                                           |
| 3    | 160.60, 161.00 | 160.80       | -0.20        | kept                                           |
| 4    | 161.40, 161.20 | 161.30       | +0.10        | removed, because 0.10 is below the 0.15 cutoff |

The largest detail value is 0.30, so the cutoff is 0.5 times 0.30, which is 0.15. The fourth pair's
detail of plus 0.10 is smaller than that, so it is set to zero. Rebuilding that pair from the smooth
value of 161.30 and a detail of zero gives 161.30 twice, which shows what denoising does: it smooths the
last two days into the same value.

Now the support vector machine extends each part by one step. Suppose it forecasts the next smooth value
as 161.55 and the next detail value as -0.10. Reassembling gives a forecast price of 161.55 - 0.10 =
161.45. Today's close is 161.20, so the forecast change is 161.45 / 161.20 - 1 = 0.0016, that is plus
0.16 percent, and the rules would buy. The bet size is 0.0016 of the account.

The table below runs that logic over six days on a 100,000 dollar account. The forecast prices are
invented for the example, the two basis points per side cost is an assumption, and the point is the
arithmetic: the bet size is the forecast change, so the whole position is always a tiny fraction of the
account.

| Day | Close  | Forecast | f (%)  | Exposure | Side  | Next close | Actual (%) | Gross | Cost  | Net   |
| --- | ------ | -------- | ------ | -------- | ----- | ---------- | ---------- | ----- | ----- | ----- |
| 1   | 160.00 | 160.32   | +0.200 | $200     | long  | 160.40     | +0.250     | $0.50 | $0.08 | $0.42 |
| 2   | 160.40 | 160.24   | -0.100 | $100     | short | 160.20     | -0.125     | $0.12 | $0.04 | $0.08 |
| 3   | 160.20 | 160.68   | +0.300 | $300     | long  | 160.80     | +0.375     | $1.12 | $0.12 | $1.00 |
| 4   | 160.80 | 160.56   | -0.149 | $149     | short | 160.60     | -0.124     | $0.19 | $0.06 | $0.13 |
| 5   | 160.60 | 161.24   | +0.399 | $399     | long  | 161.00     | +0.249     | $0.99 | $0.16 | $0.83 |
| 6   | 161.00 | 161.60   | +0.373 | $373     | long  | 161.40     | +0.248     | $0.93 | $0.15 | $0.78 |

The gross column adds to $3.85, the cost column to $0.61, and the net column to $3.24, which is about
0.003 percent of the account over six days. Two things are worth noticing. First, the bet size equals
the forecast move, so the strategy almost never risks much: a forecast of 0.4 percent puts only 0.4
percent of the account at stake. Second, four of the six days were right, yet the money made is small
precisely because the right days and the wrong days were of similar size. A high hit rate and a small
profit are not the same thing.

## What the research actually found

| Source                                             | What it measured                                                   | Result                                                                                                                                                          |
| -------------------------------------------------- | ------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| QuantConnect library page, SVM wavelet forecasting | Its own backtest of the rule on daily euro-yen closes              | Reward for risk of 0.553, against 0.463 for buying and holding the American index over the same period                                                          |
| Raimundo and Okamoto, ICICT 2018                   | The paper the page is built from, an adaptive split-then-fit model | The paper proposes the method and reports it on time series; it is a methods paper, and the page's own backtest is the trading evidence rather than the paper's |
| `2404.16467v1` (abstract)                          | Wavelet coefficients used to classify price jumps in stocks        | The wavelet view recovers volatility asymmetry and separates mean-reverting from trending jumps; this is a description of price behaviour, not a trading result |
| `2401.06139v2` (abstract)                          | A wavelet split feeding a learning model on Chinese stock returns  | The model is reported to beat several alternatives and to hold up in falling and choppy markets, but the abstract does not report costs or a live record        |
| `2110.14914v2` (pp.5, 7, 8)                        | Classifiers trained to call the next move on futures, then traded  | 52 to 56 percent accuracy, yet many configurations were unprofitable once normal slippage was charged                                                           |

The page's own comparison is a thin win: 0.553 against 0.463 is real but small, it rests on one currency
pair over one period, and the page prints no trading costs. The paper underneath the page is a methods
paper, not a large-scale trading study, so it establishes that the technique can be built, not that it
makes money. The wavelet literature read for this tutorial does show that the split has meaning: the
jump-classification work finds that the detail coefficients carry genuine structure about how a market
reacts. That is a statement about describing prices, however, and describing a price well is not the
same as forecasting its next move. The trading-side caution comes from the selective-classification
study, where a classifier right 55 percent of the time still lost money after realistic costs, because
the days it was wrong on cost more than the days it was right on.

## How this project relates to it

- [Machine Learning for Trading](../../../strategies/books/09_machine_learning_for_trading.md)
  is the closest match. Section 6 on costs, turnover and loss design is the one to read alongside this
  tutorial, because it makes the point that a one-day-ahead signal which is right a little more often
  than not still has to clear the cost of trading every day.
- [Machine learning for finance](../../../strategies/books2/04_machine_learning_for_finance.md)
  is where the difference between prediction error and portfolio outcome is documented, including the
  matched experiment in which changing the training objective moved the portfolio result more than
  changing the model did.
- [Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md)
  covers the specific danger of this page's last paragraph, which invites the reader to try other
  wavelets and other time resolutions. Each such trial is a fresh chance to find a setting that looks
  good on the past; the brief's rule is that the number of trials has to be recorded with the winner.

The repository does not implement a wavelet predictor. The nearest implemented thing is the validation
discipline those briefs describe, which is what a reader would use to judge this method if the code were
ported here.

## Where it goes wrong

- Overfitting the fit. A support vector machine fitted to a short recent window will follow that window
  closely; when the window is noise, the model has learned the noise. The page's suggestion to try other
  wavelets makes this worse, because every combination tried is another chance to find a lucky setting.
- The horizon is one day, which is the hardest horizon. Over a single day the drift that the smooth part
  is meant to capture is tiny compared with the noise, so the forecast is very close to a coin toss even
  when the method is working as designed.
- A right call is not a profitable one. A forecast can point the correct way and still lose money, and
  a high hit rate can still mean a losing year; the worked example and the accuracy figures above show it.
- The split uses future data unless it is built carefully. A whole-series wavelet transform computes
  each value using neighbours on both sides, so if the split is run once over the whole file, the
  values for an early day already contain information from later days. The honest version refits the
  split at every step using only data up to that step, and the page does not say it does.
- The threshold of 0.5 is a free choice. It decides how much of the wiggle is thrown away, and it was
  set to 0.5 rather than chosen by evidence. Change it and the forecast changes.
- Costs and turnover. The strategy trades every day, buying and selling a fraction of the account each
  time. The page prints no cost. A daily round trip at two basis points per side costs about 0.04
  percent of the traded amount per day, which is small only because the traded amount is small.
- Betting more on bigger forecasts. The bet size is the size of the forecast move, so the largest
  positions are taken on the largest predictions. Those are also the predictions most likely to be
  errors, which is the opposite of what a cautious system would do.

## Try it yourself

You need a spreadsheet and a public source of daily closes for the euro against the yen. No money and no
code are involved.

1. Put 20 consecutive closes in column A, one per row.
2. In column B, average each pair of neighbours: the average of rows 1 and 2 goes in row 1, the average
   of rows 3 and 4 in row 3, and so on down the pairs. This is the smooth part at the first level.
3. In column C, subtract the second price of each pair from the first and divide by two. This is the
   detail part.
4. Find the largest size in column C, ignoring signs, and write it in a cell. Half of that number is the
   cutoff.
5. In column D, copy each detail value from column C, but write zero wherever its size is below the
   cutoff. This is the denoised detail part.
6. In column E, rebuild the pair from the smooth and denoised values: the first price is the smooth
   value plus the kept detail, and the second is the smooth value minus the kept detail.
7. Compare column A with column E.

What to notice: column E is a smoother version of column A, with the little back-and-forth pairs
flattened out wherever the detail was small. Now look at the days near a large move, where the detail
was kept: those are the days the method says are real and worth learning from. Ask yourself which of
those large moves were followed by more of the same movement and which reversed. The answer is not the
same every time, and that mix is why the forecast is so hard: the method can separate the pieces, but
it cannot tell you in advance which large detail is the start of a trend and which is a one-day spike.

## Where this came from

- [QuantConnect: SVM wavelet](https://www.quantconnect.com/tutorials/strategy-library/svm-wavelet-forecasting),
  the rules as implemented: 152 daily closes, a three-level Symlets 10 split, a threshold of 0.5 on the
  detail parts, a one-step support vector machine forecast per part, and a bet size equal to the
  forecast change. The research page is at
  [research/15371](https://www.quantconnect.com/research/15371/svm-wavelet-forecasting/).
- Raimundo and Okamoto, "SVR-wavelet adaptive model for forecasting financial time series", ICICT 2018,
  [doi 10.1109/INFOCT.2018.8356851](https://doi.org/10.1109/INFOCT.2018.8356851), the paper the page
  cites. This paper is not in the repository's harvested corpus, so it is named but not quoted.
- `2404.16467v1`, "Riding Wavelets: A Method to Discover New Classes of Price Jumps", the source for
  what the wavelet detail coefficients carry.
- `2401.06139v2`, "Stockformer: A Price-Volume Factor Stock Selection Model Based on Wavelet Transform
  and Multi-Task Self-Attention Networks", a larger wavelet-plus-learning model and its claimed results.
- `2110.14914v2`, "Trading via Selective Classification", the source for the accuracy-versus-cost
  finding.
- [Machine Learning for Trading](../../../strategies/books/09_machine_learning_for_trading.md),
  [Machine learning for finance](../../../strategies/books2/04_machine_learning_for_finance.md) and
  [Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's own briefs on the questions this method raises.

## Words used in this tutorial

- denoising: setting small values to zero so that only the large ones survive.
- detail part: the piece of a price line that holds the fine, fast wiggles, made by subtracting
  neighbouring prices.
- exposure: the fraction of the account actually placed on a bet.
- overfitting: a model that has memorised the past it was shown rather than learned a pattern that
  carries into new data.
- reward for risk (Sharpe ratio): the average return divided by how much it wobbled, usually written per
  year; zero means no reward after the wobble is counted.
- smooth part: the slowly changing piece of a price line, made by averaging neighbouring prices.
- support vector machine: a curve-fitting method that draws the smoothest line through a set of points
  and extends it.
- wavelet: a small, wave-like shape used to split a series into pieces of different speed.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
