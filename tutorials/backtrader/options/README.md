# Options: the third Friday, and the business of selling insurance

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                       |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Gold, a fund holding gold bars, and blends of funds that stand in for option positions; the library has no real option prices                                                                                                                                                                               |
| How often it trades       | One round trip a month in the expiration-week rules, and about six put sales a year                                                                                                                                                                                                                         |
| What you need             | A spreadsheet and an option-expiration calendar for the calendar rules; Python and a data file for the put sale                                                                                                                                                                                             |
| Where the rules come from | [The Strategy Compendium, article 26, options](https://backtrader.readthedocs.io/en/latest/strategies-series/en/26-options.html)                                                                                                                                                                            |
| The underlying research   | The expiration-week idea follows Stivers and Sun, [Returns and Option Activity over the Option-Expiration Week for S&P 100 Stocks](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1571786); the put sale has no single paper behind this implementation                                                |
| How well it held up       | Disputed: the library's two implementations of the same expiration-week idea end with opposite signs, and every option price in the category is simulated rather than observed                                                                                                                              |
| Also appears in           | [Option expiration week](../../quantconnect/option-expiration-week-effect/README.md), the [volatility risk premium](../../quantconnect/volatility-risk-premium-effect/README.md) and [VIX futures term structure](../../quantconnect/exploiting-term-structure-of-vix-futures/README.md) in this collection |

## The idea in one paragraph

Two different bets live in this category, and they are only related because both use options. The
first is a calendar bet: the week in which options expire behaves differently from other weeks,
because the firms that sold the options adjust the shares they hold as the contracts approach their
end, so the rule holds the market only during that week and stays in cash the rest of the month. The
second is an insurance bet: the rule sells a contract that pays its owner if gold falls, collects a
cash fee for doing so, and keeps the fee if the fall does not happen. The first bet is on a rhythm in
the calendar. The second is on a fee that is usually too large for the risk it covers, and it wins
often and loses rarely, but heavily.

## Why anyone believed it

For the calendar bet, the mechanism is that an option seller is a forced trader. A firm that has
sold a lot of insurance must hold shares as a hedge, and it must buy or sell those shares as the
price moves; as expiry approaches, the contracts die and the hedge unwinds. That unwinding is a
predictable, mechanical order flow that arrives in the same week every month, and the strategy tries
to be on the same side as it.

For the insurance bet, the mechanism is simpler. Most people who buy options lose money, because
they pay a fee for protection that, on average, is worth more than the protection they receive.
Whoever takes the other side of that trade collects the difference. The counterparty is the buyer
who wants protection more than they want the average, whether a fund hedging a portfolio, a person
buying a lottery ticket on a large move, or a company protecting a price it needs. Selling insurance
is profitable on average precisely because it is unpleasant to do at the worst moment.

## An everyday comparison

Think of a town square that hosts a large market on the third Friday of every month. The stallholders
borrow tables and chairs from the surrounding shops and return everything at once when the market
closes. In the days around that Friday the square is unusually busy, not because the square changed,
but because a large number of people are settling up at the same time. The strategy sets up a stall
only in the week of the monthly market and packs up on the day it closes.

## The rules, step by step

The category holds five backtests. Two trade the expiration week, one blends funds that behave like
option positions, one uses the price of insurance as a timing signal on gold, and one sells puts.

| Strategy                     | Data                          | What it does                                                                |
| ---------------------------- | ----------------------------- | --------------------------------------------------------------------------- |
| Expiration week (XAUUSD)     | Gold daily, 2008 to 2025      | Long only during expiration week in four chosen months                      |
| Expiration week (GLD)        | Gold fund daily, 2008 to 2025 | Long in the bullish months, short in June to August, Monday in, Friday out  |
| Low-volatility options combo | Three funds                   | A blend of a calm equity fund, a covered-call fund and a simulated put sale |
| Options valuation            | Gold daily, 2008 to 2025      | Buys gold when insurance looks cheap against a volatility percentile        |
| GLD put write                | Gold fund daily, 2010 to 2025 | Sells a 30-day put and holds it to expiry or a stop                         |

1. Expiration week, gold-fund version. Use daily bars for a fund that holds gold. Work out the third
   Friday of the current month and the Monday four days before it. If the day is in the expiration
   week, a monthly direction applies: long in January to May and September to December, short in June
   to August. Enter at the close on the Monday, exit at the close on the Friday. Size is 95 percent of
   the account, with a 2 percent stop and a 1.5 percent target measured from the entry price.
2. Expiration week, gold version. The same calendar, but only four months qualify, March, April,
   October and December, and only the long direction is taken. The position is smaller in March and
   April than in October and December, which are weighted 1.2 times.
3. Options valuation. Do not trade options at all. Measure how expensive insurance is by the
   percentile rank of gold's own recent volatility over the last year. Buy gold when that rank falls
   below 0.2, meaning the recent past has been unusually calm, and sell when it rises above 0.8.
4. Put sale. Use daily bars for the gold fund. When the price is above its 200-day average and the
   14-day relative strength index is at or above 30, sell one contract of a put whose strike is 5
   percent below the current price and which expires in 30 days, collecting the premium. Each day,
   re-price the contract. If its price has risen by half from the premium received, buy it back and
   take the loss; if it has not, hold it to expiry. The number of contracts is set so that at most 20
   percent of the account is exposed to the strike.
5. Both expiration rules are checked once a day. The put sale is checked every day and opens a new
   contract whenever none is open and the filter is satisfied.

## The maths, with every symbol named

The calendar rule begins with a date:

```text
third Friday = the third Friday of the calendar month
expiration Monday = third Friday - 4 days
in expiration week = Monday <= today <= Friday
```

- The third Friday is counted within the month, so it is not the same date every month.
- Four days earlier is the Monday of that week, which is the entry day.
- The exit is the Friday close. If Friday is a holiday the exchange moves the expiry to Thursday.

The put sale rests on a simplified price for the contract, because the library does not compute the
standard option model:

```text
intrinsic  = max(0, strike - spot)
time value = vol * sqrt(days / 365) * spot * 0.30
premium    = intrinsic + time value
strike     = round 0.5 * (0.95 * spot)
```

- `spot` is the fund's price today, and `strike` is the price at which the put seller would have to
  buy if the owner uses the contract. Here it is 5 percent below spot and rounded to the nearest half.
- `intrinsic` is what the contract would be worth if it expired immediately. It is zero while the
  spot price is above the strike.
- `vol` is the recent realised volatility, a measure of how much the price has actually been moving,
  expressed as a fraction per year. A value of 0.20 means about 20 percent a year.
- `days` is the number of days left before expiry, and `sqrt(days / 365)` converts the yearly figure
  to that shorter period.
- The 0.30 is the library's own `premium_factor`. It is not the standard option formula; it is a
  stand-in chosen to produce plausible premiums, and it is the single largest assumption in the test.
- `premium` is what the seller receives now and hopes to keep.

At expiry the seller's result on one contract is:

```text
payoff = premium - max(0, strike - spot_at_expiry)
```

- `spot_at_expiry` is the fund's price on the expiry day.
- If the price is above the strike the second term is zero and the seller keeps the whole premium.
- If the price is below the strike the seller buys at the strike, an asset worth less, and the loss
  grows one for one with the fall.

## A worked example

First the expiration-week rule, gold-fund version, on one invented week in June, which the file
treats as a bearish month and therefore a short week. The stop sits 2 percent above the entry and the
target 1.5 percent below it.

| Day | Date   | Close  | Action                     |
| --- | ------ | ------ | -------------------------- |
| Mon | Jun 12 | 100.00 | sell short at 100.00       |
| Tue | Jun 13 | 99.40  | hold                       |
| Wed | Jun 14 | 99.10  | hold; low 98.90, no target |
| Thu | Jun 15 | 98.80  | hold                       |
| Fri | Jun 16 | 96.50  | buy back at 96.50          |

The short position sold at 100.00 and bought back at 96.50, so it gains as the price falls.

```text
Gross = (100.00 - 96.50) / 100.00 = 0.035, that is 3.5 percent
Cost  = 2 * (0.0005 + 0.0002) = 0.0014, that is 0.14 percent
Net   = 3.36 percent
```

The file charges 0.05 percent commission per side. The added 0.02 percent per side stands for the gap
between the price at which the fund can be sold and the price at which it can be bought back, which
a real short pays twice. The target price of 98.50 was never touched, because the lowest price that
week was 98.90.

Now the put sale. Suppose the fund trades at 180.00 with realised volatility of 0.20, so the strike
is 171.00 and the premium is computed as follows:

```text
time value = 0.20 * sqrt(30 / 365) * 180.00 * 0.30 = 0.20 * 0.2867 * 180.00 * 0.30 = 3.10
intrinsic  = max(0, 171.00 - 180.00) = 0
premium    = 3.10 per share, so 310.00 on one contract of 100 shares
```

The stop line is half again the premium, 4.65. The table re-prices the contract each day as the spot
price and the volatility change; the volatility column is the fund's own recent movement, which rises
as the fall quickens.

| Day | Spot   | Days left | Vol  | Intrinsic | Time value | Mark | Action           |
| --- | ------ | --------- | ---- | --------- | ---------- | ---- | ---------------- |
| 0   | 180.00 | 30        | 0.20 | 0.00      | 3.10       | 3.10 | sell, collect    |
| 1   | 179.00 | 29        | 0.22 | 0.00      | 3.33       | 3.33 | hold             |
| 2   | 176.00 | 28        | 0.25 | 0.00      | 3.66       | 3.66 | hold             |
| 3   | 172.00 | 27        | 0.30 | 0.00      | 4.21       | 4.21 | hold             |
| 4   | 168.00 | 26        | 0.32 | 3.00      | 4.30       | 7.30 | buy back at 7.30 |

Check day 4. The spot price of 168.00 is below the strike of 171.00, so the intrinsic value is 3.00.
The time value is `0.32 * sqrt(26 / 365) * 168.00 * 0.30 = 0.32 * 0.2669 * 168.00 * 0.30 = 4.30`.
The mark is 7.30, above the stop line of 4.65, so the seller buys the contract back and pays 7.30 per
share against the 3.10 received, a loss of 4.20 per share, or 420.00 on one contract. That is the
tail of this strategy made visible in five rows: four small gains and one loss four times their size.

For the holds that reach expiry, the arithmetic is the payoff formula. If the fund is at 180.00 on
expiry the seller keeps 3.10 per share, or 310.00 per contract. If it is at 165.00 the loss is
`3.10 - (171.00 - 165.00) = 3.10 - 6.00 = -2.90` per share, or -290.00. If it is at 150.00 the loss
is `3.10 - 21.00 = -17.90` per share, or -1,790.00 on a contract sold for 310.00. One such week is
worth six ordinary ones.

## What the research actually found

The library reports one run per rule. Its own numbers are below.

| Rule                       | Sample                   | Trades | Wins       | Final value | Reward for risk | Worst fall |
| -------------------------- | ------------------------ | ------ | ---------- | ----------- | --------------- | ---------- |
| Expiration week, gold fund | 2008 to 2025, 4,519 bars | 199    | 98 (49.2%) | 947,034     | -0.02           | 32.89%     |
| Expiration week, gold      | 2008 to 2025, 4,638 bars | 36     | 20 (55.6%) | 1,024,000   | 0.07            | 9.90%      |
| Put write, gold fund       | 2010 to 2025, 3,815 bars | 91     | 81 (89.0%) | 1,156,220   | not given       | not given  |

The two expiration-week runs start from a million units of currency and use the same idea on the
same metal over overlapping years. One loses 5.3 percent over seventeen years and the other makes
2.4 percent, which is 0.13 percent a year. Hard-coded monthly directions did not even survive inside
the sample. The put sale did better, ending 15.6 percent higher over sixteen years, which is under
one percent a year, with 82 of its 92 contracts expiring worthless and 9 bought back at a loss. The
89 percent win rate and the single-digit yearly return belong in the same sentence: the fee is small
and the losses are large, and the two roughly cancel.

The library's tests assert the final value, the reward for risk and the worst fall against numbers
captured when each strategy was migrated. Passing those assertions proves the engine computes exactly
what the file says, to the cent, in both engine modes. It proves nothing about whether the strategy
earns anything. In this category that distinction is unusually sharp, because the option prices
themselves are invented by the file's own formula rather than read from the market.

The repository's own options brief,
[Options, hedging and derivative instruments](../../../strategies/books/16_options_and_derivative_instruments.md),
puts the mechanism in perspective. It records that re-hedging a short option position is itself a
large market-impact operation, a metaorder, and that a dealer cannot hedge it for free: the expected
liquidity cost of a delta-hedged book is `N^2 S0 I`, quadratic in the number of options and linear
in the slope of the supply curve (`2103.15302v1`). That is the machinery the expiration-week story
leans on, and it is a cost, not a free lunch. The same brief records a real backtest of writing
weekly index options over 2018 to 2023, where the best short-call variant returned 6.504 percent a
year against 9.889 percent for simply holding the index, with a much smaller worst fall, and where
the choice of re-hedging frequency was a first-order lever (`2407.13908v1`). It also finds that
accounting for fees, slippage and funding can roughly halve an apparent yearly return
(`2512.22476v3`), which is the single most useful number for anyone reading a premium-selling
backtest.

## How this project relates to it

The repository's options brief,
[Options, hedging and derivative instruments](../../../strategies/books/16_options_and_derivative_instruments.md),
is the direct link: it explains the hedging flow the expiration-week rule trades against, prices the
cost of that hedging, and shows what a real index-option selling programme earned after costs.

The finished tutorial [Option expiration week](../../quantconnect/option-expiration-week-effect/README.md)
covers the same calendar idea on an index fund with the published research behind it, including the
finding that the third Friday itself can behave in the opposite direction to the rest of the week.
The [volatility risk premium](../../quantconnect/volatility-risk-premium-effect/README.md) tutorial
covers the insurance-selling side directly, and
[VIX futures term structure](../../quantconnect/exploiting-term-structure-of-vix-futures/README.md)
shows the same category of contract traded on an exchange rather than simulated.

## Where it goes wrong

- There are no real option prices anywhere in this category. The put sale computes its own premiums
  from a formula with a free multiplier of 0.30, and the blends of funds stand in for positions that
  would in reality be held in contracts. A backtest whose prices are generated by the strategy's own
  assumptions is a consistency check on arithmetic, not a measurement of a market.
- The calendar bet was chosen after seeing the calendar. The months, the direction of each month, the
  four months that qualify in the gold version, and the 1.2 weight on October and December are free
  choices. The two implementations of the same idea disagree in sign, which is what a fitted calendar
  looks like when it is run twice.
- Pinning is not a promise. The story that option hedgers hold the price near a strike during expiry
  week is a description of an order flow that can change with the market's structure. Weekly options
  now spread expiries across the month, which thins out the very flow a third-Friday rule trades.
- The put sale hides its risk in the average. An 89 percent win rate with nine losses is a payoff
  shape, not an edge, and the two or three worst weeks in a crisis decide the outcome. The strategy
  also assumes a seller could actually buy or sell at its own computed mark, and pays only 0.05
  percent commission, when real option spreads are far wider.
- Costs compound in a monthly rule. Twelve entries and twelve exits a year at 0.14 percent each is
  about 1.7 percent a year before any market move, which is larger than the entire measured return
  of the better expiration-week run.
- The result is market exposure. The put sale is long gold whenever it is on, so part of its return
  is the metal's long rise, and the expiration-week rules are short an asset that rose over the
  sample.

## Try it yourself

You need nothing but a spreadsheet, a public source of daily prices for a gold fund, and a calendar.

1. Write down the third Friday of each month for the last five years. Do it by hand for two months
   to be sure you have the counting right: find the first Friday, add fourteen days.
2. Build a sheet with one row per trading day and columns for the date, the fund's close, and whether
   the day falls in an expiration week.
3. For each expiration week, compute the Monday-to-Friday return of the fund.
4. In a small table, average those returns over the five years, and count how many weeks were up and
   how many were down.
5. Take the best performing month of the year and the worst, and see whether the difference is more
   than a couple of tenths of a percent a week.
6. Now shift the window one day later, so the entry is Tuesday and the exit is the following Monday,
   and recompute.

What to notice: the average weekly return will usually be positive, because the fund rose over the
period, and the interesting question is whether it is larger than the average week outside the
window. Then notice how much the answer changes when the window moves by a single day. A calendar
rule whose result depends on the exact Monday is describing the sample, not the market.

## Where this came from

- [The Strategy Compendium, article 26, options](https://backtrader.readthedocs.io/en/latest/strategies-series/en/26-options.html),
  the category inventory, the deep dives into the expiration-week rules and the put write, and every
  performance figure quoted above.
- [Options, hedging and derivative instruments](../../../strategies/books/16_options_and_derivative_instruments.md),
  this repository's options brief, including `2103.15302v1`, `2407.13908v1` and `2512.22476v3`.
- Stivers and Sun, [Returns and Option Activity over the Option-Expiration Week for S&P 100 Stocks](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1571786),
  the study behind the expiration-week idea, cited by the library article.
- [Option expiration week](../../quantconnect/option-expiration-week-effect/README.md) and the
  [volatility risk premium](../../quantconnect/volatility-risk-premium-effect/README.md), the
  finished tutorials in this collection that use real index and option data.

## Words used in this tutorial

- delta hedge: holding shares in an amount that moves with an option position, so that a small move
  in the price leaves the option seller roughly unchanged.
- expiration Friday: the day an option contract stops existing, normally the third Friday of the
  month.
- intrinsic value: what an option would be worth if it expired immediately, zero when it is not yet
  in the money.
- option: a contract giving the right, but not the obligation, to buy or sell something at a set
  price before a set date.
- premium: the cash price of an option, paid by the buyer and received by the seller.
- put option: a contract giving its owner the right to sell something at a set price, which is what
  a seller of a put would have to honour if the price falls.
- strike: the set price at which an option can be used.
- volatility: how much a price moves around its average, measured as a percentage per year.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
