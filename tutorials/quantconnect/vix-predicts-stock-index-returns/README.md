# The VIX: using the market's expected turbulence to choose between owning and shorting the market

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                   |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | A fund of large American shares, held long when expected turbulence is very high and short when it is very low                                                                                                                                                                                                                                          |
| How often it trades       | Checked every day; the position changes only when expected turbulence reaches an extreme                                                                                                                                                                                                                                                                |
| What you need             | A spreadsheet, a daily history of the VIX and of the index price                                                                                                                                                                                                                                                                                        |
| Where the rules come from | [QuantConnect strategy library, VIX predicts stock index returns](https://www.quantconnect.com/tutorials/strategy-library/vix-predicts-stock-index-returns) and the [Quantpedia entry](https://quantpedia.com/strategies/vix-predicts-stock-index-returns/) it cites                                                                                    |
| The underlying research   | Pierre Giot, [Relationships Between Implied Volatility Indexes and Stock Index Returns](https://www.pm-research.com/content/iijpormgmt/31/3/92)                                                                                                                                                                                                         |
| How well it held up       | Mixed: a very high level of expected turbulence has predicted higher later returns over a long American sample with a plausible reason, but the relationship is noisy, the calm-market short side fights the market's long-run rise, and this repository's own review finds the index forecasts volatility far more reliably than it predicts direction |
| Also appears in           | nothing else in this collection                                                                                                                                                                                                                                                                                                                         |

## The idea in one paragraph

There is a number, published every day, that measures how much the market expects prices to swing over
the next month. It is called the VIX, and it is worked out from the prices of insurance-like contracts
on the share index: when people are frightened, those contracts get expensive and the number rises.
This rule watches the VIX. When it is higher than all but the top tenth of its recent range, the rule
buys the index, on the theory that frightened markets are paid to be held. When it is lower than all
but the bottom tenth of its recent range, the rule sells the index short, on the theory that calm is
complacency. In between, it does nothing.

## Why anyone believed it

This rule has a more respectable reason than most timing rules, and it is worth following. Holding a
share is risky: the price can fall as well as rise. People are willing to hold risk only if they
expect to be paid for it, and the payment they demand tends to be larger when times feel dangerous.
The VIX is a direct read-out of how dangerous the market believes the near future to be, because it
is built from the prices people actually pay for protection. When the VIX is high, investors have
already been scared and have already pushed prices down, which means the shares on offer pay more for
the risk of holding them. Anyone who buys at that point is being paid to take the other side of
someone else's fear.

The counterparty, in this story, is the forced or frightened seller: a fund facing withdrawals, a
trader who must reduce risk before a meeting, an investor who cannot stomach the swings any longer.
They sell at a price below what the shares are worth to a calmer buyer. Because the VIX comes from
option contracts rather than from past prices, it looks forward rather than backward, which is why
the story is about the future rather than about what has already happened.

## An everyday comparison

Think of a seaside town's insurance broker. On a still week, storm cover is cheap and nobody bothers.
The week the forecast turns black, everyone wants cover, the price of it jumps, and the few people
willing to write the policies earn a great deal if the storm misses. The VIX is the price of storm
cover for the share market. A very high price for it tells you that a lot of people are worried, and
that whoever is willing to sell them that cover is being offered generous terms. The rule bets that
the seller of cover, meaning the buyer of shares, comes out ahead on average.

## The rules, step by step

1. Obtain a daily history of the VIX, which is available from the exchange that publishes it and from
   the data feed the library page uses.
2. Keep the last two years of daily VIX closes, which is about 504 values.
3. Each day, sort that two-year history from smallest to largest and find twenty equally spaced levels
   through it: the 5th percentile, the 10th, up to the 95th. The 10th percentile is the level below
   which about one tenth of the past two years' values fall, and the 90th percentile is the level
   below which nine tenths fall.
4. Compare today's VIX close with those levels.
5. If today's VIX is above the 90th percentile, hold the index fund long with the whole amount.
6. If today's VIX is below the 10th percentile, hold the index fund short with the whole amount.
7. If today's VIX is between the two, do nothing and keep whatever position is already in place.
8. Repeat every trading day. The trailing two years move with each new day, so the percentile levels
   drift over time.

## The maths, with every symbol named

The VIX itself is a percentage, and the rule is a comparison of that percentage with its own recent
history.

The VIX is the market's expected size of price swings over the next thirty days, stated as a yearly
figure:

```text
VIX = annualised expected volatility of the index over the next 30 days, in percent
```

- `VIX` is published as a number such as 15, meaning an expected annualised swing of about 15
  percent. It is not a forecast of direction: a high VIX says the market expects to move a lot, not
  that it expects to move down.
- `volatility` is how much a price moves around its average, expressed as a percentage per year.
- `annualised` means expressed as if the same rate held for a whole year, so a one-month expected
  swing is scaled up to a yearly figure.

The comparison uses percentiles of the trailing window:

```text
Signal = long  if today's VIX > P90
Signal = short if today's VIX < P10
Signal = hold  otherwise
```

- `P90` is the 90th percentile of the VIX closes over the trailing two years: nine tenths of those
  closes are below it. It is often written as the top two of twenty equally spaced boxes.
- `P10` is the 10th percentile: one tenth of the trailing closes are below it.
- `long` means owning the index fund, so a rise is a gain.
- `short` means a borrowed position that gains when the index falls.

The return the rule earns is then the index's return when it is long and minus that return when it is
short:

```text
R = r_index        when the signal is long
R = -r_index       when the signal is short
```

- `r_index` is the index fund's return for the following day, as a decimal.
- The minus sign turns a fall into a gain for the short position.

## A worked example

Eight made-up days. To keep the arithmetic visible, the trailing two-year 10th percentile is taken as
12.0 and the 90th percentile as 28.0, though in practice both levels move every day. The signal is
read from today's VIX; the return is the index's move on the following day. Costs of 0.05 percent are
charged each time the position changes.

| Day | VIX close | Reading    | Position after reading | Index return next day | Position return | Running account |
| --- | --------- | ---------- | ---------------------- | --------------------- | --------------- | --------------- |
| 1   | 30.5      | above 28.0 | long                   | +0.9 percent          | +0.9 percent    | 10,045.00       |
| 2   | 31.0      | above 28.0 | long                   | +0.4 percent          | +0.4 percent    | 10,085.20       |
| 3   | 29.5      | above 28.0 | long                   | -0.6 percent          | -0.6 percent    | 10,024.70       |
| 4   | 27.0      | between    | long (held)            | +0.2 percent          | +0.2 percent    | 10,044.70       |
| 5   | 11.5      | below 12.0 | short                  | -1.1 percent          | +1.1 percent    | 10,155.30       |
| 6   | 11.0      | below 12.0 | short                  | -0.3 percent          | +0.3 percent    | 10,185.80       |
| 7   | 12.5      | between    | short (held)           | +0.8 percent          | -0.8 percent    | 10,104.30       |
| 8   | 13.0      | between    | short (held)           | +0.1 percent          | -0.1 percent    | 10,094.20       |

The account starts at 10,000.00 and, on day 1, the position changes from nothing to long, so a cost of
0.05 percent is taken: the +0.9 percent day is recorded as about +0.85 percent after that cost, giving
10,045.00. On day 5 the account changes from long to short, another 0.05 percent. The rest of the days
carry no cost because the position does not change. After eight days the account is about 10,094.20,
a net gain of about 0.9 percent.

Two things are visible in the table. First, most days produce no trade at all, because most VIX
readings sit between the two extremes; the rule is a patient overnighter, not a busy trader. Second,
the position is short on days 5 to 8, which happens to work because the index falls and stalls, but a
short position held while the market grinds upward loses money, and that is the leg of the rule that
is hardest to defend.

## What the research actually found

The paper the strategy is built on is Giot's 2005 study, which asks whether the implied volatility
indices that were then published lead stock index returns. Its full text was not available to read
for this tutorial, so the numbers below come from a later study of the same question, which the
Quantpedia blog summarises.

| Source                                       | What it measured                                                                  | Result                                                                                                                                                                                                                                                                                                                                                  |
| -------------------------------------------- | --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Bansal and Stivers (2023)                    | The link between a high VIX and later excess returns, American data, 1990 to 2022 | A simple rule of "much higher excess returns after the VIX exceeds a high threshold" fits the data well, with the best threshold near the 80th percentile of the VIX's own recent range. The relationship holds for returns over the next 1, 3, 6 and 12 months, and in subperiods, and the authors report that it beats a straight-line use of the VIX |
| Bansal and Stivers (2023)                    | How much of the variation is explained                                            | Predictive R-squared of about 20 percent for six-month returns and about 30 percent for twelve-month returns, when the VIX threshold is combined with a market-sentiment measure. That is far more explanatory power than the lunar or calendar rules report                                                                                            |
| Bansal and Stivers (2023)                    | The interpretation                                                                | The authors argue the VIX mainly measures the level of risk while sentiment measures the price of risk, so both help: fear raises the reward for holding shares, and optimism lowers it                                                                                                                                                                 |
| Quantpedia, the entry the library page cites | Its own summary of the idea                                                       | Links the trade to the same body of research and to the VIX index; the entry itself is behind a subscription, so its performance table could not be read here                                                                                                                                                                                           |
| The library page itself                      | The specific rule it implements                                                   | Uses the trailing two-year percentiles of the VIX and a simple long-above, short-below rule, which is a more mechanical version of the threshold idea than the research version, and trades the fund OEF rather than the index itself                                                                                                                   |

The research and the library page agree on the general shape of the claim and differ on the details:
the research uses a fixed high threshold and looks months ahead, while the page uses two moving
thresholds and looks a day ahead and, importantly, also goes short when the VIX is very low. The
evidence for the high-VIX, go-long half is the stronger half; the evidence for the low-VIX, go-short
half is much thinner. The library page makes no performance claim of its own.

## How this project relates to it

This repository studies the machinery around the VIX in two places. The brief
[Options and derivative instruments](../../../strategies/books2/12_options_and_derivatives.md)
explains what an implied volatility number really is, treating it as a quantile of a probability
distribution recovered from option prices, and discusses why VIX-related quantities are useful risk
inputs. The brief
[Volatility and microstructure noise](../../../strategies/books/06_volatility_and_microstructure_noise.md)
reviews a study `2407.16780v1` that adds the VIX to volatility forecasts: it improves forecast error
against a simple model by about 35 to 46 percent, but against a stronger machine-learning baseline the
improvement is not statistically distinguishable from zero, and the same study finds directional
return prediction essentially absent. The lesson for this tutorial is direct: the VIX is a good
measure of turbulence and a much weaker guide to which way the market goes next, so a rule that uses
it to pick a direction is leaning on the weaker of its two uses.

## Where it goes wrong

- The signal overlaps the thing it predicts. The VIX rises when the market has just fallen, so "buy
  when the VIX is high" is partly "buy after a fall". That is not fatal, but it means the rule is not
  an independent forecast; it is a way of buying weakness, which exposes the holder to further falls.
- The calm-market short is the weak leg. Shorting an index when the VIX is quiet fights the market's
  long-run upward drift, and quiet periods can last a long time. The stronger published result is only
  about buying after high turbulence, not about shorting after calm.
- Few independent episodes. Over a few decades the extreme VIX readings cluster in a handful of
  crises, so a rule that trades only at extremes has far fewer independent bets than its daily
  checking suggests, and one or two episodes can dominate the whole result.
- Free parameters. The window length (two years here), the percentile cut-offs (90 and 10 here, 80 in
  the research), the index and the fund are all choices that could be tuned after seeing the answer.
  A rule that looks good only at one combination is a rule that was fitted, not found.
- Costs and borrowing. The rule can flip often around a threshold, paying the fund's spread each time,
  and the short side pays a borrow fee. These are small per trade but accumulate in a strategy whose
  edge is a handful of percent a year.
- The VIX is not a price you can trade directly. The rule trades the index fund and reads the VIX as a
  signal; the two are different instruments, and the fund's own tracking and costs sit between the
  signal and the result.

## Try it yourself

You need a spreadsheet, a public daily history of the VIX, and a matching daily history of an index
fund such as SPY.

1. Put the VIX closes and the fund prices in two columns, one row per day.
2. Add a column that computes, for each day, the 10th and 90th percentile of the previous 504 VIX
   closes. Most spreadsheets will do this with one function per percentile once you point it at the
   right range.
3. Add a signal column: write "long" when the VIX is above the 90th percentile, "short" when below
   the 10th, and "hold" otherwise. Carry the previous signal forward on "hold" days.
4. Add a column for the fund's next-day return, and a rules column that is that return when long and
   minus it when short.
5. Build a running account for the rule and a second one for simply holding the fund, both starting
   at the same amount.
6. Finally, count the days the rule is long, short and holding.

What to notice: how much of the time the rule sits doing nothing, and whether the few long episodes
land in the crises you already know about. If the result depends mostly on one or two of those
episodes, that is not a repeating edge, it is a memory of two bad years.

## Where this came from

- [QuantConnect strategy library: VIX predicts stock index returns](https://www.quantconnect.com/tutorials/strategy-library/vix-predicts-stock-index-returns),
  the rules as implemented: trailing two-year VIX percentiles, long above the top and short below the
  bottom, trading the fund OEF.
- [Quantpedia: VIX predicts stock index returns](https://quantpedia.com/strategies/vix-predicts-stock-index-returns/),
  the entry the library page cites; the page itself is behind a subscription and could not be read.
- Giot, [Relationships Between Implied Volatility Indexes and Stock Index Returns](https://www.pm-research.com/content/iijpormgmt/31/3/92),
  Journal of Portfolio Management, 2005, the paper the strategy is built on; its full text was not
  available here.
- Bansal and Stivers, [Time-varying Equity Premia with a High-VIX Threshold and Sentiment](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=4477652),
  the source of the threshold and explanatory-power figures quoted above.
- [Options and derivative instruments](../../../strategies/books2/12_options_and_derivatives.md) and
  [Volatility and microstructure noise](../../../strategies/books/06_volatility_and_microstructure_noise.md),
  this repository's briefs on implied volatility and on how well the VIX forecasts, including the
  result `2407.16780v1` (https://arxiv.org/abs/2407.16780).

## Words used in this tutorial

- VIX: an index, published from option prices, that measures how much the share market expects to
  swing over the next month, stated as a yearly percentage.
- volatility: how much a price moves around its average, measured as a percentage per year.
- implied: taken from the prices of contracts rather than measured directly, as the market's expected
  volatility is taken from option prices.
- percentile: the level below which a given share of a set of values falls; the 90th percentile has
  nine tenths of the values below it.
- long: owning an investment, so that a price rise is a gain.
- short selling: borrowing something you do not own, selling it, and buying it back later, so that a
  price fall is a gain.
- position: one holding in a trading account, either long or short.
- risk premium: the extra return investors demand for holding something risky.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
