# Intraday momentum: trading the last half-hour on how the first half-hour went

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                             |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | American index funds, each bought or sold short for only the last half-hour of a trading day, in whichever direction the price moved in the first half-hour                                                                                                       |
| How often it trades       | Every trading day, once, in the last half-hour                                                                                                                                                                                                                    |
| What you need             | Python and a file of minute-by-minute prices                                                                                                                                                                                                                      |
| Where the rules come from | [QuantConnect strategy library, intraday ETF momentum](https://www.quantconnect.com/tutorials/strategy-library/intraday-etf-momentum)                                                                                                                             |
| The underlying research   | Gao, Han, Li and Zhou, [Intraday Momentum: The First Half-Hour Return Predicts the Last Half-Hour Return](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2552752), later published as Market Intraday Momentum in the Journal of Financial Economics in 2018 |
| How well it held up       | Weak: the pattern was measured once, on one sample and one family of funds, and two later implementations, the library's own and an independent one, both earned a negative average return with a lower reward for risk than simply holding the index             |
| Also appears in           | nothing else in this collection                                                                                                                                                                                                                                   |

## The idea in one paragraph

The trading day has a shape. News arrives overnight, and the first half-hour after the opening bell is
when the market works out what it means. The last half-hour is when people who do not want to carry a
position overnight close it. This strategy looks at how the price moved from yesterday's close to the
end of the first half-hour, then takes the same direction for the last half-hour: if the morning move
was up it buys and holds to the closing bell, and if the morning move was down it sells short. It does
that every day on a handful of very actively traded funds, and holds nothing overnight.

## Why anyone believed it

Not every investor learns about the news at the same moment. A study by Bogousslavsky reports that
some traders are late to the information, and that others simply prefer to delay their orders until
the market closes, when the price they get may be better. That pushes a share of the day's buying and
selling into the first and last half-hours. If the opening move is driven by people acting on fresh
news, and the closing move is driven by people who are acting on the same news later, the two ends of
the day can point in the same direction more often than chance would suggest.

The counterparty is the trader who must act at a fixed time for a reason unrelated to the outlook: a
fund that has to match an index, a manager told to reduce risk before the day ends, or a short seller
buying back stock so as not to carry the position overnight. Their orders arrive at the close whatever
the price is doing, which is what makes the last half-hour tradable for someone watching the morning.

## An everyday comparison

Think of a farmers' market that opens at six in the morning. The early traders have read the overnight
weather report and heard yesterday's prices, and the first half-hour of trading sets the tone for the
day. The people who arrive at the end are not there to learn anything; they are there because the
stalls close soon, or because they did not want to carry produce home overnight. The rule here is to
watch the opening half-hour, take the day's mood from it, and join the same side at the close.
Sometimes the early crowd is right for a real reason, and sometimes it is a false start that the
closing crowd corrects.

## The rules, step by step

1. Choose the funds. The library uses three: SPY, which holds the largest American companies; IWM,
   which holds smaller ones; and IYR, which holds property companies. The research behind it also
   lists DIA, QQQ, EEM, FXI, EFA, VWO, XLF and TLT.
2. Write down yesterday's closing price for each fund.
3. At the end of the first half-hour of trading, write down the price and compute the morning return:
   that price divided by yesterday's close, minus one. A morning return of 0.20 percent means the
   price is 0.20 percent above yesterday's close.
4. Thirty minutes before the close, decide. If the morning return is positive, buy the fund. If it is
   negative, sell the fund short, which means borrowing it, selling it, and buying it back later. The
   library's rule puts nothing on when the morning return is exactly zero.
5. Hold the position through those last 30 minutes and close it at the closing price, using an order
   that executes in the closing auction. Do not hold overnight.
6. Do this for each fund, every day, with the same amount of money on each.
7. Between times there is nothing to do. The position is either the last half-hour of the day or
   nothing at all.

Worth knowing before reading the rest: many markets, including the American share market, do not
begin with continuous trading. The first printed price is set by an auction, which collects the orders
that arrived overnight and crosses them at a single price, chosen to trade as many shares as possible.
The same is true of the close. That is why the first price of the day can jump far from yesterday's
close and why the first half-hour and the last half-hour look different from the middle of the day.

## The maths, with every symbol named

The morning return, which is the whole signal:

```text
r_morning = (P_1000 / P_prev_close) - 1
```

- `r_morning` is how far the price has moved since yesterday's close, as a decimal: 0.002 means
  0.20 percent.
- `P_1000` is the fund's price at the end of the first 30 minutes of trading. Do not confuse it with
  yesterday's close, which is the starting point.
- `P_prev_close` is the fund's closing price on the previous trading day.

The direction to trade:

```text
s = +1 if r_morning > 0, and -1 if r_morning < 0
```

- `s` is the direction, plus one for buying and minus one for selling short. Multiplying a return by
  `s` turns a fall in the price into a gain when the signal was a short sale, and a rise into a gain
  when the signal was a purchase.

The return over the position's life, and the result for the day:

```text
r_close = (P_close / P_1530) - 1
day_result = s * r_close
```

- `P_1530` is the price 30 minutes before the close, when the position is opened.
- `P_close` is the price at the close, set by the closing auction, when the position is closed.
- `r_close` is the fund's return over the last half-hour.
- `day_result` is the return on one unit of money placed on that fund for that half-hour.

The cost of doing this every day:

```text
cost = 2 * c
```

- `c` is the cost of one crossing as a fraction of the amount traded, covering the gap between the
  buying price and the selling price plus any commission. One basis point, written 0.0001, is one
  hundredth of one percent.
- Two crossings happen each day: one to open the position and one to close it.

The rough yearly figure:

```text
year_result = 252 * average_day_result_after_cost
```

- `252` is the approximate number of trading days in a year, used here to turn a daily average into a
  yearly one. This ignores compounding, so it is a scale, not a forecast.

The research reports yearly returns produced in this way: 6.67 percent for SPY, 11.72 percent for IWM
and 24.22 percent for IYR, which average to 14.2 percent when the three are given equal weight. Those
numbers come from the study below, not from this page.

## A worked example

Six trading days of a fund priced near 400. The account holds 100,000, applied to one fund. Each
crossing is assumed to cost one basis point, 0.0001, so a day costs 0.0002. The prices are invented,
but they are of the sizes these funds produce.

| Day | Yesterday's close | Price at 10:00 | Morning return | Trade | Price at 15:30 | Closing price | Last half-hour return | Day result |
| --- | ----------------- | -------------- | -------------- | ----- | -------------- | ------------- | --------------------- | ---------- |
| 1   | 400.00            | 400.80         | +0.200%        | buy   | 401.00         | 401.48        | +0.1197%              | +0.1197%   |
| 2   | 401.48            | 402.10         | +0.1544%       | buy   | 402.40         | 402.76        | +0.0895%              | +0.0895%   |
| 3   | 402.76            | 402.00         | -0.1887%       | short | 402.10         | 402.30        | +0.0497%              | -0.0497%   |
| 4   | 402.30            | 401.60         | -0.1740%       | short | 401.50         | 401.06        | -0.1096%              | +0.1096%   |
| 5   | 401.06            | 401.90         | +0.2094%       | buy   | 401.70         | 401.98        | +0.0697%              | +0.0697%   |
| 6   | 401.98            | 401.20         | -0.1940%       | short | 401.40         | 401.64        | +0.0598%              | -0.0598%   |

Each morning return is the price at 10:00 divided by the previous close, minus one. Each last
half-hour return is the closing price divided by the price at 15:30, minus one. On day 3 the price
rose over the last half-hour while the morning had fallen, so a short position lost 0.0497 percent.
On day 4 the price fell, and the same short position gained 0.1096 percent.

```text
Gross, six days        = 0.1197 + 0.0895 - 0.0497 + 0.1096 + 0.0697 - 0.0598 = 0.2790 percent
Cost, six days         = 6 * 0.0002 = 0.0012, that is 0.1200 percent
Net, six days          = 0.2790 - 0.1200 = 0.1590 percent
In money               = 100,000 * 0.001590 = 159.00, after 279.00 earned and 120.00 paid
Average day            = 0.001590 / 6 = 0.000265
Scaled to a year       = 252 * 0.000265 = 0.0668, that is 6.68 percent
```

The yearly figure lands close to the 6.67 percent the research reports for SPY, which is a coincidence
of this invented example rather than a result. Two things are worth noticing. First, the cost line is
most of the reason the number is small: at one basis point per crossing the daily round trip costs
about 5.0 percent a year, and at three basis points per crossing it costs about 15.1 percent a year,
which is more than the 11.7 percent the six days above would earn before costs. Second, the size of
the result depends on a few days: day 4 alone is two thirds of the gross figure.

## What the research actually found

| Source                                                        | What it measured                                                              | Result                                                                                                                                                                                                                                                                                                                                                                 |
| ------------------------------------------------------------- | ----------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Gao, Han, Li and Zhou, Market Intraday Momentum               | High-frequency data for SPY and ten other funds, 1993 to 2013                 | The first half-hour return predicts the last half-hour return, statistically and economically, even after trading fees; the pattern is stronger on more volatile days, on higher-volume days, on recession days and on major news days. Yearly returns reported as 6.67 percent for SPY, 11.72 percent for IWM, 24.22 percent for IYR, and 14.2 percent equal-weighted |
| Bogousslavsky (2016), as reported in the library page         | The link between the opening and closing periods of the day                   | Late-informed investors and investors who prefer to trade at the close create a positive correlation between the direction of the opening and the direction of the close                                                                                                                                                                                               |
| QuantConnect implementation, 1 January 2015 to 16 August 2020 | SPY, IWM and IYR, traded in the last 30 minutes each day, against the S&P 500 | Reward-to-risk of -0.628 against 0.582 for the index; a printed annual standard deviation of 0.002 against 0.023, described on the page as lower and therefore more consistent; during the fall of 2020 the figures were 1.452 against -1.466, and during the recovery 0.305 against 7.925                                                                             |
| Independent implementation reported on paperswithbacktest     | Daily ETF prices from 1990 to 2026                                            | A yearly return of -1.24 percent, a reward-to-risk figure of -0.23, yearly volatility of 4.83 percent and a worst fall of 51.8 percent                                                                                                                                                                                                                                 |

Read together, the picture is this. There is a documented tendency for the last half-hour to follow
the first, it was strongest on the days when the market was already moving, and it is not visible as a
steady source of money. The library's own implementation lost money over its five years and had a
lower reward for risk than the index it traded, while doing well in the crash that made the index
fall. The independent implementation, over a much longer period, also lost money on average. A pattern
that appears only on volatile and news-heavy days is a different thing from an income stream.

## How this project relates to it

The auction mechanics that make the opening and the close unusual are collected in this repository's
brief on market design, [Market Design, Fees and Auctions](../../../strategies/books/11_market_design_fees_and_auctions.md).
Its auction section reports that the best auction length is not one number for everyone: for 77
Euronext stocks it is of the order of 2 to 10 minutes (`1906.01713v3`), against a periodic auction
that runs in about 100 milliseconds. It also reports that exchanges randomise the close on purpose,
with the London Stock Exchange adding a 30-second random period to its opening and closing auctions
(`2405.09764v2`, p.5), and that in an illiquid call auction the chance of no trade at all runs from 4
percent to 33 percent, against below one in a hundred thousand in a liquid one (`1407.4512v2`, p.8).
Those three findings are the reason the first and last half-hours behave as they do, and the last one
is a warning: on a quiet fund, an auction can simply not clear.

The second related piece is the brief on return predictability,
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md).
It reports that a momentum strategy's returns are positively skewed by construction, so a rule can win
less than half its days and still sum to a positive figure, and that skewness depends on the holding
horizon (`2101.01006v2`). It also reports a model in which a published signal's half-life shrinks as
more money runs it (`2605.23905v1`), which is exactly the risk in a rule this simple.

