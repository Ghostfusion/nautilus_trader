# Soccer clubs and their shares: shorting football clubs before big matches

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of European football clubs that are listed on a stock exchange, sold short around the days of their important matches                                                                                                                                         |
| How often it trades       | On the roughly forty days a year when the listed clubs play a major match                                                                                                                                                                                            |
| What you need             | A spreadsheet, a match calendar and daily share prices                                                                                                                                                                                                               |
| Where the rules come from | [The list's implementation file](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/soccer-clubs-stocks-arbitrage.py) and the [Quantpedia entry](https://quantpedia.com/strategies/soccer-clubs-stocks-arbitrage) it cites |
| The underlying research   | Bernile and Lyandres, [Understanding Investor Sentiment: The Case of Soccer](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1343685)                                                                                                                            |
| How well it held up       | Mixed: one six-year European sample with a large gross return and a volatility around 50 percent, but the edge is smaller than the gap between buying and selling prices in the small markets where these shares trade                                               |
| Also appears in           | Nothing else in this collection                                                                                                                                                                                                                                      |

## The idea in one paragraph

A handful of European football clubs are companies whose shares trade on a stock exchange. Fans and
small investors tend to be too optimistic before an important match and disappointed afterwards.
This strategy sells the club's shares short at the end of the business day before the match, holds
the position for one day, and buys the shares back after the result is known. Where several listed
clubs play on the same day, the money is spread equally across them. The bet is that a club's shares
fall on and just after match days often enough, and thinly traded markets move enough, for the
short sales to pay.

## Why anyone believed it

The strategy is described as an arbitrage, so that word needs unpacking. A true arbitrage is buying
something in one place and selling the identical thing in another at the same moment, so that the
profit is locked in and carries no risk. This rule is not quite that: the result of a football match
is not known in advance, so it cannot lock in a profit. It is closer to a one-sided bet on a
predictable mistake, and the paper's argument is that the mistake is systematic.

The mistake is optimism. The odds offered by bookmakers are set by a small group of specialists and
are close to the true chances, but the price of a club's shares is set by thousands of fans who
over-rate their own team's prospects. When the match ends, the fans' expectations are disappointed
more often than not, and the shares fall. The counterparty is therefore the fan who buys the shares
before the match because the team feels unstoppable, and who is reluctant to sell until the result
makes the loss obvious. Football shares also trade in tiny volumes, which is what allows a small
amount of disappointed selling to move the price.

## An everyday comparison

Think of a town shop that sells scarves in the club's colours. On the morning of a big match, everyone
is certain the team will win, and the shop can charge a high price because the queue is long. The
next morning, after a draw or a defeat, the same customers walk past without looking, and the shop
marks the scarves down to clear them. Selling the shares of the club before the match and buying them
back after it is selling the scarves at the optimistic price and buying them back at the disappointed
one. The shop's problem, and the strategy's, is that only so many scarves can be sold at once before
the price has to be cut.

## The rules, step by step

1. Choose the universe: the handful of European football clubs whose shares trade in enough volume
   to buy and sell. The implementation uses Futebol Clube do Porto, Sporting Clube de Portugal,
   Benfica, Lazio, AS Roma, Ajax, Juventus, Manchester United, Borussia Dortmund and Celtic.
2. Get the match calendar: which of those clubs play an important match, such as a Champions League
   game, on which dates.
3. At the end of the business day before the match, sell the club's shares short. Selling short means
   borrowing shares you do not own, selling them, and planning to buy them back later; if the price
   falls you keep the difference.
4. If several listed clubs have matches on the same day, sell each one short with an equal share of
   the money.
5. Hold for one day. The position covers the match and its immediate aftermath.
6. Rebuild every day: close whatever was open and open the short positions for the matches of that
   day.

A note on why the small exchanges matter. The clubs listed above trade in London, Lisbon, Amsterdam,
Rome, Milan and Frankfurt, but most of their shares are far less liquid than a large company's. Fewer
buyers and sellers, a wider gap between the buying and the selling price, and fewer shares available
to borrow are the defining features of the market this rule trades in.

## The maths, with every symbol named

The return of one short position over the match, from the buyer's point of view reversed:

```text
R_short = (P_before - P_after) / P_before
```

- `R_short` is the return kept by the short seller, written as a decimal: 0.025 means two and a half
  percent.
- `P_before` is the closing share price on the business day before the match.
- `P_after` is the closing price on the day of the match, when the buy-back happens.

If `P_after` is lower than `P_before`, `R_short` is positive: the shares were sold high and bought
back low. If the club's shares rise, `R_short` is negative. Where several clubs are shorted at once,
the return of the day is the plain average:

```text
R_day = (R_short for club 1 + R_short for club 2 + ... ) / number of clubs
```

- Each club gets an equal share of the money, so the day's return is the average of the individual
  short returns.

The annual figure scales that average by the number of match days:

```text
R_year = number of match days * average(R_day)
```

- The paper's sample has about forty match days a year, which is the multiplier used below.

Finally the cost, which is where a small market bites:

```text
Cost_day = 2 * c + borrow for one day
```

- `c` is the cost of one trade as a fraction of the amount traded, the gap between the buying and the
  selling price plus commission. For these shares a realistic figure is 0.005, fifty basis points,
  where one basis point is one hundredth of one percent.
- The `2` is because the position is opened and closed, so the gap is paid twice.
- `borrow for one day` is the rent on the borrowed shares, roughly the annual borrow rate divided by
  252 trading days. At five percent a year that is about 0.02 percent for one day.

## A worked example

Five match days, with invented but plausible share prices. The short return is the price before the
match divided into the fall since.

| Match day | Club           | Price before match | Price on match day | R_short |
| --------- | -------------- | ------------------ | ------------------ | ------- |
| 1         | Juventus       | 0.80               | 0.78               | +2.500% |
| 2         | Ajax           | 12.00              | 11.80              | +1.667% |
| 3         | Benfica        | 3.50               | 3.60               | -2.857% |
| 4         | Manchester Utd | 16.00              | 15.70              | +1.875% |
| 5         | Dortmund       | 5.00               | 4.95               | +1.000% |

For Juventus the calculation is (0.80 - 0.78) / 0.80 = 0.025, a gain of 2.5 percent on the short
position. For Benfica it is (3.50 - 3.60) / 3.50 = -0.02857, a loss of 2.857 percent.

The average short return across the five days is (2.500 + 1.667 - 2.857 + 1.875 + 1.000) / 5 =
4.185 / 5 = 0.837 percent per match day before costs. That is close to the paper's 0.88 percent a day.
At forty match days a year it would compound to `1.00837^40 - 1`, about 39 percent before costs.

Now the costs, under two assumptions:

| Assumption               | Cost per side | Two sides | Borrow for a day | Cost per day | Net per day |
| ------------------------ | ------------- | --------- | ---------------- | ------------ | ----------- |
| Optimistic, tight market | 0.05%         | 0.10%     | 0.02%            | 0.12%        | +0.717%     |
| Realistic, thin market   | 0.50%         | 1.00%     | 0.02%            | 1.02%        | -0.183%     |

Under the optimistic assumption the strategy earns a net 0.717 percent a day, about 33 percent a year
over forty match days. Under the realistic assumption for shares this small, the cost is 1.02 percent
a day and the edge is gone. That comparison, not the 39 percent headline, is the honest reading of
the strategy. The worked example says nothing about whether it works; it shows how quickly the cost of
trading in a small market can swallow a large-looking edge.

## What the research actually found

| Source                                                                                                             | What it measured                                                                 | Result                                                                                                                                                                                                                                                                                                                                      |
| ------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Bernile and Lyandres, Understanding Investor Sentiment                                                             | Returns of listed European football clubs around important matches, 2000 to 2006 | Investors were optimistic before matches and disappointed afterwards, and the shares earned negative abnormal returns after the games; the betting-exchange prices were used as the unbiased benchmark for what should have happened                                                                                                        |
| Quantpedia, summarising that paper                                                                                 | The short-selling return, 2000 to 2006                                           | About 42 percent a year, from a geometric daily return of 0.88 percent over an estimated forty match days, with volatility around 50 percent and a reward-to-risk of 0.76, taken from the paper's table 4                                                                                                                                   |
| This repository, [prediction markets and betting](../../../strategies/books2/21_prediction_markets_and_betting.md) | The cost of event-driven bets, across betting and prediction markets             | A spread bet pays 91 cents on the dollar, a 9-cent commission, and random spread betting returned about -4.4 percent (`1910.08858v2`, p.3, p.4); on a decentralized prediction market the median quoted spread was about 400 basis points in the middle of the price range and 1,300 to 1,800 basis points below 0.10 (`2604.24366v2`, p.7) |
| The list's replication record                                                                                      | 4,843 coded papers, each over its own full history                               | The median replication has a reward-to-risk of 0.37, 48 percent clear a t-statistic of 1.96, the median test window is 34 years, and the median carries a market exposure of +0.17                                                                                                                                                          |

Read together, the picture is this. The behavioural claim, that fans are too optimistic before a match
and that the shares give some of that back afterwards, is supported by one careful European sample.
The size of the prize, however, depends entirely on how cheaply the shares can be sold short, and the
repository's own brief on event-driven markets shows that the cost of taking such bets is large.

## How this project relates to it

The closest thing in this repository is the brief
[prediction markets and betting](../../../strategies/books2/21_prediction_markets_and_betting.md). It
is not about football shares, but it is where the same problem is measured: pricing an event, paying
the commission, and discovering that the toll is larger than the edge. Its figures above show that a
9-cent commission on a one-dollar bet is enough to turn random betting into a 4.4 percent loss, and
that on a small event market the quoted gap between buying and selling can run to hundreds of basis
points. Those are the numbers a reader should hold beside the 42 percent headline, because they are
the reason a real arbitrage and a real pattern are not the same thing.

## Where it goes wrong

- Arbitrage it is not. The result of the match is uncertain, so the position is a bet on behaviour,
  not a riskless trade, and a run of wins for the clubs being shorted can produce a large loss.
- Liquidity is the whole story. The edge is about 0.84 percent a day and the realistic gap between
  buying and selling prices for these shares is comparable, so most or all of the edge can be lost
  before the position is ever held.
- Borrowing may be impossible. To sell a share short you must borrow it first, and for a thinly
  traded football club there may be few shares to borrow and a high fee to pay. The names with the
  strongest signal are often the hardest to borrow.
- The market is small and the evidence is old. The sample covers 2000 to 2006 and a handful of clubs;
  leagues, listing venues and the behaviour of fans have all changed since.
- Concentration. On a day when several clubs play, the rule holds a few positions at once, and on a
  day when none play it holds nothing, so the returns arrive in bursts rather than evenly.
- The whole idea would be false if the pre-match optimism simply reflects the market's own estimate
  of the club's chances rather than a mistake, or if the average fan had learned to sell before the
  match. Comparing the share price to the bookmaker's odds, and re-checking on a recent sample, is
  what would settle it.

## Try it yourself

You need a spreadsheet, a public match calendar for one club, and its daily share prices. You do not
need any money.

1. Build a sheet with one row per trading day: the date and the closing share price.
2. Add a column marking the day before each important match.
3. Add a column with the return from the close before the match to the close on the match day.
4. Multiply every one of those returns by minus one, which is what a short seller earns.
5. Average that column, then subtract the cost of opening and closing the position, twice the gap
   between the buying and selling price.
6. Do the same for the day the club won and the day the club lost, in separate columns.

What to notice: the losses cluster on the days the club won, exactly as the optimism story predicts,
and the gains cluster after draws and defeats. Notice too how wide the gap between buying and selling
prices looks in the recorded prices, because if that gap is one percent and the average move is
smaller than one percent, the honest average after costs is a loss. If your sheet shows a large
profit, check whether you used the price on the match day itself, which a real short seller cannot,
because the short has to be placed before the match begins.

## Where this came from

- [The list's implementation file](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/soccer-clubs-stocks-arbitrage.py),
  the rules as coded: the list of clubs, the match calendar, the short sale before the match and the
  one-day hold.
- [Quantpedia: soccer clubs' stocks arbitrage](https://quantpedia.com/strategies/soccer-clubs-stocks-arbitrage),
  the indicative performance figures and the description of the effect.
- Bernile and Lyandres, [Understanding Investor Sentiment: The Case of Soccer](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1343685),
  the original study over 2000 to 2006.
- [Prediction markets and betting](../../../strategies/books2/21_prediction_markets_and_betting.md),
  this repository's brief, which supplies `1910.08858v2` and `2604.24366v2`.

## Words used in this tutorial

- abnormal return: the part of a return left after the market's own move is taken out.
- arbitrage: buying and selling the same thing at the same moment so that a profit is locked in with
  no risk; this rule is not that.
- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- borrow fee: the rent paid to borrow shares that have been sold short.
- liquidity: how easily something can be bought or sold without moving its price.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- spread: the gap between the price at which something can be bought and the price at which it can be
  sold.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
