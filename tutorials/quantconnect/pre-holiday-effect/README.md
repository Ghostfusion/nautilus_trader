# The pre-holiday effect: holding shares only on the days before a holiday

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                   |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | One fund that tracks the whole large-company American share market, bought and sold as a single share                                                                                                                                                   |
| How often it trades       | A few days before each public holiday, then back to cash, so roughly twenty trades a year                                                                                                                                                               |
| What you need             | A spreadsheet, a list of market holidays, and daily prices for one index fund                                                                                                                                                                           |
| Where the rules come from | [QuantConnect strategy library, pre-holiday effect](https://www.quantconnect.com/tutorials/strategy-library/pre-holiday-effect) and the [Quantpedia entry](https://quantpedia.com/strategies/pre-holiday-effect) it cites                               |
| The underlying research   | Peter Reinhard Hansen and Asger Lunde, [Testing the significance of calendar effects](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=388601)                                                                                                       |
| How well it held up       | Mixed: reported across many markets and a century of data, but concentrated in a handful of days a year, shrinking since the late 1980s except in small companies, and surrounded by enough other calendar rules that data mining is a live explanation |
| Also appears in           | Nothing else in this collection describes a calendar rule; the closest is its critique of data mining in the overfitting brief                                                                                                                          |

## The idea in one paragraph

Holidays are fixed in advance. The days just before a market holiday are quiet: many investors are
away and some traders close their positions before the break. The claim is that on the last trading
day before the market shuts, shares tend to rise a little more often and a little more than usual.
The strategy is therefore to own a fund that tracks the whole American share market only on the two
trading days before a public holiday, and to hold cash on every other day of the year. It takes no
view about any single company. It is a bet on the calendar itself.

## Why anyone believed it

The economic story is about who steps aside. Before a long weekend or a holiday, some investors go
quiet: they are away, or they reduce exposure so that nothing unpleasant happens while they cannot
react. Short sellers, who profit when prices fall, have a particular reason to close their positions
before a market closure, because they cannot manage the risk while the market is shut. When the
sellers step back and buyers remain, the small remaining demand lifts the price.

The counterparty, then, is the investor who leaves the market early for a holiday, and the short
seller who buys back what was borrowed before the break. Neither is acting on news about the
companies; both are acting on the calendar, and if they keep doing so, the price keeps drifting up
into the holiday.

## An everyday comparison

Think of a town where the bakery closes for a long weekend. On the last morning before the closure,
everyone who wants fresh bread for the holiday must buy it that morning, so the queue is longer than
on a normal day. The bread is no better. The queue is a reaction to the closing, not to anything about
the bread, and it forms every time the bakery shuts for a known holiday. The strategy here is to be
holding bread on the morning before the closure.

## The rules, step by step

1. Use one fund that tracks the S&P 500, the index of the largest American companies. The library uses
   the SPDR S&P 500 ETF, which trades under the ticker SPY and which you can buy and sell like a
   single share. An ETF is a fund that holds a basket of things and is itself listed on an exchange.
2. Write down the market holidays for the year. The usual American list is New Year's Day, Martin
   Luther King Jr. Day, President's Day, Good Friday, Memorial Day, Independence Day, Labor Day,
   Election Day, Thanksgiving Day and Christmas Day.
3. Remove any holiday that falls on a weekend, because the market would be closed anyway and there is
   no shortened session to trade into.
4. Each day, look at the next two calendar days. If a holiday appears there, buy the fund with the
   whole account and hold it.
5. If no holiday appears in the next two calendar days, sell the fund and hold cash.
6. Review the position once each day and follow the rule again the next morning.
7. Do not add conditions. In particular, do not check whether the market has been rising or whether
   the upcoming holiday is a large one; the rule as written pays attention to the calendar only.

Two variations are worth knowing. The library holds for the two days before a holiday, while the
underlying research measures the single last trading day before it. And the research excludes the
trading day after the holiday from the claim; the effect is about the approach, not the retreat.

## The maths, with every symbol named

The whole strategy is a chain of daily returns, most of them the return on cash.

The return on one held day:

```text
r_day = P_after / P_before - 1
```

- `r_day` is the return over that day, as a decimal: 0.0042 means 0.42 percent.
- `P_before` is the fund price at the start of the day and `P_after` at the end.

The return on a cash day, at a yearly interest rate `i` spread evenly across the year:

```text
r_cash = i / 252
```

- `r_cash` is the return earned on a day spent in cash, as a decimal.
- `i` is the yearly interest rate on cash, as a decimal: 0.04 means 4 percent a year.
- 252 is the approximate number of trading days in a year, so dividing spreads the yearly rate across
  the days the market is open.

The result for the year is the product of all the daily growth factors, one for each trading day:

```text
R_year = (1 + r_day,1) * (1 + r_day,2) * ... * (1 + r_cash) * ... - 1
```

- `R_year` is the strategy's return for the year, as a decimal.
- Each `(1 + r_day)` is a growth factor: a day that gains 0.42 percent multiplies the account by
  1.0042, and a day that loses 0.10 percent multiplies it by 0.9990.
- On the held days the factor is the fund's, and on all other days it is the cash factor, which is the
  same small number repeated.

Finally the trading cost. Each holiday costs one round trip of the whole account, so two sides:

```text
Cost = H * 2 * c
```

- `Cost` is the total cost for the year, as a decimal.
- `H` is the number of holidays in the year, here 8 in the example below and 10 in a normal year.
- 2 counts the two sides, one to buy the fund and one to sell it.
- `c` is the cost per side as a fraction of the amount traded. For a very large, heavily traded index
  fund, one basis point, that is 0.0001, is a fair figure including the gap between the buying and
  selling price and any commission. One basis point is one hundredth of one percent.

## A worked example

Eight holidays in one invented year, with invented but plausible daily returns for the fund on the
last trading day before each holiday. The cash rate is 4 percent a year.

| Holiday                    | Return on the day held | Growth factor |
| -------------------------- | ---------------------- | ------------- |
| Martin Luther King Jr. Day | +0.42%                 | 1.0042        |
| President's Day            | +0.18%                 | 1.0018        |
| Good Friday                | -0.10%                 | 0.9990        |
| Memorial Day               | +0.55%                 | 1.0055        |
| Independence Day           | +0.31%                 | 1.0031        |
| Labor Day                  | +0.12%                 | 1.0012        |
| Thanksgiving Day           | +0.47%                 | 1.0047        |
| Christmas Day              | +0.26%                 | 1.0026        |

Multiplying the eight growth factors together gives 1.0223, so the held days add 2.23 percent. The
cash days supply the rest. A year has about 250 trading days; the eight held days above plus the two
holidays not shown account for the days the market is open but the strategy is invested, and roughly
242 days are spent in cash. Each cash day earns 0.04 divided by 252, about 0.0159 percent, and 242 of
them compound to about 3.92 percent. Combining the two parts:

```text
Gross = 1.0223 * 1.0392 - 1 = 0.0623, that is about 6.23 percent
Cost  = 8 * 2 * 0.0001 = 0.0016, that is 0.16 percent
Net   = 0.0623 - 0.0016 = 0.0607, that is about 6.07 percent
```

The gross figure of about 6.2 percent sits close to the published figure of 6.39 percent below, which
is a coincidence of the invented numbers and not evidence of anything. The arithmetic shows two
honest features. First, more than half of the return comes from the cash rate, not from the holiday
effect at all. Second, the eight held days produce the whole edge, so a single unexpected fall on one
of those days can undo several good holidays.

## What the research actually found

| Source                                                                                                                                   | What it measured                                                                 | Result                                                                                                                                                                         |
| ---------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| [Quantpedia, pre-holiday effect](https://quantpedia.com/strategies/pre-holiday-effect)                                                   | The last trading day before each holiday, United States, 1896 to 2002            | Pre-holiday daily return of 0.239 percent, about ten times the ordinary day, worth roughly 6.39 percent a year including the return on cash; worst fall 13.62 percent          |
| [Hansen and Lunde, Testing the significance of calendar effects](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=388601)             | Ten national share indices, testing each effect while controlling for the others | Calendar effects significant in most markets; end-of-year effects were the largest; from the late 1980s the effects diminished except in small-company indices                 |
| [Cao, Premachandra, Bhabra and Tang](http://www.eurojournals.com/irjfe_32_13.pdf)                                                        | New Zealand, four decades                                                        | The effect persists and even grew, but is confined to the smallest companies, with nothing detectable in medium and large ones                                                 |
| [Tsiakas, The economic gains of trading stocks around holidays](http://onlinelibrary.wiley.com/doi/10.1111/j.1475-6803.2009.01260.x/pdf) | The thirty Dow Jones companies                                                   | A risk-averse investor would pay a high fee to move from a holiday-blind rule to a holiday-aware one, and the result survived reasonable transaction costs                     |
| Li, Xu, Xu, Ma, Zhong and Wang                                                                                                           | 37 Chinese "time-honored" companies around the Spring Festival, 2012 to 2019     | The pre-holiday window was significant in most years but negative in 2012 and 2014, and the positive effect was clearer after the holiday than before it `2308.00702v1` (p.12) |
| The same study, after the pandemic                                                                                                       | The same companies, 2020 and 2021                                                | The holiday effect is not significant for these two years `2308.00702v1` (p.20)                                                                                                |

Read together, the record supports a small effect that has been seen in many places over a long time,
and undercuts the simplest version of it. The Chinese study, on a different holiday and a different
market, finds the stronger effect on the return after the holiday rather than before, and finds the
effect vanish in two recent years. That is reported here as a genuine disagreement, not resolved. The
honest summary is that the pre-holiday move is real enough to appear repeatedly, small enough to be
swamped by one bad day, and concentrated in the least liquid corners of the market, where it is
hardest to capture.

## How this project relates to it

This repository implements no calendar rule, and it is worth saying so plainly. A rule that buys
before ten known dates each year is a member of a very large family of calendar rules, and the
project's research on that family is the closest thing that exists here.
[Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md)
reports that mining 240 accounting variables produced 18,113 strategies, 30.17 percent of which
cleared a t-statistic of 2.0 and 8.40 percent of which cleared 4.0, thousands of times the rate a
world with no real effect would produce, and that the recommended hurdle of 3.0 still leaves only 81
percent of accepted findings true, citing `2209.13623v3`. A holiday rule is exactly the kind of
hypothesis that such a search finds.

The second related brief is
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
which reports that the cross-sectional factor zoo is mostly genuine but that the classic time-series
predictability result fails a broad out-of-sample panel, citing `2209.00121v1`. A market-timing rule
based on the calendar is a time-series claim, and it belongs to the half of that finding the brief
treats with most caution.

## Where it goes wrong

- Data mining. Anyone with a calendar can test many rules: the day before a holiday, the day after,
  the first day of the month, Mondays, the turn of the year. Testing enough of them guarantees that
  some look good on a single sample, and the holiday rule is one of that crowd.
- Very few observations. Ten days a year for a century is a thousand days, against a backdrop of
  thousands of ordinary days, so the estimate rests on a thin slice and a single bad holiday matters.
- The effect shrank. Hansen and Lunde find it fading after the late 1980s except in small companies,
  and the Chinese study finds it absent in two recent years. Whatever crowded it out may still be
  there.
- Liquidity and capacity. The larger effects are in small, thinly traded companies, where the fund
  used here, which tracks only the largest companies, does not reach them.
- The definition of a holiday. Weekends, half-days, one-off closures and changes to the exchange
  calendar all change which days count, and the rule is sensitive to the choice.
- Cash is doing the work. In the worked example more than half the yearly return is interest on cash,
  so a backtest that credits a high cash rate to a quiet stock market looks better than the holiday
  effect alone would justify.

## Try it yourself

You need a spreadsheet, a list of the market holidays for the last twenty years, and daily closing
prices for one broad index such as the S&P 500. Any finance website will give you the prices.

1. Build a sheet with one row per trading day: date, closing index level, and a column that marks
   whether the next two calendar days contain a holiday.
2. Add a daily return column: today's level divided by yesterday's, minus one.
3. Add a second column that is the same daily return, but only on the marked pre-holiday days; leave
   the other cells blank.
4. Average the whole first column. That is the return on an ordinary day.
5. Average the second column. That is the return on a pre-holiday day. Multiply each by 252 to compare
   them as yearly rates.
6. Repeat step 5 but split the years into two halves, before 2000 and after.

What to notice: the pre-holiday average is usually higher than the ordinary-day average, which is the
effect. But when you split the sample, the gap is often much smaller in recent decades, and a handful
of large positive pre-holiday days usually account for most of it. If removing your five best days
erases the effect, the rule is resting on luck rather than on a repeatable pattern.

## Where this came from

- [QuantConnect strategy library: pre-holiday effect](https://www.quantconnect.com/tutorials/strategy-library/pre-holiday-effect),
  the implemented rule: the SPDR S&P 500 ETF, the two-day look-ahead, the weekend filter, and cash
  otherwise.
- [Quantpedia: pre-holiday effect](https://quantpedia.com/strategies/pre-holiday-effect),
  the performance figures, the instrument count, the holiday list and the reference list.
- Peter Reinhard Hansen and Asger Lunde, [Testing the significance of calendar effects](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=388601),
  the study of calendar effects across ten markets and its finding that the effects have faded since
  the late 1980s.
- The Chinese Spring Festival study, `2308.00702v1`, an independent check that finds the positive
  effect after rather than before the holiday and finds it absent in recent years.
- [Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's brief on how many rules a search tests and what survives, citing `2209.13623v3`.
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's brief on what is and is not predictable out of sample, citing `2209.00121v1`.

## Words used in this tutorial

- basis point: one hundredth of one percent, so one basis point is 0.01 percent.
- calendar anomaly: a pattern in returns tied to dates rather than to companies.
- capacity: the largest amount of money a strategy can hold before its own trading moves prices.
- cash: money held uninvested, which here earns a small interest rate.
- data mining: searching the same data with many rules until one looks good by luck.
- ETF: an exchange-traded fund, a single listed thing that holds a basket of other assets.
- index: a published list of assets whose combined price is tracked as a single number.
- liquidity: how easily something can be bought or sold without moving its price much.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