Nothing in this repository trades this rule. The closest piece of machinery is the user manual for
intraday systematic trading,
[intraday and medium-frequency systematic trading](../../../docs/usermanauls/intraday-systematic/README.md),
which teaches how to turn one written rule into a test on recorded prices. Its worked example is a
moving-average crossing, not this strategy, and it states plainly that a good backtest is not a
promise about the future.

## Where it goes wrong

- Costs eat the edge at this horizon. The position is opened and closed every single day, so the
  round trip is paid 252 times a year. At one basis point per crossing that is about 5 percent a year,
  which is most of the 6.67 percent first reported for SPY; at three basis points the rule is negative
  before any luck is involved.
- The money is made on a handful of days. The published results are strongest on volatile days, on
  high-volume days and on days with major news, so the yearly figure is decided by a few sessions and
  is not the same thing as a steady return.
- It behaves like a bet on the market moving. In the library's backtest the strategy made a positive
  reward-to-risk figure during the 2020 crash while the index made a negative one, which is the
  signature of a position that pays when prices are swinging, not of an income stream.
- The closing auction is not always tradable. The exit is an order that executes at one auction price
  in a short window; on a quiet fund, or on a day when the auction clears far from where the strategy
  expected, the exit price can differ from the one the rule assumes.
- A published rule is a crowded rule. The signal is two lines of arithmetic on public prices, so
  anyone who reads the paper can run it, which is the condition under which published edges shrink.
