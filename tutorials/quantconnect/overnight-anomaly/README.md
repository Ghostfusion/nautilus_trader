# Overnight anomaly: buying at the close and selling the next morning

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                   |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | A single fund that tracks the whole American market, held only from one day's close to the next day's open                                                                                                                                              |
| How often it trades       | Every trading day, twice: once at the close and once at the next open                                                                                                                                                                                   |
| What you need             | A spreadsheet and daily opening and closing prices for one fund                                                                                                                                                                                         |
| Where the rules come from | [QuantConnect strategy library, overnight anomaly](https://www.quantconnect.com/tutorials/strategy-library/overnight-anomaly)                                                                                                                           |
| The underlying research   | Glasserman, Krstovski, Laliberte and Mamaysky, [Does Overnight News Explain Overnight Returns?](https://arxiv.org/abs/2507.04481) (2025), and Knuteson, [Strikingly Suspicious Overnight and Intraday Returns](https://arxiv.org/abs/2010.01727) (2020) |
| How well it held up       | Weak: the gap between the overnight part and the daytime part of the day is documented across many markets and decades, but the QuantConnect page and the 2025 study both find it is not tradable once the cost of trading twice a day is counted       |
| Also appears in           | nothing else in this collection                                                                                                                                                                                                                         |

## The idea in one paragraph

A trading day is really two periods: the hours the market is open, and the hours it is closed. Split
every day in two and measure each half separately, and in many markets the closed hours have risen
far more than the open ones. This strategy buys a fund that tracks the whole American market just
before the close, sells it just after the next open, and holds cash through the trading day. It
repeats every day. There is no ranking and no filter: the rule is simply to own the market overnight
and not during the day. The hope is that the overnight half keeps earning more than the daytime half
costs.

## Why anyone believed it

Most news arrives while the market is shut. A company reports its results after the close, a
government publishes a jobs number before the open, an overseas market moves in the American night.
When the market reopens, the price has to jump to the new level, and that jump happens entirely in
the overnight period. During the day, by contrast, the market is crowded and liquid, and much of the
trading is people adjusting positions rather than reacting to fresh news. If news is what moves
prices, most of the movement should show up in the overnight gap.

The counterparty is whoever buys at the open. Some are forced to: funds that track an index must buy
wherever the market is, and investors who only act after reading the morning news are late. Others
are not able to trade at the close, so the opening auction is where their orders land. Overnight the
market is thin, so sellers demand a little extra to hold the risk, and the buyer at the open pays it.
A second, darker explanation is that some large traders deliberately push prices up before the open
and down during the day, and the pattern is the footprint of that behaviour rather than of news.

## An everyday comparison

Think of a shop that closes at six in the evening and reopens at nine the next morning. Overnight its
online listing is updated to reflect everything that happened in the evening, so the sticker price
you see when the door opens already contains the new information. Through the day, customers haggle,
compare with other shops and wait for discounts, and the price drifts back a little. If you could buy
at the evening price and sell at the morning price every day, you would collect the evening updates.
The strategy here is that idea applied to a whole market, with the shop being a fund and the two
prices being the close and the next open.

## The rules, step by step

1. Choose one fund that tracks the whole American market. The QuantConnect page uses SPY, which
   tracks the S&P 500 index of large American companies.
2. Just before the market closes each trading day, buy the fund at the closing price.
3. Just after the market opens the next morning, sell the whole holding at the opening price.
4. Hold cash for the rest of that day.
5. Repeat every trading day. There is no ranking, no average and no condition to check.
6. A later variant from the same source adds a filter: only trade overnight when a short-term
   measure of the market's mood is positive. This tutorial describes the plain version, which is the
   one the QuantConnect page implements.

Because the rule trades twice a day, about five hundred times a year, the cost of each round trip is
the whole story. The measurement below is done in basis points, where one basis point is one
hundredth of one percent.

## The maths, with every symbol named

Split each day into the part the market is closed and the part it is open.

```text
O_t = Opening_t / Closing_(t-1) - 1
```

- `O_t` is the overnight return of day `t`, written as a decimal: 0.0004 means 0.04 percent.
- `Opening_t` is the price at the open on day `t`.
- `Closing_(t-1)` is the price at the close of the previous day.

```text
I_t = Closing_t / Opening_t - 1
```

- `I_t` is the intraday return of day `t`, the move from that morning's open to that afternoon's
  close.
- `Closing_t` is the price at the close on day `t`.

The two halves multiply into the whole day from close to close:

```text
(1 + O_t) * (1 + I_t) - 1 = Closing_t / Closing_(t-1) - 1
```

- The left side is what you earn by holding overnight and then through the day; the right side is the
  single move from one close to the next. For small percentages the two halves simply add, so roughly
  the whole-day return is `O_t + I_t`.

The strategy holds only the overnight half, so in day `t` it earns `O_t` before costs. The cost of
trading twice is:

```text
Cost_t = 2 * c
```

- `c` is the cost of one trade, as a fraction of the amount traded: the gap between the buying and
  selling price plus any commission. A realistic figure for a very liquid fund is 0.0001 to 0.0002,
  that is one to two basis points per side.
- The factor 2 is there because the strategy both buys and sells every day. The round trip therefore
  costs twice `c`, so four basis points a day at two basis points a side.

The net return of one day is then:

```text
Net_t = O_t - 2 * c
```

- If the average overnight gain per day is smaller than the round-trip cost, the strategy loses money
  even though the overnight part is positive.

## A worked example

Five made-up but realistic trading days for the fund, with the overnight and intraday returns worked
out for each. Prices are in the same unit throughout.

| Day | Previous close | Opening | Closing | Overnight | Intraday |
| --- | -------------- | ------- | ------- | --------- | -------- |
| 1   | 400.00         | 400.16  | 400.08  | +0.040%   | -0.020%  |
| 2   | 400.08         | 400.24  | 400.20  | +0.040%   | -0.010%  |
| 3   | 400.20         | 400.32  | 400.30  | +0.030%   | -0.005%  |
| 4   | 400.30         | 400.46  | 400.42  | +0.040%   | -0.010%  |
| 5   | 400.42         | 400.54  | 400.52  | +0.030%   | -0.005%  |

The overnight returns add to 0.180 percent over the five days, which is 0.036 percent a day, or 3.6
basis points a day. The intraday returns add to -0.050 percent, and close to close the fund rose
0.130 percent (400.52 divided by 400.00, minus one). The two halves agree: 0.180 - 0.050 = 0.130.

Now the costs. Suppose each side of a trade costs two basis points, so a round trip costs four:

```text
Cost per day = 2 * 0.0002 = 0.0004, that is 0.040 percent
Cost over five days = 5 * 0.040 = 0.200 percent
Net over five days = 0.180 - 0.200 = -0.020 percent
```

Over five days the strategy lost two hundredths of a percent, even though every overnight period was
positive, because the trading costs were slightly larger than the overnight gains. This is the whole
point of the strategy and the reason the QuantConnect page warns about slippage and fees. It also
explains why the measured effect, about 2.75 basis points a day according to the 2025 study, is
smaller than a realistic round trip. The worked example says nothing about whether the strategy
works; it only shows how to apply the rules and how the arithmetic behaves.

## What the research actually found

| Source                                                    | What it measured                                                         | Result                                                                                                                                                                                                                                         |
| --------------------------------------------------------- | ------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Glasserman, Krstovski, Laliberte and Mamaysky, 2025       | 2.4 million news articles, American shares, 1996-2022                    | Overnight returns exceeded intraday returns by 2.75 basis points a day, about 7.2 percent a year; the authors say the turnover needed to trade it is too high for it to be viable                                                              |
| The same paper, on the size of the two halves             | Yearly correlations, 2000-2021                                           | Intraday followed intraday (0.19) and overnight followed overnight (0.29), but the two halves pulled against each other (about -0.27 each way); close-to-close showed neither (about -0.04)                                                    |
| Knuteson, 2020                                            | 21 major share-market indices, about three decades                       | Overnight curves rose sharply and intraday curves fell, for example a Canadian index at +1,062 percent overnight against -67 percent intraday; China was the exception, and the paper argues the pattern is suspicious and probably deliberate |
| QuantConnect, the library page                            | One fund, bought at the close and sold at the open, twenty years         | States that the returns are cancelled once costs are counted, and that with the Interactive Brokers fee model the fees over the backtest came to almost 25 percent of the starting cash                                                        |
| Quantpedia, the filtered variant that adds a mood measure | One fund, overnight only when a sentiment measure is positive, 2018-2021 | 15.58 percent a year, volatility 7.33 percent, reward-to-risk 2.12, but over only three and a half years and with a filter; the entry itself says to use it as an overlay rather than on its own                                               |

The sources agree the pattern is real and large in gross terms, and they disagree about the cause:
one explains it with the timing of news, the other argues that news and risk cannot explain the
consistency and that deliberate trading is the likely cause. That disagreement is unresolved. What
the sources agree on is the practical conclusion: at realistic costs, trading the overnight period
every day does not leave money behind.

## How this project relates to it

This repository's own harvest of the news literature,
[news and language-model signals](../../../strategies/books/14_news_and_language_model_signals.md),
states the finding directly: overnight and intraday returns behave as separate regimes with opposite
news sensitivities, and the overnight-minus-intraday gap of 2.75 basis points a day is not viable
after costs. It also notes that news topics create a shared exposure across names, so a risk model
that treats shares as independent will misstate the overnight risk.

The second related piece is
[manipulation, fraud and governance](../../../strategies/books2/19_manipulation_fraud_and_governance.md),
which reads the argument that a persistent overnight-against-intraday split across major indices is
the footprint of deliberate round-trip trading, and treats it as a hypothesis under dispute rather
than a finding.

The third link is
[execution at the price you get](../../project/execution-at-the-price-you-get/README.md). A strategy
that trades twice a day lives or dies on the gap between the price on the screen and the price it
actually gets, which is exactly what that tutorial explains.

## Where it goes wrong

- Costs are the whole question. The rule trades twice every day, so even a tiny cost per side adds
  up to a large annual charge. The overnight gain is a few basis points a day, and the round trip can
  be the same size or larger.
- The opening price is a single auction. The official open is one price struck once, and a large
  order can move it. If the strategy cannot trade exactly at that price, the measured result is not
  the achievable one.
- The effect is not stable. In the American market the split against you is most visible in older
  data, and the level of the overnight gain has changed over time. A rule fitted to one period may
  capture nothing in the next.
- The two halves can turn into each other. The continuation and reversal patterns in the paper show
  the relationship is not a fixed tilt; it changes sign depending on the horizon measured.
- The cause is disputed. If the pattern is the footprint of deliberate trading, then detecting and
  removing that behaviour is what a regulator would do, and the profit would disappear with it. If it
  is the timing of news, it should persist but stay too small to trade.
- It says nothing about the direction of the market. The strategy is always exposed overnight, so a
  single bad overnight gap can erase many days of small gains.

## Try it yourself

You need nothing but a spreadsheet and a public source of daily opening and closing prices for one
index fund.

1. Build four columns: `Day`, `Open`, `Close`, and a helper `Previous close` that copies yesterday's
   close into today's row.
2. Add an `Overnight` column: today's open divided by the previous close, minus one, then multiply
   by 100 so the answer is in percent.
3. Add an `Intraday` column: today's close divided by today's open, minus one, then multiply by 100.
4. Add a `Cost` column holding 0.04, the round trip at two basis points a side, expressed in percent.
5. Add a `Net` column: overnight minus cost, both already in percent.
6. Add two running totals, one for `Overnight` and one for `Net`, over a full year.
7. Finally, add a column that just holds close to close, as the comparison.

What to notice: the overnight total will usually be larger than the intraday total, which is the
pattern the strategy is built on. Then look at the net total against the close-to-close comparison.
At a round trip of four basis points a day the net line is usually below the comparison, and on some
days it is negative. If your net total looks strongly positive, check that the opening price you used
is the real auction open and not a stale quote, because a favourable opening price is the easiest way
to make this strategy look better than it is.

## Where this came from

- [QuantConnect strategy library: overnight anomaly](https://www.quantconnect.com/tutorials/strategy-library/overnight-anomaly),
  the rules as implemented and the warning about slippage and fees.
- [Quantpedia: market sentiment and an overnight anomaly](https://quantpedia.com/strategies/market-sentiment-and-an-overnight-anomaly),
  the filtered variant and the earlier literature it cites; the plain entry the QuantConnect page
  points to is at [Quantpedia screener detail 4](https://quantpedia.com/Screener/Details/4).
- `2507.04481v1`, Does Overnight News Explain Overnight Returns? (2025), the news explanation and
  the 2.75 basis points a day finding, read from the local corpus PDF.
- `2010.01727v1`, Strikingly Suspicious Overnight and Intraday Returns (2020), the manipulation
  argument, read from the local corpus PDF.
- [News and language-model signals](../../../strategies/books/14_news_and_language_model_signals.md)
  and [manipulation, fraud and governance](../../../strategies/books2/19_manipulation_fraud_and_governance.md),
  this repository's own studies of the two explanations.

## Words used in this tutorial

- basis point: one hundredth of one percent, so 2.75 basis points is 0.0275 percent.
- closing price: the price struck at the end of the trading day.
- intraday return: the move from one day's open to that day's close.
- opening price: the price struck at the start of the trading day, usually in a single auction.
- overnight return: the move from one day's close to the next day's open.
- slippage: the difference between the price you expected and the price you actually got.
- turnover: how much of the portfolio is bought and sold, here almost all of it every day.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
