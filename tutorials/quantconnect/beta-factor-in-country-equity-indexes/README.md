# Betting against beta in country funds: preferring the calmer national markets

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                              |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Funds that each track one country's share market, the calmest quarter owned and the jumpiest quarter borrowed and sold                                                                                                                                                                             |
| How often it trades       | Once a month, when the two quarters are rebuilt                                                                                                                                                                                                                                                    |
| What you need             | A spreadsheet and about a year of daily prices for the country funds and for a world benchmark                                                                                                                                                                                                     |
| Where the rules come from | [QuantConnect strategy library, beta factor in country equity indexes](https://www.quantconnect.com/tutorials/strategy-library/beta-factor-in-country-equity-indexes) and the [Quantpedia entry](https://quantpedia.com/strategies/betting-against-beta-factor-in-country-equity-indexes) it cites |
| The underlying research   | Frazzini and Pedersen, [Betting Against Beta](https://pages.stern.nyu.edu/~lpederse/papers/BettingAgainstBeta.pdf)                                                                                                                                                                                 |
| How well it held up       | Mixed: the underlying pattern is documented across many markets and asset classes, but the version the library runs is a simple two-way bet whose quoted return does not include the cost of the borrowing the theory relies on                                                                    |
| Also appears in           | nothing else in this collection                                                                                                                                                                                                                                                                    |

## The idea in one paragraph

Some national markets swing more than others. The measure of how much a fund swings when the world
market swings is called its beta: a beta of 2 means it moves twice as far as the world, a beta of 0.5
means half as far. This strategy owns the quarter of country funds with the lowest beta and bets
against the quarter with the highest, in equal amounts, and rebuilds once a month. The reason anyone
expects this to pay is a constraint: plenty of investors are not allowed to borrow money to buy more
shares, so to aim for higher returns they buy the jumpiest markets instead, which pushes those prices
up and their later returns down. Owning the calm ones and betting against the jumpy ones is what the
research calls betting against beta.

## Why anyone believed it

Many large investors cannot use borrowed money. A pension fund or an insurer is limited by its own
rules or by a regulator; a household with a fixed pot of savings simply cannot buy more than it can
afford. If someone in that position wants a higher return, the only lever left is to buy assets that
move more, and those purchases make the jumpy assets expensive. An investor who is allowed to borrow
can do the opposite: borrow a little and buy the calm markets, and sell the expensive jumpy ones.

The counterparty, then, is the constrained investor who must buy risk to chase return. Index funds
add to it, because a fund that tracks the world holds more of whatever is large, not whatever is
cheap. The pattern weakens when borrowing becomes easy or cheap for everyone, which is why the source
paper finds the strategy pays less exactly when funding is tight.

## An everyday comparison

A shopper is allowed to take home only one jar of jam at a time. To end up with more jam, that shopper
buys the largest jar on the shelf, even though the large jar costs more per spoonful than two small
ones. Because shoppers who want more are pushed into large jars, large jars become dear relative to
what is inside them, and the shopper who is allowed to buy two small jars gets more jam for the same
money. In the market, the jam is return and the jar size is beta; the shopper who cannot take two is
the investor who cannot borrow.

## The rules, step by step

1. Build the list to choose from: country share-market funds, about 35 of them in the library, one per
   country, plus one world benchmark. The library uses an American market fund as the world.
2. Collect daily prices for about a year, 253 trading days, for every fund and for the benchmark.
3. Turn prices into daily returns: today's price divided by yesterday's price, minus one.
4. For each country fund work out its beta against the benchmark over the past year. Beta is the
   covariance of the fund's daily returns with the benchmark's, divided by the variance of the
   benchmark's returns. In a spreadsheet this is the SLOPE function.
5. Rank the funds from the lowest beta to the highest.
6. Own the lowest quarter, the calmest funds, and short the highest quarter, the jumpiest, with equal
   money in each name so that half the account is long and half short.
7. Rebuild at the start of every month.

One difference from the source paper is worth stating. Frazzini and Pedersen weight each side by how
far its beta sits from one, so the calm basket is scaled up, bought partly with borrowed money, and the
jumpy basket is scaled down, until both sides have a beta of one. The library does not do this; it
simply holds equal amounts of the calmest and jumpiest quarters. That leveraging is what the theory
says creates the return and, at the same time, what costs money to run, which matters for grading the
result.

## The maths, with every symbol named

Everything starts with a return series built from prices:

```text
r(i, t) = P(i, t) / P(i, t - 1) - 1
```

- `r(i, t)` is the return of fund `i` on day `t`, a decimal where 0.01 means one percent.
- `P(i, t)` is the fund's price on day `t`, and `P(i, t - 1)` its price the day before.

Then beta, which measures how much the fund moves for each one percent the world moves:

```text
beta_i = Cov(r_i, r_m) / Var(r_m)
```

- `beta_i` is the beta of country fund `i`.
- `r_i` is the list of the fund's daily returns over the past year.
- `r_m` is the list of the benchmark's daily returns over the same days.
- `Cov(r_i, r_m)` is the covariance of the two lists: how they move together, in units of return
  squared. Positive means they tend to move the same way, negative means opposite ways.
- `Var(r_m)` is the variance of the benchmark's returns: how much the benchmark moves on its own, also
  in units of return squared.

Dividing the first by the second removes the units and leaves a pure number: how many percent the fund
moves for each one percent the benchmark moves. A beta of 1.6 says the fund swings 60 percent harder
than the world.

Then the weights, from the ranking:

```text
w_i = +1 / (2k)   for the k funds with the lowest beta
w_i = -1 / (2k)   for the k funds with the highest beta
w_i = 0           for every other fund
```

- `w_i` is the fraction of the account placed in fund `i`; a plus sign is owned, a minus sign is
  borrowed and sold.
- `k` is the number of funds in one quarter.
- Summing over one side gives `k` times `1 / (2k)`, which is 1/2, so each side uses half the money and
  the account is fully deployed and split evenly between long and short.

The account's return is `R = sum of ( w_i * r_i )`, where `r_i` is the fund's return over the month, and
the cost of rebuilding is `Cost = t * c`:

- `t` is the traded fraction, counting a sale and a purchase as two trades; `t = 2.0` means the whole
  account was replaced.
- `c` is the cost of one trade as a fraction, covering the gap between the buying and selling price
  plus commission. For large country funds a figure of 0.0005, five basis points, is reasonable, where
  one basis point is one hundredth of one percent.

## A worked example

Twelve invented country funds with the beta each showed against the world over the past year:

| Country     | Beta | In the lowest quarter? | In the highest quarter? | Weight |
| ----------- | ---- | ---------------------- | ----------------------- | ------ |
| Japan       | 0.55 | yes                    | no                      | +1/6   |
| Switzerland | 0.60 | yes                    | no                      | +1/6   |
| UK          | 0.78 | yes                    | no                      | +1/6   |
| Canada      | 0.88 | no                     | no                      | 0      |
| USA         | 0.95 | no                     | no                      | 0      |
| Germany     | 1.00 | no                     | no                      | 0      |
| Italy       | 1.12 | no                     | no                      | 0      |
| France      | 1.18 | no                     | no                      | 0      |
| Australia   | 1.25 | no                     | no                      | 0      |
| Korea       | 1.38 | no                     | yes                     | -1/6   |
| Brazil      | 1.55 | no                     | yes                     | -1/6   |
| Mexico      | 1.60 | no                     | yes                     | -1/6   |

The calm quarter is Japan, Switzerland and the UK; the jumpy quarter is Korea, Brazil and Mexico. The
real rule holds a quarter of about 35 funds; here three of each stand in for that, so each name gets
`+1/6` or `-1/6` of the account.

Now six holding months. The two quarters are re-formed each month; the columns show what each quarter
returned over the coming month, and the long-short result follows from the formula above.

| Month | Calm quarter | Jumpy quarter | R = (calm - jumpy) / 2 | Cost  | Net return | Cumulative net |
| ----- | ------------ | ------------- | ---------------------- | ----- | ---------- | -------------- |
| 1     | +1.0%        | +2.0%         | -0.50%                 | 0.10% | -0.60%     | -0.60%         |
| 2     | +0.5%        | +1.5%         | -0.50%                 | 0.10% | -0.60%     | -1.20%         |
| 3     | -2.0%        | -4.0%         | +1.00%                 | 0.10% | +0.90%     | -0.31%         |
| 4     | +1.5%        | +3.0%         | -0.75%                 | 0.10% | -0.85%     | -1.15%         |
| 5     | +0.0%        | -0.5%         | +0.25%                 | 0.10% | +0.15%     | -1.01%         |
| 6     | -1.0%        | -2.5%         | +0.75%                 | 0.10% | +0.65%     | -0.36%         |

Check month 1: (1.0 - 2.0) / 2 = -0.50, and -0.50 - 0.10 = -0.60. Check month 3: (-2.0 - (-4.0)) / 2 =
1.00, and 1.00 - 0.10 = 0.90. The cost uses `c = 0.0005` and `t = 2.0`, so it is 0.10 percent each
month.

Over the six months the gross result was +0.25 percent and the cost was 0.60 percent, leaving -0.36
percent. That is the honest shape of this example: a long-short bet that wins a little before costs can
lose after them, and at a monthly rebuild the costs are unavoidable. The jumpiness of country funds is
itself only a few percent a month, so a small cost line relative to a small signal decides the result.

## What the research actually found

| Source                                        | What it measured                                                                            | Result                                                                                                                                                                                                                                    |
| --------------------------------------------- | ------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Quantpedia, summarising Frazzini and Pedersen | Betting against beta in country funds, 1980-2009                                            | 6.8 percent a year before costs, volatility 13.08 percent, worst fall 46.56 percent, Sharpe ratio 0.51, about 13 countries                                                                                                                |
| Frazzini and Pedersen                         | American shares, 20 international share markets, government bonds, corporate bonds, futures | High beta goes with low risk-adjusted return; a betting-against-beta factor earns significant positive risk-adjusted returns; when funding constraints tighten the factor's return is low; when funding risk rises, betas move toward one |
| Berrada, Messikh, Oderda and Pictet           | Country indexes, sectors and individual shares                                              | A long-low-beta, short-high-beta rule beats the market in both a theoretical model and the examples tested                                                                                                                                |
| Hedegaard                                     | Time and country variation in the strategy's return                                         | The strategy does better when and where past market returns were high, consistent with the demand for leverage rising and falling                                                                                                         |
| Andricopoulos                                 | Re-examination of the low-beta anomaly                                                      | Argues the pattern is not a behavioural bias but comes from leverage destroying shareholder value, so the low-risk firms are the better ones for a real reason                                                                            |

The pattern is one of the best-replicated in asset pricing. The source paper finds it in American
shares, twenty international share markets, government bonds, corporate bonds and futures, which is
far more than one sample. The doubts are about the tradable version. The theory's own factor uses
borrowed money on the calm side, and financing costs are the one input with no clean historical record
in these tests. The simple quartile bet the library runs, without the leverage, is a different and
weaker object, and the 6.8 percent figure is quoted before costs. Nothing above says the strategy
works; it says the pattern has been measured widely.

## How this project relates to it

The repository measures betas, which is the input this rule needs.
[crates/analysis/src/statistics/beta_ratio.rs](../../../crates/analysis/src/statistics/beta_ratio.rs)
is where beta is computed, and
[the regimes brief](../../../strategies/books2/24_regimes_and_change_points.md) records the finding
that a beta computed over a pooled sample differs from a beta computed inside a market state, so a
single one-year beta is a choice rather than the answer. The estimation side is in
[the volatility brief](../../../strategies/books2/14_volatility_and_microstructure_noise.md), which
reports that a restricted factor model gives usable daily betas and maps that to
[crates/risk/src/engine](../../../crates/risk/src/engine) for portfolio risk. None of these runs
betting against beta; they are the measurement layer a rule like this would have to sit on.

## Where it goes wrong

- The borrowing the theory needs costs money. Scaling the calm side up requires leverage, and
  financing costs are exactly what the simple long-short version leaves out.
- Beta is unstable. A one-year window of daily prices gives a noisy estimate, and the ranking can
  change from month to month for no economic reason.
- Crowding. Betting against beta is well known and easy to run through funds, so heavy buying of the
  calm markets raises their prices and lowers their future returns.
- It fails when funding tightens. The source paper finds the factor's return is low exactly when
  funding constraints bind, so the strategy is most fragile when markets are stressed.
- The benchmark choice decides everything. Beta is measured against the world, so the answer depends
  on which fund stands for the world, and an American market fund is not the world.
- Shorting country funds is not free. Some markets restrict short sales, and borrowing a foreign fund
  can cost more than borrowing an American share.

## Try it yourself

No money and no code, just a spreadsheet and a public source of daily prices.

1. Put two columns side by side: the daily price of one country fund, and the daily price of a world
   fund, for the last year.
2. Add two more columns with the daily returns: each price divided by the previous price, minus one.
3. In an empty cell write `=SLOPE(country_return_column, world_return_column)`. That single formula is
   the beta, and it carries out the covariance divided by the variance for you.
4. Do this for ten country funds and write each beta beside its name. Sort them, lowest first.
5. Put the three lowest betas in one group and the three highest in another.
6. Next month, look up what each of the six funds returned, average the three calm ones, average the
   three jumpy ones, subtract the second from the first, and halve it.
7. Repeat for several months and chain the results, then subtract 0.10 percent for each month's cost.

What to notice: the beta values wander from month to month even for the same fund, so the groups are
never exactly the same two months running. Notice too how often the six-month result before costs is
smaller than the costs you subtracted, which is the whole difficulty of a calm-versus-jumpy bet run
through funds.

## Where this came from

- [QuantConnect strategy library: beta factor in country equity indexes](https://www.quantconnect.com/tutorials/strategy-library/beta-factor-in-country-equity-indexes),
  the rules as implemented: about 35 country funds against one world fund, a one-year daily beta, the
  lowest quarter owned and the highest quarter shorted, rebuilt monthly.
- [Quantpedia: Betting Against Beta Factor in International Equities](https://quantpedia.com/strategies/betting-against-beta-factor-in-country-equity-indexes),
  the performance figures, the instrument count, the benchmark and the underlying papers.
- Frazzini and Pedersen, [Betting Against Beta](https://pages.stern.nyu.edu/~lpederse/papers/BettingAgainstBeta.pdf),
  the original study across shares, country markets, bonds and futures.
- [The volatility brief](../../../strategies/books2/14_volatility_and_microstructure_noise.md) and
  [the regimes brief](../../../strategies/books2/24_regimes_and_change_points.md), this repository's
  own reading of how betas are estimated and how they change with the market state.

## Words used in this tutorial

- beta: how much an asset moves when the market moves one percent, measured by comparing their returns.
- benchmark: the reference investment a bet is measured against, here a fund standing for the whole
  world market.
- covariance: a number saying how two lists of returns move together, positive when they tend to rise
  and fall at the same time.
- leverage: borrowing money to hold more of an asset than your own cash allows, which magnifies both
  gains and losses.
- long: owning a fund, so the position gains when its price rises.
- rebalancing: rebuilding a portfolio back to its intended weights, which means trading.
- short: borrowing a fund, selling it, and buying it back later, so the position gains when its price
  falls.
- variance: how much a list of returns moves around its own average, the spread of the returns squared.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
