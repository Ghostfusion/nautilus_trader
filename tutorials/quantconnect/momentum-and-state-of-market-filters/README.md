# Momentum with a market filter: only betting when the whole market has been rising

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                      |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | Shares of American companies in a winners basket and a losers basket, or a government bond fund when the trade is switched off                                                                                                                                                             |
| How often it trades       | Once a month, when the ranking and the on-or-off decision are reviewed                                                                                                                                                                                                                     |
| What you need             | A spreadsheet and a year of prices for a broad market index and for the shares                                                                                                                                                                                                             |
| Where the rules come from | [QuantConnect strategy library, momentum and state of market filters](https://www.quantconnect.com/tutorials/strategy-library/momentum-and-state-of-market-filters) and the [Quantpedia entry](https://quantpedia.com/strategies/momentum-and-state-of-market-sentiment-filters) it cites  |
| The underlying research   | Cooper, Gutierrez and Hameed, [Market States and Momentum](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=299927)                                                                                                                                                                     |
| How well it held up       | Mixed: the original study found momentum profits appear almost only after the broad market has risen, and that conditioning has been replicated, but the momentum premium itself has weakened in recent decades and the filter is a timing rule whose benefit depends on the period chosen |
| Also appears in           | [Sector momentum](../sector-momentum/README.md) in this collection, the same idea applied to industries, and the regimes brief it this project's research                                                                                                                                  |

## The idea in one paragraph

Momentum means buying what has already been rising and betting against what has already been
falling. It is one of the oldest recorded patterns in share prices, but it can lose money badly when
the whole market turns down, because the shares that rose most are often the ones that fall hardest.
Researchers found that momentum's profits come almost entirely after the broad market has been
rising over the previous year; after the market has fallen, momentum has tended to do nothing or
lose. So this strategy first checks the market, and only places the momentum bet when the broad
index is higher than it was a year ago. When it is not, the strategy steps aside and holds a
government bond fund. A rule of this kind, which decides whether a strategy may run at all based on
the condition of the whole market, is called a market filter.

## Why anyone believed it

Momentum is usually explained by investors being slow to change their minds or by them piling in
late. When the broad market has been rising, the mood is confident, and confidence makes investors
overreact to good news about a company, pushing its price further than the news justifies. When the
market has been falling, investors are cautious and update more slowly, so there is less
overreaction to trade against.

A market filter is a form of risk management rather than a new source of profit. It accepts that the
momentum trade is a fair-weather one, and it takes the strategy out of the weather it does not
handle. The counterparty is the same slow or over-excited investor, but only in the periods when
that behaviour is strongest.

## An everyday comparison

A ferry operator watches the weather. On a calm day the crossing is worth running and the operator
sails. When a storm is coming, the sensible move is not to sail harder - it is to stay in port and
wait, earning almost nothing, so that the boat and the passengers are ready for tomorrow. Small
maritime operators who insist on sailing every day regardless are the ones who lose boats. The
strategy here runs the momentum crossing on calm days and sits in port on stormy ones.

## The rules, step by step

1. Choose the broad market index. The original study used a wide index of American shares, and this
   implementation used the Wilshire 5000, a total-market measure. Any index covering almost the
   whole market will do.
2. Each month, compute the index's return over the past twelve months: today's level divided by the
   level twelve months ago, minus one.
3. If that number is zero or positive, the state of the market is "UP". If it is negative, the state
   is "DOWN".
4. Also each month, compute for every share the momentum score: today's price divided by the price
   six months ago, minus one.
5. Sort the shares by that score, best first.
6. Buy the twenty with the highest scores and sell short the twenty with the lowest.
7. Spread each side equally: fifty percent of the money in the long basket, fifty percent sold short,
   so no net money is invested in shares.
8. If the state is UP, hold those two baskets until the next month.
9. If the state is DOWN, close every share position and put the whole account into a fund that holds
   long-dated government bonds, such as the Treasury bond fund used in the implementation. Long-
   dated means the bonds repay far in the future, which makes their price move more when interest
   rates change.
10. Repeat from step 2 at the start of the next month.

## The maths, with every symbol named

Two simple calculations and then a subtraction.

The state of the market:

```text
M = P_now / P_12m_ago - 1
```

- `M` is the market's twelve-month return, written as a decimal: 0.10 means 10 percent.
- `P_now` is the current level of the broad index.
- `P_12m_ago` is the level of the same index twelve months earlier.
- The state is UP when `M` is zero or more, and DOWN when `M` is less than zero.

The momentum score of one share:

```text
m_i = P_i_now / P_i_6m_ago - 1
```

- `m_i` is the momentum score of share `i`.
- `P_i_now` is the current price of share `i`.
- `P_i_6m_ago` is its price six months earlier.

Rank the shares by `m_i` from largest to smallest. The top twenty form the long basket, the bottom
twenty the short basket. In the UP state the portfolio return over the next month is half the long
basket's return minus half the short basket's return:

```text
R = 0.5 * (R_long - R_short)
```

- `R` is the return on the account for the month when the state is UP, assuming the two baskets are
  equal in size.
- `R_long` is the average return of the twenty winners.
- `R_short` is the average return of the twenty losers.
- The 0.5 appears because the strategy is half long and half short; the two halves are equal and
  opposite, so no net money is invested.

In the DOWN state `R` is simply the return of the bond fund, because the share positions are closed.

The cost of trading:

```text
Cost = T * c
```

- `T` is the two-way turnover, counted in units of the whole account. Replacing a third of the long
  basket and a third of the short basket means selling and buying 2/3 of the long side and 2/3 of
  the short side, that is `T = 1.33`. Switching entirely into the bond fund and back counts as well.
- `c` is the cost of one trade as a fraction of the amount traded, covering the gap between buying
  and selling prices plus commission. For the liquid shares in this universe 0.0015, or 15 basis
  points, is a reasonable working figure; one basis point is one hundredth of one percent.

The return the account keeps is `R - Cost`.

## A worked example

Eight invented months. The market column is the index's twelve-month return; the winner and loser
columns are the average returns of the two baskets if momentum had been run; the bond column is the
return of the long-dated government bond fund. Costs are 15 basis points on the traded notional.

| Month | Market 12-month | State | Winners | Losers | Momentum 0.5*(W-L) | Bond fund | Before cost | Turnover | Cost  | Net    |
| ----- | --------------- | ----- | ------- | ------ | ------------------ | --------- | ----------- | -------- | ----- | ------ |
| 1     | +15%            | UP    | +3.0%   | -1.0%  | +2.00%             | -         | +2.00%      | 2.0      | 0.30% | +1.70% |
| 2     | +12%            | UP    | +2.0%   | -2.0%  | +2.00%             | -         | +2.00%      | 2.0      | 0.30% | +1.70% |
| 3     | +5%             | UP    | -3.0%   | +1.0%  | -2.00%             | -         | -2.00%      | 2.0      | 0.30% | -2.30% |
| 4     | -2%             | DOWN  | -8.0%   | -1.0%  | -3.50%             | +0.5%     | +0.50%      | 1.3      | 0.20% | +0.30% |
| 5     | -8%             | DOWN  | -4.0%   | +1.0%  | -2.50%             | +0.4%     | +0.40%      | 0.3      | 0.05% | +0.35% |
| 6     | +6%             | UP    | +4.0%   | -3.0%  | +3.50%             | -         | +3.50%      | 2.0      | 0.30% | +3.20% |
| 7     | +9%             | UP    | +1.0%   | -1.0%  | +1.00%             | -         | +1.00%      | 1.3      | 0.20% | +0.80% |
| 8     | +3%             | UP    | -2.0%   | +2.0%  | -2.00%             | -         | -2.00%      | 2.0      | 0.30% | -2.30% |

The arithmetic for month 1: winners minus losers is 3.0 minus (-1.0), which is 4.0 percent, and half
of that is 2.00 percent. Turnover of 2.0 units at 15 basis points is 2.0 times 0.0015, which is
0.0030, or 0.30 percent. Net is 2.00 minus 0.30, which is 1.70 percent. In months 4 and 5 the state
is DOWN, so the share baskets are not held; the account earns the bond fund's small return and pays
only the cost of switching.

The net values add to 3.45, an average of 0.43 percent a month. Compounding the eight monthly
factors (1.0170 times 1.0170 times 0.9770 times 1.0030 times 1.0035 times 1.0320 times 1.0080 times
0.9770) gives 1.0337, a gain of 3.37 percent over the eight months.

Now run the same eight months without the filter, so the momentum trade is held every month. In
months 4 and 5 the momentum baskets lost 3.50 and 2.50 percent before costs, against a small
positive bond return. Those two months alone turn the eight-month result from a gain of 3.37 percent
into 0.9603, a loss of 3.97 percent. That is the whole purpose of the filter, shown in one small
example: it does not add a new edge, it avoids a specific kind of loss.

## What the research actually found

| Source                                    | What it measured                                                                      | Result                                                                                                                                                                                                                                                                                                               |
| ----------------------------------------- | ------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Cooper, Gutierrez and Hameed              | Six-month momentum on American shares, split by the state of the market, 1929 to 1995 | After the market had risen over the prior three years, the winners-minus-losers portfolio earned 0.93 percent a month; after the market had fallen, it earned minus 0.37 percent a month, which was not distinguishable from zero. The authors wrote that short-run momentum profits follow UP periods "exclusively" |
| The same study, one-year state definition | The same test using the market's past twelve months instead of three years            | UP-market profit 1.04 percent a month, DOWN-market profit minus 0.08 percent a month, over the same sample                                                                                                                                                                                                           |
| The same study, long horizon              | The same portfolios over holding months 13 to 60                                      | After UP markets the momentum profits reversed in the long run, at minus 0.36 percent a month; so the winners eventually gave back what they had earned                                                                                                                                                              |
| Chordia and Shivakumar                    | Whether a set of economic variables could explain momentum                            | They argued that the economic variables explained most of the momentum profit; Cooper, Gutierrez and Hameed found the claim did not survive a simple price screen and a one-month gap between measuring and trading                                                                                                  |
| This repository's sector rotation study   | 1,022 possible rotation rules on American sectors                                     | The average rule returned 0.86 percent a month against 0.89 percent for simply holding the market, and the authors concluded the outperformance of the best rules came from trying so many                                                                                                                           |

Read together: the market-state condition is one of the more robust ways to improve momentum, and it
has been replicated with different horizons. But it does not create profit where momentum is weak,
it only avoids some of the losses when momentum is bad. In the years since the original sample the
momentum premium has been smaller and has suffered abrupt losses, so the benefit of the filter
depends heavily on which years the reader looks at.

## How this project relates to it

This repository's own study of the momentum family is
[Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md). Section 3.2
states the condition momentum needs, that leaders stay leaders for several periods, and Section 8
sets out how to measure which regime the market is in, with the warning that the regime decision is
a measurement rather than a preference and should be frozen in advance.

The regime idea is implemented in
[implementation/sector-regime-engine](../../../implementation/sector-regime-engine/README.md). It
does not run a momentum strategy. It measures whether a market is in a state where such a rule is
eligible to be run at all, and it reports the evidence for that verdict rather than a profit figure.
Its [user manual](../../../implementation/sector-regime-engine/USER_MANUAL.md) states plainly what
the app does not do, and should be read before the engine is touched.

The third related brief is
[Regimes, breakpoints and state switching](../../../strategies/books2/24_regimes_and_change_points.md).
Its sharpest finding is that a state label read off a full-sample fit is not a live signal; the only
regime model in its slice that survived out of sample was re-estimated as data arrived. Any market
filter has to be rebuilt on the information available at the time or it is cheating.

## Where it goes wrong

- The definition of the state is a free choice. Three years, one year, or a sentiment survey each
  gives a different on-or-off pattern, and picking the version that looks best after the fact is how
  a rule becomes a coincidence.
- The filter is late by construction. It reacts to the market's past year, so it switches off after a
  fall has already happened and switches back on after a rise has already happened. In a sharp
  V-shaped recovery it sits in bonds through the best part.
- The filter can cost more than it saves. In the worked example it helped because the DOWN months
  were bad for momentum, but if the market falls while momentum keeps earning, the filter removes
  exactly the wrong month.
- Momentum crashes are correlated with the filter's weakness. The largest momentum losses happen in
  the recovery from a market fall, which is precisely when the twelve-month state can still read
  DOWN and then flips to UP mid-rally.
- Data snooping. Both this study and the sector rotation study above were found in a large search
  over horizons and definitions, and the more versions tried, the easier it is to find one that
  looks good.
- What would have to be true for the idea to be false: that momentum's profits are unrelated to the
  market's recent direction, and that the apparent pattern is an artefact of the 1929 to 1995 sample
  or of the many ways the state could have been defined.

## Try it yourself

You need a spreadsheet, a broad index, and the prices of a handful of well-known shares.

1. Make a column of monthly index levels for five years, and a column for the twelve-month return:
   this month's level divided by the level twelve rows up, minus one.
2. In the next column write UP if that value is zero or more and DOWN if it is less.
3. For each month, in a separate block, write the six-month momentum score of each share: this
   month's price divided by the price six rows up, minus one.
4. Each month pick the single best and the single worst share by that score. In the next row record
   their actual returns.
5. Build two columns: the strategy's return, being half of (best minus worst), only in UP months and
   zero in DOWN months; and the market's own return.
6. Total each column over the five years and compare.

What to notice: the number of UP and DOWN months will be lopsided, and most DOWN months will sit in
one or two clusters. The strategy's advantage or disadvantage will come almost entirely from what
happened in those clusters, not from the many ordinary months. That is the honest description of
what a market filter does: it changes a handful of months a lot and the rest barely at all.

## Where this came from

- [QuantConnect strategy library: momentum and state of market filters](https://www.quantconnect.com/tutorials/strategy-library/momentum-and-state-of-market-filters),
  the rules as implemented: twenty winners and twenty losers by six-month momentum, the twelve-month
  market state, and a bond fund when the state is down.
- [Quantpedia: momentum and state of market filters](https://quantpedia.com/strategies/momentum-and-state-of-market-sentiment-filters),
  the entry the implementation cites for the idea.
- Cooper, Gutierrez and Hameed, [Market States and Momentum](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=299927),
  the study the twelve-month state and the effect sizes come from.
- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), this
  repository's study of momentum at the sector level, including the 1,022-rule experiment.
- [Regimes, breakpoints and state switching](../../../strategies/books2/24_regimes_and_change_points.md),
  the repository's reading of what a live regime filter can and cannot do.

## Words used in this tutorial

- broad market index: a single number that tracks the prices of most of a country's listed shares.
- government bond fund: a fund holding loans to a government, which usually moves gently and pays
  interest.
- market filter: a rule that decides whether a strategy is allowed to run at all, based on the
  condition of the whole market.
- momentum: the tendency of something that has been rising to keep rising for a while.
- position: the amount of a holding you own, or owe if it is a short position.
- risk management: choosing what to do with the chance of loss, rather than trying to remove it.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- volatility: how much a price moves around its average, measured as a percentage per year.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
