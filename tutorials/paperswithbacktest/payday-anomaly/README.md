# The payday anomaly: owning the share index on the day after most people are paid

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                      |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | A fund that tracks the whole American share index, held for about one day each month                                                                                                                                                                       |
| How often it trades       | Once a month: one purchase before one close, one sale before the next close                                                                                                                                                                                |
| What you need             | A spreadsheet and a long history of daily index closing prices                                                                                                                                                                                             |
| Where the rules come from | [The list's implementation file](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/payday-anomaly.py) and the [Quantpedia entry](https://quantpedia.com/strategies/payday-anomaly) it cites                     |
| The underlying research   | Ma and Pratt, [Payday Anomaly](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=3257064)                                                                                                                                                                |
| How well it held up       | Mixed: the pattern is measured over three decades in one market with a t-statistic above 3, but it rests on a single paper, the claimed cause is a flow the paper does not observe, and calendar patterns are the easiest kind of effect to find by chance |
| Also appears in           | [Turn of the month](../../../tutorials/quantconnect/turn-of-the-month-in-equity-indexes/README.md), the better-known calendar effect this one extends                                                                                                      |

## The idea in one paragraph

Most households in the United States are paid on the 15th of the month or at its end, and a slice of
each pay packet is invested automatically for retirement. This strategy holds the whole American
share index for a single day each month, the day after the mid-month pay date, and stays in cash for
the rest of the month. There is no ranking and no filter: the rule is a calendar. The bet is that the
money arriving from pay packets is invested into index funds on a predictable date, that the buying
nudges prices up that day, and that the same date comes round every month.

## Why anyone believed it

Salary arrives on a schedule, and so does the money that flows from it into investments. If a
retirement contribution is deducted from a pay packet on the 15th and reaches the financial
institution at the end of that day, it can be invested on the next trading day. The funds it is
invested into are broad-market index funds, which buy whatever shares the index contains. So on the
day after payday a large, price-insensitive buyer shows up. A buyer who does not care about price is
the reason a predictable date can move a price.

The counterparty is whoever sells to that buyer: a fund trimming a position, a trader taking profits,
or a household spending the money it was just paid. The paper's further claim is that this particular
date has drawn less attention than the better-known turn-of-the-month effect, so it has been traded
away less. It also notes that although more firms now pay every two weeks, the workers with the
highest average hourly earnings are still paid twice a month, which is the arrangement the strategy
depends on.

## An everyday comparison

Think of a small town where almost every employer pays wages on the 15th. The farmers' market opens
the next morning, and the stallholders, who know what day it is, charge a little more because they
know the money has just arrived. Prices drift back over the following week. The strategy is to buy at
the market on the evening of the 15th and sell the next evening, before the prices settle back. The
traders who sell to the shoppers are the counterparty, and they need a reason to sell on that
particular morning rather than hold.

## The rules, step by step

1. Choose the instrument: a fund that tracks the S&P 500 index, or a futures contract on the same
   index.
2. Find the mid-month pay date. It is the 15th of the month in most months. If the 15th is a
   Saturday, use the Friday before it, the 14th. If the 15th is a Sunday, use the Friday before
   that, the 13th.
3. On the trading day that is the pay date, one minute before the market closes, buy the fund with
   all the money.
4. One minute before the close of the next trading day, sell everything.
5. Hold cash for the rest of the month. This is a long-only rule: it is never out of the market for
   more than the night and the day it holds.
6. Repeat from step 2 every month. There is nothing else to compute.

A note on the exact day. The rule buys at the close of the pay date and sells at the close of the
following day, so the money is at risk from the pay-date close to the next close. That next close is
the price the paper studies, and it is why the strategy is described as holding "the 16th day".

## The maths, with every symbol named

The return captured in one month:

```text
R = P_next / P_payday - 1
```

- `R` is the return of the one-day holding, written as a decimal: 0.005 means half a percent.
- `P_payday` is the closing price of the index fund on the pay date.
- `P_next` is the closing price on the next trading day.

The return over a year is the average of the twelve monthly returns, multiplied by twelve:

```text
R_year = 12 * average(R over the twelve months)
```

- `average(R over the twelve months)` is the sum of the twelve one-day returns divided by twelve.
- Multiplying by twelve turns a monthly average into a yearly figure; it is a close approximation
  because the numbers are small.

The cost is paid twice a month, once when buying and once when selling:

```text
Cost_year = 12 * 2 * c
```

- `c` is the cost of one trade as a fraction of the amount traded, covering the gap between the
  buying and the selling price plus any commission.
- The two is because the position is bought and sold once each month. For a highly liquid index fund
  a cautious figure is 0.0005, five basis points per side, where one basis point is one hundredth of
  one percent.

The net result is `R_year - Cost_year`.

## A worked example

Six months of invented but plausible index levels. The second and third columns are the index value
at the close of the pay date and at the close of the next trading day.

| Month | Pay date                 | Close on pay date | Close next day | R       |
| ----- | ------------------------ | ----------------- | -------------- | ------- |
| Jan   | Mon 15 Jan               | 4800.00           | 4824.00        | +0.500% |
| Feb   | Thu 15 Feb               | 4900.00           | 4885.30        | -0.300% |
| Mar   | Fri 15 Mar               | 5100.00           | 5125.50        | +0.500% |
| Apr   | Mon 15 Apr               | 5050.00           | 5024.75        | -0.500% |
| May   | Wed 15 May               | 5200.00           | 5226.00        | +0.500% |
| Jun   | Fri 14 Jun (15th is Sat) | 5300.00           | 5328.60        | +0.540% |

Each return is the next-day close divided by the pay-date close, minus one. For January that is
4824.00 / 4800.00 - 1 = 0.005, and for February it is 4885.30 / 4900.00 - 1 = -0.003.

The average of the six returns is (0.500 - 0.300 + 0.500 - 0.500 + 0.500 + 0.540) / 6 = 1.240 / 6 =
0.2067 percent per month. Over a year at that rate the return is 12 * 0.2067 = 2.48 percent before
costs. That sits close to the published figure of 0.214 percent a month below.

The cost is 12 * 2 * 0.0005 = 0.012, that is 1.20 percent a year at five basis points per side. The
net result is 2.48 - 1.20 = 1.28 percent a year. At a cheaper one basis point per side the cost is
0.24 percent a year and the net is 2.24 percent.

Two things are worth noticing. First, the whole edge is a little over two percent a year before
costs, so the price paid to trade is a large part of the decision. Second, the worked example says
nothing about whether the strategy works. It shows only how to apply the rules and how the arithmetic
behaves.

## What the research actually found

| Source                             | What it measured                                                | Result                                                                                                                                                                                                                       |
| ---------------------------------- | --------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Ma and Pratt, Payday Anomaly       | Daily index returns around the mid-month pay date, 1980 to 2010 | The 16th of the month was the third-best day of the month on average, and its ranking had moved up in every decade since the 1950s; the paper attributes this to the investment of semi-monthly pay packets                  |
| Quantpedia, summarising that paper | The monthly mean return, 1980 to 2010                           | A mean of 0.214 percent a month, which is about 2.57 percent a year, with volatility 4.31 percent, a worst fall of 12.06 percent, a reward-to-risk of 0.6 and a t-statistic of 3.07, taken from the paper's table 4, panel B |
| The list's replication record      | 4,843 coded papers, each over its own full history              | The median replication has a reward-to-risk of 0.37, 48 percent clear a t-statistic of 1.96, the median test window is 34 years, and the median carries a market exposure of +0.17                                           |

Read together, the picture is this. There is a measured tendency for the middle of the month to be a
better-than-average day, and the size is small: a couple of percent a year before costs. The vendor's
own page grades its confidence as strong, but the grade that matters for a reader is what is inside
the measurement. The paper shows that the day is good; it does not show the money arriving, so the
cause remains a reasoned story rather than an observed fact.

## How this project relates to it

The finished tutorial
[turn of the month](../../../tutorials/quantconnect/turn-of-the-month-in-equity-indexes/README.md)
covers the older and larger calendar effect, where the money is held in a short window at the end of
one month and the start of the next. That page is worth reading beside this one because it reports
the same kind of evidence and reaches the same awkward conclusion: the pattern is real in the data,
and the cash-flow story usually offered for it was tested and rejected by the source paper. The
repository brief
[insurance, pensions and household finance](../../../strategies/books2/09_insurance_pension_and_household_finance.md)
supplies the demand-side facts this strategy leans on, including that household flows are sticky and
low-elasticity (`1611.08330v1`) and that the median retail portfolio holds an effective two stocks or
fewer (`2503.17778v1`, p.16), which is why a small, scheduled, price-insensitive flow is a plausible
thing for a market to notice.

## Where it goes wrong

- The cause is unobserved. The paper measures the return of a day; it does not have the payroll or
  retirement-flow records that would show the money arriving, so the explanation is inferred from the
  timing rather than seen.
- Calendar patterns are easy to find. There are hundreds of candidate days in a month and a year,
  and choosing the one that has done best in the past is the classic way to find a pattern that does
  not survive. The paper's own defence is that the effect has not diminished, which is a claim about
  the past, not a guarantee for the future.
- Payroll practice changes. If employers shift further toward fortnightly pay, or if contributions
  are invested over several days rather than on one, the flow the strategy relies on may arrive on
  different dates or be spread thin.
- Costs decide the outcome. The edge is a couple of percent a year before costs and the position
  trades twice a month, so a widening of the gap between buying and selling prices can remove most or
  all of it.
- Being in cash 29 days a month. This is a timing rule, not an investment: it catches one day's
  move and misses whatever happens on the other days, including any large rise that lands on a day it
  is not holding.
- The whole idea would be false if the day-after-pay returns simply reflect a general mid-month
  pattern caused by something else, such as month-end rebalancing or the release of scheduled
  economic data. Separating the pay date from those other calendars is what would settle it.

## Try it yourself

You need a spreadsheet and a long history of daily closing prices for one index fund. You do not need
any money.

1. Build a sheet with one row per trading day: the date and the closing price.
2. Add a column marking the pay date for each month: the 15th, or the Friday before it if the 15th
   falls on a weekend.
3. Add a column with the return from the pay-date close to the next trading day's close.
4. Separate the rows by pay date and average the return of each of the twelve months across the whole
   history.
5. Rank the days of the month by their average return, best first.
6. Subtract the cost of two trades, about 0.10 percent at five basis points per side, from the
   average return of the pay-date day.

What to notice: the average return is positive but small, and a handful of months carry most of it.
Notice too how much a single year or a single crash changes the ranking, which is why the paper needs
three decades of data to make the case at all. If your sheet shows the pay-date day winning by a wide
margin, check whether you included the day of a large market fall, because one big move can turn a
small average into a large one.

## Where this came from

- [The list's implementation file](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/payday-anomaly.py),
  the rules as coded: the pay-date calculation, the buy at the pay-date close and the sale at the next
  close.
- [Quantpedia: payday anomaly](https://quantpedia.com/strategies/payday-anomaly), the indicative
  performance figures and the description of the effect.
- Ma and Pratt, [Payday Anomaly](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=3257064), the
  original study over 1980 to 2010.
- [Insurance, pensions and household finance](../../../strategies/books2/09_insurance_pension_and_household_finance.md),
  this repository's brief, which supplies `1611.08330v1` and `2503.17778v1`.

## Words used in this tutorial

- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- futures contract: an agreement to buy or sell something at a fixed price on a future date, used
  here as a cheap way to hold an index.
- index: a single number that tracks the average price of a group of shares, such as the S&P 500.
- long-only: a rule that only ever buys, and never sells something it does not own.
- price-insensitive: a buyer that buys the same amount whatever the price, for example a fund that
  has to invest a contribution it has just received.
- t-statistic: a number that says how many times larger an average is than the wobble in that
  average; above about 2 it is usually called significant.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
