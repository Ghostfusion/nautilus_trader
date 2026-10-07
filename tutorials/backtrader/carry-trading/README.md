# Carry trading: getting paid to hold one thing instead of another, and the crash that comes with it

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                       |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | The currencies of higher-interest countries against lower-interest ones, and a few commodity and metal spreads where storage and delivery costs play the same role                                                          |
| How often it trades       | Every twenty-one trading days, when the ranking is recomputed                                                                                                                                                               |
| What you need             | Python and a data file; the backtests reconstruct carry from prices because the exported data contains no interest rates                                                                                                    |
| Where the rules come from | [The compendium's carry trading article](https://backtrader.readthedocs.io/en/latest/strategies-series/en/29-carry-trading.html)                                                                                            |
| The underlying research   | Koijen, Moskowitz, Pedersen and Vrugt, [Carry](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2632697)                                                                                                                 |
| How well it held up       | Disputed: the academic carry factor earns a premium across many markets, but the four files here disagree among themselves, one losing 8.8 percent and one gaining 30.7 percent, and carry crashes make the premium fragile |
| Also appears in           | [Forex carry trade](../../quantconnect/forex-carry-trade/README.md) and [Risk premia in forex markets](../../quantconnect/risk-premia-in-forex-markets/README.md) in this collection                                        |

## The idea in one paragraph

Some currencies pay a high interest rate and some pay a low one. If you borrow in the low-paying
currency, convert the money, and hold it in the high-paying currency, you collect the difference
every day, as long as the exchange rate does not move against you. That difference is called carry.
The same idea appears in commodities, where the price of a contract for delivery in six months can
sit above or below the price of one for delivery now, and the gap is paid to whoever holds the right
position. Carry trading ranks a set of markets by this payment, buys the ones that pay the most, and
sells the ones that pay the least. The return arrives slowly in normal times and disappears all at
once in a panic, when everyone tries to exit the same trade together.

## Why anyone believed it

If a currency offers a much higher interest rate than another, that is usually because its country
has higher inflation or a central bank that is holding rates up to fight it. Over the long run those
currencies tend to weaken, so the high interest is partly pay for taking the risk of a fall in the
exchange rate. In calm markets few people want to be short the high-yielder, so the payment looks
free. The counterparty is anyone who needs to borrow the high-rate currency or sell it for reasons
of their own - a company paying for imports, a central bank intervening, a speculator unwinding a
crowded position. They keep paying the carry, and the rule keeps collecting it, until the day when
the low-rate currency soars and the trade unwinds everywhere at once.

## An everyday comparison

Think of a market stall that rents out folding tables by the day. On a normal day the rent is small
and steady, and the stall looks like a quiet, reliable earner. Once or twice a decade a storm rolls
through, the market closes for a week, everyone returns their tables at once, and the stall keeps
paying the rent it owes on tables it can no longer hire out. The daily rent is real and it is
collected most days; the risk is not that the rent stops but that one bad week erases a year of it.
That is the shape of a carry trade: small steady gains, rare large losses.

## The rules, step by step

The mechanism is: measure how much each market pays to be held, rank the markets by that payment,
hold the top of the list against the bottom, and keep the book balanced so that the overall market
direction cancels as much as possible.

1. Choose a set of markets with a comparable payment. The files here use four currencies against the
   dollar: the Australian and New Zealand dollars, which usually pay more, and the British pound and
   the euro, which usually pay less.
2. Work out each market's carry score. Where interest rates are available, the score is simply the
   foreign rate minus the domestic rate, in percent per year. Where they are not - and the exported
   price data used here has no rates - the score has to be built from prices, which is the central
   engineering problem of this category.
3. Rank the markets by the carry score, highest first.
4. Buy the two highest and sell the two lowest. Give each leg the same amount of money, so that the
   bought side and the sold side are equal in size and the overall exposure to the dollar is close to
   zero.
5. Recompute the scores and repeat every twenty-one trading days. Only trade the markets whose
   position actually needs to change.
6. Cap the total on each side, so that no single currency can dominate the book. The files cap the
   whole long side at twenty-five percent of the pot.

The four files differ mainly in step 2 and in what they trade.

The FX carry file (`test_0003_0393_carry_trading_strategy.py`). With no interest-rate data, it builds
a proxy from three ingredients. First a fixed baseline for each pair, standing in for the usual
interest difference: 0.03 for the Australian dollar, 0.025 for the New Zealand dollar, 0.01 for the
pound and -0.002 for the euro. Then the pair's own price change over 126 days, which captures the
slow drift of a high-yielder. Then minus the recent volatility over 21 days, which penalizes a
currency that has been turbulent. The score is baseline plus trend minus volatility. The rule ranks
the four by that score and goes long the top two and short the bottom two. Over 2008 to 2025 it
rebalanced 217 times and made 200 trades, ending at 912,208.05 from 1,000,000 - a loss of 8.8
percent with 41 percent of trades winning. The article's own lesson is that the trend term hijacked
the signal, quietly turning a carry strategy into a momentum strategy with a carry label.

The commodity carry file (`test_0004_0394_commodity_carry_strategy.py`). Commodity carry usually
means the futures curve: the gap between a contract for later delivery and one for earlier delivery.
The file has no curve data either, so it approximates carry as the return over a short window minus
a scaled return over a long window, ranks six commodities on that number, and holds the top two
against the bottom two. Over 118 rebalances it ended at 1,306,885.25 from 1,000,000, up 30.7 percent
with a reward-to-risk ratio of 0.52. The article's point is that this is the same proxy carry, on a
different basket, with a very different outcome: carry is a risk premium, not a law of nature.

The gold-rate carry file (`test_0001_0031_gold_rate_carry.py`). This one turns carry into a pair. It
treats gold and a bond fund as linked, fits a rolling straight line from one to the other to get a
"fair" gold price, and trades the gap. Entry needs two extremes at once: the bond side stretched
beyond one standard deviation and gold off its fair anchor by at least half a standard deviation, in
opposite directions. It exits when the gap narrows to within 0.2 standard deviations, with a stop at
three times a measure of recent price range. Over 4,258 bars it made 117 trades and a reward-to-risk
ratio of 1.13, ending at 1,032,287.54 from 1,000,000, a gain of 3.2 percent. A low hit rate carried
by a larger average win is the signature of a mean-reversion rule.

## The maths, with every symbol named

The carry of a currency position, the payment for holding it:

```text
Carry = interest_rate_foreign - interest_rate_domestic
```

- `Carry` is the payment as a percentage per year: 0.05 means five percent a year.
- `interest_rate_foreign` is the rate on the currency you bought.
- `interest_rate_domestic` is the rate on the currency you sold to buy it.

Held for `d` days, the payment collected on an amount `A` is:

```text
Payment = A * Carry * d / 365
```

- `A` is the amount held.
- `d` is the number of days held.
- Dividing by 365 converts the annual rate to the fraction of a year that `d` days represent.

The proxy score used when rates are unavailable:

```text
Score = baseline + (price_now / price_then - 1) - volatility
```

- `baseline` is the fixed prior interest difference for that market.
- `(price_now / price_then - 1)` is the price change over the long window, such as 126 days.
- `volatility` is the standard deviation of recent price changes, such as over 21 days, used as a
  penalty.

This is not carry. It is a stand-in for carry built from prices, and it changes the strategy when it
disagrees with the real interest difference. That is the honest description of step 2 above.

The commodity-carry proxy is the difference of two returns:

```text
CurveScore = return(short window) - k * return(long window)
```

- `return(short window)` is the price change over a short period, such as a few days.
- `return(long window)` is the price change over a longer period.
- `k` is a scaling constant chosen so the two terms are of comparable size.

When the short-window return is high relative to the scaled long-window return, the market is treated
as paying positive carry, standing in for a curve that slopes the right way.

The gold-rate carry file's fair value and residual:

```text
beta = covariance(log(gold), log(bond)) / variance(log(bond))
fair_value = beta * log(bond)
residual = log(gold) - fair_value
```

- `beta` is the fitted slope between the two series over the window, such as 126 days.
- `covariance` measures how the two log prices move together, and `variance` measures how much the
  bond log price moves on its own; their ratio is the slope of the best-fit line.
- `fair_value` is the level of gold the line predicts from the bond price.
- `residual` is how far gold sits from that line, the gap the rule trades.

Finally, the cross-sectional weights and the balance condition:

```text
w_i = + max_leg / n_long  for each of the top-ranked markets
w_i = - max_leg / n_short for each of the bottom-ranked markets
sum of all signed w_i is close to zero
```

- `w_i` is the signed share of the pot in market `i`, positive for bought and negative for sold.
- `max_leg` is the cap on the whole long side, such as 0.25.
- `n_long` and `n_short` are the numbers of markets on each side, such as two each.
- The sum being close to zero means the bought and sold sides are nearly equal, so a move in the
  dollar itself is cancelled out.

## A worked example

Start with the simplest possible carry: 100,000 held in a currency paying 5 percent a year, financed
by borrowing a currency paying minus 1 percent a year, which some currencies have paid in recent
years. Held for 21 days:

```text
Carry = 0.05 - (-0.01) = 0.06, or six percent a year
Payment = 100,000 * 0.06 * 21 / 365 = 345.21
```

So the position collects about 345 over three weeks before any exchange-rate move, and a fall of
more than 0.35 percent in the bought currency would wipe it out.

Now the cross-sectional rule with four pairs, equally weighted, each leg twelve and a half percent of
the pot, and a cost of 0.0002 of the amount traded on each side. The "carry" column is the score used
to rank; the "return" column is what each pair actually did over the following three weeks.

| Rebalance | AUD   | NZD   | GBP   | EUR    | Long     | Short    | Next-21-day returns                        | Portfolio move | Turnover | Cost  | Pot after  |
| --------- | ----- | ----- | ----- | ------ | -------- | -------- | ------------------------------------------ | -------------- | -------- | ----- | ---------- |
| 1         | 0.050 | 0.045 | 0.020 | -0.010 | AUD, NZD | GBP, EUR | AUD +1.0%, NZD +0.6%, GBP -0.7%, EUR -0.4% | +0.338%        | 0.50     | 10.00 | 100,327.47 |
| 2         | 0.052 | 0.044 | 0.028 | -0.005 | AUD, NZD | GBP, EUR | AUD +0.5%, NZD +0.2%, GBP -0.3%, EUR -0.1% | +0.137%        | 0.00     | 0.00  | 100,465.42 |
| 3         | 0.058 | 0.050 | 0.040 | 0.010  | AUD, NZD | GBP, EUR | AUD -1.2%, NZD -0.8%, GBP +0.5%, EUR +0.3% | -0.350%        | 0.00     | 0.00  | 100,113.79 |
| 4         | 0.030 | 0.020 | 0.050 | 0.040  | GBP, EUR | AUD, NZD | AUD -0.5%, NZD -0.3%, GBP +0.4%, EUR +0.2% | +0.175%        | 1.00     | 20.02 | 100,268.93 |
| 5         | 0.028 | 0.018 | 0.052 | 0.042  | GBP, EUR | AUD, NZD | AUD -0.2%, NZD 0.0%, GBP +0.3%, EUR +0.1%  | +0.075%        | 0.00     | 0.00  | 100,344.13 |

The portfolio move each period is the average of the long legs' returns minus the average of the
short legs' returns, because each leg is equally weighted. The turnover column is one for a full
switch, when two positions are closed and two opened, and zero when nothing changes. Over the five
periods the pot rose from 100,000 to 100,344, a gain of 0.34 percent. Two things are worth noticing.
First, the third period lost money even though the rule never changed its mind, because a stretch of
bad luck for a carry book is normal. Second, the fourth period was the switch, and it came just as
the previous winners turned - a real hazard, since the ranking is built on what has already
happened.

## What the research actually found

The academic paper that gave the idea its modern form is Koijen, Moskowitz, Pedersen and Vrugt's
"Carry", which measures the payment for holding each of a large set of futures across currencies,
bonds, equities, commodities and credit, and finds that sorting markets by that payment produces a
return that is positive across asset classes over long samples. The premium is real in the data; the
paper also documents that it comes with large, sudden losses when the low-yield assets rally, which
is why the article calls carry a premium for bearing tail risk rather than a free lunch.

The files here face a data problem the paper does not: their exported price history contains no
interest rates and no futures curve, so they rebuild carry from prices and a fixed prior. That
substitution is why the results on the compendium's own data disagree so sharply - minus 8.8 percent
for the FX version, plus 30.7 percent for the commodity version, plus 3.2 percent for the gold-rate
pair, and a commodity relative-value version at a reward-to-risk ratio just under one. The article's
conclusion is blunt: the same proxy carry on a different basket gave a wildly different outcome, so
carry must be read as a risk premium that pays in some baskets and periods and not others.

One thing must be said plainly, because it applies to every file in this category and to every other
backtest in this compendium. Each backtest asserts its final value, its reward-to-risk ratio and its
worst fall against a stored baseline, and it must produce identical numbers in the engine's two
calculation modes. The reward-to-risk ratio is the average return divided by how much the pot swung,
and the worst fall is the largest drop from a peak to the following low. Passing those assertions
proves the engine computes exactly what the file says it computes, on the stored data. It does not
prove the strategy earns anything. The assertion is a test of the software, not a claim about the
market. A file can pass every assertion and still describe a plan that loses money - as the FX
version here does.

## How this project relates to it

The closest finished tutorial in this collection is
[Forex carry trade](../../quantconnect/forex-carry-trade/README.md), which works through a classic
borrow-low, hold-high currency position and its risk, and
[Risk premia in forex markets](../../quantconnect/risk-premia-in-forex-markets/README.md), which
covers the wider family of currency return premia that carry belongs to.

The repository's own brief on macro, rates and foreign exchange,
[strategies/books2/07_macro_rates_and_fx.md](../../../strategies/books2/07_macro_rates_and_fx.md),
does not study carry directly, but it is the nearest research here on what moves currencies. Its
central finding is that the tradable macro information is concentrated in scheduled announcement
windows and moves the whole interest-rate curve, with roughly thirty percent of central-bank
announcements accompanied by a level shift of the curve (`1905.01541v1`, p.25). A carry book is
exposed to exactly those announcements, because it is a bet on interest-rate differences. The brief
also reports an AI-built currency signal with an annualised reward-to-risk ratio above 0.7
(`2608.00761v1`, p.2), which is a different kind of currency factor but shows how the same evidence
is harvested elsewhere.

## Where it goes wrong

- Carry crashes. In a panic, everyone who is long the high-yield currency and short the low-yield one
  tries to exit at the same time, the low-yield currency soars, and the losses arrive far faster than
  the payments ever did. The 2008 unwind is the standard example.
- The proxy is not the real thing. Building carry from price trends and volatility replaces the
  interest-rate difference with a bet that happens to look similar in calm times, and the FX file's
  loss is the visible cost of that substitution.
- Costs on a fast cadence. Reviewing every twenty-one days means frequent switches, each paying the
  gap between buying and selling prices on both legs. On a book that earns a few percent a year, the
  costs can consume the whole edge.
- The basket is chosen after the fact. Which currencies are "high yielders" changes with the
  interest-rate cycle; a backtest that uses today's labels on twenty years of history has chosen its
  winners with hindsight.
- The two sides are not equally easy to hold. Shorting a currency may pay less than the quoted
  interest difference after financing and borrow costs, so the measured premium can be smaller than
  the raw rates suggest.
- Tail risk is not diversifiable within the strategy. The whole book loses together on the same day,
  which is the opposite of the diversification that makes a mixed portfolio safer.

## Try it yourself

You need a spreadsheet and a public source of short-term interest rates for four currencies, plus
their exchange rates against the dollar, for the last ten years. Central banks publish the rates.

1. Make one row per month, with a column for each currency's short-term interest rate and a column
   for its exchange rate against the dollar.
2. Add a column for each currency's carry: its rate minus the dollar rate.
3. Add a column naming the two highest-carry currencies and the two lowest in that row.
4. On the next row, compute the return of a simple book that is long the two highest and short the
   two lowest, in equal amounts, using the exchange-rate moves.
5. Add the monthly carry payments to that return: the average carry of the long side minus the
   average carry of the short side, divided by twelve.
6. Chart the cumulative result, and separately chart its worst fall from a peak.

What to notice: the carrying payment arrives smoothly and the exchange-rate moves arrive in bursts,
so the curve looks calm for long stretches and then loses a year of gains in a week. If your chart
shows a smooth rise with no such week, check whether you have excluded the crises - the whole point
of the strategy is that those weeks are part of it.

## Where this came from

- [The compendium's carry trading article](https://backtrader.readthedocs.io/en/latest/strategies-series/en/29-carry-trading.html),
  the rules, the inventory and the file-level results quoted above.
- Koijen, Moskowitz, Pedersen and Vrugt, [Carry](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2632697),
  the cross-asset study of the carry premium and its crashes.
- [strategies/books2/07_macro_rates_and_fx.md](../../../strategies/books2/07_macro_rates_and_fx.md),
  this repository's brief on rates and currencies and the announcement-window finding.

## Words used in this tutorial

- carry: the payment for holding one thing financed by another, such as the interest difference
  between two currencies.
- carry crash: the sudden loss when a carry trade unwinds everywhere at once.
- dollar-neutral: a book whose bought and sold sides are equal, so a move in the dollar alone cancels.
- futures curve: the prices of contracts for the same thing delivered at different future dates.
- long: owning something, so you profit if its price rises.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- tail risk: the chance of a rare, very large loss.
- volatility: how much a price moves around its average, measured as a percentage per year.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
