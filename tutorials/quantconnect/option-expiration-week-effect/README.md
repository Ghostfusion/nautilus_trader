# Option expiration week: owning the index only in the week options expire

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                           |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | An index fund that holds the largest American companies, the ones whose options trade most heavily                                                                                                                                                                                                                                                              |
| How often it trades       | Once a month: in at the start of the expiration week, out on the expiration date, cash in between                                                                                                                                                                                                                                                               |
| What you need             | A spreadsheet and a calendar of option expiration dates                                                                                                                                                                                                                                                                                                         |
| Where the rules come from | [QuantConnect strategy library, option expiration week effect](https://www.quantconnect.com/tutorials/strategy-library/option-expiration-week-effect) and the [Quantpedia entry](https://quantpedia.com/strategies/option-expiration-week-effect) it cites                                                                                                      |
| The underlying research   | Stivers and Sun, [Returns and Option Activity over the Option-Expiration Week for S&P 100 Stocks](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1571786)                                                                                                                                                                                                  |
| How well it held up       | Mixed: a long sample from 1988 to 2010 on a fixed universe shows a consistent and statistically supported weekly pattern with a plausible mechanism, but the weekly edge is small, another study finds the third Friday itself behaving in the opposite way, and the sample predates weekly options, which changed how much open interest expires on one Friday |
| Also appears in           | Nothing else in this collection describes it                                                                                                                                                                                                                                                                                                                    |

## The idea in one paragraph

An option is a contract that gives its owner the right, but not the obligation, to buy or sell
something at a set price before a set date; that date is called the expiration date. For options on
American shares and indexes the expiration date is normally the third Friday of the month, and the
week containing that Friday is called the expiration week. Large shares with heavily traded options
have, on average, earned more in that week than in other weeks, which researchers have linked to the
way option traders adjust their share holdings as the contracts approach expiry. This strategy holds
an index fund only during the expiration week and keeps the money in cash the rest of the time. It
buys at the start of the week and sells at the end, every month.

## Why anyone believed it

An option is a one-sided deal for the buyer and an obligation for the seller. When someone buys a
call option, which is the right to buy shares, the seller of the option must be ready to hand over
those shares if the option is used. The seller, usually a firm that makes a market in options,
therefore holds a quantity of the shares as a hedge and adjusts that quantity as the price moves;
the adjustment is called a delta hedge, and it means the option seller is constantly buying and
selling the underlying shares.

As the near-term contracts approach their expiration date, the number of options still alive falls,
because they are about to become worthless or to be used up. The hedging that supported them shrinks
with them. The researchers' leading explanation is that, for the shares with the most option
activity, this shrinking leaves the dealers holding fewer offsetting short positions in the shares
than they began the week with, so they buy shares back over the week, and that buying is what lifts
the price. A second suggested channel is that the market's own sense of risk falls over the week, as
measured by the prices of the options themselves. Both are hypotheses rather than proven causes, but
both give the same prediction: the expiration week is a week when one group of large, mechanical
buyers is at work.

## An everyday comparison

Think of a town square that hosts a big monthly market on the third Friday. The stallholders borrow
tables and chairs from the shops around the square, and when the market closes they return
everything at once. In the days around that Friday the square is unusually busy, not because the
square itself changed, but because a large number of people are settling up at the same time. The
shops on the square do more business in that week than in the other three weeks of the month. The
strategy here is to set up a stall only in the week of the monthly market, and to pack up and leave
on the day it closes.

## The rules, step by step

1. Choose your market: the S&P 100, an index of about a hundred of the largest American companies,
   the ones whose options trade most heavily. You trade it through a single index fund, which is one
   listed thing that holds all the shares in the index at once, so you do not have to buy a hundred
   shares.
2. Get a calendar of option expiration dates. For each month, the expiration date is the third
   Friday, unless that Friday is a holiday, in which case it is the Thursday immediately before it.
   A broker, an exchange website or an options data provider publishes these dates.
3. Every Monday, look at the next expiration date on the calendar. If it is within five days, buy
   the index fund with the whole account.
4. Hold it through the week.
5. On the expiration date itself, sell the whole holding and return to cash.
6. Stay in cash on every other day of the month. You are invested for about one week in four.

The QuantConnect version checks the calendar every Monday and holds the fund if the next expiration
is five days away or closer, then liquidates on the expiration date. The underlying research holds
the stocks for the whole expiration week and is in cash otherwise, which is the same shape.

## The maths, with every symbol named

Three quantities matter: the return of one expiration week, the return of the year, and the cost.

The return of a single expiration week, if you are invested:

```text
r_week = P_end / P_start - 1
```

- `r_week` is that week's return, written as a decimal: 0.0053 means 0.53 percent.
- `P_start` is the index fund's price at the start of the week.
- `P_end` is the index fund's price at the end of the week.

The strategy's return over a year, if there are twelve expiration weeks and the rest of the year is
spent in cash:

```text
R_year = ( (1 + r_1) * (1 + r_2) * ... * (1 + r_12) ) * (1 + i)^(40/52) - 1
```

- `r_1` to `r_12` are the twelve expiration weeks' returns.
- `i` is the yearly interest rate paid on cash, and `40/52` is the fraction of the year the money
  sits in cash, because twelve of the fifty-two weeks are invested.
- The source's arithmetic is a shortcut to the same idea: twelve weeks at 0.528 percent each gives
  `12 * 0.528 = 6.34` percent, and three quarters of the year at about 4 percent cash interest adds
  `0.75 * 4 = 3` percent, for about 9.3 percent in total.

The cost of entering and leaving twelve times:

```text
Cost_year = 24 * c
```

- The number 24 is twelve purchases plus twelve sales.
- `c` is the cost per trade as a fraction of the amount traded, covering the gap between the buying
  and selling price plus commission. For a large index fund a realistic figure is 0.0001 to 0.0003,
  that is one to three basis points, where one basis point is one hundredth of one percent.

## A worked example

Eight weeks, four of them expiration weeks. The weekly returns are invented but are of the size the
index actually moves. Cash is assumed to earn 0.08 percent a week, which is about 4 percent a year.
The strategy's column is the index's return in an expiration week and the cash return in any other
week.

| Week | Expiration week | Index return | Cash return | Strategy return |
| ---- | --------------- | ------------ | ----------- | --------------- |
| 1    | Yes             | +0.60%       | +0.08%      | +0.60%          |
| 2    | No              | -0.30%       | +0.08%      | +0.08%          |
| 3    | Yes             | +0.45%       | +0.08%      | +0.45%          |
| 4    | No              | +0.20%       | +0.08%      | +0.08%          |
| 5    | Yes             | -0.25%       | +0.08%      | -0.25%          |
| 6    | No              | -0.60%       | +0.08%      | +0.08%          |
| 7    | Yes             | +0.70%       | +0.08%      | +0.70%          |
| 8    | No              | +0.10%       | +0.08%      | +0.08%          |

Compounding the strategy's column over the eight weeks:

```text
1.0060 * 1.0008 * 1.0045 * 1.0008 * 0.9975 * 1.0008 * 1.0070 * 1.0008 = 1.0183
```

That is a gain of about 1.83 percent. Compounding the index's own column over the same eight weeks:

```text
1.0060 * 0.9970 * 1.0045 * 1.0020 * 0.9975 * 0.9940 * 1.0070 * 1.0010 = 1.0090
```

That is a gain of about 0.90 percent. The strategy beat the index over this stretch because it sat out
two falling weeks, but note that it also sat out the small gains in the other two normal weeks, and
week 5, an expiration week, lost money. Now the costs. There were four expiration weeks, so four
purchases and four sales, eight trades in all. On an account of 10,000 at a cost of 0.0002 per
trade, that is `8 * 10,000 * 0.0002 = 16.00`, or 0.16 percent of the account, which leaves a net
gain of about 1.67 percent. Two things are worth noticing. First, the costs are small because an
index fund is cheap to trade, but they are not nothing: they eat roughly a tenth of the gain in this
example. Second, the example says nothing about whether the strategy works; it only shows how to
apply the rules and how the arithmetic behaves.

## What the research actually found

| Source                                   | What it measured                                                                  | Result                                                                                                                                                                                                            |
| ---------------------------------------- | --------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Quantpedia, summarising the source paper | S&P 100 stocks, weekly returns, 1988 to 2010                                      | 9.3 percent a year, built from 12 expiration weeks at 0.528 percent each plus about 3 percent of cash interest, volatility 8.7 percent, worst fall 15.14 percent, reward-to-risk 0.61                             |
| Stivers and Sun                          | Weekly returns of S&P 100 stocks, focused on the third-Friday week                | Returns over expiration weeks were high relative to other stocks with less option activity, to the same stock's other weeks, and to risk-adjusted measures; the fourth-Friday week slightly underperformed        |
| Cao, Chordia and Zhan                    | The link between firm-specific volatility and next-month returns, by calendar day | The link appeared mainly in the third week of the month, and the usual positive effect was absent on the third Friday itself, which they attribute to selling pressure from shares delivered at option expiration |
| Mohamed                                  | Several calendar effects across five American equity indices                      | Calendar effects differ across indices and interact with one another; the option expiration pattern is one of a family, and the study's largest single effect was the Halloween pattern rather than expiration    |

Read together, the picture is this. There is a long sample showing that the expiration week has, on
average, been a good week for large option-active shares, and the first study offers a mechanism for
why. But the edge per week is about half a percent, the work that found it used data ending in 2010,
and a second study finds the third Friday itself behaving differently from the rest of the week,
which means the effect is not uniform even inside the window the rule trades. The honest reading is
that the pattern is real in the sample it was measured on and that its size and its stability
outside that sample are open questions.

## How this project relates to it

The repository has no calendar-timing strategy, but the mechanism this rule rests on is examined
directly in its options brief,
[the options brief](../../../strategies/books/16_options_and_derivative_instruments.md).
That document reads research showing that re-hedging a short option position is itself a large
market-impact operation, a metaorder, and that a dealer cannot hedge it for free: the expected
liquidity cost of a delta-hedged book, meaning a book whose share holdings are adjusted as prices
move, is proportional to the square of the number of options (`2103.15302v1`). It also records that
under hedging impact the best policy for an option market maker is a partial hedge plus control
through its quoted prices, not a full immediate hedge (`2511.02518v2`). That is the machinery the
expiration-week story relies on: a dealer adjusting share positions as options approach expiry, on
purpose, at scale.

The second related document is
[the options and derivatives brief](../../../strategies/books2/12_options_and_derivatives.md),
which studies option risk premiums over expiration cycles and works with weekly index options of
about eight days over 415 weekly cycles (`2303.16371v1`, p.13). It is a reminder that the option
calendar is now much finer than one Friday a month, which matters for whether a third-Friday rule
still measures what the 1988 to 2010 study measured.

## Where it goes wrong

- The edge per week is tiny. About half a percent per expiration week is smaller than the daily
  noise of the index, so a single bad week inside the window can erase a whole year's advantage, and
  the published worst fall of 15.14 percent came from exactly that.
- The options market has changed. When the original study was written, most listed options expired
  on the third Friday of a month. Weekly options now exist, so a large share of open interest rolls
  off on other days, which may thin out the very effect the rule is built on.
- The costs recur every month. Twenty-four trades a year, each facing the gap between buying and
  selling prices, is a real drag on a strategy that is only invested a quarter of the time.
- Being in cash most of the year is its own bet. The rule misses the market's long-run rise for
  forty weeks a year, and the interest on cash is not guaranteed to stay where it has been. The
  smaller worst fall of 15 percent is partly because the strategy holds almost nothing most of the
  time.
- The research disagrees on the shape of the week. One study finds the third Friday itself behaves
  in the opposite way to the rest of the week, so an investor who holds through Friday morning is
  taking a different bet from one who sells on Thursday.
- Calendar rules are easy to find by accident. There are many candidate patterns in a year, from the
  turn of the month to holidays to the day of the week, and searching them all for the one that
  worked in one sample is how a coincidence gets reported as an effect.

## Try it yourself

You need nothing but a spreadsheet and a public source of index prices; any finance website will
give you daily closes for an S&P 100 index fund.

1. Write down the expiration date for each of the last sixty months: the third Friday of the month,
   or the Thursday before it if that Friday was a holiday.
2. Build a sheet with one row per trading day for those five years and columns for the date, the
   fund's closing price, and whether the day falls inside an expiration week.
3. Add a column for the weekly return, computed once a week: the Friday close divided by the previous
   Friday close, minus one. Mark each week as an expiration week or not.
4. In a separate small table, compute two averages: the average weekly return of the expiration
   weeks, and the average weekly return of all the other weeks.
5. Count how many expiration weeks were positive and how many were negative.
6. Multiply the average expiration-week return by twelve, add three quarters of a typical cash
   interest rate, and compare the total with the fund's five-year average yearly return. Then subtract
   twenty-four trades at about two basis points each.

What to notice: the average expiration-week return will usually be higher, but the difference is a
few tenths of a percent per week and it is swamped by which particular weeks fell. If the expiration
weeks in your sample win by a wide margin, the likely cause is that a small number of very good weeks
did the work, and removing the best two or three weeks will show whether the pattern is broad or
whether it rests on a handful of dates.

## Where this came from

- QuantConnect strategy library, option expiration week effect,
  [the page](https://www.quantconnect.com/tutorials/strategy-library/option-expiration-week-effect):
  the rules as implemented, the S&P 100 index fund, the expiration calendar, in at the start of the
  week, out on the expiration date.
- Quantpedia, option-expiration week effect,
  [the entry](https://quantpedia.com/strategies/option-expiration-week-effect):
  the performance figures, the weekly rebalancing, the confidence rating, the source paper and the
  related papers on the third Friday.
- Stivers and Sun, Returns and Option Activity over the Option-Expiration Week for S&P 100 Stocks,
  [the paper](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1571786): the original study of
  the weekly pattern and of the two suggested explanations for it.
- [the options brief](../../../strategies/books/16_options_and_derivative_instruments.md),
  the repository's options brief, whose Sections 1 to 3 cover option hedging as a market-impact
  operation (`1910.05056v2`, `2511.02518v2`) and the cost of a delta-hedged book (`2103.15302v1`).
- [the options and derivatives brief](../../../strategies/books2/12_options_and_derivatives.md),
  the repository's options and derivatives brief, whose Section 4 works with weekly index options
  over expiration cycles (`2303.16371v1`).

## Words used in this tutorial

- basis point: one hundredth of one percent, so two basis points is 0.02 percent.
- cash: money held in the account without being invested, which earns interest but does not move with
  prices.
- delta hedge: holding shares in an amount that moves with an option position, so that a small move
  in the price leaves the option seller roughly unchanged.
- drawdown: the fall from a peak to the following low, measured in percent.
- expiration date: the date on which an option contract stops existing, normally the third Friday of
  the month for listed American options.
- index fund: a single listed thing that holds every share in an index, so one purchase buys the
  whole index.
- option: a contract giving the right, but not the obligation, to buy or sell something at a set
  price before a set date.
- volatility: how much a price moves around its average, usually quoted as a yearly percentage.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
