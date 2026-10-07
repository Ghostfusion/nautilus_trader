# The liquidity effect in stocks: buying the shares that hardly ever change hands

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                   |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of small American companies, chosen by how little of each company changes hands over a year                                                                                                                                                                      |
| How often it trades       | About once a year, when the holding list is rebuilt                                                                                                                                                                                                                     |
| What you need             | A spreadsheet and a year of trading volumes and share counts for a list of small companies                                                                                                                                                                              |
| Where the rules come from | [QuantConnect strategy library, liquidity effect in stocks](https://www.quantconnect.com/tutorials/strategy-library/liquidity-effect-in-stocks) and the [Quantpedia entry](https://quantpedia.com/Screener/Details/18) it cites, which is available only to subscribers |
| The underlying research   | Datar, Naik and Radcliffe, [Liquidity and stock returns: an alternative test](https://ideas.repec.org/a/eee/finmar/v1y1998i2p203-219.html), 1998                                                                                                                        |
| How well it held up       | Mixed: the effect appears in several markets, but it lives in the smallest companies, its sign can flip among large ones, and much of the measured reward is the compensation for trading costs that a small investor would actually pay                                |
| Also appears in           | Nothing else in this collection; the closest is [Momentum effect in stocks](../momentum-effect-in-stocks/README.md), which ranks the same companies by past return rather than by how much they trade                                                                   |

## The idea in one paragraph

How much a company's shares change hands in a year is not the same as how big the company is. Some
small companies are traded heavily, with shares passing between owners many times a year; others
are traded hardly at all, with the same patient owners holding on. This strategy takes the smallest
quarter of a list of large American companies by market value, measures how much of each company
changes hands over the past year, and buys the ones that change hands least, while betting against
the ones that change hands most. The bet is that owners of a share that is hard to sell demand a
discount, so the least-traded shares tend to earn more over time.

## Why anyone believed it

Think about what it means to own something you cannot quickly sell. If you might need your money
soon, a share that few people trade is a worry: you can sell it only slowly, and probably at a
lower price than the last one on the screen. Investors dislike that worry, so they pay less for
such a share, and paying less means a higher return if the share pays off as expected. That extra
return is called the liquidity premium, and it is a reward for accepting something hard to sell.

The counterparty, then, is the owner who needs cash now and must sell something nobody wants, and
the buyer who refuses to hold illiquid shares at all. Both leave a gap that a patient buyer can
fill, and both have reasons that will come back, so the discount does not close permanently.

## An everyday comparison

Think of selling two bicycles. One is a popular model that everyone wants: it sells in a day, close
to the asking price. The other is an unusual model that few people recognise: months pass before
anyone is interested, and the only offer is well below the asking price. The second bicycle is not
worse than the first; it is simply harder to sell. A buyer who is happy to be patient can pay the
low price and later enjoy the difference. The strategy here is to be that patient buyer, over and
over, in the shares that other people find awkward to trade.

## The rules, step by step

1. Start with the largest 1,500 American companies by market value, where market value is the share
   price multiplied by the number of shares the company has issued.
2. Remove any share priced below 5.00, because a very low price usually means a company in trouble
   rather than a bargain. Remove funds, because the strategy is about companies. Remove any company
   worth less than 10 million in total, because the smallest ones are too small to matter.
3. Keep the smallest quarter of what is left by market value. This is the group where the effect is
   said to be strongest, and it is the group the library page uses.
4. For each company in that quarter, work out its turnover: the number of shares that changed hands
   over the past year, divided by the number of shares the company has issued. A company with 50
   million shares, of which 2.5 million changed hands, has a turnover of 0.05, meaning 5 percent of
   the company changed hands in the year.
5. Rank the companies by turnover, lowest first.
6. Buy the lowest-turnover group, the bottom 5 percent, and sell short the highest-turnover group,
   the top 5 percent. To sell short is to borrow a share you do not own, sell it, and buy it back
   later, hoping to pay less than you received.
7. Give each holding the same amount of money, so that half the money is held long and half is bet
   against the market.
8. Rebuild the list once a year, repeating steps 1 to 6.

Turnover is the key quantity, so it is worth saying in plain words what it measures. It is the
fraction of the company that changed hands: a turnover of 0.05 means one twentieth of all the
company's shares moved from one owner to another during the year, while a turnover of 1.0 means that,
counting every trade, an amount of shares equal to the whole company changed hands.

## The maths, with every symbol named

The strategy is one calculation repeated for each company, one sort, and a cost line.

Turnover:

```text
T = V_year / S_outstanding
```

- `T` is the turnover of the company over the year, as a decimal: 0.05 means 5 percent.
- `V_year` is the number of shares of the company that changed hands during the year.
- `S_outstanding` is the number of shares the company has issued.

An alternative measure of the same idea, used in the academic literature, is the Amihud illiquidity
figure: the average, over the year, of each day's absolute price change divided by that day's traded
value. A large value means the price moves a lot for little trading, which is another way of saying
the share is hard to trade. The library page uses turnover, so this tutorial does the same.

Rank the companies by `T`, smallest first, and give the holdings equal but opposite weights:

```text
w_i = +1 / N for each of the N lowest-turnover companies
w_i = -1 / M for each of the M highest-turnover companies
```

- `w_i` is the fraction of the money placed in company `i`; a plus sign means bought, a minus sign
  means sold short.
- `N` is the number of low-turnover companies bought and `M` the number sold short. In the worked
  example below, `N` and `M` are both one, so each weight is 0.5 in size.
- The longs and the shorts are equal in total size, half the money on each side, so the portfolio
  is not a bet on the market rising or falling.

The portfolio's return over the holding year:

```text
R_portfolio = (w_1 * R_1) + (w_2 * R_2) + ... + (w_i * R_i)
```

- `R_i` is the return of company `i` over the year. For a short position a fall in price produces a
  positive contribution, because the weight is negative and so is the return.

The cost of rebuilding each year:

```text
Cost = t * c
```

- `t` is the traded fraction: 2.0 when the whole portfolio is sold and replaced at once, because
  each sale and each purchase counts as one trade.
- `c` is the cost per trade as a fraction of the amount traded, covering the gap between the buying
  and selling price, the effect of your own order on the price, and commission. For the small
  companies this rule selects, `c` is not the tiny 0.0005 used for large shares; a realistic figure
  is 0.005 to 0.02, that is 0.5 to 2 percent, because very small shares are expensive to trade.

## A worked example

The table below uses a made-up list of six small companies. The returns are invented, but the
share counts and trading volumes are of the size small companies actually have.

| Company | Shares issued (millions) | Shares traded in the year (millions) | Turnover | Rank |
| ------- | ------------------------ | ------------------------------------ | -------- | ---- |
| Echo    | 50.0                     | 2.5                                  | 0.05     | 1    |
| Alpha   | 20.0                     | 2.0                                  | 0.10     | 2    |
| Charlie | 10.0                     | 1.5                                  | 0.15     | 3    |
| Bravo   | 40.0                     | 12.0                                 | 0.30     | 4    |
| Foxtrot | 15.0                     | 9.0                                  | 0.60     | 5    |
| Delta   | 25.0                     | 25.0                                 | 1.00     | 6    |

The rule buys the lowest-turnover company and sells short the highest-turnover company, half the
money on each side. So Echo is bought and Delta is sold short. Now suppose the next year produces
these returns:

| Company held  | Weight | Next-year return | Contribution   |
| ------------- | ------ | ---------------- | -------------- |
| Echo (buy)    | +0.50  | +25 percent      | +12.50 percent |
| Delta (short) | -0.50  | +8 percent       | -4.00 percent  |
| Total         | 0.00   |                  | +8.50 percent  |

So the portfolio gained 8.50 percent before costs. The rule earns half the 17-point gap between the
low-turnover company's +25 percent and the high-turnover company's +8 percent, because only half the
money is on the long side.

The list turns over completely each year, so the whole portfolio is traded on both sides:

```text
t = 2.0
Trading cost at 0.10 percent per side = 2.0 * 0.001 = 0.002, that is 0.20 percent
Trading cost at 1.00 percent per side = 2.0 * 0.010 = 0.020, that is 2.00 percent
Net return at the low cost = 8.50 - 0.20 = 8.30 percent
Net return at the realistic small-share cost = 8.50 - 2.00 = 6.50 percent
```

Two things are worth noticing. First, the cost line is not a detail. For shares this small, the
difference between the two cost assumptions is almost a quarter of the whole return, and the borrow
fee on the short side would remove a little more. Second, the example says nothing about whether
the strategy works. It only shows how to apply the rules and how the arithmetic behaves.

## What the research actually found

| Source                                       | What it measured                                                              | Result                                                                                                                                                                |
| -------------------------------------------- | ----------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Datar, Naik and Radcliffe, the source paper  | American shares, ranked by turnover, over several decades                     | Shares with low turnover earned higher returns than shares with high turnover; the effect was concentrated in the smaller companies                                   |
| Amihud, on illiquidity                       | American shares, ranked by how much the price moved for each unit of trading  | Less liquid shares earned higher average returns; the reward is read as payment for holding something hard to sell                                                    |
| Quantpedia, the entry the library page cites | The same rule, low-turnover shares bought and high-turnover shares sold short | The entry sits behind a subscription and its indicative performance could not be read, so this tutorial does not quote a figure from it                               |
| QuantConnect's own page                      | The lowest market-value quarter of the 1,500 largest American companies       | States that low-liquidity shares earn higher returns than high-liquidity ones and that the effect is strongest among small companies                                  |
| Choi, Choi and Kang, on Korean shares        | Turnover-based portfolios on the KOSPI 200, 2000 to 2011, with costs          | Low-liquidity shares beat high-liquidity ones by about 1.4 to 1.9 percent a month even after 35 basis points per basket (p.16); the result was steadier than momentum |
| The same paper, on large companies only      | The same ranking applied to the largest companies                             | The sign flips: among the largest shares, the heavily traded ones did better, so the effect is not present everywhere (p.16)                                          |

Read together, the picture is this. There is a real tendency for the least-traded shares to earn
more over time, and it has been found in more than one market, which is why the grade is Mixed
rather than Weak. But it lives in the smallest companies, its sign turns around among large ones,
and a large part of the reward is the compensation for a trading cost that the strategy itself must
pay. It is more accurate to call it a payment for holding an awkward asset than a free profit.

## How this project relates to it

This repository's closest material is about the cost of trading, which is the whole point of this
strategy. The brief [Market Impact and Trading Cost](../../../strategies/books/02_market_impact_and_trading_cost.md)
records that the price impact of an order grows roughly with the square root of its size, that it is
larger for shares that trade less, and that impact is transient, meaning a price pushed by an order
partly comes back. That is the mechanism by which the paper reward of the liquidity effect is paid
to the small investor who trades.

The second piece is the brief
[the microstructure-noise brief](../../../strategies/books2/14_volatility_and_microstructure_noise.md),
which covers the lead that a liquidity premium may sit behind extreme price moves and jump measures,
and it is careful to describe those liquidity measures as leads rather than settled effects.

## Where it goes wrong

- Costs and impact are the reward. The least-traded shares are the most expensive to buy and sell.
  The premium that a large diversified fund can collect is largely the fee that a small investor
  pays, so the effect is easy to describe and hard to capture.
- Turnover measures two things at once. Trading volume counts both buying and selling, and a single
  large owner selling out one year can make a quiet company look busy. Share counts also change
  over time as companies issue or buy back shares, so a fixed turnover figure has to be rebuilt
  with care.
- It is a small-company effect. If the smallest quarter is replaced by large companies, the effect
  weakens sharply or turns around, which means the strategy is really a small-company strategy
  wearing a liquidity label.
- Shorting the busy shares. The heavily traded shares are often the ones with news, and their prices
  can jump upward, which is exactly what hurts a short position.
- Survivorship and delisting. Very small companies fail, are taken over or are delisted. A test
  that keeps only the companies that still exist flatters the result, because the failures are the
  ones a low-turnover buyer was most likely to have owned.
- Limited room. The group is a tiny slice of the market, and trading it in any size moves the very
  price the strategy depends on.

## Try it yourself

You need nothing but a spreadsheet and a public source of share prices, trading volumes and share
counts; a finance website will give all three for a list of small companies.

1. Build a sheet with one row per company and these columns: Shares issued, Shares traded in the
   year, Turnover, and Next-year return.
2. Fill turnover as shares traded divided by shares issued.
3. Sort the rows by turnover, smallest first.
4. Split the list into the lowest-turnover quarter and the highest-turnover quarter and average the
   next-year return of each group.
5. Subtract the high-turnover average from the low-turnover average. That difference is what the
   rule tries to capture.
6. Subtract a cost of 1 percent for each side of every position that changed since the previous
   year.

What to notice: the gap between the two groups is usually small, and it is often wiped out by the
cost line once you assume a realistic small-share cost rather than the tiny cost of a large share.
If your sheet shows the low-turnover group winning by a wide margin, check whether the list includes
companies that later failed or were taken over; leaving them out is the most common way this
exercise flatters itself.

## Where this came from

- [QuantConnect strategy library: liquidity effect in stocks](https://www.quantconnect.com/tutorials/strategy-library/liquidity-effect-in-stocks),
  the rules as implemented: the 1,500 largest companies, the lowest market-value quarter, turnover
  measured as annual volume divided by shares issued, the 5 percent tails and the annual rebuild.
- [Quantpedia's entry for the liquidity effect](https://quantpedia.com/Screener/Details/18), cited
  by the library page; it is a subscription page and its figures could not be read for this
  tutorial.
- Datar, Naik and Radcliffe, [Liquidity and stock returns: an alternative test](https://ideas.repec.org/a/eee/finmar/v1y1998i2p203-219.html),
  the source paper behind the turnover version of the rule.
- Amihud, on illiquidity and stock returns, the study that gave the price-movement-to-trading
  measure of illiquidity.
- Choi, Choi and Kang, arXiv `1211.6517v1`, the turnover portfolios, the monthly returns after
  costs, and the reversal of the effect among large companies.
- [Market Impact and Trading Cost](../../../strategies/books/02_market_impact_and_trading_cost.md),
  this repository's brief on the square-root law of price impact and why small shares cost more to
  trade.
- [the microstructure-noise brief](../../../strategies/books2/14_volatility_and_microstructure_noise.md),
  this repository's brief that treats the liquidity premium as a lead rather than a settled effect.

## Words used in this tutorial

- liquidity: how easily something can be bought or sold quickly without moving its price much.
- market capitalisation: the total value of a company's shares, found by multiplying the share price
  by the number of shares.
- spread: the gap between the best buy price and the best sell price, which a trader crosses on each
  round trip.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- turnover: how much of a company's shares change hands over a period, and separately, how often a
  portfolio's holdings are replaced.
- universe: the full set of things a rule is allowed to choose from.
- volume: the number of shares traded in a period, used as a sign of how much interest there is.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
