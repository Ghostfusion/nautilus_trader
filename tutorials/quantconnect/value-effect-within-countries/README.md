# Cheap countries: owning the national markets with the lowest valuation

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                               |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Funds that each track one country's share market, held in an equal-weight basket, with cash held when nothing is cheap enough                                                                                                                                       |
| How often it trades       | About once a year in the source; the library rebuilds every month as its valuation file updates                                                                                                                                                                     |
| What you need             | A spreadsheet and a long-run valuation figure for each country, or a published source of those figures                                                                                                                                                              |
| Where the rules come from | [QuantConnect strategy library, value effect within countries](https://www.quantconnect.com/tutorials/strategy-library/value-effect-within-countries) and the [Quantpedia entry](https://quantpedia.com/strategies/value-factor-effect-within-countries) it cites   |
| The underlying research   | Mebane Faber, [Global Value: Building Trading Models with the 10 Year CAPE](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2129474)                                                                                                                            |
| How well it held up       | Mixed: the ratio predicts long-run returns on a long American sample and in several international studies, but a broad out-of-sample test of valuation ratios across sixteen countries finds no consistent gain, and the strategy inherits the whole market's falls |
| Also appears in           | nothing else in this collection                                                                                                                                                                                                                                     |

## The idea in one paragraph

National share markets are priced very differently from one another. Some trade at a high price
compared with the profits their companies make, and some trade cheaply. This strategy works out a
long-run measure of how expensive each country is, the price of its market divided by the average
profit over the past ten years, then puts equal money into the cheapest third and leaves the rest in
cash. It rebuilds about once a year. The bet is that an expensive market has more room to fall and a
cheap one more room to rise, so owning the cheap ones pays more over the long run.

## Why anyone believed it

The story is that investors overreact. They pile into the markets that have done well lately, which
makes those markets expensive, and they neglect the markets that have done badly, which makes those
cheap. If the underlying businesses in the two markets are broadly similar, the expensive market has
to fall or stall for the relationship to make sense again, and the cheap one has room to catch up.
The counterparty is the crowd chasing recent winners, and the cheap markets are cheap precisely
because that crowd has left them alone.

## An everyday comparison

Two neighbouring towns with the same wages and the same jobs. One is fashionable, and houses there
sell at a high multiple of the rent they earn. The other is out of favour, and its houses sell at a
lower multiple of the same rent. Over a decade the rents in the two towns grow at similar rates, so
the fashionable town's price gradually drifts back toward the relationship the unfashionable one
always had. The strategy buys the unfashionable town and waits, which is why it takes years rather
than weeks to pay.

## The rules, step by step

1. Build the list to choose from: the countries whose share markets can be bought easily through a
   fund, about 26 in the library's list and 32 in the source paper. Examples are Australia, Brazil,
   Canada, France, Germany, Japan, the United Kingdom and the United States.
2. For each country work out the CAPE, the cyclically adjusted price-to-earnings ratio, as described
   in the maths section.
3. Sort the countries from the lowest CAPE to the highest, cheapest first.
4. Take the cheapest third of the list.
5. From that third keep only the countries whose CAPE is below 15. If no country in the cheapest third
   is below 15, hold cash instead of any country fund.
6. Put equal money into each country that is kept; whatever is left out sits in cash.
7. Repeat about once a year. The source rebalances yearly; the library's version rebuilds every month
   as the valuation file updates, which is a faster version of the same rule.

## The maths, with every symbol named

The measure at the centre of the strategy compares today's price of a whole market with the average
profit it has made over a decade.

```text
CAPE_i = P_i / ( ( E_i(1) + E_i(2) + ... + E_i(10) ) / 10 )
```

- `CAPE_i` is the cyclically adjusted price-to-earnings ratio of country `i`.
- `P_i` is the real, that is inflation-adjusted, price level of country `i`'s share market today.
- `E_i(1)` to `E_i(10)` are the real earnings of that market in each of the last ten years, where
  earnings are the profits the companies report per share.
- Dividing the sum by ten gives the average of the last ten years, which is what "cyclically adjusted"
  means: the average smooths out the good years and the bad years of the business cycle.

In words: the ratio compares today's price with a decade of real profit. A CAPE of 20 says the market
costs twenty years of average profit, a CAPE of 10 says ten years. Because both the price and the
earnings are adjusted for inflation, the ratio compares like with like across countries and decades.

Then the selection:

```text
keep country i if its CAPE is in the cheapest third of the list AND CAPE_i < 15
w_i = 1 / k   for each kept country, and 0 for the others
```

- `k` is the number of countries kept. Each kept country gets an equal share of the whole account.
- If `k` is zero, all the money stays in cash.

The account's return over the following period is the weighted average of the funds' returns,
`R = sum of ( w_i * r_i )`, and the cost of rebuilding is `Cost = t * c`:

- `r_i` is the return of country `i`'s fund over the period.
- `t` is the traded fraction, counting a sale and a purchase as two trades; `t = 2.0` means the whole
  basket was replaced.
- `c` is the cost of one trade as a fraction, covering the gap between the buying and selling price
  plus commission. For large country funds a figure of 0.001, ten basis points, is reasonable, where
  one basis point is one hundredth of one percent.

## A worked example

First the ratio itself. Suppose one country's market has an inflation-adjusted price level of 2400 and
real earnings per share that averaged 240 over the last ten years. Then CAPE is 2400 / 240 = 10. The
market costs ten years of average profit.

Now twelve invented countries, sorted from the cheapest CAPE to the most expensive:

| Country     | CAPE | In the cheapest third? | Below 15? | Kept? | Weight |
| ----------- | ---- | ---------------------- | --------- | ----- | ------ |
| Brazil      | 8    | yes                    | yes       | yes   | 1/4    |
| Spain       | 9    | yes                    | yes       | yes   | 1/4    |
| Italy       | 10   | yes                    | yes       | yes   | 1/4    |
| UK          | 12   | yes                    | yes       | yes   | 1/4    |
| Germany     | 13   | no                     | yes       | no    | 0      |
| France      | 14   | no                     | yes       | no    | 0      |
| Korea       | 16   | no                     | no        | no    | 0      |
| Japan       | 18   | no                     | no        | no    | 0      |
| Australia   | 19   | no                     | no        | no    | 0      |
| Mexico      | 20   | no                     | no        | no    | 0      |
| Switzerland | 22   | no                     | no        | no    | 0      |
| USA         | 30   | no                     | no        | no    | 0      |

The cheapest third of twelve is four countries: Brazil, Spain, Italy and the UK. All four are below 15,
so all four are kept, each with a quarter of the account.

Now suppose next year the four cheapest are Brazil at 13, Italy at 14, Spain at 16 and Korea at 17.
Only Brazil and Italy are below 15, so `k = 2` and each kept country gets half the account; Spain and
Korea are dropped by the rule even though they are in the cheapest third. If a third year brought
Brazil at 16, Italy at 17, Spain at 18 and Korea at 19, no country would be below 15 and the whole
account would sit in cash.

Now six holding years, with the cheap-third basket measured against all the countries in the list. The
returns are invented, but of the size these markets actually show, and the difference is what the
strategy earns or loses relative to simply owning all of them.

| Year | Cheap-third basket | All countries | Difference | Cost  | Net return | Cumulative net |
| ---- | ------------------ | ------------- | ---------- | ----- | ---------- | -------------- |
| 1    | +28%               | +16%          | +12.0%     | 0.10% | +11.9%     | 11.9%          |
| 2    | -20%               | -28%          | +8.0%      | 0.10% | +7.9%      | 20.74%         |
| 3    | +16%               | +20%          | -4.0%      | 0.10% | -4.1%      | 15.79%         |
| 4    | +10%               | +5%           | +5.0%      | 0.10% | +4.9%      | 21.46%         |
| 5    | -12%               | -16%          | +4.0%      | 0.10% | +3.9%      | 26.2%          |
| 6    | +34%               | +20%          | +14.0%     | 0.10% | +13.9%     | 43.74%         |

Check year 1: 28 - 16 = 12.0, and 12.0 - 0.10 = 11.9. Check year 3: 16 - 20 = -4.0, and -4.0 - 0.10 =
-4.1. The cost uses `c = 0.001` and `t = 1.0`, so it is 0.10 percent a year, on the assumption that
about half the basket changes each year.

Compounding the six net differences gives 1.4374, so the advantage over owning all countries was about
44 percent over the six years, roughly 6.3 percent a year at that pace. The cheap basket itself
compounded at about 7.5 percent a year in this invented series, well below the 14.7 percent the source
reports over its own longer and stronger sample. Year 3 is the warning: the cheap basket rose 16
percent but the whole list rose 20 percent, so the strategy lagged even while making money.

## What the research actually found

| Source                                    | What it measured                                               | Result                                                                                                                                                                                       |
| ----------------------------------------- | -------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Quantpedia, summarising Faber             | Cheapest third of countries by CAPE, global funds, 1980-2011   | 14.7 percent a year in real terms for the cheapest third, volatility 26.1 percent, worst fall 27.17 percent, Sharpe ratio 0.56, about 10 instruments                                         |
| Faber, global value                       | CAPE across more than 30 foreign markets                       | Selecting on relative and absolute valuation produced significant outperformance, and foreign markets showed even larger bubbles and busts than the United States                            |
| Klement                                   | CAPE for 35 countries, developed and emerging                  | The ratio is a reliable long-run valuation indicator in both groups and predicts real local returns five to ten years ahead                                                                  |
| Angelini, Bormetti, Marmi and Nardini     | American index data from 1871, and several other countries     | The log earnings-to-price ratio predicts long-run returns, with t-statistics of 2.29 and 2.52 for two American series, and the explanatory power grows with the horizon (`1204.5055v2` p.15) |
| Keimling                                  | 17 MSCI country indexes from 1979                              | CAPE and price-to-book give reliable long-run forecasts; price-to-earnings, price-to-cash-flow and dividend yield do not                                                                     |
| The payout-ratio study in this repository | 48 country-and-asset combinations, 16 countries, 1870s to 2020 | No combination shows a consistent gain in sample, out of sample and for a utility-based investor, and the prediction portfolios trade several times more than the benchmark                  |

This is the clearest disagreement in the tutorial, and it should be stated as one. The CAPE studies
find that a cheap market tends to deliver more over the following five to ten years, and Faber's rule
earned 14.7 percent a year in real terms over 1980 to 2011. The broad out-of-sample study, recorded in
[the predictability brief](../../../strategies/books2/08_predictability_and_trading_strategies.md),
took the same family of payout-ratio predictors across sixteen countries over 150 years and found no
combination that gained in sample, out of sample and for a utility-based investor at once. Both can be
right: the ratio may still rank countries usefully while a simple yearly rule of trading it is not
robust. It is also worth saying plainly that 14.7 percent a year real is mostly the return of owning
shares at all, spread across many countries, not a promised extra.

## How this project relates to it

The repository does not run a country valuation rule. The measurement closest to it is in
[the predictability brief](../../../strategies/books2/08_predictability_and_trading_strategies.md),
whose first finding is that payout-ratio predictability fails a broad out-of-sample test and whose
general conclusion is that the cross-sectional findings are real but that decay and leakage are the
binding constraints. The portfolio-building side is covered in
[the allocation brief](../../../strategies/books2/10_portfolio_and_allocation.md), which is about how
much of a published edge survives once the weights are estimated rather than assumed. The long-run
prediction theory behind the formulas above is in a paper held in this repository's local research
harvest, "Value Matters: Predictability of Stock Index Returns" (`1204.5055v2`), which is where the
t-statistics and the rising explanatory power quoted above come from.

## Where it goes wrong

- Value traps. A market can be cheap for a real reason, such as weak company law, a shrinking economy
  or a coming currency devaluation, and it can stay cheap for years while the reason plays out.
- The signal is slow. The sources say valuation predicts returns over five to ten years. Over one year
  the noise swamps the signal, so any single year tells you almost nothing.
- Mostly market exposure. The account is always invested in shares, so it falls when the world falls;
  the quoted volatility of 26.1 percent and the falls of 2008 come from exactly that.
- Hindsight in the country list. Many of the 26 or 32 countries did not have an easily bought fund for
  the whole sample, so a backtest that uses today's list quietly keeps only the markets that survived
  and grew.
- The threshold of 15 is a choice, and so is the cheapest third. Both were fixed after early results
  were seen, which flatters the reported figures.
- The ratio is not the only one. Price-to-book beat CAPE in countries with structural breaks according
  to the 17-country study, so the choice of ratio is itself a decision that can be made after the fact.

## Try it yourself

No money and no code, just a spreadsheet. A public source of CAPE values by country, such as the
Barclays indices page the library uses, will supply the numbers.

1. Write down the names of fifteen countries down one column and their CAPE values beside them.
2. Sort the list by CAPE, cheapest first.
3. Mark the cheapest five, which is the cheapest third.
4. Cross out any of those five whose CAPE is 15 or above. Whatever is left is the basket.
5. Beside each kept country write an equal share of 100, so two kept countries get 50 each and four get
   25 each.
6. Next year, look up each country's fund return, multiply by its weight, and add the results. That is
   the strategy's return.
7. Do the same for the average of all fifteen countries, so you have two numbers to compare.

What to notice: for several years in a row the cheap countries may be the same ones, so the basket
barely changes and the cost is low. In other years the whole cheapest third changes hands. Notice too
how often the answer in step 6 is beaten by the simple average in step 7, and how long you would have
had to wait for the cheap ones to win.

## Where this came from

- [QuantConnect strategy library: value effect within countries](https://www.quantconnect.com/tutorials/strategy-library/value-effect-within-countries),
  the rules as implemented: 26 countries, the cheapest 33 percent by CAPE with a cut-off at 15, equal
  weights, cash when nothing qualifies.
- [Quantpedia: Value Factor - CAPE Effect within Countries](https://quantpedia.com/strategies/value-factor-effect-within-countries),
  the performance figures, the instrument count and the underlying papers.
- Mebane Faber, [Global Value: Building Trading Models with the 10 Year CAPE](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2129474),
  the original country-rotation study.
- Angelini, Bormetti, Marmi and Nardini, Value Matters: Predictability of Stock Index Returns,
  [arXiv 1204.5055](https://arxiv.org/abs/1204.5055), the long-run prediction theory; the
  t-statistics quoted above are on page 15 of `1204.5055v2`.
- [The predictability brief](../../../strategies/books2/08_predictability_and_trading_strategies.md)
  and [the allocation brief](../../../strategies/books2/10_portfolio_and_allocation.md), this
  repository's own reading of the evidence on predictability and portfolio construction.

## Words used in this tutorial

- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- CAPE: the cyclically adjusted price-to-earnings ratio, a market's price divided by its average
  earnings over the last ten years.
- earnings: the profit a company reports, usually expressed per share.
- ETF: a fund that trades on an exchange and holds a basket of assets, here the shares of one country.
- real return: a return after inflation has been subtracted, so it measures buying power rather than
  the raw number of currency units.
- rebalancing: rebuilding a portfolio back to its intended weights, which means trading.
- valuation ratio: any measure that compares a price with a company's or a market's earnings, book
  value or cash flow.
- volatility: how much a price or return moves around its average, usually given as a percentage a
  year.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
