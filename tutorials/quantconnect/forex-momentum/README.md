# Currency momentum: buying the currencies that have been climbing

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                     |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | National currencies, bought and sold in pairs against the United States dollar                                                                                                                                                            |
| How often it trades       | About once a month, when the strongest and weakest lists are recomputed                                                                                                                                                                   |
| What you need             | A spreadsheet and one year of monthly exchange rates for the currencies you want to rank                                                                                                                                                  |
| Where the rules come from | [QuantConnect strategy library, forex momentum](https://www.quantconnect.com/tutorials/strategy-library/forex-momentum) and the [Quantpedia currency momentum entry](https://quantpedia.com/strategies/currency-momentum-factor) it cites |
| The underlying research   | Menkhoff, Sarno, Schmeling and Schrimpf, [Currency Momentum Strategies](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1809776), and the Deutsche Bank currency returns study that the Quantpedia page names as its source           |
| How well it held up       | Mixed: the academic spread is large and long-lived, but the tradable version shrinks once costs and the most illiquid currencies are removed, and Quantpedia's own out-of-sample run of the simple rule was slightly negative             |
| Also appears in           | Nothing else in this collection describes currency momentum                                                                                                                                                                               |

## The idea in one paragraph

Most currencies are quoted against the United States dollar, and their value against it drifts up and
down over months. This strategy looks at how much each currency has moved against the dollar over the
past twelve months, then buys the three that climbed the most and sells short the three that fell the
most, putting the same amount of money on each side. Once a month it recomputes the twelve-month moves
and rebuilds the list. The bet is that a currency that has been climbing against the dollar tends to
keep climbing for a while, because the reasons behind a currency's move arrive gradually. It is the
same idea as following a winning streak, applied to exchange rates rather than shares.

## Why anyone believed it

A currency's value reflects a whole country at once: interest rates, inflation, trade, and the mood of
investors holding its bonds. None of those change overnight, and news about them reaches different
people at different times. A large company that must convert money for a cross-border payment, or a
pension fund slowly rebalancing into a foreign bond, acts over days and weeks, not in a single instant.
If the buyers who push a currency up keep arriving after the first rise, the rise continues, and
whoever joined early is paid.

The counterparty is therefore somebody who is late to react, or who must trade regardless of the
outlook. A central bank leaning against a currency's fall, a company that has to pay a bill in another
country on a fixed date, or a hedge fund forced to cut a losing position: each of them can be on the
other side without disagreeing about where the currency is going. The strategy treats that flow as the
fuel for a trend.

## An everyday comparison

A cold front takes days to cross a country. The town at the far end does not get cold the moment the
front begins to move; it gets colder as the front arrives, hour after hour. A farmer who notices that
each morning has been colder than the last can reasonably expect tomorrow to be colder too, even
though the weather is not predictable in any deep sense. The trend is not a forecast of why; it is a
bet that a slow-moving process is still in the middle of moving. Currencies behave the same way when a
policy change or a slow flow of money is working its way through the market.

## The rules, step by step

1. Choose the currencies to follow. The QuantConnect page uses 15 currency pairs against the dollar.
   The Quantpedia page suggests 10 to 20 currencies. A longer list includes smaller, less traded
   currencies, which matters for the costs and the risks described below.
2. For each currency, collect its exchange rate against the dollar, written as the number of dollars
   one unit of the currency buys.
3. Compute the twelve-month change for each currency: today's rate divided by the rate twelve months
   ago, minus one. A rate that went from 1.20 dollars to 1.38 dollars has climbed 15 percent.
4. Rank the currencies by that change, strongest first.
5. Buy the three strongest. Sell short the three weakest. Put the same amount of money on the long
   side as on the short side, so half the account is long and half is short.
6. Hold for one month.
7. At the start of the next month, recompute step 3 for every currency and rebuild the list. Keep any
   cash that is not being used as a deposit earning the overnight interest rate.
8. Repeat from step 4.

## The maths, with every symbol named

The twelve-month momentum of one currency:

```text
M_i = ( S_i,today / S_i,12m_ago ) - 1
```

- `M_i` is the momentum score of currency `i`, as a decimal: 0.15 means 15 percent.
- `S_i,today` is what one unit of currency `i` buys in dollars today.
- `S_i,12m_ago` is what it bought twelve months earlier.

Rank the currencies by `M_i` and take the top three and the bottom three. Give each of the six a
weight, with the long side and the short side carrying equal money:

```text
w_i = +1/6 for each of the three strongest, and -1/6 for each of the three weakest
```

- `w_i` is the fraction of the account committed to currency `i`.
- A positive weight means buying the currency; a negative weight means selling it short.
- The positive weights add to +1/2 and the negative weights to -1/2, so the account is half long and
  half short and the two sides very nearly cancel a move in the dollar itself.

The portfolio's return over the following month is then the weighted sum of the currencies' changes:

```text
R_portfolio = sum over i of ( w_i * R_i )
```

- `R_i` is the change in currency `i` against the dollar over the month, as a decimal.
- `sum over i` means add the six contributions together.
- Because the weights are equal and opposite, this equals half the average return of the strongest
  three minus half the average return of the weakest three.

Finally the cost of the monthly rebuild:

```text
Cost = t * c
```

- `t` is the traded fraction. Replacing half the book means selling half and buying half, which
  counts the money twice, so `t = 1.0`.
- `c` is the cost per trade as a fraction traded, covering the gap between the buying and selling
  price. For the most heavily traded currencies a realistic figure is 0.0002, that is two basis
  points, where one basis point is one hundredth of one percent. Smaller currencies cost more.

One cost that spot prices hide: holding a currency position earns or pays the interest-rate
difference between the two currencies, through the swap charged by the broker each day. For a
long-and-short book this roughly cancels between the two sides, but not exactly, and it must be
counted separately from the trading cost.

## A worked example

Eight currencies, ranked by their change against the dollar over the past twelve months. The numbers
are invented but of the size such moves take.

| Currency | Twelve-month change | Rank | Position |
| -------- | ------------------- | ---- | -------- |
| A        | +14.0 percent       | 1    | long     |
| B        | +9.0 percent        | 2    | long     |
| C        | +6.0 percent        | 3    | long     |
| D        | +1.0 percent        | 4    | none     |
| E        | -2.0 percent        | 5    | none     |
| F        | -7.0 percent        | 6    | short    |
| G        | -11.0 percent       | 7    | short    |
| H        | -16.0 percent       | 8    | short    |

Each of the six positions gets one sixth of the account, so 100,000 is split into six slices of
16,666.67: three bought and three sold short. Now suppose the next month moves the currencies like
this, measured as the change in what one unit buys in dollars:

| Currency | Weight  | Change next month | Contribution    |
| -------- | ------- | ----------------- | --------------- |
| A        | +0.1667 | +1.5 percent      | +0.2500 percent |
| B        | +0.1667 | +0.5 percent      | +0.0833 percent |
| C        | +0.1667 | -0.5 percent      | -0.0833 percent |
| F        | -0.1667 | -1.0 percent      | +0.1667 percent |
| G        | -0.1667 | +0.5 percent      | -0.0833 percent |
| H        | -0.1667 | -2.0 percent      | +0.3333 percent |
| Total    | 0.0000  |                   | +0.6667 percent |

The short positions gain when the currency falls, which is why the negative weights turn a fall into a
positive contribution. The book gained 0.6667 percent before costs. Now suppose that at the rebuild
half the positions change: one long name and two short names are replaced. Selling the old and buying
the new counts the money twice:

```text
t = 2 * ( 3 / 6 ) = 1.0
Cost = 1.0 * 0.0002 = 0.0002, that is 0.02 percent
Net return for the month = 0.6667 - 0.02 = 0.6467 percent
```

Twelve months at that rate, compounded, is about 8.3 percent a year before costs and 8.0 percent
after, which sits close to the published figure below. Two things are worth noticing. First, this is a
small monthly number, so the two-basis-point cost already eats a third of a typical month's gain, and a
smaller or less traded currency would eat more. Second, the worked example says nothing about whether
the strategy works; it only shows how to apply the rules and how the arithmetic behaves.

## What the research actually found

| Source                               | What it measured                                                                  | Result                                                                                                                                                                                                                                                                                                                                   |
| ------------------------------------ | --------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Menkhoff, Sarno, Schmeling, Schrimpf | 40 or more currencies against the dollar, 1976 to 2010                            | A cross-sectional spread in excess returns of up to 10 percent a year between past winners and past losers; not explained by the usual risk factors, and consistent with investors over- and under-reacting; but the profits carry large transaction costs and are concentrated in currencies with high volatility and high country risk |
| Quantpedia, currency momentum factor | 10 to 20 currencies against the dollar, long 3 and short 3, monthly, 1989 to 2009 | 7.61 percent a year, volatility 10.22 percent, worst fall 45.87 percent, reward-to-risk 0.30; confidence rated moderate, with the note that the out-of-sample backtest was slightly negative and the edge appears to be fading                                                                                                           |
| QuantConnect algorithm               | 15 currency pairs against the dollar, 2006 to 2018                                | Long the 3 currencies with the strongest twelve-month move and short the 3 with the weakest, rebuilt monthly                                                                                                                                                                                                                             |
| Bianchi, Drew and Polichronis        | G7 currencies, 1980 to 2004                                                       | Momentum exists but appears transitory, especially over longer look-back periods, and transaction costs have a material negative impact                                                                                                                                                                                                  |
| Grobys and Heinonen                  | Currency return dispersion as a measure of global risk                            | The winner-minus-loser spread is significantly larger in high-dispersion, crisis-like states than in calm ones                                                                                                                                                                                                                           |

The disagreement is narrower than for some strategies and it is about where the profit lives. The
academic camp that studies the full set of currencies finds a large spread and attributes it to slow
reaction and to risk. The trading camp that restricts the universe to currencies with open capital
accounts and liquid markets finds far weaker returns, because the large spread comes partly from
currencies that are expensive or impossible to trade (`2105.10019v2`, a cross-sectional currency study
in the local corpus). Put plainly: the effect is real in the data and hard
to collect in practice, and the two accounts are not measuring the same portfolio.

## How this project relates to it

This repository implements no currency momentum strategy. The closest work is the brief on currencies.

[Macro, rates and foreign exchange](../../../strategies/books2/07_macro_rates_and_fx.md) collects the
FX evidence. Its most relevant record uses a language model to read 174,820 economic data releases and
build a currency signal, going long the top two and short the bottom two of nine major currencies; it
reports an annualised Sharpe ratio above 0.7, strongest for 36- to 60-month look-backs, with an alpha
worth 74 percent of the return after controlling for dollar, carry, momentum and value (`2608.00761v1`,
pp.2 and 12). Crucially, the brief notes that paper reports no net-of-cost result, which is the same
gap that separates the academic and trading versions of this strategy.

[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md)
supplies the decay frame: its central finding is that cross-sectional signals are mostly genuine but
are competed away once published and widely implemented, with one model putting the half-life of a
medium-frequency factor at about 18 months under heavy adoption (`2605.23905v1`, p.1).

## Where it goes wrong

- Costs decide the answer. The gross spread is large; the net spread, after the gap between buying and
  selling prices on both sides and the monthly rebuild, is much smaller, and Quantpedia's own
  out-of-sample run came out slightly negative.
- The universe is not neutral. The biggest winners and losers in a wide list are often small currencies
  with high volatility and high country risk, which are exactly the ones that are expensive to trade
  and that can gap violently if a government changes a rule.
- Interest rates are a hidden position. A currency is not just a price; holding it also earns or pays
  the interest-rate difference. A book that only reads the spot rate has a second, unmanaged bet
  running underneath.
- Regimes end. Long currency trends are often tied to a policy cycle. When the cycle turns, the trend
  the rule is following reverses, and the published worst fall of about 46 percent is what that looks
  like.
- The look-back is a choice. Twelve months, one month, five years: each selects different currencies.
  Picking the look-back after seeing the result is the classic way this kind of rule is made to look
  better than it is.
- Crowding. Because the rule is simple and public, more money runs the same three currencies at the
  same time, which moves the entry price and leaves less for the latecomer.

## Try it yourself

You need a spreadsheet and any public source of monthly exchange rates, which finance websites publish
for the major currencies.

1. Make one column per currency and one row per month, holding the number of dollars one unit buys.
2. Add a row that is the value twelve months later divided by the value twelve months earlier, minus
   one, for each currency. That is the momentum score.
3. Sort the currencies by that score and write down the three strongest and the three weakest.
4. In the next row, write the following month's change for each of those six currencies.
5. Average the changes of the three strongest and the average of the three weakest. The strategy's
   return for that month is half the first average minus half the second.
6. Repeat down the column. Keep a running product of `( 1 + return )` rather than a sum, so the result
   compounds.
7. Finally, subtract 0.02 percent for every month in which half the positions changed.

What to notice: the winners list is fairly stable month to month, so the cost is smaller than it first
looks, but the strategy's gain in a typical month is also small, so the two are the same order of
magnitude. And check whether the three weakest include small, thinly traded currencies; if they do,
your measured result is probably flattered by prices you could not actually have traded at.

## Where this came from

- [QuantConnect strategy library: forex momentum](https://www.quantconnect.com/tutorials/strategy-library/forex-momentum),
  the rules as implemented: 15 pairs against the dollar, 2006 to 2018, long 3 and short 3 by
  twelve-month momentum, rebuilt monthly.
- [Quantpedia: currency momentum factor](https://quantpedia.com/strategies/currency-momentum-factor),
  the restated rules, the 1989 to 2009 figures and the out-of-sample note.
- Menkhoff, Sarno, Schmeling and Schrimpf, [Currency Momentum Strategies](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1809776),
  the 1976 to 2010 study of more than 40 currencies.
- [Macro, rates and foreign exchange](../../../strategies/books2/07_macro_rates_and_fx.md) and
  [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's own briefs, which is where the language-model currency signal and the decay
  findings come from.
- The arXiv identifiers `2608.00761v1`, `2605.23905v1` and `2105.10019v2` are held in the local
  corpus; the first two were read through the briefs above, and the third is a cross-sectional
  currency study in the same corpus.

## Words used in this tutorial

- basis point: one hundredth of one percent, so two basis points is 0.02 percent.
- carry: the return from holding a currency that pays more interest than the one you borrowed.
- dollar-neutral: a book whose gains do not depend on the dollar rising or falling, because the long
  and short sides are the same size.
- exchange rate: the price of one currency in another.
- momentum: the tendency of something that has been rising to keep rising for a while.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- spread: the gap between the buy price and the sell price of the same thing at one moment.
- swap: the daily interest adjustment a broker applies to a position held overnight.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
