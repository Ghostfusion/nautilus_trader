# Momentum in a small portfolio: ten winners against ten losers

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                      |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of listed companies, ten owned and ten borrowed and sold                                                                                                                                                                                                                                            |
| How often it trades       | Once a year, when the two groups of ten are rebuilt                                                                                                                                                                                                                                                        |
| What you need             | A spreadsheet, twelve months of share prices, and a rough measure of company size                                                                                                                                                                                                                          |
| Where the rules come from | [QuantConnect strategy library, momentum effect in stocks in small portfolios](https://www.quantconnect.com/tutorials/strategy-library/momentum-effect-in-stocks-in-small-portfolios) and the [Quantpedia entry](https://quantpedia.com/strategies/momentum-effect-in-stocks-in-small-portfolios) it cites |
| The underlying research   | Siganos, [Can small investors exploit the momentum effect?](https://www.researchgate.net/publication/226528840_Can_small_investors_exploit_the_momentum_effect)                                                                                                                                            |
| How well it held up       | Mixed: the original British study finds strong gains that survive its transaction costs, related studies in other markets find smaller positive results, but the out-of-sample run of this strategy is slightly negative and the reported worst fall is 88 percent                                         |
| Also appears in           | [Sector momentum](../sector-momentum/README.md) in this library, a different application of the same idea                                                                                                                                                                                                  |

## The idea in one paragraph

Some shares have risen a lot over the past year and some have fallen a lot. This strategy owns the ten
that rose the most and bets against the ten that fell the most, holding both groups for a year. Big
studies usually rank thousands of shares, which a person with a small account cannot do, so the real
question is whether the tendency still pays in a portfolio of only twenty names. The bet is that a
share which has been rising keeps rising for a while, because good news leaks out slowly and because
investors follow the crowd. The source paper found that the most extreme winners and losers carry the
strongest signal, which is exactly what a small portfolio can afford to hold.

## Why anyone believed it

The usual story is underreaction. A company reports good news, the price rises a little, and the
market takes weeks or months to absorb the rest, so the share keeps drifting up. Confirmation bias
does the same work in reverse: holders of a falling share delay selling because they believe the fall
is temporary. Investors also follow one another, buying what has already gone up and selling what has
already gone down, which pushes the same shares further in the same direction.

The counterparty is the slow seller or the forced seller. A fund facing withdrawals sells the shares
that have fallen, because those are the ones it wants to get rid of; a manager trims a winner once it
has grown too large for the portfolio. Both leave the trade open for whoever bought the winners and
sold the losers.

## An everyday comparison

A football league table. Teams near the top this season, with the same squad, the same manager and the
same budget, usually finish near the top next season too. Nothing about a high league position causes
the next season's wins; the strengths that produced the position persist. The analogy also fails the
same way the strategy does: when the squad is rebuilt or the manager leaves, last season's table tells
you nothing, and a team that has just been promoted can beat the champions.

## The rules, step by step

1. Build the list to choose from: every company listed on your market. The source paper uses British
   shares; the library uses all American-listed companies.
2. Drop the smallest quarter of the companies by market capitalisation, the share price times the
   number of shares issued. Small shares are expensive to buy, sell and borrow.
3. For each remaining company, work out its return over the past twelve months: the price now divided
   by the price twelve months ago, minus one.
4. Rank the companies from the highest return to the lowest.
5. Own the ten with the highest returns (the winners) and short the ten with the lowest returns (the
   losers).
6. Split the money evenly: half the account in the ten winners, half committed to the ten losers, so
   each name carries 5 percent of the account, written +0.05 for a winner and -0.05 for a loser.
7. Hold for one year, then recompute the ranking and rebuild both groups.

One convention worth knowing. Some researchers measure momentum over eleven months and skip the most
recent one, because prices often bounce back briefly after a sharp move. This strategy's source does
not skip a month; it uses the full twelve months.

## The maths, with every symbol named

The signal is one division per share.

```text
M_i = P_i(now) / P_i(12 months ago) - 1
```

- `M_i` is the momentum score of share `i`, a decimal: 0.40 means 40 percent.
- `P_i(now)` is the share's price today.
- `P_i(12 months ago)` is its price on the same date a year earlier.

Rank the shares by `M_i`, from largest to smallest. The ten largest are the winners and the ten
smallest are the losers. Each name gets the same weight:

```text
w_i = +0.05   for each of the ten winners
w_i = -0.05   for each of the ten losers
w_i = 0       for every other share
```

- `w_i` is the fraction of the account placed in share `i`; a plus sign is owned, a minus sign is
  borrowed and sold.
- Ten names at 0.05 each side gives a half of the account on each side, so the account is fully used
  and split evenly between owning and shorting.

The account's return over the following year is then:

```text
R = ( mean return of the ten winners - mean return of the ten losers ) / 2
```

- `R` is the account's return for the year.
- `mean return of the ten winners` is the average return those ten shares earned over the year.
- The division by two is because only half the money sits on each side.

The cost is once a year:

```text
Cost = t * c
```

- `t` is the traded fraction, counting a sale and a purchase as two trades; `t = 2.0` means the whole
  account was replaced.
- `c` is the cost of one trade as a fraction, covering the gap between the buying and selling price
  plus commission. A realistic figure is 0.001, that is ten basis points, where one basis point is one
  hundredth of one percent. Borrowing the losers usually costs extra on top.

## A worked example

The universe below has sixteen invented shares. The smallest four by size are dropped before anything
else happens, even though one of them, S4, would have ranked second with a return of +20 percent. That
is the point of the size rule: those shares are ignored whatever they did.

The twelve survivors, ranked by their return over the past twelve months:

| Share | Size, millions | Twelve-month return | Rank | Group  |
| ----- | -------------- | ------------------- | ---- | ------ |
| S1    | 5000           | +45%                | 1    | winner |
| S3    | 8000           | +30%                | 2    | winner |
| S5    | 3000           | +18%                | 3    | winner |
| S7    | 7000           | +12%                | 4    |        |
| S6    | 900            | +8%                 | 5    |        |
| S9    | 4000           | +5%                 | 6    |        |
| S16   | 300            | +2%                 | 7    |        |
| S10   | 600            | -2%                 | 8    |        |
| S14   | 1000           | -5%                 | 9    |        |
| S11   | 6000           | -8%                 | 10   | loser  |
| S13   | 2000           | -15%                | 11   | loser  |
| S15   | 500            | -22%                | 12   | loser  |

The four dropped shares were the smallest, with sizes of 120, 150, 200 and 250 million.

So the winners are S1, S3 and S5, and the losers are S11, S13 and S15. The real rule holds ten of
each; here three of each stand in for the ten, to keep the arithmetic on one page, so each name gets
`+1/6` or `-1/6` of the account.

Now eight holding years. The two groups are re-formed once a year; the columns show what each group
returned over the coming year, and the long-short result follows from the formula above.

| Year | Winners' return | Losers' return | R = (W - L) / 2 | Cost  | Net return | Cumulative net |
| ---- | --------------- | -------------- | --------------- | ----- | ---------- | -------------- |
| 1    | +55%            | -5%            | +30.0%          | 0.20% | +29.8%     | 29.8%          |
| 2    | +40%            | -4%            | +22.0%          | 0.20% | +21.8%     | 58.1%          |
| 3    | +30%            | -6%            | +18.0%          | 0.20% | +17.8%     | 86.24%         |
| 4    | +42%            | -8%            | +25.0%          | 0.20% | +24.8%     | 132.42%        |
| 5    | -35%            | +45%           | -40.0%          | 0.20% | -40.2%     | 38.99%         |
| 6    | +58%            | -6%            | +32.0%          | 0.20% | +31.8%     | 83.19%         |
| 7    | +25%            | -5%            | +15.0%          | 0.20% | +14.8%     | 110.3%         |
| 8    | +33%            | -7%            | +20.0%          | 0.20% | +19.8%     | 151.94%        |

Check year 1: (55 - (-5)) / 2 = 30.0, and 30.0 - 0.20 = 29.8. Check year 5: (-35 - 45) / 2 = -40.0,
and -40.0 - 0.20 = -40.2. The cost uses `c = 0.001` and `t = 2.0`, so it is 0.20 percent each year.

The eight net returns compound to 2.519, so the account gained about 152 percent over the eight years,
roughly 12.2 percent a year at that pace. Year 5 is the instructive one. In that year the winners fell
35 percent and the losers rose 45 percent, the pattern of a rebound after a market fall, and the pair
lost 40 percent in a single year. The account had to make that back before it made anything else, which
is why the eight-year result is far below the source's headline.

## What the research actually found

| Source                                        | What it measured                                               | Result                                                                                                                                                                                                                                          |
| --------------------------------------------- | -------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Quantpedia, summarising Siganos               | Long ten and short ten extreme shares, British data, 1988-2006 | 32 percent a year net of commissions, stamp duty, short-selling costs and the gap between buying and selling prices, worst fall 88.09 percent; confidence rated moderate, and the note records that the out-of-sample run was slightly negative |
| Siganos, can small investors exploit momentum | British shares, the most extreme winners and losers            | Strong momentum gains remain even after transaction costs, which the paper reads as evidence against the simplest form of market efficiency                                                                                                     |
| Ammann, Moellenbeck and Schmid                | Large American shares, long single shares and short the index  | Large and statistically significant gains, robust to several ways of adjusting for risk                                                                                                                                                         |
| Foltice and Langer                            | New York shares, 1991-2010, accounts from 5,000 dollars        | A long-only winner rule can beat the market after realistic costs for accounts above a minimum size; trading more often helps only up to a point                                                                                                |
| Piras                                         | American shares, 1999-2019                                     | Fewer names gave higher measured returns, but the result depends on which names are in the universe; a long-only ten-share version returned 11.3 percent a year against 6.0 percent for the MSCI USA index                                      |
| Baltussen and others                          | Global markets, up to 150 years                                | Momentum is robust across periods and design choices and is exposed to crash risk; risk-managed versions reduce the crashes                                                                                                                     |

The two headline numbers disagree, and the disagreement is the finding. Quantpedia's 32 percent a year
comes from one British sample over 1988 to 2006, and its own note records that the out-of-sample run
was slightly negative. The later studies find smaller but still positive results and warn that they
depend on the universe and on how many names are held. Read together: momentum has been measured many
times and in many markets, but the size of the prize for a twenty-name portfolio is far less certain
than the single 32 percent figure suggests.

## How this project relates to it

The repository implements the raw signal. The rate-of-change indicator in
[crates/indicators/src/momentum/roc.rs](../../../crates/indicators/src/momentum/roc.rs) is exactly step
3, today's price divided by a price a fixed window back. The evidence on what such rules survive is
collected in
[the predictability brief](../../../strategies/books2/08_predictability_and_trading_strategies.md),
whose conclusion is that the cross-sectional findings are mostly real, so the binding constraints are
decay and leakage rather than the statistics, and that a factor claim should be restated under value
weighting before it is treated as a signal. The persistence question is set out for industries in
[Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), whose rule is
that momentum needs positive serial dependence: winners keep winning for several periods, and the
measured reward turns negative when the rotation is random.

## Where it goes wrong

- Momentum crashes. In a sharp rebound the shares that fell most bounce hardest and the ones that rose
  most stall, so both legs lose at once. The -40 percent year in the worked example is this, and
  Quantpedia's quoted worst fall is 88 percent.
- The 32 percent is one sample. It comes from British shares over 1988 to 2006 on a 10,000 pound
  account, and Quantpedia's own note records a slightly negative out-of-sample run.
- The short side is costly. Borrowing ten falling shares can be expensive or impossible, and a short
  position loses without limit if the share rises.
- Twenty names is little diversification. A single company's surprise can dominate the year, and the
  result depends on which twenty names the universe happened to contain.
- Turnover and size. A small account trading twenty names a year pays commissions and price gaps that
  a large fund negotiates down.
- The ranking is a design choice. Twelve months, one month skipped or not, ten names or fifty, yearly
  or monthly: every combination is a separate rule, and choosing the best after seeing the results is
  how a modest edge gets reported as a large one.

## Try it yourself

No money and no code, just a spreadsheet and any public source of yearly share prices.

1. Write the names of the twenty largest companies in your market down one column, with their size
   next to them.
2. Add two columns: the price today and the price one year ago. Add a third that computes today's
   price divided by last year's price, minus one.
3. Sort the twenty by that column, high to low.
4. Put +5 in a fourth column beside the top ten names and -5 beside the bottom ten. That is the trade
   the rules would have placed.
5. Next year, look up what each of those twenty shares actually returned. Add the returns of the top
   ten and divide by ten, then do the same for the bottom ten, subtract the second from the first, and
   halve it. That is the strategy's return for the year.
6. Repeat the exercise for five different starting years and chain the five results.

What to notice: some years the two groups are far apart and some years the losers beat the winners. The
average of five or six years is a small number with a large wobble around it. That wobble, not the
average, is what a twenty-name portfolio actually experiences.

## Where this came from

- [QuantConnect strategy library: momentum effect in stocks in small portfolios](https://www.quantconnect.com/tutorials/strategy-library/momentum-effect-in-stocks-in-small-portfolios),
  the rules as implemented: American-listed shares, the smallest quarter removed, ten winners and ten
  losers by twelve-month return, equal weights, rebuilt yearly.
- [Quantpedia: momentum effect in stocks in small portfolios](https://quantpedia.com/strategies/momentum-effect-in-stocks-in-small-portfolios),
  the performance figures, the instrument count, the account size and the underlying papers.
- Siganos, [Can small investors exploit the momentum effect?](https://www.researchgate.net/publication/226528840_Can_small_investors_exploit_the_momentum_effect),
  the original small-portfolio momentum study on British data.
- [The predictability brief](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's own reading of the cross-sectional predictability literature.
- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), the same
  persistence question asked of industries.

## Words used in this tutorial

- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- long: owning a share, so the position gains when its price rises.
- market capitalisation: a company's size, its share price multiplied by the number of shares issued.
- momentum: the tendency of something that has been rising to keep rising for a while.
- short: borrowing a share, selling it, and buying it back later, so the position gains when its price
  falls.
- transaction cost: everything you pay to trade, the gap between the buying and selling price,
  commission and any tax.
- volatility: how much a price or return moves around its average, usually given as a percentage a
  year.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
