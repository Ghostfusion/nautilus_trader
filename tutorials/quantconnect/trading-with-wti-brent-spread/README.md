# The WTI-Brent spread: buying the gap between two grades of oil when it is unusually small

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                          |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | The price gap between two grades of crude oil, West Texas Intermediate and Brent, using contracts that track each price                                                                                                                                                                                                                                        |
| How often it trades       | A few times a month at most, whenever the gap crosses its own recent average                                                                                                                                                                                                                                                                                   |
| What you need             | A spreadsheet and daily prices for the two oils                                                                                                                                                                                                                                                                                                                |
| Where the rules come from | [QuantConnect strategy library, trading with WTI Brent spread](https://www.quantconnect.com/tutorials/strategy-library/trading-with-wti-brent-spread) and the [Quantpedia entry](https://quantpedia.com/strategies/trading-wti-brent-spread) it cites                                                                                                          |
| The underlying research   | Dunis, Laws and Evans, [Trading futures spreads: an application of correlation and threshold filters](https://ideas.repec.org/a/taf/apfiec/v16y2006i12p903-914.html)                                                                                                                                                                                           |
| How well it held up       | Mixed: a long backtest shows a positive reward for the risk and there is a clear economic reason for the gap to close, but the summary that catalogues the strategy rates its confidence only moderate, records that its own out-of-sample implementation was slightly negative, and notes that a related paper was publicly attacked for using the wrong data |
| Also appears in           | Nothing else in this collection describes it                                                                                                                                                                                                                                                                                                                   |

## The idea in one paragraph

West Texas Intermediate and Brent are two grades of crude oil, close enough that a barrel of one can
usually be traded for a barrel of the other. Their prices are still not identical: each is set in
its own market, so the gap between them drifts, usually by a few dollars a barrel. Most of the time
that gap wanders around a long-run average, because anyone who sees the gap unusually wide can buy
the cheap grade and sell the expensive one, which pushes the two prices back together. This
strategy watches the gap, and when the gap is far below its recent average it buys the cheap grade
and sells the expensive one, betting the gap widens back; when the gap is far above its average it
does the opposite. It closes the trade when the gap returns to a level that the recent relationship
between the two prices implies.

## Why anyone believed it

A barrel of oil is a barrel of oil. The two grades differ only in small chemical ways, and a
refinery that can run one can usually run the other with minor adjustments. That near-substitutability
is the whole argument: if the gap ever widens enough to pay for moving the oil and processing it,
someone will do exactly that, buying the cheaper grade and selling the dearer one. Their buying and
selling is what drags the two prices back together. The gap is not random noise; it has a centre.

The counterparty is the trader who cannot wait. A refinery that must have barrels this week will pay
whatever the market asks for one grade rather than switch its whole process, and a producer who
must sell into a weak market takes what it can get. Those people are the ones whose pressure moves
the gap away from its centre, and they are the ones the strategy trades against. There is also a
mechanical reason the gap moves: the two grades are produced and transported in different places and
shipped to different ports, so a pipeline outage or a shipping delay can pull the two apart for
weeks at a time, and only later does the oil flow back.

## An everyday comparison

Two bakeries sell the same loaf of bread in two neighbouring towns. Deliveries and local demand make
the price drift a few pence apart, but the gap cannot grow far, because a shopper who sees the loaf
cheap in one town can carry it home from the other. When the gap widens, someone notices and buys
the cheap loaves, which pushes their price up and closes the gap. The trade here is to notice when
the gap between the two bakeries' prices is unusually wide or unusually narrow, and to buy the cheap
loaf and sell the dear one, expecting the ordinary shopper to close the gap again.

## The rules, step by step

1. Get daily prices for the two oils. The QuantConnect version uses contracts-for-difference, which
   track a price without you owning the oil; the underlying research uses oil futures, which are
   standard contracts to deliver oil later. Either way you need one daily price for West Texas
   Intermediate and one for Brent.
2. Compute the spread each day: West Texas Intermediate price minus Brent price. A negative number
   means Brent is the dearer of the two.
3. Compute the twenty-day average of the spread: add the spread on each of the last twenty days and
   divide by twenty. This is the "recent average" the page keeps referring to.
4. Also compute a fair value from a regression, described in the maths section, which fits the two
   prices to each other over the last year. This fair value is the exit level.
5. If today's spread is above the twenty-day average, sell the spread: sell West Texas Intermediate
   and buy Brent, with the same money on each side.
6. If today's spread is below the twenty-day average, buy the spread: buy West Texas Intermediate
   and sell Brent, again with the same money on each side.
7. Hold the position. Close it on the first day the spread crosses the fair value in the direction
   that says the gap has finished closing. Then wait for the next crossing of the average.
8. Review the position every day. There is no fixed holding period; the trade lasts until the gap
   returns to fair value.

One thing to be precise about: the page describes the average as the thing the spread must cross to
start a trade, and the fair value as the thing it must cross to finish one. In the plain description
the average is used for both, and in the QuantConnect code the fair value comes from a regression.
This tutorial shows both, because the difference between them is the difference between a rule you
can run on a spreadsheet and the exact version in the code.

## The maths, with every symbol named

Three calculations, repeated every day.

The spread itself:

```text
Spread_t = P_WTI,t - P_Brent,t
```

- `Spread_t` is the gap in dollars per barrel on day `t`.
- `P_WTI,t` is the price of West Texas Intermediate on day `t`.
- `P_Brent,t` is the price of Brent on day `t`.
- A positive spread means West Texas Intermediate is dearer; a negative spread means Brent is.

The recent average:

```text
SMA20_t = ( Spread_t + Spread_(t-1) + ... + Spread_(t-19) ) / 20
```

- `SMA20_t` is the twenty-day simple moving average of the spread, that is the ordinary average of
  the last twenty daily values, recomputed each day.
- The subscript `t-1` means the day before, `t-19` means nineteen days before.

The fair value, from a line fitted to the two prices over the last year:

```text
P_Brent = beta * P_WTI + alpha
Fair value = (1 - beta) * P_WTI - alpha
```

- `beta` is the slope of the fitted line: how many dollars Brent moves for each dollar West Texas
  Intermediate moves. A value near 1 says the two move almost one for one.
- `alpha` is the height of the fitted line: the gap left over when the first term is removed. A
  negative `alpha` means Brent sits above West Texas Intermediate by that amount.
- The second line follows from the first by subtracting the fitted Brent price from the West Texas
  Intermediate price: spread equals `P_WTI - (beta * P_WTI + alpha)`, which is `(1 - beta) * P_WTI -
  alpha`.
- The fair value is the spread the fitted relationship implies today, so it moves as prices move.

And the cost of a round trip:

```text
Cost = 2 * n * c * NotionalPerLeg
```

- `n` is the number of legs, which is 2 here because the trade always has one leg in each oil.
- `c` is the cost per trade as a fraction of the amount traded. For a liquid crude contract this is
  small but not zero; the crude-oil paper cited below measures the average daily half-spread as
  5.80 basis points for Brent and 20.24 basis points for West Texas Intermediate (`2309.00875v3`,
  p.12), where one basis point is one hundredth of one percent.
- `NotionalPerLeg` is the dollar amount placed on each oil, which is half the account.

## A worked example

To keep the table small, this example uses a five-day average and a nine-day window; the rule itself
uses twenty days. The prices are invented but are the size of real crude prices. The fitted line is
invented too, with `beta = 1.02` and `alpha = -0.40`, so that the fair value is
`(1 - 1.02) * P_WTI + 0.40`.

| Day | WTI   | Brent | Spread | Average of last 5 days | Action                                         |
| --- | ----- | ----- | ------ | ---------------------- | ---------------------------------------------- |
| 1   | 70.00 | 72.00 | -2.00  | -                      | -                                              |
| 2   | 70.50 | 72.20 | -1.70  | -                      | -                                              |
| 3   | 71.00 | 72.30 | -1.30  | -                      | -                                              |
| 4   | 71.50 | 72.40 | -0.90  | -                      | -                                              |
| 5   | 72.00 | 72.50 | -0.50  | -1.28                  | -                                              |
| 6   | 73.00 | 72.60 | +0.40  | -0.80                  | Spread above average: sell the spread          |
| 7   | 72.80 | 72.70 | +0.10  | -0.44                  | Hold                                           |
| 8   | 72.20 | 72.90 | -0.70  | -0.32                  | Hold                                           |
| 9   | 71.60 | 73.00 | -1.40  | -0.42                  | Fair value is -1.03; spread is below it: close |

Read day 6 as the entry. The five-day average is `(-1.70 - 1.30 - 0.90 - 0.50 + 0.40) / 5 = -0.80`,
and the spread of `+0.40` is above it, so the rule sells the spread: short West Texas Intermediate,
long Brent. On day 9 the fair value is `(1 - 1.02) * 71.60 + 0.40 = -0.02 * 71.60 + 0.40 = -1.03`,
and the actual spread of `-1.40` has crossed below it, so the rule closes both legs.

The profit of the round trip, on an account of 10,000 with 5,000 on each leg:

| Leg                           | Entry | Exit  | Units   | Profit  |
| ----------------------------- | ----- | ----- | ------- | ------- |
| Short West Texas Intermediate | 73.00 | 71.60 | 68.4932 | +95.89  |
| Long Brent                    | 72.60 | 73.00 | 68.8705 | +27.55  |
| Total                         |       |       |         | +123.44 |

The units are `5,000 / entry price` in each case. The West Texas Intermediate leg gains
`68.4932 * (73.00 - 71.60) = 95.89`; the Brent leg gains `68.8705 * (73.00 - 72.60) = 27.55`. The
total of 123.44 on an account of 10,000 is a gain of 1.234 percent. Costs are four trades (two legs
opened, two legs closed) of 5,000 each at a cost of 0.0002, that is two basis points per trade:
`4 * 5,000 * 0.0002 = 4.00`, which is 0.04 percent of the account. The net gain is about 1.19
percent.

Two things are worth noticing. First, the whole move is worth about a dollar and eighty cents on a
price near seventy dollars, which is under three percent; an error of a few basis points in the cost
assumption is a real fraction of the profit. Second, the example says nothing about whether the
strategy works. It only shows how to apply the rules and how the arithmetic behaves.

## What the research actually found

| Source                                         | What it measured                                                            | Result                                                                                                                                                                                                                     |
| ---------------------------------------------- | --------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Quantpedia, summarising the source paper       | The WTI-Brent spread, daily, two instruments                                | 9.92 percent a year as a weighted average of the in-sample and out-of-sample periods, volatility 11.27 percent, worst fall 68.86 percent, reward-to-risk 0.88, over a 1995 to 2004 backtest                                |
| Quantpedia's own note on the same entry        | Its own out-of-sample implementation of the rule                            | Slightly negative, with the comment that the strategy's edge appears to be deteriorating in the out-of-sample period; it also warns that the parameters come from a short history and may be data-mined                    |
| Dunis, Laws and Evans                          | Twenty-day moving-average, regression, MACD and neural models on the spread | The best model was reported as profitable in and out of sample, with annualised out-of-sample returns of 26.35 percent after transaction costs; a related version of the same work reported an ARMA model at 34.94 percent |
| Lubnau, on spread trading in crude oil futures | Bollinger-band rules on a WTI-Brent hedge portfolio, 1992 to 2013           | Some settings were reported profitable in every five-year period tested, with reward-to-risk ratios above three for the best hedge-ratio method; the study used 22 years of data                                           |
| Donninger, published as a reply to that work   | The same crude-oil spread strategy                                          | Argues the apparent Sharpe ratios above three come from using the wrong data, and that the result collapses once the data are fixed                                                                                        |

Read together, the picture is this. The economic reason for the gap to close is strong and does not
depend on the data, and there is more than one separate study reporting a profit. But the reports
disagree in a way that matters: one public study says the best model earned 26 percent a year after
costs, another says the whole effect is a data error, and the catalogue that summarises the strategy
ranks its confidence only moderate and records its own out-of-sample attempt as slightly negative.
The honest reading is that the gap mean-reverts, that extracting the mean reversion in a way that
survives costs is hard, and that the winner of any particular backtest is often the setting that
happened to fit that sample best.

## How this project relates to it

Nothing in the repository runs this spread strategy. The closest work is the harvest of crude-oil
research in
[the predictability brief](../../../strategies/books2/08_predictability_and_trading_strategies.md).
Its Section 6 reads a study of statistical arbitrage in Brent, West Texas Intermediate and Shanghai
crude oil futures (`2309.00875v3`) that models the spread as a mean-reverting process whose
behaviour switches between regimes, and reports that the strategies it builds dominate simple fixed
bands. That document supplies two numbers this page uses: the per-contract transaction costs of
5.80 basis points for Brent and 20.24 basis points for West Texas Intermediate (p.12), and the
warning that a fixed rule is the version most likely to be a product of its sample.

The second related document is
[the energy and commodity brief](../../../strategies/books2/18_energy_and_commodities.md).
Its Section 6 records that commodity futures curves, not just crude oil, carry a tradeable signal
in the slope between near and far contracts, and that the nearest four contracts hold 82.2 percent
of open interest (`2308.00383v1`, p.8). It is a reminder that the WTI-Brent gap is one instance of
a much larger family of spread trades, and that liquidity is concentrated in the contracts closest
to expiry, which is where this strategy would trade.

## Where it goes wrong

- The gap is not perfectly stationary. West Texas Intermediate and Brent have spent long stretches
  with a wide or narrow gap for a real reason, such as the American shale boom changing what could
  be exported and where, so waiting for the gap to return to its old average can mean waiting for a
  relationship that has genuinely changed.
- The parameters are borrowed from a short sample. The underlying study covers 1995 to 2004, and the
  twenty-day window and the one-year regression window are chosen on that history. That is exactly
  the setting where data mining is easy, as the catalogue itself warns.
- Costs, financing and roll. Contracts-for-difference charge daily financing, futures must be rolled
  from one expiry to the next, and the two legs can have different financing costs. The published
  worst fall of 68.86 percent happened with costs included.
- Leverage. These products are usually traded with borrowed money. A leveraged short position has no
  ceiling on its loss, and the two legs do not cancel each other's losses: a jump in both prices
  moves the two legs by different amounts.
- A tiny absolute move. The gap is a few dollars against a price near seventy, so the signal is a
  small fraction of the account. A wrong assumption about the gap between buying and selling prices
  can turn a measured profit into a loss.
- Disagreement about the evidence. One study reports reward-to-risk ratios above three and a reply
  says the data are wrong, while the catalogue's own out-of-sample test was slightly negative. When
  credible sources reach opposite conclusions, the only safe posture is to treat the size of the
  prize as unknown.

## Try it yourself

You need nothing but a spreadsheet and a public source of daily crude oil prices; several finance
websites publish daily closes for both grades.

1. Build a sheet with one row per day for the last two years, and columns for West Texas Intermediate
   price, Brent price, and the spread (the first minus the second).
2. Add a column that computes the twenty-day average of the spread, the value in that row averaged
   with the nineteen rows above it.
3. Add a column that marks, each day, whether the spread is above or below that average.
4. Add a column for the fair value from a regression: use the spreadsheet's own linear-regression
   function to fit Brent on West Texas Intermediate over the previous year's rows, then compute the
   fair value `(1 - beta) * WTI - alpha`, where `beta` is the fitted slope and `alpha` the fitted
   height.
5. Starting from the first day on which the spread crosses its average, assume you sell the spread
   if it crossed upward and buy it if it crossed downward, and hold until the spread crosses the
   fair value.
6. Record the entry spread, the exit spread, and the gross profit as the change in the spread times
   the direction you took. Then subtract about two basis points of cost per trade on each leg.

What to notice: many days pass between trades, so the rule is not busy, but the trades cluster when
the gap is unusually wide, which is precisely when the market is most unsettled. Compare the
strategy's line with simply buying and holding Brent: on some stretches the strategy is flat while
Brent rises strongly, which is the cost of being in a spread rather than in the market.

## Where this came from

- QuantConnect strategy library, trading with WTI Brent spread,
  [the page](https://www.quantconnect.com/tutorials/strategy-library/trading-with-wti-brent-spread):
  the rules as implemented, the spread, its twenty-day average, the regression fair value, and half
  the account on each leg.
- Quantpedia, trading WTI/BRENT spread,
  [the entry](https://quantpedia.com/strategies/trading-wti-brent-spread):
  the performance figures, the instrument count, the confidence rating, the out-of-sample note and
  the list of source papers.
- Dunis, Laws and Evans, Trading futures spreads: an application of correlation and threshold
  filters,
  [the study](https://ideas.repec.org/a/taf/apfiec/v16y2006i12p903-914.html): the rules'
  origin, including its twenty-day model and its reported out-of-sample returns.
- [the predictability brief](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's harvest of the predictability literature, whose Section 6 covers
  regime-switching statistical arbitrage in crude oil (`2309.00875v3`) and supplies the per-contract
  cost figures.
- [the energy and commodity brief](../../../strategies/books2/18_energy_and_commodities.md),
  the repository's energy and commodity brief, whose Section 6 covers the tradeable slope of
  commodity curves (`2308.00383v1`).

## Words used in this tutorial

- basis point: one hundredth of one percent, so twenty basis points is 0.20 percent.
- contract-for-difference: a contract that pays the change in a price without anyone owning the
  underlying thing, usually financed daily.
- drawdown: the fall from a peak to the following low, measured in percent.
- futures: a standard contract to buy or sell something at a fixed price on a fixed future date.
- long: owning something, so that you gain when its price rises.
- mean reversion: the idea that a price which has moved far from its usual level tends to come back.
- regression: a statistical method that estimates how one quantity relates to another.
- short: selling something you do not own, so that you gain when its price falls.
- spread: the gap between two prices, here the difference between two grades of oil.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
