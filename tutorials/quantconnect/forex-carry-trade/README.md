# Forex carry trade: earning the interest-rate difference between two currencies

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                     |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Pairs of currencies: it holds the ones that pay the most interest and borrows the ones that pay the least                                                                                                                                                                 |
| How often it trades       | About once a month, when the pairs are rebuilt                                                                                                                                                                                                                            |
| What you need             | A spreadsheet and each currency's central-bank interest rate                                                                                                                                                                                                              |
| Where the rules come from | [QuantConnect strategy library, forex carry trade](https://www.quantconnect.com/tutorials/strategy-library/forex-carry-trade) and the [Quantpedia entry](https://quantpedia.com/strategies/fx-carry-trade) it cites                                                       |
| The underlying research   | Deutsche Bank, [Currency Returns](http://globalmarkets.db.com/new/docs/dbCurrencyReturns_March2009.pdf) (2009), with the century-long evidence of [Doskov and Swinkels](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2060207) (1900-2012)                          |
| How well it held up       | Mixed: the pattern is documented over more than a century and across many currencies, but the reward for the risk is low in the long record, the losses arrive in rare sharp bursts, and the published reward-to-risk is the weakest of the four strategies in this batch |
| Also appears in           | nothing else in this collection                                                                                                                                                                                                                                           |

## The idea in one paragraph

Every country's central bank sets an interest rate, and the rates differ a lot: one country might pay
1 percent a year to hold its money and another might pay 8 percent. In the foreign-exchange market
you always buy one currency by selling another, so you can borrow the low-rate currency and use the
money to hold the high-rate currency. Each day you collect the difference in interest, and the
strategy simply picks the currencies that pay the most and the ones that pay the least, holds the
first set and borrows the second set. It rebuilds the pairs once a month. The bet is that the
interest difference is bigger than the amount the high-rate currency tends to fall.

## Why anyone believed it

A textbook rule called uncovered interest parity says there should be no free money here. In plain
words, if a currency pays 7 percent more interest than another, it should be expected to fall by 7
percent against the other over the year, leaving the two investments equal. The puzzle, documented
for decades, is that high-rate currencies do not fall by enough. On average the interest difference
is larger than the currency's decline, so the trade has made money. Economists argue about why: the
usual story is that holders of high-rate currencies are being paid to carry a risk that shows up at
the worst possible moment. The person on the other side of the trade gives up some interest in
exchange for protection.

The counterparty is whoever wants the low-rate currency as a hiding place. In a panic, money runs to
the currencies of stable, low-inflation countries, which pays nothing in calm times but holds its
value when everything else falls. The carry trader is effectively selling that protection, earning a
small premium in the calm periods and paying it back, with interest, in the panics.

## An everyday comparison

Imagine you can borrow money from a bank that charges 1 percent a year, and put it in an account that
pays 8 percent a year. If both were in your own currency you would have found free money, and the
bank would have closed that gap long ago. The catch is that the 8 percent account is in another
country's money. You earn 7 percent a year on the difference, but if that country's money weakens by
7 percent against yours, you have earned nothing, and if it weakens by 20 percent you have lost far
more than you made. The trade is not really about interest at all; the interest is the small, steady
payment, and the exchange rate is the large, unsteady risk.

## The rules, step by step

1. Choose 10 to 20 currencies that have a central-bank interest rate you can look up. The QuantConnect
   page uses 9, quoted against the American dollar.
2. For each currency, note the central bank's policy rate, the rate the central bank sets for
   overnight lending between banks. That rate anchors the interest a bank pays on deposits in that
   currency.
3. Rank the currencies by that rate, highest first.
4. Hold the currencies with the highest rates and borrow the ones with the lowest. The QuantConnect
   version holds the single highest and borrows the single lowest. Quantpedia holds the top three and
   borrows the bottom three, which spreads the risk.
5. Keep any money not tied up as margin in an overnight deposit, so it earns a little interest.
6. Hold for one month, then rebuild from step 2. Since foreign exchange always involves a pair, the
   trade is written as buying the high-rate currency and selling the low-rate one.

Borrowing a currency means selling it now and agreeing to buy it back later; if it falls, you buy it
back cheaper and keep the difference, and if it rises, you lose. This is the same as selling short.

## The maths, with every symbol named

Over a chosen period, the interest you collect on a pair is the difference between the two rates:

```text
C = r_high - r_low
```

- `C` is the carry for the period, as a decimal: 0.07 means 7 percent over the year.
- `r_high` is the yearly interest rate of the currency you hold.
- `r_low` is the yearly interest rate of the currency you borrowed.
- For a month, divide the yearly difference by twelve.

The return of the trade is the carry plus the change in the exchange rate:

```text
R = C + E
```

- `R` is the total return of the pair over the period.
- `E` is the change in the value of the high-rate currency against the low-rate one, as a decimal:
  0.01 means it rose 1 percent, -0.01 means it fell 1 percent.
- When `E` is positive the trade earns more than the carry; when `E` is negative enough, the whole
  return goes negative.

Uncovered interest parity, the textbook rule, predicts the opposite of the trade:

```text
Expected E = -(r_high - r_low)
```

- In words, the high-rate currency is expected to fall by exactly the interest difference, so the
  expected total return is zero. The carry trade exists because, on the historical average, `E` has
  been smaller in size than the rule predicts, leaving a positive `R`. That gap is the puzzle, and it
  is also the risk.

The cost of rebuilding the pairs each month is the traded fraction times the cost per trade:

```text
Cost = t * c
```

- `t` is the traded fraction, 2.0 when both legs of every pair are closed and reopened.
- `c` is the cost of one currency trade as a fraction of the amount, covering the gap between the
  buying and selling price. A realistic figure for a major pair is 0.0001 to 0.0002, one to two basis
  points, and more for a less-traded currency.

## A worked example

Two currencies. Currency A pays 8 percent a year and currency B pays 1 percent a year. The monthly
carry is the yearly difference divided by twelve, (8 - 1) / 12 = 0.5833 percent a month. Suppose the
value of A against B moves as shown. The percentages are invented but of a realistic size.

| Month | Carry           | Change in A against B | Total           |
| ----- | --------------- | --------------------- | --------------- |
| 1     | +0.5833 percent |           0.0 percent | +0.5833 percent |
| 2     | +0.5833 percent | -0.4 percent          | +0.1833 percent |
| 3     | +0.5833 percent | +0.3 percent          | +0.8833 percent |
| 4     | +0.5833 percent | -0.2 percent          | +0.3833 percent |
| 5     | +0.5833 percent | -1.0 percent          | -0.4167 percent |
| Total | +2.9165 percent | -1.3 percent          | +1.6165 percent |

Five months of interest added 2.9165 percent. The exchange-rate moves subtracted 1.3 percent, and
month 5 alone, a fall of 1.0 percent, took back almost two months of interest. The gross result over
the five months is +1.6165 percent. Now the cost of rebuilding the pairs each month, at two basis
points per side and both legs replaced:

```text
Cost per month = t * c = 2.0 * 0.0002 = 0.0004, that is 0.04 percent
Cost over five months = 5 * 0.04 = 0.20 percent
Net over five months = 1.6165 - 0.20 = 1.4165 percent
```

Two things are worth noticing. First, the interest line is steady and small; the currency line is
lumpy and can be several times its size, so the whole result depends on the exchange rate, not on the
interest. Second, a single bad month of currency movement, the kind that happens in a global panic,
can wipe out many months of carry. The worked example says nothing about whether the strategy works;
it only shows how to apply the rules and how the arithmetic behaves.

## What the research actually found

| Source                                         | What it measured                                                                                         | Result                                                                                                                                                                                                                       |
| ---------------------------------------------- | -------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Quantpedia, summarising the Deutsche Bank work | Ten to twenty currencies, the three highest rates held and the three lowest borrowed, monthly, 1989-2009 | 7.27 percent a year, volatility 9.6 percent, worst fall 32.05 percent, reward-to-risk 0.29, measured on the Deutsche Bank Currency Carry index                                                                               |
| Deutsche Bank, Currency Returns (2009)         | The carry trade in the foreign-exchange market                                                           | States that carry is one of the most widely known and profitable currency strategies, and that it exploits the forward-rate bias, the finding that the forward price is not an unbiased forecast of the future exchange rate |
| Doskov and Swinkels, 1900-2012                 | Twenty currencies over more than a century                                                               | A reward-to-risk of 0.2 to 0.4 over the long run, markedly lower than the values above 0.6 reported for modern samples, with occasional substantial losses                                                                   |
| Lustig, Roussanov and Verdelhan                | The cross-section of currency returns                                                                    | High-rate currencies share a common exposure to a global risk factor and do badly in bad times, which is the long-run story behind the premium                                                                               |
| Daniel, Hodrick and Lu                         | Carry trades among the major currencies, with the dollar removed                                         | The part of the trade that is neutral to the dollar has insignificant abnormal returns; the significant return sits in exposure to the dollar itself, and hedging with options reduces but does not remove it                |
| QuantConnect, the library page                 | The rule as implemented, using central-bank rate data                                                    | Notes that the interest-rate dataset it used was discontinued in 2016, so the implementation cannot be run on current data without a replacement                                                                             |

The sources agree on the pattern and disagree on how much of it is a reward for risk. On the one
side, the trade has paid over a century and in many currencies, which is why it is called a puzzle.
On the other side, once the exposure to the dollar and to global risk is removed, much of the
measured profit shrinks, and the long-run reward-to-risk is low. What all the sources agree on is the
shape of the outcome: many small gains and a few very large losses, which is the signature of selling
insurance rather than of finding a mispriced asset.

## How this project relates to it

This repository's harvest of the macro and foreign-exchange literature,
[macro, rates and foreign exchange](../../../strategies/books2/07_macro_rates_and_fx.md), treats carry
as one of the standard currency factors. It reports a currency signal built from economic data whose
extra return survived controlling for carry, and it states plainly that the result is gross of costs
and turnover, which is the same caution that applies here.

The second related piece is
[banking, credit and funding liquidity](../../../strategies/books2/02_banking_credit_and_funding.md).
It shows that overnight funding is not an independent market but the leftover of payment flows, which
is the plumbing behind the phrase "borrowing one currency to hold another": the rate you actually pay
to borrow depends on the balance sheet behind the trade, not only on the central bank's headline
rate.

The third link is
[execution at the price you get](../../project/execution-at-the-price-you-get/README.md), which
explains why the gap between the quoted price and the price a trade actually gets matters most for a
strategy that rebuilds the whole book every month.

## Where it goes wrong

- Carry crashes. In a global panic, money runs to the safe low-rate currencies and the high-rate ones
  fall sharply, all at once, because everyone is trying to unwind the same trade. The loss in a
  single month can dwarf a year of interest, which is where the 32 percent worst fall comes from.
- You are selling insurance. The steady interest is the premium; the rare loss is the payout. A
  strategy whose whole appeal is small steady payments is fragile by construction, because the tail
  is not in the average.
- The puzzle can close. The trade exists because uncovered interest parity fails in a particular
  way. If more money runs the trade, the high-rate currency falls earlier and the premium shrinks.
- Borrowing is not free and is not the headline rate. The broker's funding charge and margin
  requirements may differ from the central bank's rate, and a sharp move can force the position
  closed at the worst time.
- The data are awkward. Rates change often, the QuantConnect dataset was discontinued in 2016, and
  the currencies that pay the most today are not the ones that paid the most a decade ago.
- The short leg can be dangerous. The currency you borrow is the one that rises in a panic, so the
  loss concentrates exactly when it is hardest to exit.

## Try it yourself

You need nothing but a spreadsheet and two public numbers per month: each currency's central-bank
rate and the exchange rate between them.

1. Pick two currencies, for example the American dollar and the Japanese yen.
2. Build columns: `Month`, `Rate A` (yearly, in percent), `Rate B` (yearly, in percent),
   `Carry` = (`Rate A` - `Rate B`) / 12, `Change in A` (the month's percentage move of A against B),
   and `Total` = `Carry` + `Change in A`.
3. Do this for the last 24 months.
4. Add a running total of the `Total` column.
5. Add a second column that totals the `Change in A` moves on their own.

What to notice: the running total of the carry column will rise in a steady line, and the running
total of the currency moves will be far more jagged and will often be the larger of the two. Then
look at the months where the currency column is strongly negative: those are the months that decide
the whole result, and there are usually only a handful of them in the two years. If your final total
looks positive, ask whether one or two good currency months, rather than the interest, produced it.

## Where this came from

- [QuantConnect strategy library: forex carry trade](https://www.quantconnect.com/tutorials/strategy-library/forex-carry-trade),
  the rules as implemented: nine currencies quoted against the dollar, the highest rate held and the
  lowest borrowed, rebuilt monthly.
- [Quantpedia: FX carry trade](https://quantpedia.com/strategies/fx-carry-trade), the performance
  figures, the instrument count and the underlying papers.
- Deutsche Bank, [Currency Returns](http://globalmarkets.db.com/new/docs/dbCurrencyReturns_March2009.pdf)
  (2009), the source paper that states the forward-rate bias, read as an abstract.
- Doskov and Swinkels, [Empirical Evidence on the Currency Carry Trade, 1900-2012](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2060207),
  the century-long record, read as an abstract.
- [Macro, rates and foreign exchange](../../../strategies/books2/07_macro_rates_and_fx.md) and
  [banking, credit and funding liquidity](../../../strategies/books2/02_banking_credit_and_funding.md),
  this repository's own studies of the currency and funding evidence.

## Words used in this tutorial

- carry: the interest difference you collect for holding one currency against another.
- central bank: the institution that sets a country's base interest rate.
- drawdown: the fall from a peak to the following low, measured in percent.
- exchange rate: the price of one currency in another.
- leverage: borrowing so that a small amount of your own money controls a larger position.
- margin: the money a broker requires you to set aside as a cushion against losses.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- uncovered interest parity: the textbook rule that the interest difference between two currencies
  should be cancelled by the expected move in their exchange rate.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
