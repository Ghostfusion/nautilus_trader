# Trend following in stocks: buying a share at a new high and leaving on a trailing stop

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                 |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of American companies that are large and heavily traded                                                                                                                                                                                                                                                                                        |
| How often it trades       | It looks at prices every day, so a holding can open or close on any day                                                                                                                                                                                                                                                                               |
| What you need             | A spreadsheet with daily share prices, and daily highs and lows                                                                                                                                                                                                                                                                                       |
| Where the rules come from | [The list's implementation file](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/trend-following-effect-in-stocks.py), which restates [the Quantpedia entry](https://quantpedia.com/strategies/trend-following-effect-in-stocks/)                                                                        |
| The underlying research   | Cole Wilcox and Eric Crittenden, [Does Trend Following Work on Stocks?](https://www.cis.upenn.edu/~mkearns/finread/trend.pdf)                                                                                                                                                                                                                         |
| How well it held up       | Mixed: two American samples decades apart both report a strong result before costs and the source includes 0.5 percent round-turn costs, but the 2025 revisit finds fewer than 7 percent of trades carry the profit and that realistic trading costs erase the advantage for accounts below about 1 million dollars                                   |
| Also appears in           | [Commodities futures trend following](../../../tutorials/quantconnect/commodities-futures-trend-following/README.md), [Asset class trend following](../../../tutorials/quantconnect/asset-class-trend-following/README.md) and [Momentum and state of market filters](../../../tutorials/quantconnect/momentum-and-state-of-market-filters/README.md) |

## The idea in one paragraph

This strategy only buys a share when it makes a new highest price since the company started trading, which is
called an all-time high. Once bought, the share is held while it keeps rising. It is sold when the price falls
below a moving line drawn a fixed distance under the most recent price, and that line only ever moves up. Each
qualifying share gets an equal slice of the money, and the whole list is checked every day. The bet is that a
share making new highs will keep going, and the trailing line is there to limit how much is given back when it
stops.

## Why anyone believed it

Prices do not move in neat steps. When good news arrives about a company, it arrives in pieces over weeks, and
investors who hear it late buy after the first move, which pushes the price further. A share at an all-time
high has no earlier owner waiting to sell at a profit, because nobody bought it higher and is now losing
money on it, so there is less of that kind of seller above the current price.

The counterparty is the investor who is forced to sell for reasons that have nothing to do with the company:
a fund paying out withdrawals, a manager trimming a holding that grew too large, or a short seller who has to
buy the share back. If those sellers keep appearing, a share that has been climbing keeps climbing for a
while. The other half of the story is behavioural: people anchor on the old price, so they under-react, and
the price rises in stages rather than all at once.

## An everyday comparison

Think of walking up a mountain path in fog, with a rule about when to turn back. You head in the direction
that has been rising under your feet, and you keep a marker one hour's walk below your highest point. Every
time you reach a new high you raise the marker. You only turn around when the ground drops a whole hour
below your best point, which means one stumble is not enough to send you home. The rule gets you out of a
long descent without reacting to every dip, but on a day of small up-and-down moves it can also send you home
on a wobble that turns out to be nothing.

## The rules, step by step

1. Build the list of shares to watch. The list uses American shares priced above 5 dollars, ranked by the
   dollar value traded each day, and keeps the 100 most heavily traded. The original study applied a minimum
   price and a minimum liquidity test instead of a fixed count.
2. For each share, find its all-time high close: the highest daily closing price it has ever recorded, from
   the first day it traded up to today. A company with a short history may be at a new high within weeks.
3. Buy the share when today's closing price is greater than or equal to that all-time high. This is the only
   entry condition; there is no separate rule about the company, the market, or the economy.
4. For each share you hold, compute its average true range over the last 10 trading days. The true range of a
   day is the largest of three numbers: the day's high minus its low, the day's high minus yesterday's close,
   and yesterday's close minus the day's low. The average of the last ten true ranges is the 10-day average
   true range.
5. Set the trailing stop at today's price minus the 10-day average true range. Tomorrow, recompute the stop
   from the new price and keep whichever is higher, the old stop or the new one. The stop therefore rises
   with the price and never falls.
6. Sell the share when its price falls to or below the trailing stop. After selling, the share can only be
   bought again by making a fresh all-time high.
7. Hold every share that has met the entry rule and has not been stopped out. Give each one an equal share of
   the money, so with three holdings each gets one third, and rebalance back to equal weights each day.
8. Pay the costs. The original study deducts 0.5 percent of the traded amount on a round turn, that is from
   buying to selling. The implementation file uses a much smaller fee of 0.005 percent, which leaves the
   trading cost almost entirely out of the result.

## The maths, with every symbol named

The entry rule compares today's close with the highest close so far.

```text
H_t = max(P_0, P_1, ..., P_t)
Entry when P_t >= H_t
```

- `P_t` is today's closing price.
- `H_t` is the highest closing price from the first day of the company's trading history up to today.
- `max` means "the largest of the numbers in the brackets".

So a share is bought only on a day on which its closing price sets a new record for that company.

The stop needs the true range and its average.

```text
TR_t = max(High_t - Low_t, |High_t - C_{t-1}|, |C_{t-1} - Low_t|)
ATR_t = (TR_t + TR_{t-1} + ... + TR_{t-9}) / 10
S_t = max(S_{t-1}, P_t - ATR_t)
Exit when P_t <= S_t
```

- `High_t` and `Low_t` are today's highest and lowest traded prices.
- `C_{t-1}` is yesterday's closing price.
- The two vertical bars mean "drop the minus sign if there is one", so a drop counts as much as a rise.
- `TR_t` is the day's true range, the largest of the three distances listed.
- `ATR_t` is the average of the last ten true ranges.
- `S_t` is today's trailing stop level.
- `max(S_{t-1}, P_t - ATR_t)` keeps the stop from falling when the price falls.

Each slot in the portfolio is the same size:

```text
w_i = 1 / N
```

- `w_i` is the fraction of the money placed in holding `i`.
- `N` is the number of shares currently held, so with four holdings each gets one quarter of the money.

The cost of a round trip is charged on the money traded:

```text
Cost = t * c
```

- `t` is the traded fraction of the account, counting selling and buying separately, so closing a position
  worth half the account and opening another worth half the account gives `t` of 1.0.
- `c` is the cost per side as a fraction of the amount traded. The source study uses 0.005, that is 0.5
  percent of the traded amount for the whole round trip.

## A worked example

One share is at a new all-time high and is bought. The numbers are invented but the sizes are ordinary for a
large company. The account is 100,000 dollars, the price is 100.00, and 1,000 shares are bought.

| Day | Close  | 10-day ATR | Stop at today's price | Stop actually used | Action              |
| --- | ------ | ---------- | --------------------- | ------------------ | ------------------- |
| 0   | 100.00 | 2.00       | 98.00                 | 98.00              | Buy at the new high |
| 1   | 103.00 | 2.20       | 100.80                | 100.80             | Stop rises          |
| 2   | 105.00 | 2.40       | 102.60                | 102.60             | Stop rises          |
| 3   | 104.00 | 2.30       | 101.70                | 102.60             | Stop unchanged      |
| 4   | 106.00 | 2.50       | 103.50                | 103.50             | Stop rises          |
| 5   | 103.00 | 2.60       | 100.40                | 103.50             | Price hits the stop |
| 6   | 101.00 | 2.70       | 98.30                 | 98.30              | No holding remains  |

The share is bought at 100.00 and sold at 103.50, so the gross return is:

```text
(103.50 / 100.00) - 1 = 0.035, that is 3.5 percent
```

The source study charges 0.5 percent on the round turn, so the net return is:

```text
3.5 percent - 0.5 percent = 3.0 percent
```

On the 100,000 dollar account that is 3,000 dollars, and the position size was the whole account only
because there was one holding; with four holdings each would get one quarter.

The stop is what stops the loss from being larger. Suppose instead the share is bought at 50.00 with an ATR
of 1.00, giving a stop at 49.00, and the next day it closes at 48.00. It is sold at about 49.00, which is a
gross loss of 2.0 percent and a net loss of 2.5 percent after the 0.5 percent round-turn cost. A run of
three such quick losses in a row costs about 7.5 percent and produces no gain, which is the ordinary
experience of this rule.

## What the research actually found

| Source                                                                   | What it measured                                                                                            | Result                                                                                                                                                                                                                                                                                                                  |
| ------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Wilcox and Crittenden, Does Trend Following Work on Stocks?              | 24,000 or more American securities, 1983 to 2004, delisted companies included, 0.5 percent round-turn costs | 19.3 percent a year, volatility 15.6 percent, worst fall 33.74 percent, reward-to-risk 1.24 (pages 12 and 13, as extracted by Quantpedia's page)                                                                                                                                                                        |
| Zarattini, Pagani and Wilcox, Does Trend-Following Still Work on Stocks? | All liquid American shares, 1950 to November 2024, more than 66,000 simulated trades                        | Fewer than 7 percent of trades produce the cumulative profit; out of sample from 2005 to 2024 the result holds with a modest decline; gross growth 15.19 percent a year from 1991 to 2024 with an annual advantage of 6.18 percent over the market, but not viable below about 1 million dollars once costs are charged |
| The awesome-systematic-trading list, its own replication record          | 4,843 coded papers; aggregate statistics only                                                               | The median strategy returns a reward-to-risk of 0.37 and only 48 percent clear a statistical significance bar of 1.96; this is the list's own aggregate measurement, not a figure for this strategy in particular                                                                                                       |

Read together: the earlier study measured a large advantage with costs included, and the later study, on a
much longer and survivorship-free sample, confirmed the shape of the result but showed that the profit sits
in a small number of very large winners. The list publishes no reward-to-risk number for this strategy on
its own, only the aggregate record above. The later study's central practical finding is that daily trading
costs, not the signal, decide whether the rule survives.

## How this project relates to it

This repository already contains trend-following tutorials for other markets, and they are the closest
thing here to this idea.
[Commodities futures trend following](../../../tutorials/quantconnect/commodities-futures-trend-following/README.md)
applies the same trailing-stop logic to a basket of commodities, and
[Asset class trend following](../../../tutorials/quantconnect/asset-class-trend-following/README.md) applies
it to whole markets through index funds.
[Momentum and state of market filters](../../../tutorials/quantconnect/momentum-and-state-of-market-filters/README.md)
is the version that switches the signal on and off with the market's own trend.

For the research behind the idea,
[the predictability brief](../../../strategies/books2/08_predictability_and_trading_strategies.md)
collects what the momentum and trend-following literature measured, including the finding that the shape of
the return distribution is a property of the position rule rather than of the market. That is the same
mechanism this strategy is built on: many small losses and a few very large gains.

## Where it goes wrong

- It is mostly the market. The long-only version is invested in shares at all times, so it falls when the
  market falls. Quantpedia's own note says a large part of the measured return comes from the equity market
  itself, and the correlation to the broad market is very high.
- Daily turnover is expensive. New highs and stop-outs can happen any day, and the later study found the
  profit disappears for small accounts once each trade is charged realistically. A rule that trades rarely
  is a different rule.
- The profit is concentrated. Fewer than 7 percent of trades carry the whole cumulative gain, according to
  the 2025 study. Most trades are small losses, and a sample that happens to miss the few big winners shows
  nothing.
- Survivorship flatters old numbers. A study run only on companies that still exist today misses the ones
  that failed, which is where trend following's long losses live. The 2025 study is careful to avoid this;
  older data sets often are not.
- Whipsaw. In a market that chops sideways, a share makes a new high, drifts down, and stops out over and
  over. Each round trip costs money and the equity curve bleeds even though the rule is being followed
  correctly.
- All-time highs need full history. A company that has traded for twenty years must clear a much higher bar
  than one listed last year, so the list of holdings drifts towards newly listed companies unless that is
  controlled.

## Try it yourself

You need nothing but a spreadsheet and a public source of daily share prices, including each day's high and
low. Pick 20 well-known large companies.

1. Make one sheet per company with columns `Date`, `High`, `Low`, `Close`.
2. Add a column `RunningHigh` that holds the highest close from the first row down to the current row.
3. Add a column `TrueRange` with the formula for the day's true range, using the previous row's close.
4. Add a column `ATR10` holding the average of the last ten true-range values.
5. Add a column `Stop` that is today's close minus `ATR10`, and a column `StopUsed` that keeps the larger of
   yesterday's `StopUsed` and today's `Stop`.
6. Mark a row `Buy` when `Close` is at least `RunningHigh`, and `Sell` when `Close` is at or below
   `StopUsed`. Assume you act at the close of that day.
7. Track the profit or loss of each completed round trip, then subtract 0.5 percent of the traded amount per
   round trip.

What to notice: the number of trades is large, most of them lose a little, and one or two winners carry the
whole result. Also notice how sensitive the outcome is to the 10-day setting: run it again with 5 days and
with 20 days and count how many round trips each version makes. The one that trades more pays more.

## Where this came from

- [The awesome-systematic-trading list](https://github.com/paperswithbacktest/awesome-systematic-trading), and
  its implementation file for
  [trend-following-effect-in-stocks](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/trend-following-effect-in-stocks.py),
  which states the price, liquidity, entry, stop and cost rules used here.
- [Quantpedia: Trend-following Effect in Stocks](https://quantpedia.com/strategies/trend-following-effect-in-stocks/),
  the rules and the extracted performance figures, and Quantpedia's confidence rating of Strong.
- Cole Wilcox and Eric Crittenden, [Does Trend Following Work on Stocks?](https://www.cis.upenn.edu/~mkearns/finread/trend.pdf),
  the original study on 1983 to 2004 American data with costs included.
- Carlo Zarattini, Alberto Pagani and Cole Wilcox,
  [Does Trend-Following Still Work on Stocks?](https://ssrn.com/abstract=5084316), the 2025 revisit on
  survivorship-free data to November 2024, which is where the profit concentration and the cost finding
  come from.
- [the predictability brief](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's brief on momentum and trend-following design.

## Words used in this tutorial

- all-time high: the highest price a share has ever traded at since it was first listed.
- average true range: the average over a chosen number of days of how far a price travelled in a day, taking
  gaps into account.
- drawdown: the fall from a peak to the following low, measured in percent.
- liquidity: how much of something trades each day, and how easily it can be bought or sold.
- momentum: the tendency of something that has been rising to keep rising for a while.
- round turn: a complete purchase followed by a sale, or a sale followed by a purchase.
- trailing stop: a selling line that follows the price upwards but never moves down.
- weight: the share of your money placed in one holding.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
