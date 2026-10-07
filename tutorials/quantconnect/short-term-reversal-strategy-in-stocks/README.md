# Buying the shares that just fell the hardest, and selling the ones that just soared

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                        |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of the 100 largest American companies, bought when they have just fallen and sold short when they have just risen                                                                                                                                                     |
| How often it trades       | About once a week                                                                                                                                                                                                                                                            |
| What you need             | A spreadsheet and a source of daily share prices                                                                                                                                                                                                                             |
| Where the rules come from | [QuantConnect strategy library, short term reversal strategy in stocks](https://www.quantconnect.com/tutorials/strategy-library/short-term-reversal-strategy-in-stocks) and the [Quantpedia entry](https://quantpedia.com/strategies/short-term-reversal-in-stocks) it cites |
| The underlying research   | De Groot, Huij and Zhou, [Another Look at Trading Costs and Short-Term Reversal Profits](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1605049), published in the Journal of Banking and Finance in 2012                                                               |
| How well it held up       | Mixed: the effect is well documented over the 1990 to 2009 sample and survives costs when restricted to large companies, but independent later work finds it has weakened, and the QuantConnect test itself loses to the index over 2016 to 2021 apart from the 2020 crash   |
| Also appears in           | [Sector momentum](../sector-momentum/README.md) in this collection, which ranks shares the opposite way over a much longer horizon                                                                                                                                           |

## The idea in one paragraph

Take the most heavily traded American shares and measure how much each one moved over the past month.
Buy the ten that fell the most. At the same time, sell short the ten that rose the most, meaning you
borrow those shares, sell them, and hope to buy them back cheaper. Hold the whole book for a week,
then rebuild it. The bet is that a sharp move over a few weeks goes too far and partly comes back, so
the recent losers recover a little and the recent winners give a little back. This is the opposite of
the well-known momentum idea, where the winners of the past year keep winning; the two live at
different horizons.

## Why anyone believed it

Two stories are told, and they fit together. The first is overreaction: when bad news arrives, or when
a share simply falls for a few days, many investors sell first and think later, and the price drops
further than the news deserves. The second is liquidity. Someone who needs to sell a large amount
quickly must accept a lower price to find a buyer, because the buyer is doing the seller a favour by
taking the shares off their hands. The return the reversal trader earns is the fee for being that
buyer. Nagel, in a paper called Evaporating Liquidity, showed that the returns of these strategies
move with the price investors charge for providing liquidity, and that the fee spikes during market
turmoil.

The counterparty, then, is the impatient or forced seller: a fund meeting withdrawals, a manager
cutting a position in a hurry, or anyone who overreacts to a headline. If such sellers keep appearing,
the price keeps overshooting, and the patient buyer keeps collecting the fee. The short side has its
own counterparty: the euphoric buyer who chases a share that has just jumped.

## An everyday comparison

A greengrocer marks the fruit down late in the day. The discount is not a statement that the fruit is
bad; it is the price of emptying the shelf before closing. A shopper who arrives at that hour and buys
the marked-down fruit usually gets a bargain, because the shop wants the shelf empty more than it
wants the last few pennies. The strategy here is the shopper arriving at closing time, every day, and
the short side is the shopper who quietly bets that tomorrow's marked-up fruit will not sell.

## The rules, step by step

1. Build the universe. Each week, take all American shares, sort them by dollar volume (shares traded
   multiplied by their price, which measures how actively a share changes hands), and keep the 100
   most heavily traded.
2. For each of those shares, compute its change over the past month: take today's price, take the
   price about 22 trading days ago, and divide. A share that went from 100.00 to 92.00 has a change of
   minus 8 percent.
3. Rank the shares by that change, largest first.
4. Buy the ten with the smallest change, putting one twentieth of the account into each, so the long
   side uses half the money. In a spreadsheet, 5 percent per share.
5. Sell short the ten with the largest change, one twentieth of the account in each, so the short side
   also uses half the money. Selling short means borrowing shares, selling them now, and buying them
   back later; if the price falls you gain, and if it rises you lose.
6. Hold for one week. Do not look at the prices in between.
7. At the start of the next week, recompute step 2 for all shares and repeat from step 3. Sell
   anything that has left the list, buy anything that has entered, and close any position whose share
   has dropped out of the 100 largest.

The momentum reader should notice how different this is from the twelve-month rule. That rule buys the
best performers of the past year and holds for a month or more. This rule buys the worst performers of
the past month and holds for a week. Same data, opposite direction, different horizon.

## The maths, with every symbol named

The reversal score of one share:

```text
r = P_today / P_month_ago - 1
```

- `r` is the share's change over the month, as a decimal: -0.08 means minus 8 percent.
- `P_today` is the share price today.
- `P_month_ago` is the share price about 22 trading days ago.

Then rank the shares by `r`, from smallest to largest. The ten smallest get a positive weight and the
ten largest get a negative weight:

```text
w = +0.05 for each of the ten biggest fallers
w = -0.05 for each of the ten biggest risers
```

- `w` is the fraction of the account placed in that share. Positive means bought, negative means sold
  short.

The portfolio's return over the following week is the sum of each weight multiplied by that share's
return:

```text
R_portfolio = sum over all held shares of (w_i * R_i)
```

- `w_i` is the weight of share `i`, positive for a long position and negative for a short one.
- `R_i` is that share's return over the next week, as a decimal.
- `sum` means add the products together across the twenty positions.

One consequence of the negative weights is worth spelling out. If a share you sold short falls 2
percent, its `R_i` is -0.02, and a weight of -0.05 turns that into a positive contribution of 0.001,
or 0.10 percent, because a short position gains when the price falls.

The costs are the traded fraction times the cost per trade, plus a fee for borrowing the shares you
sold short:

```text
Cost = t * c + b
```

- `t` is the traded fraction: 2.0 if every position is closed and reopened each week, since both the
  sale and the purchase count.
- `c` is the cost per unit traded, covering the gap between buying and selling prices plus commission;
  a realistic figure for the largest American shares is 0.0005, that is five basis points, where one
  basis point is one hundredth of one percent.
- `b` is the borrowing fee on the short positions, a small charge per week for the loan of the shares.

## A worked example

Ten shares from the liquid universe, with their change over the past month. The numbers are invented,
but they are of the size monthly share moves actually take.

| Share | One-month change (percent) | Rank (smallest first) | Action |
| ----- | -------------------------- | --------------------- | ------ |
| F     | -9                         | 1                     | buy    |
| H     | -6                         | 2                     | buy    |
| B     | -5                         | 3                     | none   |
| D     | -3                         | 4                     | none   |
| I     | 0                          | 5                     | none   |
| E     | +1                         | 6                     | none   |
| A     | +2                         | 7                     | none   |
| G     | +4                         | 8                     | none   |
| J     | +6                         | 9                     | short  |
| C     | +8                         | 10                    | short  |

A ten-share universe gives a smaller example, so the two biggest fallers F and H are bought and the
two biggest risers C and J are sold short, one quarter of the account on each. Now suppose the next
week produces these returns.

| Share | Position   | Weight | Next-week return | Contribution    |
| ----- | ---------- | ------ | ---------------- | --------------- |
| F     | bought     | +0.25  | +3.0 percent     | +0.7500 percent |
| H     | bought     | +0.25  | +2.0 percent     | +0.5000 percent |
| C     | sold short | -0.25  | -2.0 percent     | +0.5000 percent |
| J     | sold short | -0.25  | -1.5 percent     | +0.3750 percent |
| Total |            | 0.00   |                  | +2.1250 percent |

The two long positions rose, as a rebound would, and the two short positions fell, which a short
position turns into a gain. The total before costs is +2.1250 percent for the week. Suppose the whole
book is rebuilt, so every position is closed and a new one opened:

```text
t = 2.0
Cost = 2.0 * 0.0005 + 0.0002 = 0.0010 + 0.0002 = 0.0012, that is 0.12 percent
Net return for the week = 2.1250 - 0.12 = 2.0050 percent
```

That weekly figure is far above anything the literature reports, which is a reminder that the example
is invented and easy. The published net figures, quoted below, are a small fraction of a percent per
week, and they are the honest benchmark for what this rule is claimed to earn.

## What the research actually found

Three independent sources came to three different conclusions, and the disagreement is the finding.

De Groot, Huij and Zhou (2012) argue that the reason earlier studies found reversal profits vanishing
after costs is that those studies traded too much in small companies, where the gap between buying and
selling prices is wide. Restricting the universe to the largest companies cuts the trading cost, and
they report that reversal strategies can then earn 30 to 50 basis points per week net of trading
costs. They also report that the net profits are positive among large companies over the most recent
decade of their sample, a period of unusually high market liquidity.

Quantpedia summarises that paper as follows: the 100 largest companies, long the ten worst performers
of the previous week and short the ten best of the previous month, rebalanced weekly, returned 16.25
percent a year net over 1990 to 2009, with volatility of 14.94 percent, a worst fall of 52.94 percent
and a reward-to-risk ratio of 1.09. Its own description mixes a weekly and a monthly lookback in one
sentence, so treat the lookback as about one month, which is what the underlying paper uses.

QuantConnect then runs its own version on American data from 2016 to 2021 and reports that it loses to
the index. Its table shows the strategy with a reward-to-risk ratio of 0.287 against 0.754 for the
index fund over the five years, with roughly twice the variance. During the 2020 crash from February
to March the strategy did better than the index, with a ratio of -1.075 against -1.467, but during the
recovery from March to June it did far worse, 1.987 against 7.942. That is the pattern the literature
predicts: a strategy that supplies liquidity does well when others are forced to sell, and gives up
ground in a sharp rebound.

Two more pieces push the same way. Blitz, van der Grient and Honarvar (2023) report that the classic
short-term reversal effect has steadily weakened and has vanished in most regions, and that it can be
revived by countering its tendency to trade against short-term momentum in industries and factors.
Frazzini, Israel and Moskowitz, measuring real institutional trading costs on nearly a trillion
dollars of live trades, find short-term reversal to be the style most constrained by trading costs of
the four they test. Read together: the effect was real, it was small, it was concentrated where costs
are manageable, and it has since shrunk.

## How this project relates to it

This repository studies weekly contrarian rules directly in the brief
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md).
Two findings there are close enough to quote. A weekly contrarian portfolio on the S&P 500 that ranks
shares by how far they have recovered from a drawdown rather than by raw past return earned 0.0872
percent per week against 0.0420 percent for the plain version, and cut the worst fall from 72.56
percent to 43.99 percent (`1403.8125v4`, p.18). On the KOSPI 200 index the same recovery rule returned
0.146 percent per week against 0.073 percent (`1403.8125v4`, p.8). A second paper in the same brief
weighted past moves by trading volume and reached 0.2607 percent per week against 0.0685 percent on
the KOSPI 200 (`1208.2775v5`, p.14). All of those comparisons are before costs, which is exactly where
a weekly strategy is most at risk.

