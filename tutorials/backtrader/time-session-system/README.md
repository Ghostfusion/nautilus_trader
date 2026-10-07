# Trading the clock: gold at fixed hours, from the night channel to the 18:45 short

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                            |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | The price of gold against the American dollar, one fixed position at a time                                                                                                                                                      |
| How often it trades       | Once or twice a day, at fixed clock times, whenever the price is inside a chosen window                                                                                                                                          |
| What you need             | A spreadsheet and a data file with the time of each price bar                                                                                                                                                                    |
| Where the rules come from | [Strategy Compendium, article 20, time session system](https://backtrader.readthedocs.io/en/latest/strategies-series/en/20-time-session-system.html)                                                                             |
| The underlying research   | None: the rules are ports of MetaTrader expert advisors, so the windows come from the code and not from a study; the intraday rhythm they lean on is documented in market-microstructure research, but these exact hours are not |
| How well it held up       | Weak, resting on a handful of expert-advisor ports over about three months of one asset, where one strategy produced a single trade and another annualises a three-month move into double digits                                 |
| Also appears in           | Nothing else in this collection                                                                                                                                                                                                  |

## The idea in one paragraph

Gold trades every hour of the day and night, but the people trading it are not the same people at
every hour: European desks go home in the evening, American desks thin out, the Asian desks arrive
later. This family of seven strategies ignores what the price looks like and follows a timetable
instead. One of them sells gold short at a quarter to seven in the evening and buys it back two hours
later, every day, whatever the price is doing. Another waits until just after midnight and then buys or
sells depending on where the price sits inside the box traced by the last three hours. Nothing else
decides: no chart pattern, no moving average, no announcement.

## Why anyone believed it

A price moves because someone is willing to pay up and someone else is willing to accept. If the people
present at 18:45 are not the people present at 20:45, the balance between the two sides can change with
the hour, and the price can drift for reasons that have nothing to do with the outlook for gold. The
story behind the evening short is that the desks buying gold during the European day are closing their
books while the evening session is thinner, so the selling that remains meets less buying and the price
sags until the next shift of desks arrives.

The night channel uses the same idea in reverse. In the quiet hours few new orders arrive and the price
wanders inside a narrow box; near the edge of that box the only people trading are small ones who
cannot shift the price. The counterparty in both cases is someone who has to trade at that hour for
reasons unconnected to what gold is worth: a fund settling a position before its day ends, or a dealer
filling a small order in a market with no depth.

## An everyday comparison

Picture a wholesale flower hall that closes at a fixed hour each day. Sellers who still have carnations
on the trolley as the bell approaches cannot keep them, so they cut the price, and the cheapest moment
arrives at the same time every day. A buyer who sells flowers at the start of that final hour and buys
them back at the end collects the fall, without knowing whether carnations are in fashion. The night
channel is the same hall an hour earlier, when almost nobody is in it and the few prices bounce inside a
narrow range.

## The rules, step by step

1. Get a file of gold prices where every bar carries the clock time at which it ended, written here as
   XAUUSD. The seven strategies use one-minute, five-minute or fifteen-minute bars, and one builds an
   extra hourly copy of the same data to read its signals from.
2. Choose two clock times, an open time and a close time. The pair is fixed before the test starts and
   never depends on the price.
3. At the open time, if nothing is open, buy or sell one fixed amount. The direction comes either from
   the clock alone or from a price condition checked at that moment, such as where the price sits
   inside a range built from the last few hours.
4. Size every position the same way: 0.1 lots. One lot of gold is 100 ounces, so 0.1 lots is 10 ounces
   and a move of one dollar per ounce is worth 10.00 dollars.
5. While the position is open, watch only the exits the strategy was given: a stop loss a fixed
   distance away, a take profit a fixed distance away, and sometimes a trailing stop that follows the
   price and only steps once the trade is ahead.
6. At the close time, flatten. Any position still open is closed at the market, whatever the price.
7. Repeat every day of the data, one position at a time, and never a second entry inside a window.

### What is in this category

| Strategy                   | What it does in one clause                                                                                          | Source file                               |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------- | ----------------------------------------- |
| Simple Pending Orders Time | Places a pair of stop orders either side of the price at 15:00 and cancels whatever is left when the window ends    | `test_0001_simple_pending_orders_time.py` |
| Night Flat Trade           | Builds a channel from the last three hourly bars and buys its floor or sells its ceiling between midnight and 02:00 | `test_0002_night_flat_trade.py`           |
| OpenTime                   | Sells at 18:45 every day and buys back at 20:45, with no price rule at all                                          | `test_0003_opentime.py`                   |
| 21hour                     | Arms breakout stops at 08:00 and 22:00 and forces everything flat at 21:00 and 23:00                                | `test_0004_21hour.py`                     |
| Opening Closing on Time v2 | Enters at 05:00 in whichever direction a 50 and a 200 bar average point, and flattens at 21:01                      | `test_0005_opening_closing_on_time_v2.py` |
| Exp_TimesDirection         | Opens and closes at two fixed times in a fixed direction, with no price condition                                   | `test_0006_times_direction.py`            |
| Open Close on Time         | Enters on the first bar past the open time and exits on the first bar past the close time                           | `test_0007_open_close_on_time.py`         |

### Night Flat Trade (test_0002)

The data are gold one-minute bars from 2026-03-05 to 2026-03-10, 4,562 bars, plus an hourly feed built
by resampling them. Cash starts at 1,000,000, commission is 0.0, margin is 0.01 of the position value
and the multiplier is 100.

1. Consider a signal only when the clock hour of the hourly bar is 0 or 1; at any other hour, do
   nothing.
2. Take the highest high and the lowest low of the three hourly bars the strategy can see, which are
   the bar still being formed and the two before it. Call them H and L, and call the width D = H - L.
3. Keep the channel only if D is greater than 10.00 and less than 40.00. Those two numbers are pips
   worth 0.10 each in this file, so the gate reads as wider than 100 pips and narrower than 400.
4. Buy when the one-minute price is above L and at or below L + D/4, the bottom quarter of the box.
5. Sell when it is below H and at or above H - D/4, the top quarter.
6. Put the stop loss a third of the box beyond the far edge: L - D/3 for a buy, H + D/3 for a sell.
7. Put the take profit 50 pips, meaning 5.00, in the trade's favour, and add a trailing stop 15 pips
   away that only moves in steps of 5 pips once the trade is 20 pips ahead.
8. Trade 0.1 lots, never more than one position, and never a new signal while one is open.

### OpenTime (test_0003)

The data are gold fifteen-minute bars from 2025-12-03 to 2026-03-10, 6,129 bars. Every bar's timestamp
is moved forward fifteen minutes at load, so the bar stamped 18:45 is the bar that closed at 18:45, not
the one that opened then. Cash starts at 1,000,000, commission is 0.0, margin is 0.01, multiplier 100.

1. At 18:45 each day, if nothing is open, sell 0.1 lots at the market.
2. If a position is open, or the same day and time has already opened one, do nothing. That latch is
   what stops a window from trading repeatedly.
3. At 20:45 each day, close whatever is open, at the market.
4. Set no stop loss, no take profit and no trailing stop: the only exit is the clock.

## The maths, with every symbol named

The night channel is one box with a position inside it.

```text
H = the largest of the three hourly highs
L = the smallest of the three hourly lows
D = H - L
```

- `H` is the ceiling of the box the night price has been living in.
- `L` is the floor of that box.
- `D` is the width of the box, in dollars per ounce of gold.

The strategy keeps the box only when `10.00 < D < 40.00`, which in this file's units is 100 to 400 pips.

Where the entries and the protective stop sit:

```text
buy  when      L     < price <= L + D / 4
sell when  H - D / 4 <= price <      H
stop for a buy  = L - D / 3
stop for a sell = H + D / 3
```

- `price` is the closing price of the one-minute bar being examined at that moment.
- `L + D / 4` is a quarter of the box above its floor, so the buy zone is the bottom quarter, and
  `H - D / 4` makes the sell zone the top quarter.
- `D / 3` is a third of the box, the distance past the far edge at which the stop is parked.

What any of it is worth in money:

```text
profit = size * multiplier * (sell price - buy price)
```

- `size` is the number of contracts traded, here 0.1.
- `multiplier` is what one contract earns for a one dollar move in the price, here 100.
- The two prices are the price at which the position was opened and the one at which it was closed.

`size * multiplier` is 10 here, so every 1.00 of movement in the gold price is worth 10.00. For a sale
opened first and bought back later, the result is positive when the price falls.

## A worked example

### Night Flat Trade (test_0002)

Six hourly checks during one week. The highs and lows are invented, but of a plausible size for gold.

| Clock time   | H       | L       | D     | width gate passed? | price tested | which quarter | decision      |
| ------------ | ------- | ------- | ----- | ------------------ | ------------ | ------------- | ------------- |
| Day 1, 00:00 | 5149.00 | 5145.00 | 4.00  | no, too narrow     | 5146.80      | none          | nothing       |
| Day 1, 01:00 | 5154.00 | 5148.00 | 6.00  | no, too narrow     | 5150.00      | none          | nothing       |
| Day 2, 00:00 | 5152.00 | 5140.00 | 12.00 | yes                | 5146.10      | middle        | nothing       |
| Day 2, 01:00 | 5158.00 | 5143.00 | 15.00 | yes                | 5150.00      | middle        | nothing       |
| Day 3, 00:00 | 5151.00 | 5138.00 | 13.00 | yes                | 5149.50      | top           | sell 0.1 lots |
| Day 3, 01:00 | 5149.00 | 5140.00 | 9.00  | no, too narrow     | 5144.00      | none          | nothing       |

The check on Day 3 at 00:00 passes: the box is 13.00 wide, so a quarter is 3.25, and the sell zone runs
from 5151.00 - 3.25 = 5147.75 up to but not including 5151.00. A price of 5149.50 is inside it.

| Item                                                     | Price per ounce | Arithmetic                         | Money                            |
| -------------------------------------------------------- | --------------- | ---------------------------------- | -------------------------------- |
| Sell 0.1 lots at the close of the signal bar             | 5149.50         |                                    |                                  |
| Protective stop, H + D/3                                 | 5155.33         | 5151.00 + 13.00 / 3                | risk of 5.83 per ounce, or 58.33 |
| Take profit, 50 pips below the entry                     | 5144.50         | 5149.50 - 5.00                     |                                  |
| Exit when the target is reached                          | 5144.50         | 5149.50 - 5144.50 = 5.00 in favour | 0.1 * 100 * 5.00 = 50.00         |
| The gap between the buying and selling price, round trip |                 | 10 ounces * 0.20                   | -2.00                            |
| Net for the trade                                        |                 | 50.00 - 2.00                       | 48.00                            |

The 0.20 per ounce spread is not in the file: the backtest charges commission 0.0 and uses one price for
buying and selling, so the gap is never paid. A spread that size costs 2.00 on 10 ounces and turns 50.00
into 48.00, or 0.0048 percent of the 1,000,000 account. The file's own single trade is a short that
netted 61.30, taking the account to 1,000,061.30; at 10 dollars per 1.00 of price, that is consistent
with a favourable move of about 6.13, a little past the 5.00 target, perhaps because the closing order
fills on the bar after the target is touched.

### OpenTime (test_0003)

Six consecutive days, one position per day, selling at the 18:45 price and buying back at 20:45.

| Day   | Price at 18:45, sold | Price at 20:45, bought back | Move in the trade's favour | Gross on 10 ounces         | Spread cost | Net    |
| ----- | -------------------- | --------------------------- | -------------------------- | -------------------------- | ----------- | ------ |
| 1     | 5100.00              | 5096.20                     | +3.80                      | 0.1 * 100 * 3.80 = 38.00   | -2.00       | 36.00  |
| 2     | 5104.50              | 5107.10                     | -2.60                      | 0.1 * 100 * -2.60 = -26.00 | -2.00       | -28.00 |
| 3     | 5112.30              | 5105.90                     | +6.40                      | 0.1 * 100 * 6.40 = 64.00   | -2.00       | 62.00  |
| 4     | 5098.70              | 5101.40                     | -2.70                      | 0.1 * 100 * -2.70 = -27.00 | -2.00       | -29.00 |
| 5     | 5120.10              | 5113.00                     | +7.10                      | 0.1 * 100 * 7.10 = 71.00   | -2.00       | 69.00  |
| 6     | 5107.60              | 5110.20                     | -2.60                      | 0.1 * 100 * -2.60 = -26.00 | -2.00       | -28.00 |
| Total |                      |                             |                            | 94.00                      | -12.00      | 82.00  |

Three of the six days made money, a win rate of 50 percent, and the six days together gained 82.00 on
1,000,000, or 0.0082 percent. The file's real run has the same shape over a longer stretch: 67 days, 37
wins and 30 losses, and 2,199.70 in total, about 0.22 percent of the account.

## What the research actually found

Every backtest in this compendium asserts three numbers against the values captured when the test was
written: the final value of the account, the reward-to-risk ratio the files call Sharpe, and the worst
fall from a peak the account suffered, called the maximum drawdown. The assertion compares the engine's
output with that baseline of its own. Passing it proves only that the engine still computes exactly what
the file says it computes, and proves nothing about the strategy: the same assertion would pass on a
file whose rules lose money steadily, and on one whose rules make money only by accident of the sample.
No number below is evidence that the idea earns anything.

| Strategy                     | Data and window                                                                        | Trades | What the file records                                                                                                                                                |
| ---------------------------- | -------------------------------------------------------------------------------------- | ------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Night Flat Trade (test_0002) | gold, one-minute bars with an hourly signal feed, 2026-03-05 to 2026-03-10, 4,562 bars | 1      | one short, final value 1,000,061.30, net 61.30, win rate 100 percent, Sharpe 8.93, maximum drawdown 0.00 percent, annual figure 0.49 percent                         |
| OpenTime (test_0003)         | gold, fifteen-minute bars, 2025-12-03 to 2026-03-10, 6,129 bars                        | 67     | 37 wins and 30 losses, win rate 55.22 percent, profit factor 1.53, final value 1,002,199.70, Sharpe 7.09, maximum drawdown 0.14 percent, annual figure 13.89 percent |
| 21hour (test_0004)           | gold, five-minute bars, 2025-12-03 to 2026-03-10, 18,328 bars, as the article reports  | 129    | win rate 56.6 percent, profit factor 0.836, final value 996,443.90, which is a loss                                                                                  |

Four readings. First, a win rate means nothing without the number of trades: Night Flat Trade is right
100 percent of the time over one trade, and one trade is not a sample of anything. Second, OpenTime's
13.89 percent a year is not the money made: the account gained 0.21997 percent over about three months,
the annual figure compounds that per-bar gain over a year of bars, and stretching the three-month gain
over twelve months on the calendar gives about 0.8 percent, so the quoted number is about seventeen
times that. Third, 21hour wins 56.6 percent of the time and still ends lower, because the average loser
was bigger than the average winner, which the profit factor of 0.836 states directly. Fourth, every run
charges zero commission and uses the same price for buying and selling, so the main cost of a session
trade in gold is not paid at all.

One more caution: the Sharpe figures are computed over bars rather than calendar years, and OpenTime's
quoted 7.09 sits beside a three-month gain of 0.22 percent, so the ratio flatters a very small move.

## How this project relates to it

Two of this repository's research briefs measure the machinery a session story would need, though
neither tests these hours. The first is
[volatility and microstructure noise](../../../strategies/books2/14_volatility_and_microstructure_noise.md),
which reports a study of 301 American shares from January 2015 to December 2022 that identified 37,452
price jumps inside the 10:30 to 15:00 window. Only about 4.3 percent of them fell within three minutes
of a news item, and 60 percent showed a positive mean-reversion score, meaning the price partly took the
jump back afterwards. That is the mechanism the night channel assumes: sharp moves in thin conditions
that are not news and that partly reverse. It is measured on shares during the American daytime, not on
gold between midnight and 02:00.

The second is [market making and inventory](../../../strategies/books/04_market_making_and_inventory.md),
which reports that the spread behaves as a self-exciting process with power-law decay, meaning one jump
in the gap between the buying and selling price makes another more likely for a while, and that mean
spreads were 1.51 ticks for the CAC40 future and 2.44 ticks for the AXA share. That is the mechanism
behind fixed-hour trading: the cost of crossing the gap has its own clock. It is measured on French
instruments at second and minute horizons and says nothing about 18:45 or 20:45 in gold.

## Where it goes wrong

- One trade is not a result. Night Flat Trade's perfect win rate comes from a single short over five
  days, so any conclusion drawn from it is unsupported.
- The annual figures flatter a short sample. A three-month gain of 0.22 percent becomes 13.89 percent
  when a per-bar average is compounded over a year of bars; the honest description of the same
  measurement is a fraction of a percent.
- Cost is assumed away. Commission is 0.0 and the buying and selling prices are the same number, so the
  spread is never paid, and a session trade pays it every single day.
- The hours look chosen after the fact. The category holds seven strategies with seven clocks, all ports
  of expert advisors whose windows were presumably picked because they had worked for someone; when many
  windows are tried and the best kept, the winner is partly luck.
- One asset, one regime. Every run is gold over about three months in which gold moved in one direction
  overall, and a two-hour evening short makes money in a falling market whether or not the clock has
  anything to do with it.
- A timestamp is a hidden assumption. OpenTime shifts every bar forward fifteen minutes so that 18:45
  means the bar that closed then; were that shift wrong by one bar, every entry would sit fifteen
  minutes from where it was meant to be.

## Try it yourself

You need one year of hourly gold prices with the time attached; a free finance site will give you hourly
closes in a spreadsheet. Then:

1. Build five columns: Date, Price at 18:45, Price at 20:45, Difference, Up or down.
2. In Difference put the 18:45 price minus the 20:45 price, which is what a short held across the two
   hours earned per ounce before costs.
3. In Up or down put 1 when Difference is positive and 0 otherwise. Those are the days a short won.
4. Add a row with the count of 1s, the count of 0s, their sum and their ratio, which is the win rate.
5. Repeat on a second sheet for a different pair of hours, say 10:00 to 12:00, and a third for 22:00 to
   00:00, and compare the three win rates.

What to notice: the sign is not stable. A year of hourly gold is around 250 trading days, and a win rate
near 60 percent measured over 250 days has a standard error of about three percentage points, so a year
at 55 percent and a year at 60 percent are not clearly different. If 18:45 to 20:45 beats the other
windows in your year, change nothing and try the year before; a real clock effect should survive that.

## Where this came from

- [Strategy Compendium, article 20, time session system](https://backtrader.readthedocs.io/en/latest/strategies-series/en/20-time-session-system.html),
  the rules, the inventory of seven strategies, and the reported results for 21hour.
- [test_0002_night_flat_trade.py](https://raw.githubusercontent.com/cloudQuant/backtrader/development/tests/functional/strategies/time_session_system/test_0002_night_flat_trade.py),
  the channel, quadrant, stop and sizing rules and the asserted single trade of 1,000,061.30.
- [test_0003_opentime.py](https://raw.githubusercontent.com/cloudQuant/backtrader/development/tests/functional/strategies/time_session_system/test_0003_opentime.py),
  the 18:45 and 20:45 rules, the timestamp shift, and the asserted 67 trades and 1,002,199.70.
- [test_0004_21hour.py](https://raw.githubusercontent.com/cloudQuant/backtrader/development/tests/functional/strategies/time_session_system/test_0004_21hour.py),
  the 08:00 and 22:00 breakout windows and the forced flat at 21:00 and 23:00.
- [Volatility and microstructure noise](../../../strategies/books2/14_volatility_and_microstructure_noise.md),
  this repository's brief on jump clustering and mean reversion, citing `2404.16467v1`.
- [Market making and inventory](../../../strategies/books/04_market_making_and_inventory.md),
  this repository's brief on the spread as a self-exciting process, citing `2303.02038v2`.

## Words used in this tutorial

- pip: a fixed small step in a price, in this file one tenth of a dollar per ounce of gold.
- lot: the standard size of one contract; one lot of gold is 100 ounces, so 0.1 lots is 10 ounces.
- spread: the gap between the price at which something can be bought and the price at which it can be
  sold at the same moment, a cost paid on every trade.
- short: selling something you do not own, borrowing it, and buying it back later, so that a fall in the
  price is a gain.
- stop loss: a pre-set price at which a losing position is closed automatically.
- take profit: a pre-set price at which a winning position is closed automatically.
- profit factor: the money won by all the winning trades divided by the money lost by all the losing
  trades, so a number below 1 means the losers were larger.
- drawdown: the fall from a peak in the account to the low that follows it, measured in percent.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
