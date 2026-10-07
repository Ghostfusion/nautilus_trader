# Currency value: judging a currency by what the same basket of goods costs in each country

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                          |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Exchange rates, through futures contracts on major currencies such as the euro, the yen and the British pound                                                                                                                                                  |
| How often it trades       | About once a year in the coded rule, and the vendor allows monthly or quarterly                                                                                                                                                                                |
| What you need             | A spreadsheet, a table of exchange rates and a table of countries' price levels                                                                                                                                                                                |
| Where the rules come from | [The awesome-systematic-trading list](https://github.com/paperswithbacktest/awesome-systematic-trading), which keeps the coded rule, and the [Quantpedia currency value entry](https://quantpedia.com/strategies/currency-value-factor-ppp-strategy/) it cites |
| The underlying research   | Menkhoff, Sarno, Schmeling and Schrimpf, [Currency Value](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2492082)                                                                                                                                         |
| How well it held up       | Weak: one vendor's own index shows a modest reward, but that index is a single measurement, and the vendor's later test outside the sample, and an independent historical study, both reduced the value rule to nothing or below                               |
| Also appears in           | No other tutorial in this collection states the currency value rule; the nearest is [Value effect within countries](../../quantconnect/value-effect-within-countries/README.md), which ranks share markets by value rather than currencies                     |

## The idea in one paragraph

Purchasing power parity is the old idea that a basket of goods should cost the same everywhere once you
convert the money, so any gap between the "fair" price of a currency and its market price should close
over time. This strategy measures how expensive or cheap a currency looks against that fair price, buys
the three that look cheapest, and sells short the three that look dearest, in equal amounts. It holds
them for a quarter or a year and then rebuilds. The bet is that a currency that looks cheap eventually
rises towards its fair value, and the expensive one falls.

## Why anyone believed it

The value case rests on the oldest argument in economics: if the same goods can be bought in one country
and shipped to another, and prices differ by more than the cost of shipping, someone will buy where it is
cheap and sell where it is dear until the gap closes. At the level of a whole economy, the same force is
said to pull a currency's purchasing power back towards its fair level, not in days but over years and
decades.

The counterparty is the investor who cannot wait. Closing a valuation gap can take years, and anyone who
is judged on a few quarters of results, or who needs the money back soon, will sell the cheap currency and
buy the dear one before the gap closes, because the gap can widen first. A second counterparty is the
government or central bank defending a currency for reasons of prestige or exports. If those sellers keep
appearing, the cheap currency stays cheap for longer than a patient owner would like.

## An everyday comparison

Visit two towns a hundred miles apart and price the same shopping list: bread, milk, rice, soap, a haircut.
The basket costs less in one town. If the towns used different money, you could not compare the totals
until you converted them at the going rate. Purchasing power parity says that, converted at the fair rate,
the two totals should be the same. When they are not, the money of the cheap town is said to be worth
less than it should be, and the bet is that it will strengthen. The catch is that you cannot put a haircut
in a lorry and drive it to the other town, so the two baskets never get forced together.

## The rules, step by step

The coded rule uses seven currencies. Here it is as the list's description states it.

1. Choose a set of countries and their currencies. The code uses Australia, Britain, Canada, the euro area,
   Japan, New Zealand and Switzerland.
2. For each country, find the most recently published figure for the price of the standard basket, called
   the implied purchasing power parity conversion rate. It is expressed as the number of units of that
   country's money that would buy the same basket that one American dollar buys in America.
3. Bring that figure up to date. The vendor's description says to apply the country's changes in consumer
   prices and the changes in its exchange rate since the figure was published, producing a fair value for
   the month before the current one. The coded rule uses the published figure as it stands.
4. Compare each currency's market exchange rate with its fair value, and work out how expensive or cheap
   it looks.
5. Buy the three that look cheapest, in equal amounts: one third of the money on each.
6. Sell short the three that look dearest, in equal amounts: one third on each.
7. Hold, then rebuild. The vendor allows monthly or quarterly; the coded rule rebuilds each January.
8. Money set aside as a deposit for the contracts is placed in an overnight deposit so that it earns the
   short-term interest rate.

The reader should know that step 4 is where a common shortcut hides. The code ranks currencies by the
published basket figure itself, without dividing by the market exchange rate. Comparing baskets across
countries only means something after that division, so this tutorial uses the division in the arithmetic
below and flags the shortcut under "Where it goes wrong".

## The maths, with every symbol named

Purchasing power parity is the statement that converting money at the fair rate should even out prices.

The fair rate:

```text
fair = the number of local units that buy the standard basket that one American dollar buys
```

- `fair` is the implied purchasing power parity conversion rate, published by the OECD and passed on by
  the list's data source.
- It is a property of a country's prices, not of its currency market.

The dirt-cheap test compares the fair rate with the rate the market is charging:

```text
V = S / fair - 1
```

- `S` is the market exchange rate, in local units per American dollar, the rate you actually get in the
  market.
- `fair` is the fair rate above.
- `V` is the valuation gap as a decimal: 0.10 means the market rate is ten percent away from the fair
  rate in the direction that makes the local money cheap.

When `S` is larger than `fair`, one dollar buys more local money than the basket needs, so the local money
is cheap and `V` is positive. When `S` is smaller than `fair`, the local money is expensive and `V` is
negative. Rank the currencies by `V` from largest to smallest, buy the top three and sell short the bottom
three:

```text
w_i = +1/3 if currency i is among the three cheapest
w_i = -1/3 if currency i is among the three dearest
w_i = 0 otherwise
```

- `w_i` is the fraction of the account placed in currency `i`.
- Positive means buy, negative means sell short, and the six weights cancel so the account is not exposed
  to the average level of the market, only to the differences between the chosen currencies.

The package's return over the holding period:

```text
R_package = w_1 * r_1 + w_2 * r_2 + ... + w_7 * r_7
```

- `r_i` is the change in currency `i` against the American dollar over the period.
- Adding the weighted moves gives the package's return before costs.

Costs, as always, are the traded amount times the cost per trade:

```text
Cost = t * c
```

- `t` is the amount traded, counted on both sides: selling one third and buying a replacement counts as
  two thirds.
- `c` is the cost per trade, about 0.0002 to 0.0005 for liquid futures, that is two to five basis points,
  where one basis point is one hundredth of one percent.

A short position held in the spot currency market also pays a borrow fee, which for currencies shows up as
the interest difference between the two currencies. In a futures contract that same cost is built into the
contract's price rather than charged separately.

## A worked example

Seven currencies. For each, the market rate in local units per American dollar and the fair rate from the
basket of goods; the valuation gap is the market rate divided by the fair rate, minus one. The figures are
invented but of the size these gaps actually take.

| Currency | Market rate S | Fair rate | V       | Rank |
| -------- | ------------- | --------- | ------- | ---- |
| JPY      | 145.00        | 100.00    | +0.4500 | 1    |
| EUR      | 0.90          | 0.80      | +0.1250 | 2    |
| GBP      | 0.75          | 0.70      | +0.0714 | 3    |
| CAD      | 1.30          | 1.25      | +0.0400 | 4    |
| AUD      | 1.45          | 1.40      | +0.0357 | 5    |
| NZD      | 1.60          | 1.55      | +0.0323 | 6    |
| CHF      | 0.90          | 1.00      | -0.1000 | 7    |

The cheapest three are the yen, the euro and the pound, so the package buys those, one third of the money
each. The dearest three are the Swiss franc, the New Zealand dollar and the Australian dollar, so it sells
short those, one third each. The Canadian dollar sits in the middle and is ignored.

Now suppose the next quarter brings these moves, measured as the change in each currency against the
American dollar (a positive number means the currency strengthened, so one dollar buys fewer local units):

| Currency | Weight  | Next-quarter move | Contribution    |
| -------- | ------- | ----------------- | --------------- |
| JPY      | +0.3333 | +2.0 percent      | +0.6667 percent |
| EUR      | +0.3333 | +1.0 percent      | +0.3333 percent |
| GBP      | +0.3333 | +0.5 percent      | +0.1667 percent |
| CHF      | -0.3333 | -1.0 percent      | +0.3333 percent |
| NZD      | -0.3333 | -1.5 percent      | +0.5000 percent |
| AUD      | -0.3333 | -0.5 percent      | +0.1667 percent |
| Total    | 0.0000  |                   | +2.1667 percent |

The package gained 2.1667 percent before costs. The short positions helped because the currencies sold
short weakened. Suppose all six positions changed from the previous quarter, so the whole package was
rebuilt:

```text
t = 2 (buy one third of six, and sell one third of six)
Cost = 2 * 0.0003 = 0.0006, that is 0.06 percent
Net return for the quarter = 2.1667 - 0.06 = 2.11 percent
```

That is one invented quarter. It shows how to apply the rules and how the arithmetic behaves, and it says
nothing about whether the idea works. The important thing to notice is the size of `V` for the yen: a gap
of 45 percent looks like a huge opportunity, but gaps of that size have lasted for decades, which is the
whole problem with this strategy.

## What the research actually found

| Source                                                                              | What it measured                                                                | Result                                                                                                                                                                                                                                                |
| ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Deutsche Bank currency valuation index, reported by Quantpedia                      | Ten to twenty currencies, 1989 to 2009                                          | 7.82 percent a year, volatility 9.33 percent, worst fall 39.38 percent, reward-to-risk 0.36                                                                                                                                                           |
| Quantpedia's own note on the idea                                                   | The coded rule, after the source sample                                         | Confidence graded Moderate, and Quantpedia reports that its out-of-sample test showed slightly negative performance, with the apparent alpha deteriorating                                                                                            |
| Menkhoff, Sarno, Schmeling and Schrimpf, Currency Value                             | Real exchange rates across a broad set of currencies                            | Valuation measures do contain predictive information, but the authors state the predictability mostly reflects persistent country differences in risk and does not support the simple story that exchange rates revert because of the basket of goods |
| Accominotti and Chambers, Out-of-Sample Evidence on the Returns to Currency Trading | Currencies in the 1920s and 1930s, an era almost untouched by later researchers | Carry and momentum returns from the modern period also appeared in this earlier sample, but the returns to a simple currency value rule were negative, which is a direct contradiction of the value story on an independent sample                    |
| The awesome-systematic-trading list's replication record, across all its papers     | 4,843 coded strategies                                                          | The median replication returned a Sharpe ratio of 0.37, and only 48 percent cleared a t-statistic of 1.96, so half the published record cannot be distinguished from zero on its own sample                                                           |

The disagreement is the honest headline. One vendor's index over one twenty-year window shows a positive
return. An independent study of an earlier, separate period finds the value rule losing money. And the
academic literature that does find predictability says it comes from countries being differently risky,
not from a basket of goods pulling prices together.

## How this project relates to it

This repository's brief on rates and currencies,
[Macro, rates and foreign exchange](../../../strategies/books2/07_macro_rates_and_fx.md), explains how a
fair rate can be inferred and compared across places, including the recovery of interest rates from
prices where no bond market exists, which is the same kind of "price the fair value and trade the gap"
problem one level down. Its Section 2 is the closest material to the arithmetic on this page.

Nothing in this repository runs a currency value rule. The finished tutorial closest to the idea is
[Value effect within countries](../../quantconnect/value-effect-within-countries/README.md), which buys
the cheaper national share markets rather than the cheaper currencies, and therefore tests the same
question about whether "cheap" mean-reverts in a different market.

## Where it goes wrong

- Purchasing power parity is very slow. The gaps it measures can widen for ten or twenty years before they
  narrow, which is longer than most investors can hold a losing position. The rule is a bet on a horizon
  most people cannot stay solvent through.
- The baskets differ. Countries consume different goods and services in different proportions, so the
  "same basket" is never actually the same. Comparing totals is an approximation dressed up as a law.
- Things that cannot be shipped. A haircut, a restaurant meal and rent are cheap in poor countries because
  wages are low, not because the money is mispriced. A large part of any measured gap is this effect, and
  it does not predict that the currency will rise.
- The shortcut in the code. The coded rule ranks the published basket figure directly, without comparing
  it with the market rate. Since the basket figure is in each country's own units, a yen figure of about
  100 and a pound figure of about 0.7 are not on the same scale, so ranking them as published is not the
  comparison the description intended.
- The published figure goes stale. Official basket figures are revised only occasionally, and using one
  that was measured in a different base year, or a country whose money has since changed, silently mixes
  two different rulers.
- For the whole idea to be false, it is enough that the most recent price gap reflects something real,
  such as different productivity or different risk, and not a temporary mispricing. Then the gap is not an
  opportunity and the strategy is buying a permanent fact and hoping it changes.

## Try it yourself

You need a spreadsheet, a table of exchange rates, and a table of price levels. A well-known public index
of the price of a standard basket in many countries is published every year by The Economist as the Big
Mac index, and official purchasing power figures are published in the OECD's data portal.

1. Build a sheet with one row per country. Columns: the official basket figure, the market exchange rate
   on the same date, and a column that is the market rate divided by the basket figure, minus one.
2. Sort the rows by that last column. The largest values are the countries whose money looks cheapest.
3. Look at the top three and the bottom three. Write down the date you are using next to each row.
4. Now find the same table for three years ago and for ten years ago, and repeat steps 1 to 3 for those
   dates.
5. Compare: are the same countries still at the top? Take the three cheapest from ten years ago and check
   whether their currencies actually strengthened over the following ten years.

What to notice: the cheapest countries are almost always the same handful, decade after decade. That is the
central problem. A value signal that never changes is either a permanent truth, in which case there is no
gap to close, or a wrong measurement, in which case the trade has no basis. If your ten-year check shows
the cheap currencies did strengthen, look again at whether those countries had a crisis in the meantime;
what looks like convergence is often a crash that no patient investor would have enjoyed living through.

## Where this came from

- [Currency Value Factor - PPP Strategy](https://quantpedia.com/strategies/currency-value-factor-ppp-strategy/),
  the page that states the rules and reports the Deutsche Bank index figures and the out-of-sample note.
- Menkhoff, Sarno, Schmeling and Schrimpf,
  [Currency Value](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2492082), the study of value
  measures built from real exchange rates.
- Accominotti and Chambers,
  [Out-of-Sample Evidence on the Returns to Currency Trading](https://research.mbs.ac.uk/accounting-finance/Portals/0/docs/Out-of-Sample%20Evidence%20on%20the%20Returns.pdf),
  the study of carry, momentum and value in the 1920s and 1930s.
- [The awesome-systematic-trading list](https://github.com/paperswithbacktest/awesome-systematic-trading),
  which holds the coded rule and the project's own replication record.
- [Macro, rates and foreign exchange](../../../strategies/books2/07_macro_rates_and_fx.md), this
  repository's brief on how fair rates are inferred and compared, Section 2 in particular.

## Words used in this tutorial

- basis point: one hundredth of one percent, so five basis points is 0.05 percent.
- consumer prices: the prices of the goods and services an ordinary household buys, tracked over time.
- fair value: the price an argument or a model says something ought to trade at.
- mean reversion: the tendency of something that has moved away from an average to come back to it.
- purchasing power parity: the idea that converting money at the fair rate should even out the price of the
  same basket of goods in different countries.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- value: buying what looks cheap and selling what looks expensive, on the belief that the gap will close.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
