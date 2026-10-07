# Buying a dip below the lower band, but only when momentum agrees

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                                |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | One cryptocurrency pair on one exchange, bought and sold as a single position                                                                                                                                                                                                                                                                                        |
| How often it trades       | Between a few times a day on one-minute candles and a few times a month on hourly candles, depending on which of the three files is used                                                                                                                                                                                                                             |
| What you need             | A spreadsheet                                                                                                                                                                                                                                                                                                                                                        |
| Where the rules come from | [BbandRsi.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/BbandRsi.py), [Low_BB.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/Low_BB.py) and [Bandtastic.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/Bandtastic.py) |
| The underlying research   | Lento, Gradojevic and Wright, [Investment information content in Bollinger Bands?](https://www.tandfonline.com/doi/abs/10.1080/17446540701206576), and Fang, Jacobsen and Qin, [Popularity versus Profitability: Evidence from Bollinger Bands](https://acfr.aut.ac.nz/__data/assets/pdf_file/0007/29896/100009-Popularity-vs-Profitability-BB-August-Final.pdf)     |
| How well it held up       | Mixed: the lower-band buy worked on long samples before the indicator was widely known and on a Taiwanese stock sample in 2020, but its measured edge faded to nearly nothing in most markets after 2001                                                                                                                                                             |
| Also appears in           | [The relative strength index](../../fastquant/rsi/README.md) in the fastquant collection, which covers the momentum measure used here as the second condition                                                                                                                                                                                                        |

## The idea in one paragraph

A Bollinger band is a moving average with an upper and a lower rail drawn a fixed number of standard
deviations away, where a standard deviation is simply how far prices typically scatter around that
average. When the price closes below the lower rail it is unusually low relative to its own recent
average, which is often a moment to buy. The trouble is that the price can also be unusually low
simply because it is drifting down slowly. So the source files add a second, independent condition:
a momentum measure must also say the fall has been persistent. Only when both agree does the
strategy buy. The second condition removes many signals and delays the ones that remain.

## Why anyone believed it

Prices overshoot. When a large holder sells quickly, or when a wave of forced selling hits a thin
market, the price can fall further than the news justifies, because the sellers value speed over
price. A buyer who stands ready at unusually low prices is being paid to provide that speed.

The counterparty is the seller in a hurry: a leveraged trader facing a margin call, a fund raising
cash, or an automated system told to reduce risk. Those sellers keep appearing, and the more
volatile and thinly traded the pair, the more the price moves for the same amount of selling. The
second condition is there because a falling price on its own does not tell you whether you are
being offered a discount or whether the market has genuinely changed its mind about the coin.

## An everyday comparison

Think of a supermarket that cuts the price of a product when too much of it is sitting on the shelf.
A shopper who buys anything with a discount sticker will sometimes get a bargain and sometimes get
yesterday's fish. A shopper who adds a second rule, such as checking the date on the packet, buys
less often and sometimes buys later, after the shelf has been picked over, but far less often ends
up with something spoiled. The momentum condition is that date check. It cannot tell bad fish from
a bargain on its own, and it costs the shopper the items that sold out while it was being careful.

## The rules, step by step

1. Pick one cryptocurrency pair on one exchange, such as a Bitcoin pair against a stablecoin.
2. Compute the Bollinger bands. Take the typical price of each candle, which is the high plus the
   low plus the close, divided by three. Take the average of the last 20 of those values; that is
   the middle band. Work out the standard deviation of the same 20 values, where the standard
   deviation is a measure of how far apart they are. The upper rail is the middle plus two standard
   deviations and the lower rail is the middle minus two.
3. Compute a momentum index over the same candles. BbandRsi uses the relative strength index over 14
   periods, which compares the size of recent gains with the size of recent losses on a 0 to 100
   scale. Bandtastic uses the money flow index, which is the same idea but weights each candle by how
   much was traded.
4. Buy only when both conditions are true at the same time: the close is below the lower rail, and
   the momentum index is in its low region. There is no ladder and no averaging; this is a single
   position.
5. Sell when the momentum index reaches its high region, or when the profit target or the stop loss
   is reached first.
6. The exact stored settings differ between the three files.

   - BbandRsi, hourly candles: buy when the 14-period index is below 30 and the close is below the
     lower rail of the 20-period, two-standard-deviation band. Sell when the index is above 70. Stop
     loss 25 percent, profit target 10 percent, written as `minimal_roi = {"0": 0.1}`.
   - Low_BB, one-minute candles: buy when the close is at or below 98 percent of the lower rail,
     which is a second, deeper threshold on top of the band. It has no sell signal at all; the file
     sets the exit column to zero for every candle, so only the stop and the target close a
     position. Stop loss 1.5 percent. The profit target is written as
     `minimal_roi = {"0": 0.9, "1": 0.05, "10": 0.04, "15": 0.5}`, where the numbers are minutes held:
     90 percent at zero minutes, 5 percent after one minute, 4 percent after ten and 50 percent after
     fifteen. The 90 and the 50 are almost certainly a mistake in the file, and a reader should treat
     this ladder as 5 percent dropping to 4 percent.
   - Bandtastic, fifteen-minute candles: four pairs of bands at one, two, three and four standard
     deviations over 20 periods, plus the two momentum indexes and a grid of exponential averages. In
     its stored default it buys when the close is below the one-standard-deviation lower rail and the
     candle traded some volume, and sells when the money flow index is above 46 and the close is above
     the two-standard-deviation upper rail. Its stop loss is 34.5 percent and it uses a trailing stop
     that follows the price up once the profit reaches 5.8 percent.

## The maths, with every symbol named

The middle band is an average and the rails are that average plus or minus a multiple of the spread
of the same values.

```text
T_t = (H_t + L_t + C_t) / 3
M_t = (T_t + T_(t-1) + ... + T_(t-19)) / 20
S_t = square root of ( the sum of (T_i - M_t)^2 over the last 20 values, divided by 19 )
Lower_t = M_t - 2 * S_t
Upper_t = M_t + 2 * S_t
```

- `T_t` is the typical price of candle `t`.
- `H_t`, `L_t` and `C_t` are that candle's high, low and close.
- `M_t` is the middle band, the average of the last 20 typical prices.
- `S_t` is the standard deviation, dividing by 19 rather than 20 because the average itself was
  estimated from the same values.
- `Lower_t` and `Upper_t` are the two rails; about 95 percent of a stable series falls between them.

The momentum index is on a fixed scale. For the relative strength index:

```text
RSI = 100 - 100 / (1 + average_gain / average_loss)
```

- `average_gain` is the average of the upward movements in the close over the last 14 candles, with
  downward movements counting as zero.
- `average_loss` is the average of the downward movements, with upward movements counting as zero.
- The result runs from 0 to 100; below 30 means losses have recently outweighed gains by a wide
  margin.

The money flow index replaces each candle's close with its typical price and multiplies by volume:

```text
money_flow_t = T_t * V_t
MFI = 100 - 100 / (1 + sum_of_positive_flow / sum_of_negative_flow)
```

- `V_t` is the amount traded in candle `t`, in coins.
- A candle counts as positive when its typical price is above the previous candle's, and negative
  when it is below.
- `MFI` also runs from 0 to 100; below 20 is the low region the source files look for.

## A worked example

This uses the BbandRsi rule with the band shortened to ten candles so the arithmetic fits on the
page; the file uses twenty. All prices are invented but of a size crypto pairs really take. The
assumption is that the typical price equals the close, which keeps the example readable.

| Candle | Close  | Close minus mean | Squared difference |
| ------ | ------ | ---------------- | ------------------ |
| 1      | 100.00 | 0.90             | 0.81               |
| 2      | 101.00 | 1.90             | 3.61               |
| 3      | 99.00  | -0.10            | 0.01               |
| 4      | 100.00 | 0.90             | 0.81               |
| 5      | 101.00 | 1.90             | 3.61               |
| 6      | 100.00 | 0.90             | 0.81               |
| 7      | 99.00  | -0.10            | 0.01               |
| 8      | 100.00 | 0.90             | 0.81               |
| 9      | 101.00 | 1.90             | 3.61               |
| 10     | 90.00  | -9.10            | 82.81              |

The mean of the ten closes is 99.10. The squared differences add to 96.90, so the variance is
`96.90 / 9 = 10.7667` and the standard deviation is `3.2813`. The lower rail is
`99.10 - 2 * 3.2813 = 92.54`. The final close of 90.00 is below 92.54, so the band condition is met.

Now the momentum condition, over the last five changes of the close: 101 to 100 is a loss of 1.00,
100 to 99 a loss of 1.00, 99 to 100 a gain of 1.00, 100 to 101 a gain of 1.00, and 101 to 90 a loss of
11.00. Gains total 2.00 across five candles, so the average gain is 0.40. Losses total 13.00, so the
average loss is 2.60. The index is `100 - 100 / (1 + 0.40 / 2.60) = 13.33`, comfortably below 30.
Both conditions are true, so the strategy buys at 90.00.

The profit target is 10 percent, so the position is sold once the price reaches `90.00 * 1.10 =
99.00`, before any momentum exit can matter.

| Item                            | Arithmetic        | Result         |
| ------------------------------- | ----------------- | -------------- |
| Buy at 90.00 with 0.10 pct fee  | 90.00 * 1.001     | 90.09 paid     |
| Sell at 99.00 with 0.10 pct fee | 99.00 * 0.999     | 98.90 received |
| Return                          | 98.90 / 90.09 - 1 | +9.78 percent  |

Note what the ten bars show. The close was above the lower rail on nine of them, and on the tenth
the momentum index was already below 30. Requiring both is what produces a single signal in ten
candles; requiring only the band would have fired earlier and at worse prices.

## What the research actually found

| Source                                                    | What it measured                                                        | Result                                                                                                                                                                        |
| --------------------------------------------------------- | ----------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Fang, Jacobsen and Qin, Journal of Portfolio Management   | Fourteen stock markets, daily data from 1885 to 2014, one percent costs | Buy signals on the bands beat the market strongly before 1983, weakened through 1983 to 2001, and in 12 of 14 markets had no edge after 2002; the United States lost it first |
| Ni, Day, Huang and Yu, Physica A 2020                     | The 50 largest Taiwanese stocks, 2020                                   | Buying when the price hit the lower band produced positive abnormal returns, and buying the upper band worked too, which is the opposite of the contrarian reading            |
| Lento, Gradojevic and Wright, Applied Financial Economics | United States, Canadian and currency markets, 1995 to 2004              | The bands did not beat the market on their own, and the authors found signals improved when combined with other indicators, which is the case for the second condition here   |
| Balsara, Chen and Zheng, quoted in Fang, Jacobsen and Qin | Three American indices, 1990 to 2007                                    | The standard bands underperformed the market, while a contrarian version produced positive returns                                                                            |
| Bandtastic's own header comment                           | One year of data, a parameter search over 40,000 combinations           | Reports 30,918 trades, an average profit of 0.39 percent per trade and a total profit of 119.93 percent, with an objective value of -127.6                                    |

That last row is not evidence that the strategy works. It is the record of a search over 40,000
settings on one year of one author's data, and it is the clearest illustration of why the first
four rows matter more. The second condition narrows the signals and, on the combined-signal tests,
raised their quality; it did not rescue the bands where the bands had already stopped working.

## How this project relates to it

The repository's brief on
[volatility and microstructure noise](../../../strategies/books/06_volatility_and_microstructure_noise.md)
explains why a band built on an average is a fragile predictor: the price series is noisy, and much
of the apparent structure in short-term returns does not survive measurement. That is the reason
the second condition is added, and also the reason it does not help as much as it sounds.

The brief on [Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md)
covers the parameter search that produced Bandtastic's header numbers, and the brief on
[Crypto venues, AMMs and perpetual futures](../../../strategies/books/13_crypto_amm_and_perpetuals.md)
describes the crypto markets these rules are aimed at, including why fees and the gap between
buying and selling price are larger than a stock investor expects.

The momentum measure used as the second condition is explained from scratch, with its own worked
example, in [The relative strength index](../../fastquant/rsi/README.md) in the fastquant part of
this collection; this tutorial does not repeat that definition.

## Where it goes wrong

- The band is relative, not absolute. A price below the lower rail can still be far above where it
  was a year ago; the indicator says "low versus the last 20 candles", not "cheap".
- The trend is the enemy. In a sustained fall the price stays near the lower rail for weeks, and
  every signal is a losing one until the fall ends. The momentum condition reduces this but cannot
  remove it, because momentum is low all the way down.
- The second condition costs trades. Fewer signals, later entries and missed recoveries are built
  into the design, and on a one-minute file such as Low_BB the stop of 1.5 percent is smaller than
  the gap between buying and selling prices on a quiet pair.
- The evidence decayed on its own. The strongest measured edge in the literature is in the period
  before the indicator was published; after 2001 most markets showed nothing, which the authors
  attributed to everyone using the same rule.
- Costs on a fast timeframe. On hourly candles a round trip might cost 0.2 percent; on one-minute
  candles the same percentage is paid many times a day, and it is the same 0.05 to 0.10 percent per
  side every time.
- Two of the three files have defects. Low_BB's profit ladder jumps back to 50 percent, and
  Bandtastic's stored defaults leave almost all of its own conditions switched off, so its published
  header numbers belong to a different setting than the one the file ships with.

## Try it yourself

You need a spreadsheet and hourly closing prices for one coin for a year.

1. Column A: the hour. Column B: the close. Column C: the high plus the low plus the close, divided
   by three.
2. Column D: the 20-hour average of column C.
3. Column E: the 20-hour standard deviation of column C, which most spreadsheets provide directly.
4. Column F: the lower rail, which is column D minus two times column E.
5. Column G: a flag that says "band" when column B is below column F, and is empty otherwise.
6. Column H: the 14-hour relative strength index of column B, computed from average gains and losses.
7. Column I: a flag that says "buy" only when column G says "band" and column H is below 30.
8. Count the "band" flags and the "buy" flags separately.

What to notice: the "band" count is several times the "buy" count. That gap is the price of the
second condition, and it is the whole argument about whether the condition is worth keeping. Now
sort the "buy" rows by what happened over the following day; if the wins are no more frequent than
the losses, the second condition added delay without adding information.

## Where this came from

- [BbandRsi.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/BbandRsi.py),
  the hourly band plus relative strength rule.
- [Low_BB.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/Low_BB.py),
  the one-minute rule with the 98 percent threshold and no sell signal.
- [Bandtastic.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/Bandtastic.py),
  the four-band file with the trailing stop and the search header.
- John Bollinger, Bollinger on Bollinger Bands, 2001, the original description of the bands and the
  20-period, two-standard-deviation default.
- Fang, Jacobsen and Qin, [Popularity versus Profitability: Evidence from Bollinger Bands](https://acfr.aut.ac.nz/__data/assets/pdf_file/0007/29896/100009-Popularity-vs-Profitability-BB-August-Final.pdf),
  the study of how the measured edge faded after the indicator became popular.
- Ni, Day, Huang and Yu, [The profitability of Bollinger Bands: Evidence from the constituent stocks of Taiwan 50](https://ideas.repec.org/a/eee/phsmap/v551y2020ics0378437120300078.html),
  the 2020 sample in the table above.
- [Volatility and microstructure noise](../../../strategies/books/06_volatility_and_microstructure_noise.md),
  this repository's brief on why short-horizon price structure is hard to measure.

## Words used in this tutorial

- Bollinger band: a moving average with rails placed a chosen number of standard deviations above
  and below it.
- momentum: the tendency of a price move to persist for a while; here measured on a 0 to 100 scale.
- moving average: the mean of the last N values, recomputed after each new value.
- relative strength index: a 0 to 100 momentum measure comparing recent gains with recent losses.
- standard deviation: a measure of how widely a set of values is spread around its average.
- trailing stop: a stop that follows the price upward, locking in part of a rising profit.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