The study [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md) supplies
the sharper conceptual point: a reversal strategy needs negative dependence, meaning a tendency to
move back, and not merely the absence of a tendency to continue. Independent prices have no expected
reversal, so a contrarian rule built on them loses after costs just like a momentum rule would. The
tool [implementation/sector-regime-engine](../../../implementation/sector-regime-engine/README.md)
measures exactly that condition and refuses to admit a directional rule unless the dependence is both
statistically significant and large enough to clear its own costs.

## Where it goes wrong

- Costs rule the outcome. Trading the whole book every week means roughly a hundred round trips a
  year. At five basis points a side that is about 2.5 percent a year before anything else goes wrong,
  which is the same order as the published net profit. If your costs are higher than the paper's, the
  edge can disappear entirely.
- Small companies break it. The effect is often reported to be strongest in small, thinly traded
  shares, which are precisely the shares where the gap between buying and selling prices is widest.
  Restricting to large companies is what makes the rule survive costs, and that restriction is also
  what shrinks the raw effect.
- The short side is dangerous. Short selling requires borrowing shares, paying a fee, and sometimes
  being forced to return them at a bad moment. A share you sold short can rise without limit, a short
  squeeze can force you out at a loss, and some accounts simply cannot short at all.
- The effect has weakened. Independent work finds the plain version gone in most regions. A rule that
  worked on 1990 to 2009 data is not the same rule on 2016 to 2021 data, and the QuantConnect test
  illustrates the gap.
