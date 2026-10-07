# Reading a second market as a filter: the informative pair

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                  |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | A smaller cryptocurrency priced in United States dollars, watched on five-minute candles, while Bitcoin against the dollar is watched at the same time on fifteen-minute candles                                                                                                       |
| How often it trades       | The five-minute rule alone fires many times a day; the Bitcoin filter removes some of the stretches when the whole market is drifting down                                                                                                                                             |
| What you need             | Nothing but this page, or a spreadsheet and two price series if you want to repeat the arithmetic                                                                                                                                                                                      |
| Where the rules come from | [InformativeSample.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/InformativeSample.py), which states the rules, and its companion [hlhb.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/hlhb.py)           |
| The underlying research   | [Liu, Tsyvinski and Wu, Common Risk Factors in Cryptocurrency](https://www.nber.org/papers/w25882) for the shared movement, and [Sifat, Mohamad and Mohamed Shariff, Lead-Lag relationship between Bitcoin and Ethereum](https://ideas.repec.org/a/eee/riibaf/v50y2019icp306-321.html) |
| How well it held up       | Weak: the file's own description says it is "not performing very well", the shared movement between coins is documented, and the lead-lag study reports that intraday traders can barely exploit it                                                                                    |
| Also appears in           | Nothing else in this collection; [reading several timeframes at once](../multiple-timeframes/README.md), in this same group, is the same trick applied to one coin's own slower charts                                                                                                 |

## The idea in one paragraph

Trade a small cryptocurrency, but only when Bitcoin agrees. The strategy watches the small coin on
five-minute candles and asks whether its short-term average price sits above its longer-term average:
if so, the small coin is drifting up. At the same moment it watches Bitcoin against the dollar on
fifteen-minute candles and asks whether Bitcoin's price sits above its own twenty-period average: if
so, the whole market is drifting up. It buys only when both answers are yes, and it sells when both
turn to no. Bitcoin is the largest and most watched market in crypto, so its direction stands in for
the direction of the market as a whole.

## Why anyone believed it

A cryptocurrency is not priced on its own merits alone. Money enters and leaves the whole market at
once, and Bitcoin is the coin that traders use to move in and out: someone who wants to hold a small
coin usually has to buy it with Bitcoin or with a coin priced like Bitcoin, so when Bitcoin falls the
small coin is sold too. The small coin is also noisier than Bitcoin, which means its own five-minute
chart gives many false starts, while Bitcoin's fifteen-minute chart reflects the same market with
less noise. Filtering the small coin's signal with the big market's direction should therefore trim
the entries that were fighting the market.

The counterparty is the same one a trend rule usually faces: traders who are slow to react, and
traders who must sell for reasons unrelated to the coin, such as a fund withdrawing money or a
holder taking a profit. If the market as a whole keeps drifting up, that selling is absorbed and the
drift continues, and a rule that checks the big market first avoids buying just as that absorption
stops.

## An everyday comparison

A small stream runs past a house, and its level tells you very little: it rises after a shower
upstream, then falls again while the sky here is clear. The river a mile up the valley tells you much
more, because it gathers all the rainfall of the region. A person deciding whether to take a boat
down the stream checks the river gauge first. The stream is the small coin and the river is Bitcoin:
the outside series does not predict the stream exactly, it just tells you whether the whole valley is
getting wetter, which is most of what the stream's level is made of.

## The rules, step by step

1. Choose the coin to trade, called the traded pair. It is quoted against a dollar-like coin, so its
   price is a number of dollars, and it is observed on five-minute candles. A candle is the opening,
   highest, lowest and closing price of one interval; a five-minute candle covers five minutes.
2. Compute two exponential moving averages of the traded coin's closing prices, one over twenty
   candles and one over fifty. An exponential moving average is an average that leans more on the
   newest prices, so it turns sooner than a plain average.
3. Set up a second series, called the informative pair: Bitcoin against the same dollar-like coin,
   on fifteen-minute candles, and compute the twenty-candle simple moving average of its closing
   prices. A simple moving average is the plain average of the last twenty values.
4. Buy when both of the following are true at once: the traded coin's twenty-candle average is above
   its fifty-candle average, and Bitcoin's latest fifteen-minute close is above its twenty-candle
   average.
5. Because the two series run on different clocks, the last completed Bitcoin value is carried
   forward and used by the three five-minute candles that fall inside it. The rules never look at a
   Bitcoin candle that has not yet finished.
6. Sell when both conditions are false: the traded coin's twenty-candle average is below its
   fifty-candle average, and Bitcoin's close is below its twenty-candle average.
7. A loss of ten percent closes the trade at any moment, whatever the averages say. This is the stop
   loss, the fixed loss at which the rule admits the trade is wrong.
8. A profit ladder closes the trade if it is ahead by five percent at any time, four percent after
   twenty minutes, three percent after thirty minutes, or one percent after sixty minutes. A ladder
   like this is called a minimal return on investment table.
9. Only one position is open at a time; the file does not allow a second purchase in the same pair.
10. As a contrast, the same repository holds a well-known short-term system, [hlhb.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/hlhb.py),
    which uses no second pair at all. Its informative list is empty. Instead it measures the strength
    of the traded pair's own trend with a number called the ADX, and trades only when that number is
    above twenty-five. The contrast is the whole point of this page: a filter can be taken from an
    outside series, or from a second measurement inside the one series.

## The maths, with every symbol named

The shared movement that makes the filter plausible is correlation. For two series of returns, one
value per period, correlation is:

```text
rho = covariance(x, y) / (sd(x) * sd(y))
```

- `rho` is the correlation, a number between minus one and plus one.
- `x` and `y` are the two series of returns, with one pair of values per period.
- `covariance(x, y)` is the average of `(x_i - mean_x) * (y_i - mean_y)` over all periods.
- `mean_x` and `mean_y` are the ordinary averages of each series.
- `sd(x)` and `sd(y)` are the standard deviations, the typical distance of each value from its own
  average.

A correlation near plus one means the two series rise and fall together; near zero means neither
tells you anything about the other; near minus one means they move in opposite directions. If the
small coin and Bitcoin have a high positive correlation, then the Bitcoin condition throws away
exactly the days when the small coin is falling with the market, and that is the reason to use it.

The two conditions are combined with "and", so the signal exists only where both are true:

```text
enter = (ema20 > ema50) and (btc_close > btc_sma20)
```

- `ema20` and `ema50` are the two exponential moving averages of the traded coin's close.
- `btc_close` is the most recently completed fifteen-minute Bitcoin close.
- `btc_sma20` is the twenty-period simple average of those Bitcoin closes.

An exponential moving average is built one candle at a time:

```text
ema_today = close_today * k + ema_yesterday * (1 - k), with k = 2 / (N + 1)
```

- `close_today` is the closing price of the newest candle.
- `k` is the weight given to it; for twenty candles `k` is 2 / 21, about 0.0952.
- `N` is the number of candles in the average, twenty or fifty here.

Finally the cost. If a fraction `c` of the amount traded is charged on each side, the round trip
costs `2 * c` of the position. A realistic figure for a major coin on a large exchange is 0.05 to
0.10 percent per side, that is 0.1 to 0.2 percent per round trip, and more in a thin pair because the
gap between the buying and selling price is wider.

## A worked example

First the Bitcoin side. Twenty fifteen-minute closes, in thousands of dollars:

```text
60.0 60.2 59.8 60.5 61.0 61.4 61.1 60.7 60.3 60.0
59.6 59.2 58.9 58.6 58.4 58.6 58.9 59.3 59.8 60.1
```

They add up to 1196.4, so the twenty-period average is 1196.4 / 20 = 59.82. The newest close, 60.1,
is above 59.82, so the Bitcoin condition is true at that moment. One fifteen-minute candle later the
average has risen to 60.04 while the newest close is 59.90, so the condition is false again.

Now the traded coin, which we call ALTC and price in dollars. Its two averages are given as the
software computed them from the closes shown.

| Five-minute candle | ALTC close | ema20 | ema50 | Coin trending up? | Bitcoin filter on? | Both true? | Action                         |
| ------------------ | ---------- | ----- | ----- | ----------------- | ------------------ | ---------- | ------------------------------ |
| 1                  | 2.000      | 1.980 | 2.000 | no                | no                 | no         | wait                           |
| 2                  | 2.010      | 1.987 | 1.999 | no                | no                 | no         | wait                           |
| 3                  | 2.030      | 1.999 | 1.998 | yes               | no                 | no         | wait, filter refuses           |
| 4                  | 2.040      | 2.013 | 1.998 | yes               | no                 | no         | wait, filter refuses           |
| 5                  | 2.050      | 2.025 | 1.999 | yes               | yes                | yes        | buy at the next open           |
| 6                  | 2.060      | 2.036 | 2.001 | yes               | yes                | yes        | hold                           |
| 7                  | 2.100      | 2.055 | 2.006 | yes               | yes                | yes        | hold                           |
| 8                  | 2.160      | 2.080 | 2.014 | yes               | yes                | yes        | profit ladder closes the trade |
| 9                  | 2.140      | 2.092 | 2.023 | yes               | no                 | no         | flat                           |
| 10                 | 2.120      | 2.098 | 2.031 | yes               | no                 | no         | flat                           |

The buy happens at the opening price of candle 6, which is the closing price of candle 5, 2.050. The
sell signal never fires, because it needs the coin's own trend to break as well. The profit ladder
closes the trade at candle 8, where the position is 2.160 / 2.050 - 1 = 5.37 percent ahead, above the
five percent it asks for during the first twenty minutes. The account buys 500 coins, and the
exchange charges 0.10 percent of the amount traded on each side:

```text
Buy   500 coins at 2.050 = 1025.00, fee 0.10% = 1.0250, total paid 1026.03
Sell  500 coins at 2.160 = 1080.00, fee 0.10% = 1.0800, total received 1078.92
Profit 1078.92 - 1026.03 = 52.89
Return 52.89 / 1026.03 = 0.0515, about 5.16 percent
```

The price rose 5.37 percent gross, and the fees took about 0.20 percent, leaving 5.16 percent. Now
the honest comparison. A trader who ignored the Bitcoin filter would have bought at the opening price
of candle 4, which is 2.030, and the same ladder would have closed the position at the same candle,
because at candle 7 the position was only 3.45 percent ahead:

```text
Buy   500 at 2.030 = 1015.00, fee 1.0150, total paid 1016.02
Sell  500 at 2.160 = 1080.00, fee 1.0800, total received 1078.92
Return 62.90 / 1016.02 = 0.0619, about 6.19 percent
```

So in this one example the filter cost about one percentage point, because it refused two candles
that turned out to be profitable. In a falling market the same refusal would have avoided a loss
instead. One example settles nothing: it only shows how to apply the rules and how the arithmetic
behaves.

## What the research actually found

The idea that coins move together is not folklore; it is measured. Liu, Tsyvinski and Wu report that
three factors, the cryptocurrency market itself, size and momentum, capture the expected returns of
cryptocurrencies, and that nine factors built from the stock-market literature form profitable
long-short strategies whose profits are explained by those three. A factor here means a shared
characteristic that moves many assets at once. Their market factor is the direct ancestor of the
Bitcoin condition: it says that most of what one coin does is what the whole market does.

Whether the lead can be turned into a trade is a separate question, and the evidence is discouraging.
Sifat, Mohamad and Mohamed Shariff studied Bitcoin and Ethereum on one year of hourly and daily data,
from August 2017 to September 2018, using several statistical tests for who moves first. They found
largely two-way causation rather than a clean leader, and they conclude that intraday traders can
barely exploit the Bitcoin-Ethereum price-discovery process to their advantage. The Eross,
McGroarty, Urquhart and Wolfe study of Bitcoin's intraday behaviour makes the same point from the
other side: within Bitcoin, returns, volume and volatility are related to each other in both
directions at once, and the search for a single clean leader finds none.

What the source file itself says is the most useful evidence of all. Its description reads that it is
"[n]ot performing very well - but should serve as an example how to use a referential pair against
USDT". That is the author telling the reader that the file exists to demonstrate the machinery, not
to trade.

## How this project relates to it

This repository does not run any Freqtrade strategy, so there is no code here that reads a second
pair. What it does have is the research on what makes a second series trustworthy.
[Market data quality](../../../strategies/books2/27_market_data_quality.md) is the brief on series
that are wrong in ways that survive cleaning, including prices that are correctly recorded but
measure the wrong thing; a filter built on a second exchange's Bitcoin price inherits exactly that
problem. [Order flow and toxicity](../../../strategies/books/07_order_flow_and_toxicity.md) collects
who leads whom across venues and finds the answer depends on the venue and the day, which is the
closest thing here to a measurement of the lead-lag this strategy assumes. The general mistake of
testing a rule on data that includes the future is set out in
[how a backtest lies](../../foundations/07_how-a-backtest-lies.md), and the specific danger with two
clocks is a merge that lets a five-minute candle see an unfinished fifteen-minute value.

## Where it goes wrong

- Correlation is not protection. If Bitcoin falls, a correlated coin falls with it; the filter does
  not remove market risk, it only delays the entry until the big market turns up. Everything the
  filter buys is still exposed to Bitcoin turning down afterwards.
- The filter throws away trades. As the worked example shows, it refuses entries that would have
  made money. A rule that trades half as often has half the evidence, so its result is easier to
  mistake for skill.
- Two clocks invite a look-ahead error. If the merge attaches the newest, still-forming fifteen-minute
  Bitcoin value to the last five-minute candle, the strategy is reading tomorrow's newspaper. This
  repository keeps deliberately broken files in a folder named `lookahead_bias` for exactly this kind
  of mistake.
- The outside series can be wrong, or the wrong series. It comes from an exchange and a quote currency
  that need not match the traded pair, and a stale or missing Bitcoin candle silently removes the
  filter rather than warning the reader.
- Crowding. Everyone in crypto watches Bitcoin, and a filter everyone can see stops being
  information. The lead-lag study above is one measurement of that.
- The thresholds are unmeasured. Why twenty candles on Bitcoin and not ten, why the twenty and fifty
  averages on the coin, is not stated anywhere and was not tested against an alternative.

## Try it yourself

You need a spreadsheet and two public price series: one small coin against the dollar, and Bitcoin
against the dollar.

1. Put five-minute candles of the small coin in one sheet, with columns `time` and `close`.
2. Put fifteen-minute Bitcoin closes in a second sheet, with columns `time` and `close`, and add a
   third column `btc average` that averages the last twenty closes.
3. Copy the Bitcoin average across to the first sheet by matching each five-minute time to the most
   recent fifteen-minute time that has already finished. Never use a Bitcoin candle whose interval
   still contains the five-minute candle you are looking at.
4. Add an `above btc average` column that says yes when the Bitcoin close is above that average.
5. Add columns for the small coin's twenty-candle and fifty-candle averages, and an `enter` column
   that says buy when the coin's average is above the longer one and `above btc average` is yes.
6. Walk down the rows, and each time a trade opens, follow it until either condition fails, then
   subtract 0.20 percent for the round trip.
7. Do the whole walk a second time with the Bitcoin condition ignored, and compare the two lists of
   trades.

What to notice: the two lists contain many of the same trades, and the filter mainly removes entries
that happen while Bitcoin is below its own average. Whether that helped depends entirely on what
Bitcoin did next. Now count how many trades each version made. The filtered version usually makes
noticeably fewer, and fewer trades means a result that is much easier to get by luck, which is the
reason this page grades the idea Weak.

## Where this came from

- [InformativeSample.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/InformativeSample.py),
  the source of the buy and sell conditions, the ten percent stop, the profit ladder and the empty
  reference to Bitcoin against the dollar on fifteen-minute candles.
- [hlhb.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/hlhb.py),
  the same repository's short-term system, cited here for its empty informative list and its ADX
  filter, and described at
  [the BabyPips explanation of the HLHB system](https://www.babypips.com/trading/forex-hlhb-system-explained)
  that the file names.
- [The Freqtrade strategy documentation](https://www.freqtrade.io/en/stable/strategy-customization/),
  which explains how a second pair and a second timeframe are fetched and merged, and warns about
  future data in backtests.
- Yukun Liu, Aleh Tsyvinski and Xi Wu, [Common Risk Factors in Cryptocurrency](https://www.nber.org/papers/w25882),
  NBER working paper 25882 (2019), published in the Journal of Finance 77(2), pages 1133 to 1177
  (2022), for the shared market factor behind coins.
- Imtiaz Mohammad Sifat, Azhar Mohamad and Mohammad Syazwan Bin Mohamed Shariff,
  [Lead-Lag relationship between Bitcoin and Ethereum](https://ideas.repec.org/a/eee/riibaf/v50y2019icp306-321.html),
  Research in International Business and Finance 50, pages 306 to 321 (2019), for the measured
  difficulty of trading the lead.
- [Market data quality](../../../strategies/books2/27_market_data_quality.md), this repository's
  brief on series that are wrong in ways cleaning cannot fix.

## Words used in this tutorial

- candle: the opening, highest, lowest and closing price of one fixed interval of trading.
- correlation: a number between minus one and plus one describing how much two series move together.
- exponential moving average: an average of recent prices that gives more weight to the newest values.
- informative pair: a second price series, from another coin or another timeframe, that a strategy
  reads but does not trade.
- moving average: the plain average of the last few values, recalculated as time moves on.
- stop loss: the fixed loss at which a trade is closed, here ten percent below the entry price.
- trend: a stretch in which a price moves mainly in one direction.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
