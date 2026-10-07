# Intraday arbitrage between index funds: trading the gap when two funds that hold the same thing disagree

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                              |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Two American funds that both track the S&P 500 index, bought and sold at the same moment in opposite directions                                                                                                                    |
| How often it trades       | Rarely. Only when the two prices separate by more than a small threshold, which may happen a few times a day and may not happen at all for months                                                                                  |
| What you need             | Python and a file of bid and ask quotes                                                                                                                                                                                            |
| Where the rules come from | [QuantConnect strategy library, intraday arbitrage between index ETFs](https://www.quantconnect.com/tutorials/strategy-library/intraday-arbitrage-between-index-etfs)                                                              |
| The underlying research   | Marshall, Nguyen and Visaltanachoti, [ETF arbitrage: Intraday evidence](https://doi.org/10.1016/j.jbankfin.2013.05.014), Journal of Banking and Finance, 2013, and Kakushadze and Serur, 151 Trading Strategies, Section 6.4       |
| How well it held up       | Weak: one published study documents the deviations and how fast they close, a practitioner catalogue supplies the threshold, and the library's own five-year implementation lost money with a lower reward for risk than the index |
| Also appears in           | nothing else in this collection                                                                                                                                                                                                    |

## The idea in one paragraph

Two funds can hold almost the same basket of shares and still not trade at the same price at every
instant. One has a slightly different mix, or slightly different costs, or simply meets a wave of
buyers while the other meets a wave of sellers. When the gap between them becomes wide enough to
cover the cost of trading, a trader can buy the cheaper fund, sell the dearer one short, and wait for
the two to come back together. That is the whole strategy. It puts the same amount of money on each
side, so a rising or falling market pushes one leg up and the other down and the two cancel, and the
profit comes from the gap closing rather than from any view about the market.

## Why anyone believed it

The two funds hold almost identical baskets, so a share of one is very close to a share of the other.
When a large investor wants a position in the index quickly, or a fund receives a wave of money to
invest, that flow tends to hit one fund more than the other, and the price that fund is quoted at
moves away from its twin for a moment. The counterparty is therefore the impatient buyer or seller
who needs to trade now and is willing to pay a slightly worse price to do it.

On the other side of the trade, the reason the gap closes is mechanical. Market makers and other fast
traders watch the pair, and as soon as the gap is wide enough to pay for the trade they step in,
which pushes the two prices back together. The research observes exactly this: the gap between the
buying and selling prices of both funds widens just before the opportunity appears, the order flow
becomes one-sided, and then the two prices return to parity quickly.

## An everyday comparison

Two supermarkets on the same street sell the same brand of milk. Most days both charge 1.00. On a
Sunday the small shop runs out and, rather than lose a customer, it charges 1.02; the large shop,
which has plenty, still charges 1.00, and its price for a customer selling milk back is a fraction
under 1.00. If you could buy at the large shop and, at the same instant, sell to the small shop, the
two cents would be yours once the small shop restocked and the prices met again. The catch is that the
two-cent gap has to be wide enough to cover the errand, and that the restocking may take an afternoon.

## The rules, step by step

1. Choose two funds tracking the same index. The library uses SPY and IVV, which both track the S&P
   500; the study it follows looks at two of the most heavily traded S&P 500 funds for the same
   reason. Their daily returns move together almost perfectly, with a correlation above 0.99.
2. For each fund keep two prices at every moment: the bid, which is what you receive when you sell,
   and the ask, which is what you pay when you buy.
3. Work out a ratio: the bid of one fund divided by the ask of the other. This uses the price you
   would actually get and the price you would actually pay, so the gap between buying and selling
   prices is already inside the number.
4. Work out the normal level of that ratio: its average over the last 400 quote updates. The ratio
   drifts slowly, because two funds tracking the same index do not pay identical dividends or hold
   exactly the same shares, so the average is subtracted to leave only the temporary part.
5. If the ratio of fund 2's bid to fund 1's ask, minus its average, is at least 0.0002, that is 0.02
   percent, then fund 2 has become expensive relative to fund 1: buy fund 1 at its ask and sell fund
   2 short at its bid. A short sale means borrowing the fund, selling it, and buying it back later,
   which gains when its price falls. Check the same test the other way round for the opposite trade.
6. Before acting, require the separation to have lasted. The study requires the condition to hold for
   15 seconds; the library requires it to hold for three consecutive quote updates.
7. Close the position when the reverse ratio, minus its own running average, comes back to zero or
   above: sell the fund you own at its bid and buy back the fund you sold short at its ask.
8. Put the same amount of money on each leg. The library trades the moment the signal appears, because
   the gap is measured in seconds.
9. Do nothing when no gap is wide enough. On most days that is the whole story.

## The maths, with every symbol named

The ratio the strategy watches, and the part of it that is temporary:

```text
r_t = B_2(t) / A_1(t)
g_t = r_t - m_t
```

- `r_t` is the ratio at moment `t`.
- `B_2(t)` is the price you receive for selling fund 2 right now, its bid.
- `A_1(t)` is the price you pay for buying fund 1 right now, its ask.
- `m_t` is the average of `r` over the last 400 quote updates, the normal level of the ratio.
- `g_t` is how far the ratio sits above or below normal, written as a decimal: 0.0002 means 0.02
  percent.

The entry rule, and the exit rule:

```text
enter long fund 1 and short fund 2 when g_t >= 0.0002
exit when (B_1(t) / A_2(t)) - m'_t >= 0
```

- `B_1(t)` is the price you receive for selling fund 1 at the exit, its bid.
- `A_2(t)` is the price you pay to buy back fund 2 at the exit, its ask.
- `m'_t` is the average of `B_1 / A_2` over the trailing window, its normal level.

The first line says: act only when fund 2 has become dear relative to fund 1 by more than two
hundredths of a percent. The second says: close when fund 1 has become dear relative to fund 2,
because that is the moment the two have swapped places and the gap is as closed as the rule asks.

What the round trip earns, per unit of money `M` placed on each leg:

```text
Profit / M = (B_1_exit / A_1_entry) - (A_2_exit / B_2_entry)
```

- `A_1_entry` and `B_2_entry` are the prices paid and received when the position is opened.
- `B_1_exit` and `A_2_exit` are the prices received and paid when it is closed.
- `M` is the money on each leg, so the total committed is `2 * M`.

Every price in that formula is a side the trader actually has to trade on, so the gap between buying
and selling prices is already included. The consequence is blunt: the threshold has to be larger than
whatever else the trade costs, or the position is at a loss the moment it is opened. One basis point,
written 0.0001, is one hundredth of one percent.

## A worked example

Two funds that normally trade within a hundredth of a percent of each other. Money on each leg is
100,000, so 200,000 is committed. Commission is assumed to be half a basis point of everything traded,
and the prices below are invented but of the size these funds produce.

| Step  | Fund 1 ask (paid) | Fund 2 bid (received) | Fund 1 bid (received) | Fund 2 ask (paid) |
| ----- | ----------------- | --------------------- | --------------------- | ----------------- |
| Open  | 400.00            | 400.12                |                       |                   |
| Close |                   |                       | 400.10                | 400.02            |

At the open the ratio of fund 2's bid to fund 1's ask is 400.12 divided by 400.00, which is 1.00030.
Its normal level is taken to be 1.00000, so the temporary part is 0.00030, which is at least 0.0002.
The rule acts: buy fund 1 at 400.00 and sell fund 2 short at 400.12. At the close the reverse ratio is
400.10 divided by 400.02, which is 1.00020, above its normal level of 1.00000, so the rule closes both
legs.

```text
Profit / M        = (400.10 / 400.00) - (400.02 / 400.12)
                  = 1.00025000 - 0.99975007
                  = 0.00049993
Profit            = 100,000 * 0.00049993 = 49.99
Traded            = 200,000 at the open + 200,000 at the close = 400,000
Commission        = 400,000 * 0.00005 = 20.00
Net               = 49.99 - 20.00 = 29.99, on 200,000 committed, which is 0.015 percent
```

Two things are worth noticing. The whole prize is about five hundredths of a percent per leg, which is
one fiftieth of a percent, so a single extra basis point of cost on each side takes most of it. And the
threshold matters enormously: the library uses 0.02 percent, while the practitioner catalogue that
describes the same rule uses 0.2 percent, ten times larger. At the catalogue's threshold there would
be far fewer trades and each would be ten times bigger; at the library's there are many small
opportunities and the cost of trading is proportionally larger.

## What the research actually found

| Source                                                                | What it measured                                                                          | Result                                                                                                                                                                                                                                                                                                   |
| --------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Marshall, Nguyen and Visaltanachoti, ETF arbitrage: Intraday evidence | Two extremely liquid S&P 500 funds, intraday prices                                       | The funds are treated by investors as close substitutes; the gap between buying and selling prices widens just before an arbitrage opportunity appears, order flow becomes one-sided, and the price deviations are then followed by a tendency to correct quickly back towards parity                    |
| Kakushadze and Serur, 151 Trading Strategies, Section 6.4             | The rule itself, written for practitioners                                                | Buy the cheap fund and sell the dear one when the bid of one reaches the ask of the other multiplied by about 1.002, close when the ratio reverses, and use fill-or-kill orders; it adds that these opportunities are ephemeral and need a very fast execution system or the cost of trading erases them |
| QuantConnect implementation, 11 August 2015 to 11 August 2020         | SPY and IVV, a 0.02 percent threshold, three updates of confirmation, against the S&P 500 | Reward-to-risk of -0.447 against 0.732 for the index; during the autumn 2015 turbulence 2.837 against -0.724; during the crash of early 2020 -4.196 against -1.243, and during the recovery -3.443 against 13.761                                                                                        |

Read together: the deviations are real and they do close, and the study that documents them measures
the conditions around them rather than a profit after costs. The library's version, with a threshold
ten times smaller than the catalogue's, lost money over five years and had a lower reward for risk
than simply holding the index, and it did worst during the two periods when prices were moving
fastest. That is consistent with the catalogue's own warning: at this size, speed and cost decide the
outcome, not the existence of the gap.

## How this project relates to it

This repository's brief on the subject is
[Cross-Venue Structure and Arbitrage](../../../strategies/books/10_cross_venue_and_arbitrage.md). Its
headline is the one that matters here: gross spreads are not profit, and once fees, latency and
settlement are priced many printed opportunities disappear. Three of its findings bear directly on
this rule.

The first is that a printed price can be wrong. On 11 August 2015 the consolidated feed that carries
American share prices reported 66.2 percent of Apple's 482,578 trades out of sequence (`1810.11091v1`),
and quotes appeared in that feed that were locked or crossed, meaning the buying price was at or above
the selling price, while the individual exchanges never showed that state. A gap between two funds
measured from such a feed can be an error in the data rather than an opportunity.

The second is that the window is tiny. The same brief reports that a cited arbitrage window may last
500 microseconds or less (`1810.11091v1`, p.14), which is half a thousandth of a second, while this
rule waits three quote updates before acting.

The third is a caution about the measurement itself. The brief reports a paper proving that under
cross-venue feedback the standard statistics for deciding which market leads become powerless, with
power equal to size (`2608.09188v1`), and another arguing that a positive expected gain is not the
same thing as an arbitrage (`1907.09218v2`). A screen that prints a gap is not proof of a profit.

Nothing in this repository trades this rule. The closest code is the execution and data machinery the
brief points at, in the analysis and data layers, whose job is to reject impossible quotes rather than
trade them.

## Where it goes wrong

- Speed decides the outcome. The opportunity exists for seconds or less. The library waits three quote
  updates before acting, and the catalogue that describes the rule insists on a very fast execution
  system; a slower trader arrives after the gap has closed and pays the cost anyway.
- The threshold is smaller than a typical cost. At 0.02 percent per leg, an extra half basis point on
  each of the four crossings takes most of the profit, and the library's own five-year record was
  negative.
- The normal level of the ratio drifts. Two funds tracking the same index still differ in dividends,
  fees and holdings, so the running average the rule subtracts is a moving target and can be stale
  exactly when it matters.
- The exit condition can leave the position open. The rule waits for the reverse ratio to return to
  its normal level. If the two funds have genuinely changed, that level never returns and the position
  stays on while the loss grows.
- Both legs can be interrupted. Selling short means borrowing a fund and paying a fee for it, and the
  lender can recall it at a bad moment. A trading halt in one of the two funds leaves the other leg
  unhedged.
- The data can lie. As the repository's brief shows, a consolidated feed can print out-of-sequence
  trades and impossible quotes, and a strategy that believes them trades on an error.

## Try it yourself

You need a spreadsheet and a public source of daily closing prices for two funds that track the same
index. Daily data will not show this trade, because the gaps live inside the day, but it will show how
stable the relationship is.

1. Make one row per day and these columns: Date, Closing price of fund 1, Closing price of fund 2,
   Ratio, Average ratio over the last 20 days, Difference, and Size of the difference as a percentage.
2. Ratio is the price of fund 2 divided by the price of fund 1.
3. Average ratio is the average of the Ratio column over the last 20 rows.
4. Difference is Ratio minus Average ratio. As a percentage, divide it by the average and multiply by
   100.
5. Mark every row where the percentage is above 0.02 or below -0.02. Those are the days the rule would
   have thought there was something to do, if it could have traded inside the day.

What to notice: on most days the difference is far below 0.02 percent, which is the honest reason this
strategy trades rarely. Now recompute the same column using the two funds' buying and selling prices
rather than their closing prices, if your source shows them, and watch how much of the gap disappears
into that spread. Then ask how many of the marked days were followed by the ratio returning to its
average within a week.

## Where this came from

- [QuantConnect strategy library: intraday arbitrage between index ETFs](https://www.quantconnect.com/tutorials/strategy-library/intraday-arbitrage-between-index-etfs),
  the rules as implemented: SPY and IVV, a ratio adjusted by its trailing mean, a 0.02 percent
  threshold, three updates of confirmation, and the backtest figures quoted above.
- Marshall, Nguyen and Visaltanachoti,
  [ETF arbitrage: Intraday evidence](https://doi.org/10.1016/j.jbankfin.2013.05.014), Journal of
  Banking and Finance, 2013, volume 37, issue 9, pages 3486 to 3498. This is the study of the
  deviations and the conditions around them.
- Kakushadze and Serur, 151 Trading Strategies, Section 6.4, arXiv `1912.04492v1`, which states the
  rule with a threshold near 1.002 and the warning about execution speed. The page is at
  [1912.04492](https://arxiv.org/abs/1912.04492).
- [Cross-Venue Structure and Arbitrage](../../../strategies/books/10_cross_venue_and_arbitrage.md),
  this repository's brief on fragmentation, feeds and net-of-cost opportunity, which cites
  `1810.11091v1`, `2604.20067v1`, `2608.09188v1` and `1907.09218v2`. The arXiv pages are at
  [1810.11091](https://arxiv.org/abs/1810.11091), [2604.20067](https://arxiv.org/abs/2604.20067),
  [2608.09188](https://arxiv.org/abs/2608.09188) and [1907.09218](https://arxiv.org/abs/1907.09218).

## Words used in this tutorial

- arbitrage: buying and selling the same thing in two places at once to profit from a price gap, with
  no view about the future.
- ask: the price at which someone is currently willing to sell, which is the price a buyer pays.
- basis point: one hundredth of one percent, so two basis points is 0.02 percent.
- bid: the price at which someone is currently willing to buy, which is the price a seller receives.
- index: a published list of assets whose combined price is tracked as a single number.
- short selling: borrowing something you do not own, selling it, and buying it back later, so you gain
  when its price falls.
- spread: the gap between the best buying price and the best selling price of the same thing at one
  moment.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
