# Short-term reversal in futures: buying the contract that just fell hardest

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                 |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Eight futures contracts on currencies and stock-market indexes, the largest ones traded in Chicago                                                                                                                                                                                                    |
| How often it trades       | Once a week, on a Wednesday to Wednesday rhythm                                                                                                                                                                                                                                                       |
| What you need             | A spreadsheet and a public source of weekly prices, trading volumes and open interest                                                                                                                                                                                                                 |
| Where the rules come from | [QuantConnect strategy library, short-term reversal with futures](https://www.quantconnect.com/tutorials/strategy-library/short-term-reversal-with-futures) and the [Quantpedia entry](https://quantpedia.com/strategies/short-term-reversal-with-futures) it cites                                   |
| The underlying research   | Changyun Wang and Yu, [Trading activity and price reversals in futures markets](https://www.researchgate.net/profile/Changyun_Wang/publication/222566966_Trading_activity_and_price_reversals_in_futures_markets)                                                                                     |
| How well it held up       | Mixed: a long and well-cited weekly reversal survived in one 1983 to 2000 sample of 24 futures, but no independent replication is attached to it, the profits are concentrated in stressed weeks, and the library's own version changes the universe and reverses the volume and open-interest labels |
| Also appears in           | Nothing else in this collection describes the futures version; its equity cousin is the better-known short-term reversal effect                                                                                                                                                                       |

## The idea in one paragraph

Some futures contracts are traded very heavily while few people hold them overnight. This strategy
picks the handful of contracts that combine heavy trading with little holding, buys the one that fell
most over the past week, and sells short the one that rose most. A futures contract is an agreement to
buy or sell something at a fixed price on a fixed future date, and it is bought and sold on an
exchange like a share. A week later it looks again, closes the two positions, and opens a new pair.
The bet is that a sharp one-week move is often an overreaction that partly unwinds, and that the
unwinding is strongest where trading is busy but few people are committed to holding.

## Why anyone believed it

Short-horizon reversal is usually described as the market punishing an overreaction. When news hits,
traders who are too confident update their view by more than the news deserves, and the price
overshoots. A day or a week later the excess is given back, so the contract that swung hardest down
tends to bounce and the one that swung hardest up tends to sag.

The counterparty, then, is the overconfident or hurried trader who trades on the same day's news.
Volume is the visible trace of that trading, and the research argument is that a large jump in volume
marks the moments when the overshoot is biggest. Open interest is the second clue. It is the number of
futures contracts still open at that moment, counting each agreement once, and it stands for money
that is actually committed to holding a position. When open interest is low, most of the trading is
short-lived: people open a position and close it the same day, with nobody left holding it. There are
fewer patient buyers and sellers to steady the price, so the overshoot can run further and the
following reversal can be sharper.

## An everyday comparison

Think of a farmers' market on the last morning of the week. The farmer must clear the stall before
travelling home, so the final crate of tomatoes is knocked down to half price in the last ten minutes.
Nothing about the tomatoes changed. The discount is a reaction to a seller who has to leave, not to
news about the value of a tomato, and the next market day the same crate is priced normally again.
This strategy buys the crate that was knocked down hardest and sells the one that a rushed buyer
overpaid for.

## The rules, step by step

1. Choose eight futures contracts: four currencies (the Swiss franc, the British pound, the Canadian
   dollar and the euro) and four stock-index futures (the Nasdaq 100, the Russell 2000, the S&P 500
   and the Dow 30, all the smaller of the two contract sizes).
2. Work in weeks that run from one Wednesday to the next Wednesday. Use the price, the trading volume
   and the open interest recorded at each Wednesday.
3. For each contract work out three numbers over the week that just finished: how much the price
   moved, how much the weekly trading volume changed, and how much the open interest changed.
4. Rank the eight contracts by the change in volume. Keep the four with the largest increase in
   trading.
5. Separately rank the eight contracts by the change in open interest. Keep the four with the
   smallest increase in open interest, that is the four that attracted the least new holding.
6. Keep the contracts that appear on both lists. These are the heavily traded, lightly held ones. The
   list can be empty or hold only one contract, in which case there is nothing to trade that week.
7. Among the contracts still on the list, find the one whose price fell the most over the past week.
   Buy that one. Find the one whose price rose the most. Sell that one short (borrow it, sell it now,
   and buy it back later, so that a fall is a gain).
8. Put roughly 30 percent of the account on each side, hold both positions for one week, then close
   both and start again from step 2.
9. Measure volume and open interest as percentage changes over the week, not as levels, because the
   research defines the signal from changes. One warning: the QuantConnect page's prose describes the
   group as high volume with low open interest, but the code on the same page comments that it takes
   the lowest volume change with the highest open-interest change, which is close to the opposite by
   label. This tutorial follows the prose and the paper. Treat the ambiguity itself as part of the
   honest record.

## The maths, with every symbol named

The three measurements for one contract over one week:

```text
r = P_this / P_last - 1
v = V_this / V_last - 1
o = OI_this / OI_last - 1
```

- `r` is the price return over the past week, written as a decimal: 0.021 means 2.1 percent.
- `P_this` is the price at this Wednesday and `P_last` the price one week earlier.
- `v` is the change in weekly trading volume, as a decimal. Volume is how many contracts changed hands
  during the week.
- `V_this` is this week's volume and `V_last` last week's.
- `o` is the change in open interest. Open interest is the count of still-open agreements, each
  counted once rather than once for the buyer and once for the seller.
- `OI_this` is open interest this Wednesday and `OI_last` one week earlier.

The selection is a filter followed by a sort. Keep the contracts with the largest `v` and the
smallest `o` at the same time, then within that group:

```text
long  = the contract with the smallest r
short = the contract with the largest r
```

- `long` is the position that gains when the price rises.
- `short` is the position that gains when the price falls.

The portfolio's result over the following week, before costs, is the two positions added together:

```text
G = w_L * R_long + w_S * R_short
```

- `G` is the gross return of the portfolio over the next week, as a decimal.
- `w_L` is the weight on the long side, here 0.30, because 30 percent of the account is placed there.
- `w_S` is the weight on the short side, here -0.30, because it is a short position and moves opposite
  to the price.
- `R_long` and `R_short` are the next-week price returns of the two contracts chosen.

Then the cost of rebuilding the two positions each week. Selling the old pair and buying the new pair
trades both sides twice:

```text
t = 2 * (|w_L| + |w_S|) = 2 * (0.30 + 0.30) = 1.20
Cost = t * c
```

- `t` is the traded notional: 1.20 account units are bought or sold each week, because each 30
  percent position is closed once and opened once.
- `c` is the cost per unit traded, covering the gap between the buying and selling price plus
  commission. A realistic figure for these liquid contracts is 0.0005, that is five basis points,
  where one basis point is one hundredth of one percent.
- `Cost` is therefore about 0.0006, or 0.06 percent, for a fully changed pair each week.

## A worked example

The table is one week of invented but plausible data for the eight contracts. Volume and open
interest are shown in thousands of contracts.

| Contract        | Volume before | Volume now | v      | Open interest before | Open interest now | o     | Past-week return |
| --------------- | ------------- | ---------- | ------ | -------------------- | ----------------- | ----- | ---------------- |
| Swiss franc     | 40.0          | 42.8       | +7.0%  | 120.0                | 111.6             | -7.0% | -0.4%            |
| British pound   | 60.0          | 64.8       | +8.0%  | 150.0                | 160.5             | +7.0% | +0.9%            |
| Canadian dollar | 30.0          | 31.5       | +5.0%  | 90.0                 | 94.5              | +5.0% | +1.1%            |
| Euro            | 100.0         | 118.0      | +18.0% | 200.0                | 218.0             | +9.0% | +0.3%            |
| Nasdaq 100      | 50.0          | 56.0       | +12.0% | 100.0                | 98.0              | -2.0% | -2.6%            |
| Russell 2000    | 35.0          | 36.4       | +4.0%  | 80.0                 | 83.2              | +4.0% | +0.6%            |
| S&P 500         | 120.0         | 132.0      | +10.0% | 300.0                | 288.0             | -4.0% | +1.4%            |
| Dow 30          | 25.0          | 25.5       | +2.0%  | 70.0                 | 65.8              | -6.0% | +0.2%            |

The four largest volume changes are the euro (+18.0 percent), the Nasdaq 100 (+12.0), the S&P 500
(+10.0) and the British pound (+8.0). The four smallest open-interest changes are the Swiss franc
(-7.0), the Dow 30 (-6.0), the S&P 500 (-4.0) and the Nasdaq 100 (-2.0). The contracts on both lists
are the Nasdaq 100 and the S&P 500. Of those two the Nasdaq 100 fell the most (-2.6 percent), so it is
bought, and the S&P 500 rose the most (+1.4 percent), so it is sold short.

Now six invented weeks of results, each row applying the rule to a fresh week of rankings. The long
and short columns name the contracts chosen; the return columns are what those contracts did the
following week.

| Week | Long | Short | Long return | Short return | Gross = 0.30 x (long - short) | Cost  | Net    |
| ---- | ---- | ----- | ----------- | ------------ | ----------------------------- | ----- | ------ |
| 1    | NQ   | ES    | +1.8%       | -0.9%        | +0.81%                        | 0.06% | +0.75% |
| 2    | GBP  | YM    | -0.9%       | +1.1%        | -0.60%                        | 0.06% | -0.66% |
| 3    | CAD  | NQ    | +0.4%       | -1.3%        | +0.51%                        | 0.06% | +0.45% |
| 4    | NQ   | ES    | +2.2%       | -0.6%        | +0.84%                        | 0.06% | +0.78% |
| 5    | CHF  | GBP   | -0.3%       | +0.8%        | -0.33%                        | 0.06% | -0.39% |
| 6    | ES   | NQ    | +0.5%       | -0.7%        | +0.36%                        | 0.06% | +0.30% |
| Sum  |      |       |             |              | +1.59%                        | 0.36% | +1.23% |

The gross column is 0.30 times the long return minus the short return, because the short side gains
when its contract falls. So week 1 is 0.30 times (1.8 minus -0.9), which is 0.30 times 2.7, or 0.81
percent. The six weeks gain 1.23 percent after costs, an average of about 0.205 percent a week. At
that rate fifty-two weeks is roughly 10.7 percent a year before compounding, though the invented
numbers sit below the published figures below and the example says nothing about whether the idea
works. It only shows how the rules and the arithmetic behave.

## What the research actually found

| Source                                                                                                                                                                                | What it measured                                                      | Result                                                                                                                                              |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| Wang and Yu, [Trading activity and price reversals](https://www.researchgate.net/profile/Changyun_Wang/publication/222566966_Trading_activity_and_price_reversals_in_futures_markets) | 24 United States futures, weekly Wednesday to Wednesday, 1983 to 2000 | Strong weekly reversals; profits rose with past volume changes and fell with past open-interest changes, and were strongest when both were combined |
| [Quantpedia, short-term reversal with futures](https://quantpedia.com/strategies/short-term-reversal-with-futures)                                                                    | Long high-volume, low-open-interest losers against gainers            | 29.64 percent a year, volatility 31.4 percent, worst fall 58.65 percent, reward-to-risk 0.82, all before costs and from the same single sample      |
| Quantpedia's confidence label                                                                                                                                                         | The same record                                                       | "Strong", resting on the size and age of the sample, with no separate out-of-sample test reported                                                   |
| [QuantConnect strategy library](https://www.quantconnect.com/tutorials/strategy-library/short-term-reversal-with-futures), implementation                                             | Two of eight contracts chosen, roughly 30 percent a side              | A change of universe from 24 contracts to 8 and of weighting from the paper's proportional rule to a single long and a single short                 |

The paper's own weighting is worth stating, because the library simplifies it away. The paper does not
pick one contract per side. It gives every contract in the chosen group a weight proportional to its
past-week return minus the average return of the group, so the biggest losers get the largest positive
weights and the biggest gainers the largest negative ones. Read together, the record says there is a
real weekly reversal in futures with a large raw return, but almost the whole return comes from
periods of stress, it is measured in one sample, and it is quoted before costs. The strategy is also,
in effect, a seller of insurance against market shocks, which is why its worst fall is so deep.

## How this project relates to it

This repository keeps its own research on short-horizon reversal and on the cost of acting on a
rule. [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md)
reports that recovery- and drawdown-based ranking rules beat plain cumulative return at matched
horizons, winning weekly contrarian portfolios in the S&P 500 and the KOSPI 200 while cutting the
worst fall roughly in half, citing `1403.8125v4`, and separately reports that a signal's half-life is
compressed by crowding, citing `2605.23905v1`. The same brief, in its mapping table, points factor
claims at `python/nautilus_trader/analysis/statistic.py`, where this project requires a claim to be
restated under a factor model before it is treated as a signal.

The execution side is covered by
[Optimal execution and liquidation](../../../strategies/books/01_execution_and_liquidation.md), which
finds that an adaptive execution policy pays only in mean-reverting regimes, citing `1701.08972v2`,
the same condition a reversal trade needs to survive. The closest thing this project actually
measures for a mean-reversion screen is the specification written up in
[Relative-value screening](../../../docs/design/relative_value_screening.md), which restates a sector
mean-reversion playbook in the project's own terms and measures, among other things, the false
positives that a screen over eleven series produces. The code for that screen is in this repository at
`python/nautilus_trader/optimization/relative_value.py`.

## Where it goes wrong

- Crowding erodes it first. Once a weekly reversal rule is widely run, the bounce is bought before it
  happens, the return arrives sooner and smaller, and the last buyers pay for the correction.
- It is a short-volatility trade. Weeks with the largest reversals are weeks after a market fall, and
  a reversal rule is forced to buy the assets that performed worst and sell short the ones that
  performed best, exactly when prices are gapping. The published worst fall of 58.65 percent comes
  from this.
- Costs are quoted away. A strategy that changes its two positions every week pays about 2.4 percent
  a year at ten basis points a side, more if the pair turns over completely, and the paper's headline
  figure is before any of that.
- The open-interest label is easy to reverse. The QuantConnect page's prose and its own code comment
  disagree on whether the signal wants high or low open interest. A rule whose sign is ambiguous is
  one whose result is easily fitted after the fact.
- The universe changed. The paper used 24 contracts across currencies, bonds, agriculture and metals.
  The library uses 8, only currencies and stock indexes, so the two are not the same trade.
- The evidence is one sample with no independent replication. For the whole idea to be false it is
  enough that the weekly reversal was specific to the years studied, or that it lives only in
  contracts too illiquid to trade at the quoted prices.

## Try it yourself

You need a spreadsheet and a public source of weekly futures prices, volumes and open interest. Your
broker's research page or an exchange data page will do.

1. Build a sheet with one row per contract per week for the last two years. Columns: date, contract,
   closing price, weekly volume, open interest.
2. Add a column for the past-week return: this week's price divided by last week's, minus one.
3. Add a column for the volume change and one for the open-interest change, each as a percent of the
   prior week.
4. Add two ranking columns: a rank for volume change, largest first, and a rank for open-interest
   change, smallest first. Mark the contracts in the top half of both.
5. On each row, of the marked contracts, note the one with the lowest past-week return and the one
   with the highest.
6. In the next row down, take the average of those two contracts' next-week returns with a plus sign
   on the low one and a minus sign on the high one, and multiply by 0.30. That is the week's gross
   result.
7. Subtract 0.06 percent for costs each week the pair changes.

What to notice: in many weeks one or both lists are nearly empty, so the rule does nothing. In a few
weeks after a sharp market fall the result is a large gain or a large loss, and those few weeks drive
the whole total. If your sheet earns a smooth, steady profit, you have probably looked up too late a
price, the return that the rule was supposed to predict.

## Where this came from

- [QuantConnect strategy library: short-term reversal with futures](https://www.quantconnect.com/tutorials/strategy-library/short-term-reversal-with-futures),
  the implemented rules: eight contracts, the weekly Wednesday-to-Wednesday rhythm, the volume and
  open-interest groups, and the single long and single short.
- [Quantpedia: short-term reversal with futures](https://quantpedia.com/strategies/short-term-reversal-with-futures),
  the performance figures, the instrument count, the proportional weighting and the reference list.
- Changyun Wang and Yu, [Trading activity and price reversals in futures markets](https://www.researchgate.net/profile/Changyun_Wang/publication/222566966_Trading_activity_and_price_reversals_in_futures_markets),
  the original study of weekly reversals in 24 United States futures.
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's brief on reversal, its decay and its evaluation, citing `1403.8125v4` and
  `2605.23905v1`.
- [Optimal execution and liquidation](../../../strategies/books/01_execution_and_liquidation.md),
  this repository's brief whose finding that adaptation pays only in mean-reverting regimes, citing
  `1701.08972v2`, is the condition this strategy needs.
- [Relative-value screening](../../../docs/design/relative_value_screening.md), this repository's
  measured specification for a sector mean-reversion screen.

## Words used in this tutorial

- basis point: one hundredth of one percent, so five basis points is 0.05 percent.
- futures contract: an agreement to buy or sell something at a fixed price on a fixed future date.
- gross return: the return before costs and fees are subtracted.
- long: owning something, so that a rise in its price is a gain.
- open interest: the number of still-open futures agreements at a moment, counting each agreement once.
- overreaction: a price move larger than the news justified, which then partly unwinds.
- reversal: the tendency of a sharp move to be followed by a move back the other way.
- short selling: borrowing something you do not own and selling it now, to buy it back later, so a fall is a gain.
- volatility: how much a price moves around its average, often quoted as a percentage per year.
- volume: the amount traded over a period, here the number of contracts changing hands in a week.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