- It is not a hedge in a crash. The QuantConnect numbers show the strategy still lost money in
  February and March 2020, just less than the index. Reversal returns spike exactly when markets are
  most volatile, and the positions are hardest to hold then.
- A fall can be information. If a share fell because the company is genuinely in trouble, the price is
  not wrong, and it will not bounce. The rule cannot tell the two cases apart; it buys both.

## Try it yourself

You need nothing but a spreadsheet and a public source of daily prices.

1. Choose ten large, well-known companies and list them down the side.
2. Make one column per week, and put each company's closing price in the row and column of that week.
3. Add a column that computes the change over the past month: today's price divided by the price four
   weeks earlier, minus one.
4. Add a column naming the two companies with the smallest change and the two with the largest. Those
   are the ones the rule would buy and short this week.
5. In the next column over, write what each of those four did over the following week.
6. Repeat down the rows for six months.

What to notice: the biggest bounces cluster in the weeks after the whole market has fallen, when the
rule looks most wrong just before it helps. Then look at how often the same share appears on the buy
and the short list within a few weeks of itself. Every one of those flips is a pair of trades, and both
trades pay the spread.

## Where this came from

- [QuantConnect strategy library: short term reversal strategy in stocks](https://www.quantconnect.com/tutorials/strategy-library/short-term-reversal-strategy-in-stocks),
  the weekly rules, the 100-share universe, the 22-day lookback and the 2016 to 2021 performance
  table.
- [Quantpedia: short term reversal effect in stocks](https://quantpedia.com/strategies/short-term-reversal-in-stocks),
  the indicative performance, volatility, maximum fall, sample period, instrument count and the
  underlying paper.
- De Groot, Huij and Zhou, [Another Look at Trading Costs and Short-Term Reversal Profits](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1605049),
  the original study and the 30 to 50 basis points per week net figure.
- Nagel, [Evaporating Liquidity](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1971476),
  the argument that reversal returns are the fee for supplying liquidity.
- Blitz, van der Grient and Honarvar, [Reversing the Trend of Short-Term Reversal](https://ssrn.com/abstract=4575689),
  the finding that the classic effect has weakened and can be revived.
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's reading of the arXiv papers on recovery ranking and mass-weighted momentum,
  including `1403.8125v4` and `1208.2775v5`.

## Words used in this tutorial

- contrarian: doing the opposite of what the crowd just did, buying what fell and selling what rose.
- dollar volume: the number of shares traded multiplied by their price, a measure of how actively a
  share changes hands.
- liquidity: how easily something can be bought or sold without moving its price.
- momentum: the tendency of something that has been rising to keep rising for a while.
- overreaction: a price move that goes further than the news justifies.
- short selling: borrowing something you do not own, selling it, and buying it back later, gaining if
  the price falls and losing if it rises.
- spread: the gap between the price at which something can be bought and the price at which it can be
  sold.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