- Short selling has its own costs and constraints. Half the days require borrowing a fund, paying a
  fee for it, and accepting that the lender can ask for it back at a bad moment.

## Try it yourself

You need a spreadsheet and a public source of minute-by-minute prices, which most finance websites
provide for large funds.

1. Make one row per trading day and these columns: Date, Previous close, Price at 10:00, Morning
   return, Direction, Price at 15:30, Closing price, Last half-hour return, Strategy return.
2. Morning return is the price at 10:00 divided by the previous close, minus one. Direction is 1 when
   that is positive and -1 when it is negative.
3. Last half-hour return is the closing price divided by the price at 15:30, minus one. Strategy
   return is Direction multiplied by that.
4. Add a Cost column holding 0.0002 every day, and a Net column that subtracts it.
5. Sum the Net column. Separately, compute what you would have earned by simply holding the fund over
   the same month.

What to notice: the wins and losses look close to a coin flip, and the total is small next to the
column of costs. Count how many days produce a gain larger than 0.2 percent; on most months three or
four days decide the whole result, and the rest cancel out. Then double the cost to two basis points
per crossing and watch the sum turn negative. That sensitivity is the honest headline of this rule.

## Where this came from

- [QuantConnect strategy library: intraday ETF momentum](https://www.quantconnect.com/tutorials/strategy-library/intraday-etf-momentum),
  the rules as implemented: three funds, a 30-minute morning window, a 30-minute close window, and
  the backtest figures quoted above.
- Gao, Han, Li and Zhou,
  [Intraday Momentum: The First Half-Hour Return Predicts the Last Half-Hour Return](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2552752),
  later published as Market Intraday Momentum in the Journal of Financial Economics, 2018.
- [Market Design, Fees and Auctions](../../../strategies/books/11_market_design_fees_and_auctions.md),
  this repository's brief on auctions, which cites `1906.01713v3`, `2405.09764v2` and `1407.4512v2`.
  The arXiv pages are at [1906.01713](https://arxiv.org/abs/1906.01713),
  [2405.09764](https://arxiv.org/abs/2405.09764) and [1407.4512](https://arxiv.org/abs/1407.4512).
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's brief on what is predictable and how fast a published signal decays, which cites
  `2101.01006v2` and `2605.23905v1`. The arXiv pages are at
  [2101.01006](https://arxiv.org/abs/2101.01006) and [2605.23905](https://arxiv.org/abs/2605.23905).
- [Intraday momentum, the first half-hour return predicts the last half-hour return](https://paperswithbacktest.com/strategies/intraday-momentum-the-first-half-hour-return-predicts-the-last-half-hour-return),
  the independent implementation whose figures are quoted above.

## Words used in this tutorial

- auction: a moment when orders are collected and crossed at a single price, rather than traded one
  against another continuously, which is how the opening and closing prices are set.
- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- closing price: the single price set by the auction at the end of the trading day.
- crossing: buying the price someone is asking or selling at the price someone is bidding, which is
  what a trader does when they want the trade done now.
- short selling: borrowing something you do not own, selling it, and buying it back later, so you gain
  when its price falls.
- volatility: how much a price moves around its average, usually quoted as a yearly percentage.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
