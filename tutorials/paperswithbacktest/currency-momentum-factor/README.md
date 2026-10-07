# Currency momentum: buying the currencies that have already risen and selling the ones that have fallen

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                   |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Exchange rates, through futures contracts on major currencies such as the euro, the yen and the Australian dollar                                                                                                                                       |
| How often it trades       | About once a month, when the whole basket is rebuilt                                                                                                                                                                                                    |
| What you need             | A spreadsheet and twelve months of exchange rates                                                                                                                                                                                                       |
| Where the rules come from | [The awesome-systematic-trading list](https://github.com/paperswithbacktest/awesome-systematic-trading), which keeps the coded rule, and the [Quantpedia currency momentum entry](https://quantpedia.com/strategies/currency-momentum-factor/) it cites |
| The underlying research   | Menkhoff, Sarno, Schmeling and Schrimpf, [Currency Momentum Strategies](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1809776)                                                                                                                    |
| How well it held up       | Mixed: a large and widely replicated spread in academic samples, but replications that include the real cost of trading the currencies shrink it, and one vendor's own out-of-sample run turned slightly negative                                       |
| Also appears in           | [Forex momentum](../../quantconnect/forex-momentum/README.md) and [Risk premia in forex markets](../../quantconnect/risk-premia-in-forex-markets/README.md) in this collection                                                                          |

## The idea in one paragraph

An exchange rate is just the price of one country's money in another country's money. Some currencies spend
months at a time drifting in one direction: the Australian dollar strengthens for a year, then the yen
weakens for a year. This strategy looks at how each of a handful of currencies has moved against the
American dollar over the past twelve months, buys the three that rose the most, and sells short the three
that fell the most, putting the same amount on each. It holds that package for a month, then looks again
and rebuilds. The bet is that a currency that has been rising keeps rising for a while longer.

## Why anyone believed it

Exchange rates move because of interest rates, trade flows and news, and all three arrive gradually.
Central banks change interest rates in small steps and signal the direction months in advance.
Companies that export and import have to convert money on a schedule, so they keep selling the same
currency week after week regardless of the price. Investors who hear the news late buy after the first
move, which pushes the price further.

The person on the other side of the trade is the one who is slow to update, or who must trade for a
reason unrelated to the outlook. A central bank trying to stop its currency from rising will sell it
into a trend. A firm that must pay a foreign bill will sell its own currency whatever the price. If
those sellers keep appearing, the currency that has been rising keeps rising.

## An everyday comparison

Picture a footpath across a wet field. The first walker picks a line by chance, but their feet press the
grass flat, and the next walker can see that track and follows it. Each new walker makes the track more
visible, so even more follow it, and the path that started as one person's guess becomes the road
everyone takes. Nothing says the first line was the best line, only that it was taken first and that
following a visible track is easier than choosing a fresh one. Currency momentum is the same story told
with exchange rates instead of mud.

## The rules, step by step

The coded rule in the list uses eight currency futures, and the vendor's description allows ten to
twenty. Here is the rule with eight, which is what the code runs.

1. Choose a set of major currencies that all trade against the American dollar. The code uses the
   Australian dollar, the British pound, the Canadian dollar, the euro, the Japanese yen, the Mexican
   peso, the New Zealand dollar and the Swiss franc.
2. For each currency, measure its change over the past twelve months. In the code this is the
   percentage change in price over the last 252 trading days, which is twelve months of roughly 21
   working days each.
3. Rank the currencies from the largest twelve-month change to the smallest.
4. Buy the three with the largest changes, in equal amounts: one third of the money on each.
5. Sell short the three with the smallest changes, in equal amounts: one third on each.
6. Hold for one month. Do not react to prices in between.
7. At the start of the next month, recompute step 2 for all eight currencies and repeat from step 3.
   Close anything that has dropped out of the two groups and open whatever has entered.
8. Money that is set aside as a deposit for the futures contracts, and is therefore not doing any work,
   is placed in an overnight deposit so that it earns the short-term interest rate.

One detail matters for the reader with a spreadsheet: the code skips nothing at the end of the window;
it uses the full twelve months. Some other researchers leave out the most recent month, using the return
from twelve months ago to one month ago. Over days and weeks, prices tend to bounce back after a sharp
move, and that bounce works against the signal, so leaving the last month out is a common refinement.

## The maths, with every symbol named

The strategy is one calculation per currency, one sort, and one weighted average.

The momentum of a currency:

```text
M_i = P_i / P_i_252days_ago - 1
```

- `M_i` is the momentum score of currency `i`, written as a decimal: 0.14 means a rise of 14 percent.
- `P_i` is the price of currency `i` today, in American dollars per unit of that currency.
- `P_i_252days_ago` is its price 252 trading days earlier.

Rank the currencies by `M_i` from largest to smallest. Buy the top three and sell short the bottom three,
giving each the same weight:

```text
w_i = +1/3 if currency i is in the top three
w_i = -1/3 if currency i is in the bottom three
w_i = 0 otherwise
```

- `w_i` is the fraction of the account placed in currency `i`.
- A positive weight is a purchase, a negative weight is a short sale (borrowing the thing, selling it
  now, and buying it back later).
- The three long weights add to +1 and the three short weights add to -1, so no net money is at risk
  from currency moves alone, but twice the account is exposed in total, one time long and one time short.

The return of the whole package over the following month is the sum of each weight times that currency's
move:

```text
R_package = w_1 * r_1 + w_2 * r_2 + ... + w_8 * r_8
```

- `r_i` is the change in currency `i`'s price over the next month, in American dollars.
- Multiplying each move by its weight, then adding, gives the package's return for the month.

Then subtract the cost of changing the positions:

```text
Cost = t * c
```

- `t` is the amount traded, counted twice because selling then buying the replacement is two trades.
  Selling one third and buying a different one third is a traded amount of two thirds.
- `c` is the cost per trade as a fraction of the amount traded, covering the gap between the price a
  seller receives and the price a buyer pays. For liquid currency futures this gap is small, about
  0.0002 to 0.0005, that is two to five basis points, where one basis point is one hundredth of one
  percent.

A short sale of a currency future is not the same as borrowing shares: there is no lender and no borrow
fee to pay, because the futures contract is a two-sided agreement. The only cost is the gap and any
commission.

## A worked example

Eight currencies, ranked by their change over the past twelve months. The figures are invented but of the
size these moves actually take.

| Currency | Price a year ago | Price today | M     | Rank |
| -------- | ---------------- | ----------- | ----- | ---- |
| AUD      | 0.7000           | 0.7980      | 0.14  | 1    |
| EUR      | 1.1000           | 1.2320      | 0.12  | 2    |
| NZD      | 0.6300           | 0.6993      | 0.11  | 3    |
| CAD      | 0.7500           | 0.8175      | 0.09  | 4    |
| GBP      | 1.2500           | 1.3250      | 0.06  | 5    |
| MXN      | 0.0500           | 0.0525      | 0.05  | 6    |
| CHF      | 1.1000           | 1.0670      | -0.03 | 7    |
| JPY      | 0.0068           | 0.006324    | -0.07 | 8    |

The three with the largest changes are the Australian dollar, the euro and the New Zealand dollar, so the
package is long those three, one third of the money each. The three with the smallest changes are the yen,
the Swiss franc and the Mexican peso, so it is short those three, one third each. The Canadian dollar and
the pound are ignored this month.

Now suppose the next month brings these moves, measured in American dollars:

| Currency | Weight  | Next-month move | Contribution    |
| -------- | ------- | --------------- | --------------- |
| AUD      | +0.3333 | +1.5 percent    | +0.5000 percent |
| EUR      | +0.3333 | +0.8 percent    | +0.2667 percent |
| NZD      | +0.3333 | +1.0 percent    | +0.3333 percent |
| JPY      | -0.3333 | -0.5 percent    | +0.1667 percent |
| CHF      | -0.3333 | -0.2 percent    | +0.0667 percent |
| MXN      | -0.3333 | -1.2 percent    | +0.4000 percent |
| Total    | 0.0000  |                 | +1.7333 percent |

The package gained 1.7333 percent before costs. The short positions contributed positively because a
currency that falls makes money for the seller: the negative weight times a negative move is a positive
number. Now the cost. Suppose the whole package was new this month, so six positions of one third each
were opened, a traded amount of two on each side:

```text
t = 2 * (3 positions of one third) = 2
Cost = 2 * 0.0003 = 0.0006, that is 0.06 percent
Net return for the month = 1.7333 - 0.06 = 1.67 percent
```

That is one invented month. It shows how to apply the rules and how the arithmetic behaves, and it says
nothing about whether the idea works. Note how small the trading cost is here compared with a monthly
rule in shares: currency futures are cheap to trade, which is why this idea survived contact with costs
long enough to be studied.

## What the research actually found

| Source                                                                          | What it measured                        | Result                                                                                                                                                                                                                                                                                                 |
| ------------------------------------------------------------------------------- | --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Menkhoff, Sarno, Schmeling and Schrimpf, Currency Momentum Strategies           | More than 40 currencies, 1976 to 2010   | A spread of up to 10 percent a year between past winners and past losers, not explained by the usual risk factors; but the winners are currencies with high volatility and high country risk, and the authors state there are "effective limits to arbitrage" that stop the returns being easily taken |
| The same paper                                                                  | The cost of trading                     | Momentum portfolios "incur large transaction costs" and are heavily skewed towards hard-to-trade currencies, which the authors give as the reason the spread is not simply a free lunch                                                                                                                |
| Deutsche Bank currency momentum index, reported by Quantpedia                   | Ten to twenty currencies, 1989 to 2009  | 7.61 percent a year, volatility 10.22 percent, worst fall 45.87 percent, reward-to-risk 0.30                                                                                                                                                                                                           |
| Bianchi, Drew and Polichronis, reported by Quantpedia                           | The G7 currencies, 1980 to 2004         | Momentum is present but "transitory", and transaction costs have "a material negative impact" on the excess return                                                                                                                                                                                     |
| Quantpedia's own out-of-sample run                                              | The coded rule, after the source sample | Quantpedia grades its confidence in the idea as Moderate and notes that its out-of-sample test showed slightly negative performance, with the apparent alpha deteriorating                                                                                                                             |
| The awesome-systematic-trading list's replication record, across all its papers | 4,843 coded strategies                  | The median replication returned a Sharpe ratio of 0.37, and only 48 percent cleared a t-statistic of 1.96, so half the published record cannot be distinguished from zero on its own sample                                                                                                            |

Two things are worth separating. That currencies trend, and that a trend-following rule makes money after
costs, are different claims. The first is well documented. The second is where the evidence thins: the
academic spread is large, but the authors themselves warn that it sits in currencies that are expensive
to trade, and the vendor that coded this particular rule found that taking the costs into account, and
then testing outside the original sample, removed most of the reward.

## How this project relates to it

This repository studies the same family of ideas. Section 4 of
[Macro, rates and foreign exchange](../../../strategies/books2/07_macro_rates_and_fx.md) collects the
high-frequency evidence on how exchange rates and interest rates react to scheduled announcements, which
is the same news-and-slow-reaction mechanism momentum is meant to capture, and reports a cross-sectional
currency signal built from economic data releases with an annualised Sharpe ratio above 0.7.

The finished tutorials closest to this one are
[Forex momentum](../../quantconnect/forex-momentum/README.md), which applies the twelve-month ranking
rule to a single pair rather than a basket, and
[Risk premia in forex markets](../../quantconnect/risk-premia-in-forex-markets/README.md), which
explains the carry and value styles that momentum is usually compared against.

## Where it goes wrong

- Crowding. Once a rule is published and cheap to trade, more money runs it. The buying happens earlier,
  the move arrives smaller, and the latecomers pay for the reversal. Currency momentum has been public
  since at least 2010, which is exactly the window in which the vendor's own out-of-sample test faded.
- Costs and liquidity. The winners in the academic sample are not the cleanest currencies; they are ones
  with high volatility and high country risk, which are the expensive ones to trade in size. A rule that
  looks profitable on a chart can lose money once the gap between buying and selling prices is charged.
- It is a two-sided bet. The package sells short, so it needs a broker willing to let you do that, and it
  can lose on both sides at once when a set of large moves goes against the ranking.
- The signal inverts over short horizons. Over days to weeks, prices tend to bounce back rather than
  continue, which is why some researchers skip the most recent month. A rule built on one month of
  returns can lose money even when the twelve-month version earned it.
- The number of rules tried. There are many ways to define momentum: which lookback, whether to skip a
  month, how many currencies to hold, how often to rebuild. Choosing the best of them after seeing the
  results is how a rule that looks strong turns out to be fitted to the past.
- For the whole idea to be false, it is enough that the trend reflects no persistent behaviour and that
  the apparent profits are the result of a few large lucky moves in a small sample. The list's own
  median Sharpe ratio of 0.37 across thousands of papers is the clearest warning that this is common.

## Try it yourself

You need a spreadsheet and a public source of exchange rates. Any finance website will give you monthly
values for a handful of currencies against the American dollar.

1. Build a sheet with one column per currency and one row per month for the last ten years.
2. Add a column for each currency holding the twelve-month change: this month's value divided by the
   value twelve rows above, minus one.
3. For each row, write down which three currencies have the largest value and which three have the
   smallest. Those are the ones the rule would buy and sell short.
4. In the row below, average the next month's moves of the three largest, and of the three smallest. The
   first number is what the long side earned; the negative of the second is what the short side earned.
5. Add the two, then subtract about 0.05 percent for each side of each position that changed from the
   previous month.
6. Keep a second column that simply holds the average of all the currencies each month, so you have
   something to compare against.

What to notice: some months none of the six positions change, so the cost is near zero, and other months
all six do. Over ten years the rule will usually be close to the average, sometimes ahead and sometimes
behind. If your sheet shows it winning by a wide margin, check whether you used currencies that no longer
exist or exchange rates that were fixed by a government at the time; both are ways of accidentally using
information you would not have had.

## Where this came from

- [Currency Momentum Factor](https://quantpedia.com/strategies/currency-momentum-factor/), the page that
  states the rules and reports the Deutsche Bank index figures and the out-of-sample note.
- Menkhoff, Sarno, Schmeling and Schrimpf,
  [Currency Momentum Strategies](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1809776), the study
  of more than 40 currencies over 1976 to 2010.
- Bianchi, Drew and Polichronis, reported on the same page, on momentum and transaction costs in the G7
  currencies over 1980 to 2004.
- [The awesome-systematic-trading list](https://github.com/paperswithbacktest/awesome-systematic-trading),
  which holds the coded rule and the project's own replication record.
- [Macro, rates and foreign exchange](../../../strategies/books2/07_macro_rates_and_fx.md), this
  repository's brief on announcement-driven currency and rate moves.

## Words used in this tutorial

- basis point: one hundredth of one percent, so five basis points is 0.05 percent.
- carry: the return from holding a higher-interest currency and funding it with a lower-interest one.
- drawdown: the fall from a peak to the following low, measured in percent.
- futures contract: an agreement to buy or sell something at a set price on a set future date.
- momentum: the tendency of something that has been rising to keep rising for a while.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- spread: here, the gap between the price at which something can be bought and the price at which it can
  be sold.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
