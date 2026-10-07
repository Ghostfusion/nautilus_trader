# Buying only when the money flow agrees with the price

Date: 2026-10-07. Revision 1.

| Field                     | What                                                                                                                                                                                                                        |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | One cryptocurrency pair on one exchange, bought and held as a single position                                                                                                                                               |
| How often it trades       | A few times a week on fifteen-minute candles, when three separate measures all reach their extreme at once                                                                                                                  |
| What you need             | A spreadsheet                                                                                                                                                                                                               |
| Where the rules come from | [CMCWinner.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/CMCWinner.py)                                                                                                 |
| The underlying research   | Cong, Li, Tang and Yang, [Crypto Wash Trading](https://www.nber.org/papers/w30783), 2022, for what the volume figure is actually worth                                                                                      |
| How well it held up       | Weak: the three measures are practitioner inventions with no independent published test, and the one measured fact about the ingredient they rely on is that reported volume on many crypto exchanges is largely fabricated |
| Also appears in           | Nothing else in this collection                                                                                                                                                                                             |

## The idea in one paragraph

Three different measures have to agree before this strategy buys. One measures how far the price is
from its own recent average; one measures the balance of recent gains and losses; and one does the
same as the second but weighs each candle by how much was traded. All three must be at an extreme
low at the same time. The bet is that a price fall backed by heavy trading is more likely to be an
exhausted panic than a quiet drift, and that requiring the volume-aware measure to agree filters out
falls that nobody is actually trading. The strategy then sells when the same three measures reach
their extreme high together.

## Why anyone believed it

A price can fall for two very different reasons. It can fall because order after order is hitting
the market, which means real sellers are meeting real buyers at lower and lower prices; or it can
fall in a thin market where one small order moves the last traded price, which means almost nothing
has changed. The first kind of fall tends to end in a visible panic and a bounce; the second kind
contains no information at all.

The counterparty is the forced seller during a high-volume panic: a leveraged position being
liquidated, or a holder selling into a falling market out of fear. That seller does not care about
price, only about getting out, and the buyer on the other side is paid for taking the other side of
a frightening move. Requiring the volume-weighted measure to agree is an attempt to insist that the
move is the first kind, not the second.

## An everyday comparison

A nightclub can make itself look popular by paying people to stand in a queue outside. A passer-by
who joins the longest queue is treated to the same music either way, but if the queue was rented,
the club will be empty inside and the night will be disappointing. Checking that the queue is
moving, and that people inside are actually buying drinks, is the volume test. It is a useful check
up to the point where the club starts paying those people to buy drinks too. That last problem is
the central one for this strategy, and it is not hypothetical in crypto.

## The rules, step by step

1. Pick one cryptocurrency pair on one exchange, such as a Bitcoin pair against a stablecoin.
2. Use fifteen-minute candles.
3. Compute the three measures on the same candles. The first is the commodity channel index, which
   compares the typical price with its own recent average and divides by how far the typical prices
   usually spread. The second is the Chande momentum oscillator, a 0 to 100 measure of gains against
   losses. The third is the money flow index, which is the same idea as the second but each candle
   is scaled by how much was traded. All three use the technical-analysis library's default of 14
   candles.
4. Buy when all three are at an extreme low on the last completed candle: the channel index below
   minus 100, the money flow index below 20, and the momentum oscillator below minus 50. The file
   reads the previous candle's values rather than the candle still forming, so the decision uses a
   finished candle and is made on the next one.
5. Sell when all three are at an extreme high on the last completed candle: the channel index above
   100, the money flow index above 80, and the momentum oscillator above 50.
6. The profit target and the stop do most of the selling in practice. The stored settings are
   `timeframe = '15m'`, `stoploss = -0.05`, and
   `minimal_roi = {"0": 0.05, "20": 0.03, "30": 0.02, "40": 0.0}`, where the keys are minutes held:
   sell at a 5 percent profit immediately, at 3 percent after twenty minutes, at 2 percent after
   thirty and at any profit at all after forty minutes.
7. Do not add to a losing position. There is no ladder and no second purchase in the file.

## The maths, with every symbol named

Everything starts from the typical price of a candle and, for the volume measure, the amount traded.

```text
T_t = (H_t + L_t + C_t) / 3
V_t = amount traded in candle t
```

- `T_t` is the typical price of candle `t`.
- `H_t`, `L_t` and `C_t` are that candle's high, low and close.
- `V_t` is the volume, in coins, matched during that candle.

The commodity channel index measures the distance of the typical price from its own average,
scaled by the usual spread of that distance.

```text
CCI = (T_now - average(T over 14 candles)) / (0.015 * mean absolute deviation of T)
```

- `T_now` is the latest typical price.
- `mean absolute deviation` is the average of the distances between each of the 14 typical prices
  and their own average, with all distances taken as positive.
- The 0.015 divisor is a constant chosen by the measure's author so that the result usually lands
  between minus 100 and plus 100; below minus 100 is treated as unusually low.

The momentum oscillator counts gains and losses over 14 candles.

```text
CMO = 100 * (sum_of_gains - sum_of_losses) / (sum_of_gains + sum_of_losses)
```

- `sum_of_gains` adds up every movement in the close that went up, over 14 candles.
- `sum_of_losses` adds up every movement that went down, as positive numbers.
- The result runs from minus 100, when every move was down, to plus 100, when every move was up.

The money flow index is where the volume enters. Each candle's typical price is multiplied by the
volume traded, and the result is split into positive and negative groups.

```text
money_flow_t = T_t * V_t
MFI = 100 - 100 / (1 + positive_flow / negative_flow)
```

- A candle is positive when its typical price is above the previous candle's, and negative when it
  is below.
- `positive_flow` is the sum of `money_flow` over the positive candles of the last 14, and
  `negative_flow` the same over the negative ones.
- If the price fell on heavy volume, `negative_flow` is large and the index is low.

## A worked example

Five candles of made-up but plausible data. The close and the typical price are set equal so the
arithmetic can be followed, and the volume is in thousands of coins.

| Candle | Close  | Volume | Money flow | Direction |
| ------ | ------ | ------ | ---------- | --------- |
| 1      | 100.00 | 10     | 1000.0     |           |
| 2      | 101.00 | 12     | 1212.0     | positive  |
| 3      | 100.50 | 8      | 804.0      | negative  |
| 4      | 99.00  | 15     | 1485.0     | negative  |
| 5      | 97.00  | 20     | 1940.0     | negative  |
| 6      | 95.00  | 30     | 2850.0     | negative  |

Over the last five movements the positive flow is 1212.0 and the negative flow is
`804.0 + 1485.0 + 1940.0 + 2850.0 = 7079.0`. So the ratio is `1212.0 / 7079.0 = 0.1712` and the
index is `100 - 100 / 1.1712 = 14.62`, which is below 20. Note how much of that came from candle 6:
the heaviest volume of the run landed on a falling candle, which is exactly what pushes this measure
down.

Now the other two. Over the last five typical prices, 101.00, 100.50, 99.00, 97.00 and 95.00, the
average is 98.50 and the mean absolute deviation is
`(2.50 + 2.00 + 0.50 + 1.50 + 3.50) / 5 = 2.00`. The channel index is therefore
`(95.00 - 98.50) / (0.015 * 2.00) = -3.50 / 0.03 = -116.67`, below minus 100. The momentum
oscillator uses the five changes of the close: a gain of 1.00 and losses of 0.50, 1.50, 2.00 and
2.00, so gains total 1.00 and losses 6.00, giving
`100 * (1.00 - 6.00) / (1.00 + 6.00) = -71.43`, below minus 50.

All three conditions are true on candle 6, so the strategy buys on candle 7 at 95.00. Suppose the
price recovers and the three measures later reach their high regions at a price of 99.00, before the
forty-minute profit ladder forces the position out.

| Item                            | Arithmetic        | Result         |
| ------------------------------- | ----------------- | -------------- |
| Buy at 95.00 with 0.10 pct fee  | 95.00 * 1.001     | 95.10 paid     |
| Sell at 99.00 with 0.10 pct fee | 99.00 * 0.999     | 98.90 received |
| Return                          | 98.90 / 95.10 - 1 | +4.00 percent  |

The worked example shows the arithmetic and nothing else. It does not show that buying a
high-volume fall makes money; it shows how the three conditions are checked and how a 4 percent
gross move becomes 4 percent net at these fees.

## What the research actually found

| Source                                                  | What it measured                                               | Result                                                                                                                                                                                                                                                         |
| ------------------------------------------------------- | -------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Cong, Li, Tang and Yang, Management Science 2023        | 29 crypto exchanges, 448 million trades, July to November 2019 | Wash trading, meaning trades an exchange makes with itself, averaged over 70 percent of reported volume on unregulated exchanges, with a median of 79.1 percent; the authors put the fake flow at over 4.5 trillion dollars in the first quarter of 2020 alone |
| Cong, Li, Tang and Yang, Management Science 2023        | The same data                                                  | The exchanges that fabricate most are newer and smaller, and the fabrication is what lifts them in the ranking tables buyers consult                                                                                                                           |
| This repository's brief on market data quality          | A Bitcoin futures launch on a major exchange                   | Order flow in the final ten seconds ran about 50 percent above the pre-launch level and reversed within ten seconds, so the spike was manufactured and the signature was the reversal, not the volume                                                          |
| Park and Irwin, The Profitability of Technical Analysis | 95 studies of technical rules                                  | Results are uneven and weaker after costs, and oscillator rules of this kind are not separately validated by independent samples                                                                                                                               |

There is no published test of this three-condition rule, on these pairs, with these thresholds.
What is measured is that the volume figure the rule depends on is unreliable on exactly the venues
where a pair is thin enough for the rule to matter.

## How this project relates to it

The repository's brief on
[Order flow and toxicity](../../../strategies/books/07_order_flow_and_toxicity.md) is the closest
research to the question the volume measure is trying to answer: it studies whether the direction
and aggression of trades predict anything, and it finds that the predictable part of order flow is
mostly absorbed by price impact rather than turning into a tradeable drift. That is the honest
ceiling on a volume-based signal.

The brief on [Market data quality and integrity](../../../strategies/books2/27_market_data_quality.md)
documents cases where a recorded number is not the event it names, including manufactured end of
session flow, and the brief on
[Crypto venues, AMMs and perpetual futures](../../../strategies/books/13_crypto_amm_and_perpetuals.md)
explains why crypto venues report what they report. Read those before trusting any reported volume.

## Where it goes wrong

- Volume can be fabricated. The largest measured finding in this area is that most of the reported
  volume on unregulated exchanges was the exchange trading with itself, so the volume-aware measure
  can be reading exactly the wrong thing. On a small exchange, "heavy volume" may mean "heavy
  fabrication".
- Volume is venue-specific. There is no single crypto price or volume feed; the same pair reads
  differently on each exchange and on each aggregator, and a rule tuned on one venue may be reading
  a number that does not exist on another.
- The measures are correlated. Two of the three look only at the price, so requiring them to agree
  removes far more trades than a reader might expect without adding three times the information.
- All three are contrarian. Buying a falling price and selling a rising one is the opposite of
  trend following, and in a market that falls for a week every one of those signals is a loss. The
  5 percent stop and the forty-minute ladder end many of them quickly.
- The stop is close and the target is closer. A 5 percent stop against a 5 percent target that
  shrinks with time means the outcome depends heavily on fees; at 0.05 to 0.10 percent per side a
  round trip costs a meaningful slice of the target.
- The file's own comment says it is a test strategy. The header calls it "a test strategy to
  inspire you", and the freqtrade project's README says these files are starting points, which is
  about as direct as a warning gets.

## Try it yourself

You need a spreadsheet and fifteen-minute prices with volume for one coin for a month.

1. Column A: the timestamp. Columns B and C: the close and the volume.
2. Column D: the typical price, which here equals the close.
3. Column E: the money flow, which is column D multiplied by column C.
4. Column F: the sign, which is positive when column D is above the previous row's column D and
   negative otherwise.
5. Column G: the positive flow, which repeats column E when column F is positive and is zero
   otherwise. Column H: the same for negative flow.
6. Column I: the money flow index, which is 100 minus 100 divided by one plus the 14-row sum of
   column G divided by the 14-row sum of column H.
7. Column J: a flag for rows where column I is below 20.
8. Repeat the idea for the channel index and the momentum oscillator in two more columns, then make
   column M say buy only when all three flags are set.

What to notice: sort the rows by volume and compare the index in column I on the busiest rows with
its value on the quietest rows. Then ask whether the busiest rows are ones you would trust. If the
buy flags cluster on a single day, look up that day and see whether the volume was a real event or a
listing, an airdrop or a promotion, which are the moments when reported volume is least trustworthy.

## Where this came from

- [CMCWinner.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/CMCWinner.py),
  the fifteen-minute rule with the three conditions and the forty-minute profit ladder.
- Cong, Li, Tang and Yang, [Crypto Wash Trading](https://www.nber.org/papers/w30783), the study of
  fabricated volume on 29 exchanges.
- Donald Lambert, Commodity Channel Index: Tools for Trading Cyclical Trends, Commodities magazine,
  1980, the original description of the first of the three measures.
- Gene Quong and Avrum Soudack, the money flow index, a volume-weighted version of the relative
  strength index, published in the late 1980s.
- Tushar Chande and Stanley Kroll, The New Technical Trader, 1994, which introduced the momentum
  oscillator used here.
- [Order flow and toxicity](../../../strategies/books/07_order_flow_and_toxicity.md) and
  [Market data quality and integrity](../../../strategies/books2/27_market_data_quality.md),
  this repository's briefs on what order flow and reported numbers are worth.

## Words used in this tutorial

- commodity channel index: a measure of how far a price is from its own recent average, scaled by
  the usual size of that distance.
- money flow index: a 0 to 100 measure of gains against losses in which each candle is weighted by
  the amount traded.
- momentum: the tendency of a price move to persist for a while.
- oscillator: a measure that swings between a low and a high value rather than trending.
- volume: the amount of an asset traded during a period, in coins or contracts.
- wash trading: buying and selling with yourself to create the appearance of activity.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
