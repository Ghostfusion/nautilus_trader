# Momentum and turnover: buying winners that trade heavily, shorting losers that trade heavily

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                               |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of American companies listed on the New York and Nasdaq exchanges, bought and sold short                                                                                                                                                                                     |
| How often it trades       | Once a month: each holding is kept for three months, so a third of the book is refreshed each month                                                                                                                                                                                 |
| What you need             | A spreadsheet, prices, daily trading volume and the number of shares on issue                                                                                                                                                                                                       |
| Where the rules come from | [QuantConnect strategy library, combining momentum effect with volume](https://www.quantconnect.com/tutorials/strategy-library/combining-momentum-effect-with-volume) and the [Quantpedia entry](https://quantpedia.com/strategies/combining-momentum-effect-with-volume/) it cites |
| The underlying research   | Charles Lee and Bhaskaran Swaminathan, [Price Momentum and Trading Volume](https://onlinelibrary.wiley.com/doi/10.1111/0022-1082.00280) (Journal of Finance, 2000)                                                                                                                  |
| How well it held up       | Mixed: the pattern is clear and reproducible in the original American sample from 1965 to 1995, but an independent replication on 1996 to 2024 finds every part of it keeping its sign yet losing statistical significance, and momentum itself has been weaker since the 1990s     |
| Also appears in           | nothing else in this collection                                                                                                                                                                                                                                                     |

## The idea in one paragraph

Take every share listed on the two big American exchanges. For each one, measure how much its price
rose over the past year, and measure how heavily it is traded as a fraction of its size. Start with
the shares that rose the most and, among them, buy the ones that trade the most heavily. Then take the
shares that fell the most and, among them, sell short the ones that trade the most heavily. Hold all
of these for three months, refreshing a third of the book each month so that the whole portfolio
turns over smoothly rather than all at once. The bet is that a share which has risen and is being
traded heavily is a share that many people are excited about, and that the excitement carries the
price a little further before it fades.

## Why anyone believed it

The plain momentum story is that news spreads slowly. When a company's results improve, some
investors understand at once and buy, others take weeks to notice, and their late buying pushes the
price up a bit more. Trading volume is a way of seeing that process happen. A share that is being
traded much more heavily than usual is a share that many people are currently paying attention to, so
it is the share where the slow-arriving buyers are most likely to keep pushing on the price. In that
view, volume is not a separate signal; it is a measure of how many people are still in the middle of
reacting.

The counterparty is the investor who sells too early: someone who takes a quick profit after a jump,
or a value-minded investor who refuses to pay more than they think the company is worth and keeps
selling into the rise. On the losing side, the counterparty is the investor who holds on to a
falling, heavily traded share, refusing to accept the loss while more and more people trade out of
it.

## An everyday comparison

Think of a second-hand car market where some models sell quickly and others sit for months. A car
that has been selling above its usual price for a year, and that is turning over quickly on the lot,
is one that many buyers are chasing today. A car that is cheap and barely traded is one nobody has
got round to loving yet. This strategy deliberately prefers the crowded lot: it buys the models that
have risen and are changing hands fastest, and bets against the ones that have fallen and are still
being dumped in large numbers.

## The rules, step by step

1. Start with every ordinary share listed on the New York Stock Exchange and Nasdaq, excluding funds
   and other special structures, and excluding any share without the fundamental data needed for the
   turnover measure.
2. For each share, compute its return over the past twelve months: take today's price, divide by the
   price twelve months ago, and subtract one.
3. For each share, compute its turnover: the number of shares traded each day divided by the number
   of shares the company has on issue. Average that over the same twelve months, so turnover is a
   percentage per day. A share trading 1.2 percent of itself every day is heavily traded; one trading
   0.2 percent a day is quiet.
4. Sort the shares by their twelve-month return, best first. Take the top fifth as the winner group
   and the bottom fifth as the loser group.
5. Within the winner group, sort by turnover and take the most heavily traded shares. Within the loser
   group, do the same. The library page takes the top one percent of each group by turnover, which on
   a large universe is a handful of names.
6. Buy the chosen winners in equal amounts, and sell short the chosen losers in equal amounts, so the
   money bet long and the money bet short are the same size.
7. Hold each name for three months. Refresh one third of the book each month, so that on any given
   month the oldest third is sold and a new third is bought.
8. No stop losses and no second look during the three months. Names leave only when their turn in the
   three-month rotation comes up.

## The maths, with every symbol named

Two scores decide which shares are bought and which are sold short.

The momentum score, which is the price return over the past year:

```text
M = P_today / P_twelve_months_ago - 1
```

- `M` is the momentum score, as a decimal: 0.45 means the share rose 45 percent over the year.
- `P_today` is the share's price today.
- `P_twelve_months_ago` is its price on the same day a year earlier. Prices must be adjusted for
  splits and dividends, or the score is meaningless.

The turnover score, which is how fast the shares change hands:

```text
T = (shares traded per day) / (shares on issue)
```

- `T` is the turnover, usually written as a percentage per day: 0.012, or 1.2 percent, means that in
  an average day a little over one percent of the company's shares changed hands.
- `shares traded per day` is the daily trading volume, averaged over the same twelve months.
- `shares on issue` is the total number of the company's shares, so dividing puts small and large
  companies on the same footing.

The portfolio's return in a period is the average return of the shares held long minus the average
return of the shares held short, because the two sides are the same size:

```text
R = (r_long1 + r_long2 + ...) / N_long - (r_short1 + r_short2 + ...) / N_short
```

- `r_long1`, `r_long2`, and so on are the returns of the shares bought, as decimals.
- `r_short1`, `r_short2`, and so on are the returns of the shares sold short; subtracting them means
  a fall in those shares helps the portfolio.
- `N_long` and `N_short` are the numbers of shares on each side; dividing averages each side.

The cost each month is charged on whatever is traded:

```text
Cost = t * c
```

- `t` is the fraction of the book traded that month, counted on both sides: selling a holding and
  buying a replacement counts twice.
- `c` is the cost per trade as a fraction of the amount traded: the gap between the buying and selling
  price plus commission. For heavily traded American shares a realistic figure is 0.0005 to 0.001,
  that is five to ten basis points, where one basis point is one hundredth of one percent. Shorting
  adds a borrow fee on the shorted names.

## A worked example

Ten made-up shares, with a twelve-month return and an average daily turnover. The winner group is the
top forty percent by return and the loser group the bottom forty percent; within each group the two
most heavily traded names are chosen.

| Share | Twelve-month return | Turnover (percent a day) | Group  | Chosen?                |
| ----- | ------------------- | ------------------------ | ------ | ---------------------- |
| A     | +45 percent         | 1.2                      | winner | long, heaviest traded  |
| B     | +38 percent         | 0.4                      | winner | no, quieter            |
| C     | +30 percent         | 0.9                      | winner | long, second heaviest  |
| D     | +25 percent         | 0.2                      | winner | no, quietest           |
| E     | +10 percent         | 0.6                      | middle | no                     |
| F     | +2 percent          | 0.3                      | middle | no                     |
| G     | -5 percent          | 0.7                      | loser  | no, second quietest    |
| H     | -12 percent         | 1.5                      | loser  | short, heaviest traded |
| I     | -20 percent         | 0.5                      | loser  | no, quiet              |
| J     | -28 percent         | 1.1                      | loser  | short, second heaviest |

So the book is long A and C and short H and J, in equal amounts on each side. Now suppose the next
three months produce these returns:

| Share | Side  | Return     | Contribution to the side  |
| ----- | ----- | ---------- | ------------------------- |
| A     | long  | +6 percent | -                         |
| C     | long  | +4 percent | average long: +5 percent  |
| H     | short | -7 percent | short gains +7 percent    |
| J     | short | -3 percent | average short: +5 percent |

With 10,000.00 in the account, suppose 5,000.00 is held long and 5,000.00 short. The long side earns
5 percent of 5,000.00, which is 250.00, and the short side earns 5 percent of 5,000.00, which is
another 250.00. That is 500.00, or 5 percent of the account, before costs. Entering and later closing
four positions means eight trades; if each is 2,500.00 at a cost of 0.10 percent, the total cost is
8 times 2,500.00 times 0.001, which is 20.00, or 0.2 percent of the account. The net result for the
quarter is about 4.8 percent.

Two cautions sit inside that number. The library page refreshes a third of the book every month, so
over a year the whole book is turned over about four times, which multiplies the cost line. And the
short side of the trade is doing real work in this example; in the published data the heaviest-traded
losers earn close to nothing, so the profit comes mostly from the long side.

## What the research actually found

The paper behind the rule sorts shares by past return and by past turnover and looks at the returns
that follow.

| Source                                                    | What it measured                                                                                          | Result                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| --------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Lee and Swaminathan (2000)                                | NYSE and AMEX shares, monthly sorts on 3 to 12 month returns crossed with 3 turnover groups, 1965 to 1995 | Two findings matter here. First, the spread between winners and losers is larger among heavily traded shares: for a six-month formation and six-month holding, the winner-minus-loser return was about 0.54 percent a month among quiet shares and about 1.46 percent among heavily traded shares. Second, the heaviest-traded losers earned almost nothing, about 0.09 percent a month, which makes them natural candidates to sell short |
| An independent 2026 replication of the paper's main table | The same sorts, rebuilt from the same data source                                                         | Reproduced the qualitative claims: the winner-minus-loser spread rose across the three turnover groups, and the heaviest-traded losers earned near zero. At the six-month setting the spread was 0.69 percent a month among quiet shares and 1.52 percent among heavily traded shares, both statistically significant                                                                                                                      |
| The same replication, run on 1996 to 2024                 | The identical rules on data collected after the paper                                                     | Every pattern kept its sign but lost significance: the heaviest-traded losers' volume spread was -0.40 percent a month with a t-statistic of -1.15, and the extra momentum spread from heavy trading was 0.33 percent a month with a t-statistic of 1.00. The replication's verdict is that the effect is weakened, and it notes that momentum itself has been weaker since 1995                                                           |
| Quantpedia, the entry the library page cites              | Its own summary of the idea                                                                               | Links the trade to the same body of research; the entry itself is behind a subscription, so its performance table could not be read here                                                                                                                                                                                                                                                                                                   |

The original paper's contribution was not really the long-and-short portfolio that the library page
builds. It was the observation that turnover changes how momentum behaves: heavily traded winners
tend to keep going and then reverse sharply, while heavily traded losers earn very little, and quiet
stocks behave differently again. The library page keeps the simplest usable slice of that idea, buying
the heaviest-traded winners and shorting the heaviest-traded losers. The published numbers are
monthly returns of a percent or two before costs, in a strategy that trades a great deal, which is the
crux of the difficulty.

## How this project relates to it

This repository studies the same question in
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md).
That brief reports a related finding on the KOSPI index, in a paper `1208.2775v5`
(https://arxiv.org/abs/1208.2775): momentum portfolios weighted by volume or turnover beat the plain
cumulative-return version, but the improvement does not appear for every weighting scheme, and the
comparison is made without transaction costs, so the ranking is gross. The brief's general conclusion
applies to this tutorial exactly: cross-sectional momentum is one of the most replicated findings in
finance, but the binding constraints are not discovery, they are decay, crowding and the cost of the
trading the rule needs.

## Where it goes wrong

- High turnover cuts both ways. The heaviest-traded winners are where momentum is strongest, but they
  are also the shares that reverse fastest once the crowd moves on. The same volume that signals
  attention also signals how crowded the position has become.
- Costs decide the result. Buying the most heavily traded names and refreshing a third of the book
  every month means turning the whole portfolio over several times a year. At a spread plus
  commission of ten basis points a trade, the cost line is a few percent a year, which is the same
  order as the gross effect.
- The short leg barely pays. The published data show the heaviest-traded losers earning about nothing,
  so shorting them adds little, costs a borrow fee, and can be hard to borrow at all in the small and
  troubled names where the effect is largest.
- Momentum crashes. After a market-wide fall, the biggest losers are often the heavily traded,
  distressed shares that bounce hardest. A short position there takes a large loss just when the rest
  of the book is also suffering.
- The effect weakened. On data after 1995 the same rules keep their sign but lose significance, and
  the extra boost from turnover shrinks to almost nothing. A rule fitted to 1965 to 1995 does not
  describe the decades that followed.
- The data must be right. Turnover needs volumes and share counts, both of which must be adjusted for
  splits and share issues. A single unadjusted share count can put a quiet company into the
  heaviest-traded group and quietly corrupt the whole result.

## Try it yourself

You need a spreadsheet and a public list of shares with prices, volumes and share counts; many finance
websites give all three.

1. Pick twenty shares you recognise, from different industries and of different sizes.
2. Add a column for the twelve-month return of each, and a column for average daily turnover:
   average daily volume divided by shares on issue.
3. Sort by return and mark the top five as winners and the bottom five as losers.
4. Within the winners, mark the two with the highest turnover; within the losers, mark the two with
   the highest turnover. Those four are the ones the rule would trade.
5. Wait a month and fill in each share's return over that month. Compute the winners' average return,
   the losers' average return, and take the difference.
6. Repeat for three months, and keep a running note of how often the chosen four change.

What to notice: how often the four names change, which is what drives the cost; and how often the
heaviest-traded loser actually falls. In the published data that loser often barely moves, which is
why the long side, not the short side, carries the result.

## Where this came from

- [QuantConnect strategy library: combining momentum effect with volume](https://www.quantconnect.com/tutorials/strategy-library/combining-momentum-effect-with-volume),
  the rules as implemented: the top and bottom momentum groups, the most heavily traded names within
  each, equal weights, and a three-month hold refreshed one third at a time.
- [Quantpedia: combining momentum effect with volume](https://quantpedia.com/strategies/combining-momentum-effect-with-volume/),
  the entry the library page cites; the page itself is behind a subscription and could not be read.
- Lee and Swaminathan, [Price Momentum and Trading Volume](https://onlinelibrary.wiley.com/doi/10.1111/0022-1082.00280),
  Journal of Finance, 2000, the original study of momentum and turnover.
- An independent 2026 [replication of Lee and Swaminathan](https://dmurav.com/replications/releases/v2026-09-06/reports/paper-41b15017c8b7.pdf),
  which reproduces the 1965 to 1995 table and then reports that the effect weakens after 1995.
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's brief on which cross-sectional signals survive, including the KOSPI result
  `1208.2775v5` (https://arxiv.org/abs/1208.2775).

## Words used in this tutorial

- momentum: the tendency of something that has been rising to keep rising for a while.
- turnover: the value of shares traded over a period, divided by the size of the company, here
  expressed as a percentage of the shares changing hands each day.
- long: owning a share, so that a price rise is a gain.
- short selling: borrowing a share you do not own, selling it, and buying it back later, so that a
  price fall is a gain.
- position: one holding in a trading account, either long or short.
- universe: the full list of shares a rule is allowed to choose from.
- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
