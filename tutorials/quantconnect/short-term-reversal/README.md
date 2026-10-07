# Short-term reversal: buying last month's worst shares

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                      |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Two baskets of American shares at once: last month's worst performers are bought, and last month's best are sold short                                                                                                                                     |
| How often it trades       | About once a month, when the whole book is rebuilt                                                                                                                                                                                                         |
| What you need             | A spreadsheet and one year of monthly closing prices for the shares you want to rank                                                                                                                                                                       |
| Where the rules come from | [QuantConnect strategy library, short term reversal](https://www.quantconnect.com/tutorials/strategy-library/short-term-reversal) and the [Quantpedia short-term reversal entry](https://quantpedia.com/strategies/short-term-reversal-in-stocks) it cites |
| The underlying research   | Groot, Huij and Zhou, [Another Look at Trading Costs and Short-Term Reversal Profits](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1605049), with Nagel on the liquidity-provision reading                                                          |
| How well it held up       | Mixed: the effect is measured reliably in large shares and survives careful costs in one study, but another line of work finds it has steadily weakened and in recent periods all but vanished in most markets, so the honest answer depends on the sample |
| Also appears in           | [Telling which kind of market you are in](../../project/measuring-the-regime/README.md) in this collection, which measures whether rankings persist or reverse, the question this strategy answers in the negative                                         |

## The idea in one paragraph

In any month, some shares do much better than others. Some of the biggest movers moved for a reason
that will last, and some moved because a few large buyers or sellers were pushing the price around
that week. This strategy bets on the second kind. It ranks a list of large American shares by how they
did over the past month, buys the ten that did worst, and sells short the ten that did best, putting
the same amount of money on each side. Once a month the whole book is thrown away and rebuilt from the
new ranking. The bet is that a share which fell too far for a temporary reason tends to bounce back,
and one that rose too far tends to give some of it back.

## Why anyone believed it

When a big investor needs to sell a large holding quickly, the price has to fall to attract enough
buyers to take the other side. That fall is not a judgement about the company; it is the price of
speed. The same happens in reverse for a buyer in a hurry. The share that was pushed down is then
cheap relative to its twin, and once the hurried seller is finished, the price drifts back.

The counterparty is therefore the impatient trader, and the reversal strategy is paid for supplying
the willingness to trade with them at the moment they most need it. This is why researchers call
short-term reversal a reward for providing liquidity: the strategy is on the other side of somebody
else's urgency, and it earns a fee for that service, the way a shop earns by holding stock for a
customer who needs it today. A second, related story is overreaction: people read too much into the
last piece of news, and the price corrects once the excitement fades.

## An everyday comparison

A hall is auctioning identical chairs. Lot after lot sells for about the same price, because the same
crowd is bidding each time. The auctioneer reaches one lot at the exact moment two of the regular
bidders step out for coffee. The chairs sell cheaply to whoever is left. Nothing about the chairs
changed; the price did, because the demand was temporarily thin. When the next identical lot comes up
with everyone back in the room, it sells at the normal price again. The reversal strategy is the buyer
who took the cheap lot, and it is paid because it was willing to buy when others had left the room.

## The rules, step by step

1. Choose the shares to rank. The QuantConnect page screens out any share priced below 4 dollars,
   keeps the 100 with the most dollars traded, and from those keeps the 20 with the largest total
   market value. The Quantpedia page uses the 100 biggest companies by market value.
2. For each share, collect its closing price today and its closing price one month ago.
3. Compute the past-month return: today's price divided by the price a month ago, minus one. A share
   that went from 40.00 to 46.00 has a past-month return of 15 percent.
4. Rank the shares by that return, best first.
5. Sell short the ten best performers and buy the ten worst performers. Put the same amount of money on
   the long side as on the short side, so the account is half long and half short.
6. Hold for one month.
7. At the start of the next month, recompute every share's past-month return and rebuild the whole
   book from the new ranking, closing everything that is no longer in the list.
8. Repeat from step 4.

Note that the two sources differ on the rhythm. The QuantConnect page rebalances monthly and ranks on
the past month. The Quantpedia page describes a weekly rebalance that buys last week's worst and sells
short last month's best, which is a faster version of the same idea. This tutorial follows the
monthly version.

## The maths, with every symbol named

The past-month return of a share:

```text
r_i = ( P_i,today / P_i,1m_ago ) - 1
```

- `r_i` is share `i`'s return over the past month, as a decimal.
- `P_i,today` is its closing price today.
- `P_i,1m_ago` is its closing price one month earlier.

Rank the shares by `r_i` and take the ten lowest and the ten highest. Give each a weight, with the two
sides carrying equal money:

```text
w_i = +1/20 for each of the ten worst, and -1/20 for each of the ten best
```

- `w_i` is the fraction of the account committed to share `i`.
- A positive weight means buying the share; a negative weight means selling it short.
- The positive weights add to +1/2 and the negative weights to -1/2, so half the account is long and
  half is short and a move in the market as a whole very nearly cancels between the two sides.

The portfolio's return over the following month is the weighted sum of the shares' returns:

```text
R_portfolio = sum over i of ( w_i * R_i )
```

- `R_i` is the following month's return of share `i`, as a decimal.
- `sum over i` means add the twenty contributions together.
- Because the weights are equal and opposite, this equals half the average return of the ten worst
  minus half the average return of the ten best.

Finally the cost, which matters more here than almost anywhere because the whole book turns over:

```text
Cost = t * c + borrow
```

- `t` is the traded fraction. Rebuilding the whole book means selling one account's worth and buying
  another, so `t = 2.0`.
- `c` is the cost per trade as a fraction traded, covering the gap between the buying and selling
  price. Large American shares cost about 0.001, that is ten basis points, where one basis point is
  one hundredth of one percent.
- `borrow` is the fee paid to the lender for as long as each short position is open, which for a
  monthly book is roughly the annual borrow rate divided by twelve.

## A worked example

Six shares, ranked by their return over the past month. Six is a stand-in for twenty: the rules buy
one fifth of the list and sell short one fifth, so with six shares the book holds two on each side.

| Share | Past-month return | Rank | Position |
| ----- | ----------------- | ---- | -------- |
| A     | +18 percent       | 1    | short    |
| B     | +12 percent       | 2    | short    |
| C     | +5 percent        | 3    | none     |
| D     | -3 percent        | 4    | long     |
| E     | -9 percent        | 5    | long     |
| F     | -15 percent       | 6    | none     |

A and B are sold short; D and E are bought. Inside a 100,000 account the long side holds 50,000, split
evenly, and the short side holds 50,000, split evenly. Now suppose the next month brings these returns:

| Share   | Weight  | Next-month return | Contribution    |
| ------- | ------- | ----------------- | --------------- |
| D long  | +0.2500 | +1.5 percent      | +0.3750 percent |
| E long  | +0.2500 | +2.0 percent      | +0.5000 percent |
| A short | -0.2500 | -1.0 percent      | +0.2500 percent |
| B short | -0.2500 | -0.5 percent      | +0.1250 percent |
| Total   | 0.0000  |                   | +1.2500 percent |

The short positions gain when the shares fall, which is why the negative weights turn a fall into a
positive contribution. The book gained 1.2500 percent before costs. Now the costs, assuming the entire
book is rebuilt from a fresh ranking:

```text
t = 2.0
Cost = 2.0 * 0.001 = 0.002, that is 0.20 percent
Net return for the month = 1.2500 - 0.20 = 1.05 percent
```

The borrow fee would come off on top of that: selling 50,000 of shares short for one month at an
annual borrow rate of 0.5 percent costs about 50,000 * 0.005 / 12 = 20.83, which is 0.02 percent of the
account. Twelve months like this, compounded, is about 16.1 percent a year before costs and about 13.4
percent after. Two things are worth noticing. The cost line is large because the turnover is the
heaviest of any of the ideas in this library: buying last month's losers and selling last month's
winners means almost nothing carries over. And the worked example says nothing about whether the
strategy works; it only shows how to apply the rules and how the arithmetic behaves.

## What the research actually found

| Source                             | What it measured                                                      | Result                                                                                                                                                                                                                                             |
| ---------------------------------- | --------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Groot, Huij and Zhou               | US shares, 1990 to 2009, and the effect of trading costs              | Most of the measured profit is destroyed by trading small, expensive shares; limiting the list to large companies and lowering turnover produces 30 to 50 basis points a week net of costs, which is a serious challenge to standard asset pricing |
| Quantpedia, short-term reversal    | The 100 largest US companies, weekly rebalance, 1990 to 2009          | 16.25 percent a year, volatility 14.94 percent, worst fall 52.94 percent, reward-to-risk 1.09, confidence rated strong; the underlying paper's own net figure is 0.29 percent a week for the 100 largest stocks with weekly rebalancing            |
| QuantConnect algorithm             | The 20 largest liquid US shares, past-month return, monthly rebuild   | Long the 10 worst and short the 10 best of the prior month                                                                                                                                                                                         |
| Frazzini, Israel and Moskowitz     | Nearly a trillion dollars of real institutional trading, 1998 to 2011 | Real trading costs are less than a tenth of what earlier studies assumed; among size, value, momentum and reversal, reversal is the most constrained by trading costs                                                                              |
| Nagel, "Evaporating Liquidity"     | The returns of short-term reversal as a proxy for liquidity supply    | Reversal returns spike exactly when liquidity is scarce; they are highly predictable with the market's own volatility index, which says the profit is a payment for supplying liquidity in bad times                                               |
| Blitz, van der Grient and Honarvar | Recent decades, across regions                                        | The classic effect has weakened steadily and by their sample has all but vanished in most regions; it can be revived by countering its tendency to fight short-term industry and factor trends, which doubles the risk-adjusted result             |
| Miwa                               | Intraday versus overnight price moves                                 | The reversal comes from past intraday moves, not overnight ones, and is stronger in less liquid shares and in volatile markets, which supports the liquidity-provision story over a news story                                                     |

The disagreement is sharp and it is about the trend. One account finds the effect alive and profitable
in large shares once costs are measured properly; another finds it has decayed to nothing in recent
periods and needs to be rebuilt to work at all. Both are credible, and they are not looking at
identical samples or identical costs, so a reader should treat the size of the effect as unknown and
the direction as agreed. There is also a real oddity: a strategy that is described as reliable can
still have a worst fall of about 53 percent, because it fails exactly when markets are in turmoil, at
the moment its liquidity service is most needed and least rewarded.

## How this project relates to it

This repository's own study of the persistence question is
[Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md). Its Section 5
collects the reversal evidence and draws the line that matters here: a stock-level reversal result
does not automatically transfer to baskets of shares, because averaging a group removes much of the
effect. It reports Hameed and Mian's finding of pervasive intra-industry monthly reversals, stronger
after market declines and in volatile periods, and an industry-adjusted stock reversal of about 0.53
percent a month with a reward-to-risk near 0.74. Its Section 7.1 then reports a pre-registered test on
2010 to 2026 sector data that froze its rules before reading the data and found none of 24 cells
confirmed, which is the same warning from the other direction.

The sibling tutorial
[Telling which kind of market you are in](../../project/measuring-the-regime/README.md) turns the
question into a measurement: it builds the average correlation between one period's ranking and the
next, with a test attached, so a reader can see from the data whether rankings persist or reverse
rather than assuming either.

[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md)
reports the supporting result: ranking on maximum drawdown and recovery, rather than on raw cumulative
return, beat the plain contrarian rule on the S&P 500, returning 0.0872 percent a week against 0.0420
percent and cutting the worst fall from 72.56 percent to 43.99 percent (`1403.8125v4`, p.18).

## Where it goes wrong

- Turnover eats the edge. Buying last month's losers and selling last month's winners means the book
  is replaced almost entirely every month, so it pays the full cost twice a month, and the cost is the
  same order as the signal.
- The universe is a trap. The effect is strongest in small, thinly traded shares, which are exactly
  where costs and the gap between buy and sell prices are largest. Restricting to large shares makes
  the costs survivable and shrinks the effect at the same time.
- It is short the crowd's favourites. Selling short the month's biggest winners means the book is
  exposed to a continued rally in exactly those shares, and a sharp market-wide rally can hurt badly.
- It fails in a crisis, precisely when it is supposed to be a hedge. The worst fall of about 53 percent
  is the evidence, and it occurs because the strategy's gains depend on liquidity returning.
- The effect has decayed. One line of research finds it all but gone in recent periods across most
  regions, which is what happens to a published, easy-to-implement pattern.
- The ranking is a choice. One month, one week, skip the most recent week, large shares only: each
  variation changes the answer. Picking the version that worked best in the past is the classic way
  this strategy is made to look better than it is.

## Try it yourself

You need a spreadsheet and monthly closing prices for about ten large shares, from any finance
website.

1. Make one column per share and one row per month for two years, holding each month's closing price.
2. Add a row for each month's return: this month's price divided by last month's, minus one.
3. For each month, write the two shares with the lowest return, and the two with the highest. That is
   the book that month: buy the two lows, sell short the two highs.
4. In the next row, write the following month's return for those four shares, taken from your return
   table.
5. Average the two lows and the two highs. The strategy's return for the month is half the first
   average minus half the second.
6. Repeat for every month. Keep a running product of `( 1 + return )` rather than a sum, so the result
   compounds, and subtract 0.20 percent every month for the rebuild.

What to notice: count the months where the book made money and the months where it lost, and look
specifically at the worst two months. The good months are usually numerous but small, and the bad
months are few but large, because the strategy loses when a market-wide rally runs through the shares
it sold short. If your sheet shows steady gains with no large fall, the sample has simply not included
a crisis.

## Where this came from

- [QuantConnect strategy library: short term reversal](https://www.quantconnect.com/tutorials/strategy-library/short-term-reversal),
  the rules as implemented: the 20 largest liquid shares, long the 10 worst and short the 10 best by
  prior-month return, rebuilt monthly.
- [Quantpedia: short term reversal effect in stocks](https://quantpedia.com/strategies/short-term-reversal-in-stocks),
  its restated rules, the 1990 to 2009 figures, and the source and other papers, including Groot, Huij
  and Zhou, Frazzini, Israel and Moskowitz, Nagel, Miwa, and Blitz, van der Grient and Honarvar.
- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), Sections 5 and
  7.1, this repository's own study of reversal and its pre-registered test.
- [Telling which kind of market you are in](../../project/measuring-the-regime/README.md), the sibling
  tutorial that measures rank persistence directly.
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's brief, Section 4, where the drawdown and recovery contrarian result lives.
- The arXiv identifier `1403.8125v4` is held in the local corpus and was read through that brief.

## Words used in this tutorial

- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- borrow fee: the rent paid to the lender of something you have sold short.
- liquidity: how easily something can be bought or sold without moving its price.
- long: owning something, and gaining when its price rises.
- market value: the price of one share multiplied by the number of shares a company has issued.
- reversal: the tendency of something that has just moved far to move back a little.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- turnover: how much of the book is replaced at each rebuild.
- weight: the share of your money placed in one holding.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
