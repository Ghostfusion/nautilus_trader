# The lunar cycle: buying emerging markets before a new moon and selling them short before a full moon

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                     |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | A fund that tracks the shares of developing, "emerging" economies, bought and then sold short as the moon changes phase                                                                                                                                                                                                   |
| How often it trades       | Twice a lunar month, roughly every two weeks                                                                                                                                                                                                                                                                              |
| What you need             | A spreadsheet, a table of moon-phase dates, and daily prices                                                                                                                                                                                                                                                              |
| Where the rules come from | [QuantConnect strategy library, lunar cycle in equity market](https://www.quantconnect.com/tutorials/strategy-library/lunar-cycle-in-equity-market) and the [Quantpedia entry](https://quantpedia.com/strategies/lunar-cycle-in-equity-market/) it cites                                                                  |
| The underlying research   | Yuan, Zheng and Zhu, [Are Investors Moonstruck? Lunar Phases and Stock Returns](https://personal.lse.ac.uk/YUAN/papers/lunar.pdf)                                                                                                                                                                                         |
| How well it held up       | Weak: one published sample of forty-eight countries from 1973 to 2001, a small effect that explains a tiny share of the movement in returns, a trading rule whose measured edge is close to its own costs, and no mechanism, so the finding cannot be told apart from one of many calendar patterns that appear by chance |
| Also appears in           | nothing else in this collection                                                                                                                                                                                                                                                                                           |

## The idea in one paragraph

The moon goes from dark to full and back to dark again about every twenty-nine and a half days. This
rule claims that share prices are a little lower around the full moon and a little higher around the
new moon. To trade that, it buys a fund of emerging-market shares about seven days before each new
moon, holds it through the new moon, then sells the fund short about seven days before each full moon
and holds that short position through the full moon. It repeats, two position changes a month,
forever. The claim is that the moon's phase changes how investors feel, and that a change in mood
shows up in prices.

## Why anyone believed it

This is the one strategy in this collection whose reason is entirely about psychology, so it is worth
stating the argument fairly rather than dismissing it. The paper's own reasoning runs as follows.
Moon phases have no tangible effect on farming, factories, shipping or interest rates, so a link
between the moon and prices cannot be a link through the economy. Researchers in psychology and
biology have reported mood and behaviour differences around the full moon, and separate studies
suggest that mood shapes how willing people are to take risk. If a full moon darkens mood, and a dark
mood makes people less willing to hold risky shares, then prices could dip around the full moon and
recover around the new one. In that story the counterparty is a nervous individual investor who sells
slightly too cheaply near the full moon, and the buyer is whoever is willing to take the other side.

The paper is careful to say why this matters beyond the moon itself. Because the lunar cycle is
common to the whole planet and does not depend on where an investor lives, a genuine link would be
hard to square with the idea that prices already reflect all available information. That makes it a
sharp test, and also a claim that needs strong evidence, because the moment one calendar cycle is
allowed to explain prices, every other calendar pattern becomes a candidate too.

## An everyday comparison

Suppose you notice that the stock market rose on the days you happened to wear red socks. You could
invent a reason, and with enough days you will find one: red makes you feel bolder, bolder people
buy more. But the honest position is that the socks cannot reach the market. Before you change your
wardrobe, you would want to see the pattern in data you had never looked at, checking many colours
and many years, and you would want at least one story that connects the sock to the price. The moon
rule is the red socks of finance, and the test of it is not whether the pattern can be found once. It
is whether the pattern survives being looked for honestly, in advance, in places and times chosen
before the answer is known.

## The rules, step by step

1. Choose one liquid fund of emerging-market shares. The library page uses EEM, a fund that tracks a
   broad index of developing-country shares.
2. Obtain a table of the exact dates and times of the four moon phases. The paper uses the United
   States Naval Observatory; the library page reads a downloaded file that lists new moon, first
   quarter, full moon and last quarter.
3. Find the day of the next new moon and the day of the next full moon. In a lunar month the gap from
   new moon to full moon is about fifteen days.
4. Seven days before the new moon, buy the emerging-market fund with the whole amount. Hold it until
   seven days before the following full moon, which is about two weeks.
5. Seven days before the full moon, sell the fund and hold the position short instead. Hold the short
   position until seven days before the next new moon, about two weeks later.
6. Repeat. The position is therefore long for about the half of each lunar month around the new moon
   and short for the half around the full moon, with no days out of the market.
7. The library page also takes a simpler view of the timing that gives the same result: it treats the
   "last quarter" as the signal to buy and the "first quarter" as the signal to sell short, because
   those two phases fall about seven days before the new moon and the full moon respectively.

## The maths, with every symbol named

The whole rule is a series of half-month windows and one comparison.

A lunar month, in the technical sense, is the average time from one new moon to the next:

```text
L = 29.53 days
```

- `L` is the mean length of a lunar month, also called a synodic month: about twenty-nine and a half
  days, so there are about 12.4 lunar months in a year.

The return of one pass through the cycle is the return earned while long minus the return earned
while short:

```text
R_cycle = r_long - r_short
```

- `r_long` is the fund's return over the window that ends at the new moon: from seven days before the
  new moon to seven days before the full moon, written as a decimal.
- `r_short` is the fund's return over the window that ends at the full moon: from seven days before
  the full moon to seven days before the next new moon.
- Subtracting `r_short` is what a short position does: if the fund falls by two percent over that
  window, the short position gains two percent, so a fall helps.

The cost is charged on each of the two position changes in a lunar month:

```text
Cost = n * c
```

- `n` is the number of position changes, two per lunar month: closing the long and opening the short,
  then closing the short and opening the long again.
- `c` is the cost of one change, as a fraction of the amount traded: the gap between the buying and
  selling price plus commission. For EEM, a widely traded fund, a realistic figure is 0.0006, that is
  six basis points, where one basis point is one hundredth of one percent. Shorting also adds a
  borrowing fee, small for such a fund but not zero.

The net return of one cycle is then `R_cycle - Cost`, and a year contains about twelve of them.

## A worked example

Six lunar months of made-up but plausible numbers for the emerging-market fund, with the account
starting at 10,000.00. The fund's return is the change over each half-month window; for the window in
which the rule is short, a fall in the fund is a gain.

| Lunar month | Fund return, new-moon half (rule is long) | Fund return, full-moon half (rule is short) | Gross return  | Cost   | Net return | Account   |
| ----------- | ----------------------------------------- | ------------------------------------------- | ------------- | ------ | ---------- | --------- |
| 1           | +1.5 percent                              | -1.0 percent                                | +2.50 percent | 0.12pp | +2.38pp    | 10,238.00 |
| 2           | -0.8 percent                              | +1.2 percent                                | -2.00 percent | 0.12pp | -2.12pp    | 10,020.90 |
| 3           | +0.6 percent                              | +0.4 percent                                | +0.20 percent | 0.12pp | +0.08pp    | 10,028.90 |
| 4           | +2.0 percent                              | -1.5 percent                                | +3.50 percent | 0.12pp | +3.38pp    | 10,368.10 |
| 5           | -1.2 percent                              | -0.9 percent                                | -0.30 percent | 0.12pp | -0.42pp    | 10,324.50 |
| 6           | +0.3 percent                              | +1.1 percent                                | -0.80 percent | 0.12pp | -0.92pp    | 10,229.50 |

Here `pp` means percentage points of the whole account, and the gross return in each row is the
fund's return in the long window minus its return in the short window, as in the formula. In lunar
month 4, for example, the fund rose two percent while the rule was long and fell one and a half
percent while the rule was short, so the rule gained on both halves: 2.0 minus -1.5 equals 3.5
percent before costs.

Across the six months the account went from 10,000.00 to 10,229.50, a gain of 2.3 percent, or an
average of about 0.40 percent per lunar month after costs. That average is close to the figure the
paper reports for a related version of the rule, which is the point of the example: the arithmetic
works, and the size of the result is small. It is nowhere near large enough to survive a poor
estimate of costs, and six months is far too short to mean anything.

## What the research actually found

The paper measured the difference between returns around the new moon and around the full moon,
across forty-eight countries.

| Source                                       | What it measured                                                                       | Result                                                                                                                                                                                                                                                                                                                                                                                               |
| -------------------------------------------- | -------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Yuan, Zheng and Zhu (2006)                   | Country share indices for 48 developed and emerging markets, January 1973 to July 2001 | For an equal-weighted world portfolio, the return difference between the new-moon period and the full-moon period was 40.26 basis points per lunar month for a 15-day window and 27.48 basis points for a 7-day window, both significant at the 5 percent level. For a value-weighted portfolio the figures were 30.44 and 25.87 basis points. Spread over a year: about 3 to 5 percent before costs |
| The same paper                               | The same panel, split by market type                                                   | The effect was strongest in emerging markets, at 5.60 basis points a day for the 15-day window and 11.27 basis points a day for the 7-day window, against 2.43 and 2.45 basis points a day for the seven largest developed markets. That gap by market type is why the library page trades emerging markets rather than a broad index                                                                |
| The same paper                               | A direct trading test and its costs                                                    | A long-in-new-moon, short-in-full-moon strategy returned 40.26 basis points per lunar month, about 4.8 percent a year, significant at the 5 percent level. Using a fund spread of about 0.10 percent and twelve round trips a year, the authors estimate transaction costs near 1.2 percent a year, leaving net returns of about 1.9 to 3.6 percent                                                  |
| The same paper                               | Explanatory power and robustness                                                       | The statistical models explain between 0.1 and 0.5 percent of the variation in returns. The results survive controls for trading volume, volatility, macroeconomic announcements and other calendar effects, and a test of shifted 30-day cycles finds the pattern only when the cycle tracks the actual moon                                                                                        |
| Quantpedia, the entry the library page cites | Its own summary of the same idea                                                       | Reports the effect as concentrated in emerging markets and links the rules to the paper above; the entry itself is behind a subscription, so its performance table could not be read here                                                                                                                                                                                                            |

Read carefully, the paper is honest about its own weakness. It says the lunar cycle is attractive
precisely because it has no economic mechanism, that a genuine link would be hard to reconcile with
market efficiency, and that with so many patterns in prices some will be found by chance. Its
robustness checks reduce, but do not remove, the worry that the result is one such chance pattern: all
of them stay inside the single sample of forty-eight countries and twenty-eight years, and none of
them is a test on data collected after the paper was written.

## How this project relates to it

Nothing in this repository implements or measures the lunar rule, and that is worth saying plainly.
The closest things are the two pages that teach the reader to test a claim like this. The foundations
page [How to read a claim](../../foundations/10_how-to-read-a-claim.md) sets out eight questions for
exactly this kind of statement, including who is on the other side and what would have to happen for
the claim to be false. The brief
[Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md)
explains, with the arithmetic, how a pattern can appear in a long list of possibilities purely by
chance, which is the central question here.

## Where it goes wrong

- There is no mechanism, and that is the whole problem. A moon phase cannot reach a broker's account.
  For most strategies the reason to keep believing a measured pattern is that there is a plausible
  party on the losing side of the trade; here the losing party is only a mood, and the paper itself
  says the pattern would be surprising. A finding with no mechanism is not forbidden, but it carries
  the entire burden of proof on the statistics alone.
- The statistics are thin. The effect explains between 0.1 and 0.5 percent of the movement in
  returns, which means the great majority of what prices do is unrelated to the moon. Several of the
  headline numbers sit at the 5 percent significance line, and across dozens of countries and window
  lengths from one to fifteen days, some will appear significant even if nothing is happening. The
  authors' own random-cycle check helps, but it stays within the one sample.
- The trading rule is close to its own costs. Two position changes a lunar month is about twenty-four
  trades a year, and shorting adds a borrow fee. The paper's own net estimate, 1.9 to 3.6 percent a
  year, rests on an assumed spread rather than measured fills, and a slightly worse assumption erases
  it.
- The evidence and the tradable instrument are different things. The measured panel is country
  indices from 1973 to 2001. The fund the library page trades, EEM, did not exist until 2003, so no
  part of the published result was produced by trading the thing the page trades.
- It is a calendar pattern among many. Once the calendar is allowed to move prices, the number of
  candidate patterns is large: phases of the moon, days of the week, turn of the month, months of the
  year, holidays. Searching all of them for the strongest is how a real pattern becomes
  indistinguishable from an accident.

What would be needed to take the claim seriously at all: a test written down in advance and run on
data collected after 2001, in markets liquid enough to trade cheaply, with shorting costs included
and measured rather than assumed, showing that the effect survives in a pre-chosen window length and
market group; and, ideally, a mechanism, or at least a demonstration that the result is not the best
of many calendar searches. Until then the honest description is a curiosity from one historical
sample, not a rule to trade.

## Try it yourself

You need a spreadsheet, a public table of moon-phase dates, and daily prices for one broad
share index. Nothing else.

1. Make one row per moon phase over the last twenty years with columns: date, phase name (new moon or
   full moon), and the index price.
2. For each new moon and each full moon, compute the index return over the seven days before and the
   seven days after that date. That is four numbers per lunar month.
3. Average the returns into two groups, one around the new moon and one around the full moon, and
   take the difference.
4. Redo the whole exercise using a three-day window and then a fifteen-day window.
5. Redo it once more on a second index, from a different country.

What to notice: how much the difference moves when you change the window length or the index, and
how much of the total seems to come from a handful of dates. If the pattern were a property of the
moon rather than of the sample, it should not care which index you chose or whether the window is
three days or fifteen. That sensitivity is the most useful thing you can learn from the exercise, and
it applies to almost every calendar claim you will meet.

## Where this came from

- [QuantConnect strategy library: lunar cycle in equity market](https://www.quantconnect.com/tutorials/strategy-library/lunar-cycle-in-equity-market),
  the rules as implemented: long an emerging-market fund seven days before the new moon, short it
  seven days before the full moon, using moon-phase data from the United States Naval Observatory.
- [Quantpedia: lunar cycle in equity market](https://quantpedia.com/strategies/lunar-cycle-in-equity-market/),
  the entry the library page cites; the page itself is behind a subscription and could not be read.
- Yuan, Zheng and Zhu, [Are Investors Moonstruck? Lunar Phases and Stock Returns](https://personal.lse.ac.uk/YUAN/papers/lunar.pdf),
  published in the Journal of Empirical Finance in 2006, the source of every number quoted above.
- [How to read a claim](../../foundations/10_how-to-read-a-claim.md) and
  [Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's guides to testing a pattern like this one.

## Words used in this tutorial

- emerging market: a poorer or faster-growing country's economy, whose shares are often more
  volatile and more influenced by individual investors.
- new moon: the phase when the moon is dark, between the end of one cycle and the start of the next.
- full moon: the phase when the moon is fully lit, about two weeks after the new moon.
- short selling: borrowing something you do not own, selling it, and buying it back later, so that a
  price fall is a gain and a price rise is a loss.
- return: the change in value of an investment over a period, expressed as a percentage.
- significant: unlikely to have arisen by chance alone, in the usual statistical convention.
- basis point: one hundredth of one percent, so forty basis points is 0.40 percent.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
