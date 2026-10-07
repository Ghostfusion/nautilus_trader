# REIT momentum: buying the property trusts that have already been rising

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                                        |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of American real estate investment trusts, companies that own income-producing property and pay out almost all of their income                                                                                                                                                                                                                                        |
| How often it trades       | Every three months, when the list of trusts is rebuilt                                                                                                                                                                                                                                                                                                                       |
| What you need             | A spreadsheet and a list of property trusts with their prices                                                                                                                                                                                                                                                                                                                |
| Where the rules come from | [QuantConnect strategy library, momentum effect in REITs](https://www.quantconnect.com/tutorials/strategy-library/momentum-effect-in-reits) and the [Quantpedia entry](https://quantpedia.com/strategies/momentum-factor-effect-in-reits) it cites                                                                                                                           |
| The underlying research   | Derwall, Huij, Brounen and Marquering, [REIT Momentum and the Performance of Real Estate Mutual Funds](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1161160)                                                                                                                                                                                                          |
| How well it held up       | Mixed: the effect was strong over a long American sample from 1980 to 2008 and has been re-found since, but one study that extended the same test to 2009 reported the profit statistically indistinguishable from zero, and later papers attribute part of the payoff to a different effect, earnings drift, so the grade rests on the sample and on what is being measured |
| Also appears in           | [Sector momentum](../sector-momentum/README.md) in this collection, the same cross-sectional momentum idea applied to industry funds                                                                                                                                                                                                                                         |

## The idea in one paragraph

A real estate investment trust, usually shortened to REIT, is a company that owns property which
produces income, such as offices, shopping centres, apartments, warehouses and data centres, and
collects rent from the tenants. In the United States a REIT must pay out almost all of its taxable
income to its owners each year to keep a special tax treatment, so people buy it mainly for that
regular payout, and its shares trade on an exchange like any other company's. This strategy sorts
all American REITs by how far their prices rose over roughly the past year, setting aside the most
recent month, and buys the best-performing third of them in equal amounts. It holds them for three
months and then rebuilds the list. The bet is the same one that has been documented in ordinary
shares: a trust that has been rising tends to keep rising for a while.

## Why anyone believed it

A REIT is a landlord with a share price. Its rent tends to rise slowly and in steps, because leases
are signed for years and are renegotiated one at a time, so good news about a trust arrives over
months rather than in a single morning. That slowness is the first half of the argument: if a
landlord is doing well, the market learns it gradually, and the price drifts up as each new lease
or rent rise is reported. The momentum documented in REITs is exactly that drift.

The second half is who is on the other side. REITs are held largely by income investors and by
specialist real estate funds, and money moves between them on the basis of recent results. A trust
that has beaten its peers attracts money, because a fund manager who owns none of it looks wrong,
and that buying pushes the price further before any new rent is signed. The sellers are the
income investors who sell a trust that has risen to take the profit, and the funds that must sell a
trust after it is dropped from the index they track. Those mechanical sellers reappear month after
month, which is what lets the drift persist.

## An everyday comparison

Think of a seaside hotel that was fully booked every summer. The travel agents keep recommending it
because it sold well last year, the guests who enjoyed it rebook, and the coach tours are scheduled
around the hotels that filled up before. A hotel that was half empty last summer is recommended by
nobody and gets fewer bookings, even though the two buildings are much the same. The gap between
the two hotels widens for reasons that have nothing to do with the sea view: the winner is easier
to sell because it is already the winner. The strategy here buys the three hotels with the fullest
books and ignores the rest.

## The rules, step by step

1. Build the universe. Start from every REIT listed on an American exchange, then remove any whose
   share price is below one dollar, any without published company financial data, and any whose
   shares barely trade. The QuantConnect version drops trusts that trade fewer than ten thousand
   shares a day, so that a small account can still buy and sell them.
2. For each remaining trust, compute its return over the past eleven months, ending one month ago.
   Take the closing price about a year ago and the closing price about a month ago, and divide.
   The QuantConnect code uses the price 365 days back and the price 30 days back.
3. Rank the trusts by that return, best first.
4. Cut the ranked list into three equal groups. The top group is called the top tercile, a third of
   the list.
5. Buy every trust in the top tercile, in equal amounts. If there are thirty trusts in the tercile,
   each gets one thirtieth of the account.
6. Hold for three months. Do not look at the prices in between.
7. At the end of the three months, rebuild the whole list from step 2 and repeat from step 3. Sell
   whatever has fallen out of the top tercile and buy whatever has entered it.

A note on the one-month gap in step 2. Momentum measured over the last month alone tends to reverse,
because prices bounce back after a sharp move, so the rule measures the momentum up to a month ago
and stops there. That convention is usually written as 11-1 momentum: eleven months of return,
ending one month ago.

## The maths, with every symbol named

The strategy is one calculation, one sort and one average.

The momentum score of one trust:

```text
M_i = P_i_one_month_ago / P_i_one_year_ago - 1
```

- `M_i` is the momentum score of trust `i`, written as a decimal: 0.30 means 30 percent.
- `P_i_one_month_ago` is that trust's closing price about a month before the ranking date.
- `P_i_one_year_ago` is that trust's closing price about a year before the ranking date.
- The gap between the two dates is therefore about eleven months, which is why the score is called
  an eleven-month return ending one month ago.

Rank the universe by `M_i`, largest first, and keep the top third. If the top third contains `N`
trusts, give each one the same weight:

```text
w_i = 1 / N for each of the N trusts in the top tercile, and 0 for all the others
```

- `w_i` is the fraction of the account placed in trust `i`.
- The weights of the chosen trusts add up to 1, so the whole account is invested, split evenly.

The account's return over the next three months is then the average of the chosen trusts' returns,
because every slice is the same size:

```text
R_portfolio = ( R_1 + R_2 + ... + R_N ) / N
```

- `R_1` to `R_N` are the next-three-month returns of the chosen trusts.
- Dividing by `N` is the same as multiplying each by `1/N` and adding.

Finally the cost of a rebuild. If a fraction `t` of the account is traded at the rebuild:

```text
Cost = t * c
```

- `t` is the traded fraction, counting both the sold side and the bought side. Selling one third of
  the account and buying one third of the account is `t = 2/3`, because `1/3` is sold and `1/3` is
  bought. Replacing half the holdings is `t = 1.0`, and replacing all of them is `t = 2.0`.
- `c` is the cost per trade as a fraction of the amount traded, covering the gap between the buying
  and selling price plus commission. A realistic figure for shares of medium-sized property trusts
  is 0.0005 to 0.002, that is five to twenty basis points, where one basis point is one hundredth
  of one percent. Some trusts are thinly traded, and their gap is at the wide end of that range.

## A worked example

Nine REITs, ranked by their return over the past eleven months ending one month ago. The prices are
invented, but they are of the size property trusts actually move.

| Trust | Price a year ago | Price a month ago | M      | Rank | Tercile |
| ----- | ---------------- | ----------------- | ------ | ---- | ------- |
| A     | 100.00           | 130.00            | +0.300 | 1    | Top     |
| B     | 50.00            | 60.00             | +0.200 | 2    | Top     |
| C     | 80.00            | 92.00             | +0.150 | 3    | Top     |
| D     | 40.00            | 44.00             | +0.100 | 4    | Middle  |
| E     | 25.00            | 26.50             | +0.060 | 5    | Middle  |
| F     | 60.00            | 62.40             | +0.040 | 6    | Middle  |
| G     | 30.00            | 29.40             | -0.020 | 7    | Bottom  |
| H     | 75.00            | 72.00             | -0.040 | 8    | Bottom  |
| I     | 20.00            | 18.60             | -0.070 | 9    | Bottom  |

The top tercile is A, B and C, one third of the account each. Now six quarters pass. Each row below
is a quarter: the trusts the rules would hold, their returns over that quarter, the gross result
from the average formula, the traded fraction at the rebuild, the cost, and the net result.

| Quarter | Trusts held | Their returns | Gross  | t      | Cost   | Net    |
| ------- | ----------- | ------------- | ------ | ------ | ------ | ------ |
| 1       | A, B, C     | +4%, -2%, +1% | +1.00% | 1.0000 | 0.100% | +0.90% |
| 2       | A, B, E     | +2%, +3%, -1% | +1.33% | 0.6667 | 0.067% | +1.27% |
| 3       | A, B, E     | +5%, +1%, +2% | +2.67% | 0.0000 | 0.000% | +2.67% |
| 4       | A, D, E     | -1%, +2%, +1% | +0.67% | 0.6667 | 0.067% | +0.60% |
| 5       | A, D, E     | +3%, +1%, +2% | +2.00% | 0.0000 | 0.000% | +2.00% |
| 6       | A, D, E     | +1%, -1%, +2% | +0.67% | 0.6667 | 0.067% | +0.67% |

Read quarter 2 as an example. C fell out of the top tercile and E entered, so one third of the
account is sold and one third is bought, giving `t = 2/3 = 0.6667`. At a cost of `c = 0.001`, that
is ten basis points, the cost is `0.6667 * 0.001 = 0.000667`, or 0.067 percent. The gross `+1.33%`
becomes `+1.27%` after costs.

Compounding the six net figures: `1.0090 * 1.0127 * 1.0267 * 1.0060 * 1.0200 * 1.0067 = 1.0836`, so
the account gained about 8.4 percent over six quarters, which is a year and a half, or about 5.5
percent a year. The published long-only figure is 9.51 percent a year, so the invented numbers here
produce a smaller result; that is fine, because the example is invented and is not a claim. Two
things are worth noticing. First, the cost is paid only when the composition of the tercile changes,
and the top tercile changes slowly, so the costs are far smaller than for a rule that trades every
month. Second, the example says nothing about whether the strategy works; it only shows how to apply
the rules and how the numbers behave.

## What the research actually found

| Source                                   | What it measured                                                                                          | Result                                                                                                                                                                                                                        |
| ---------------------------------------- | --------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Quantpedia, summarising the source paper | All American REITs, past 11-month return one month lagged, terciles, hold the best third for three months | 9.51 percent a year long-only over a 1980 to 2008 backtest, worst fall 55.36 percent, about 50 trusts in a tercile, the strategy rated as simple and monthly in its review cycle                                              |
| Derwall, Huij, Brounen and Marquering    | REIT returns and the performance of real estate mutual funds                                              | REITs show a large and prevalent momentum effect that conventional factor models do not capture, and that momentum explains much of the abnormal return that actively managed REIT funds earned in aggregate                  |
| Huerta and Rivas                         | The same REIT momentum rule, extended to 2009                                                             | The momentum returns were positive and significant before 1990 and larger between 1990 and 1999, but extending the sample to 2009 made them statistically indistinguishable from zero; the authors conclude it had dissipated |
| Hung and Glascock                        | REIT momentum and different kinds of volatility                                                           | Momentum returns were higher when volatility was higher; the losers carried more firm-specific risk than the winners, which explains part of the payoff                                                                       |
| Feng, Price and Sirmans                  | REIT momentum against the drift after earnings news                                                       | The two effects are closely related, and the payoff to a REIT momentum strategy is subsumed by the earnings-news drift, meaning it may be that effect in disguise                                                             |
| Guidolin and Pedio                       | REIT factors over 1993 to 2018                                                                            | Support was found for value, size, momentum, investment and profitability factors in REITs, with only profitability unsupported                                                                                               |

Read together, the picture is this. There is a documented tendency for a property trust that has been
rising to keep rising over months, and it was large enough over the long sample to be noticed by more
than one research team. But the effect is not stable: one careful study found it had become
statistically indistinguishable from zero once the sample was extended, and another attributes the
payoff to a different effect entirely, the drift after earnings news. The honest reading is that the
REIT universe is small, that its returns are driven heavily by interest rates, and that the reward
for this rule has at times been real and at times absent.

## How this project relates to it

The repository has no REIT strategy and no REIT data. The closest work is
[Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), this
repository's own study of momentum applied to a group of equity portfolios. Its Section 3.2 states
the condition such a rule needs, that the leaders stay leaders for several months, and its Section
3.3 collects the tests, including an experiment over 1,022 rotation rules in American sectors whose
average rule returned 0.86 percent a month against 0.89 percent for simply holding the market. A
REIT universe is smaller and more concentrated than the sector universe, but the same arithmetic of
trying many rules and keeping the best applies.

The second related document is the harvest of the momentum literature in
[the predictability brief](../../../strategies/books2/08_predictability_and_trading_strategies.md),
which records that a momentum strategy's return is positively skewed by construction and depends on
the holding horizon `2101.01006v2` (p.3), and that a published signal's edge is competed away as
more money runs it (`2605.23905v1`, p.1, p.5). Both findings apply directly to a rule this simple:
the reward arrives in lumps, not steadily, and it is the kind of rule that many people can find.

## Where it goes wrong

- The sample ends before the hard years. The figure of 9.51 percent a year comes from 1980 to 2008,
  and the study that extended the same test to 2009 found the profit had gone. A backtest that stops
  in 2008 has not been tested on the years when property trusts were most tested.
- REITs are interest-rate sensitive. They borrow heavily to buy property and they are bought for
  their payout, so when interest rates rise both the borrowing cost and the appeal of the payout
  change at once. A momentum rule has no defence against a whole-sector fall caused by rates.
- Price-only momentum ignores most of the income. A REIT pays out almost all of its income, so a
  large part of its total return is the dividend and not the price. This rule ranks trusts by the
  price alone, so it can rank a high-payout trust low and a low-payout trust high. The dividends
  are also the reason a REIT's price can fall while its owner is still being paid.
- The universe is small and it changes. There are only a few hundred American REITs, and a third of
  that is a few dozen names, so the rule is concentrated. The list of what counts as a REIT also
  changes over time, which means a long backtest is partly a study of the data provider's
  classification rather than of the market.
- Momentum crashes. After a market-wide fall, the trusts that fell furthest are often the ones the
  previous ranking had just bought, and they often bounce hardest. The published worst fall of 55
  percent happened with the rule in place.
- The number of ways to define it is large: eleven months or twelve, one month of gap or none,
  terciles or quartiles, three-month or monthly holding. Choosing the best of these after seeing
  the results is how a real effect gets confused with a lucky setting.

## Try it yourself

You need nothing but a spreadsheet and a public list of property trusts with their prices; the
QuantConnect page names the data field that identifies them, and a finance website will give you the
prices.

1. Write down fifteen to thirty property trusts that traded throughout the last five years. Keep the
   list to trusts that existed for the whole period, so you are not looking at the future.
2. Build a sheet with one column per trust and one row per month of closing prices.
3. Add a row, once per quarter, that computes each trust's eleven-month return: the price one month
   ago divided by the price twelve months ago, minus one.
4. In that row, mark the top third of trusts by that value. These are what the rules would have
   bought.
5. In the next three rows down, average the monthly returns of those trusts. That is the strategy's
   return for the quarter, before costs.
6. Add a column that counts how many names changed at each rebuild, multiply by one third, and
   subtract that fraction times 0.001 from the next quarter's return.

What to notice: the top third is often almost the same list from one quarter to the next, so the
cost is small and the turnover is low, which is unusual among momentum rules. Then compare the
strategy against simply holding all the trusts equally. If your sheet shows the strategy winning by
a wide margin, the likely cause is that you chose the trust list after seeing which ones had done
well, or that you used today's list of surviving trusts for the whole five years, which quietly
removes the ones that went under.

## Where this came from

- QuantConnect strategy library, momentum effect in REITs,
  [the page](https://www.quantconnect.com/tutorials/strategy-library/momentum-effect-in-reits):
  the rules as implemented, the REIT filter, the eleven-month return ending one month ago, the top
  third in equal amounts, rebuilt every three months.
- Quantpedia, momentum factor effect in REITs,
  [the entry](https://quantpedia.com/strategies/momentum-factor-effect-in-reits):
  the performance figures, the instrument count, the source and later papers, and the caution that
  a recent study documents the effect losing its significance.
- Derwall, Huij, Brounen and Marquering, REIT Momentum and the Performance of Real Estate Mutual
  Funds,
  [the paper](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1161160): the original study of
  the REIT momentum effect.
- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), this
  repository's own study of rotation rules, and specifically Sections 3.2 and 3.3, which are where
  the 1,022-rule experiment and the condition a rotation rule needs come from.
- [the predictability brief](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  the repository's harvest of the momentum literature, which supplies `2101.01006v2` on momentum
  skewness and `2605.23905v1` on the decay of a published signal.

## Words used in this tutorial

- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- dividend: a cash payment a company makes to its shareholders out of profit.
- long: owning something, so that you gain when its price rises.
- momentum: the tendency of something that has been rising to keep rising for a while.
- REIT: a real estate investment trust, a company that owns income-producing property, collects rent
  and pays out almost all of its income to its owners.
- tercile: one of three equal groups made by sorting a list and cutting it in two places.
- universe: the full set of things a rule is allowed to choose from.
- weight: the share of your money placed in one holding.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
