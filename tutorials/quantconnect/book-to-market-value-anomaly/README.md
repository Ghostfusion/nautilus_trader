# Book to market: buying companies whose accounts value them cheaply

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                         |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of American companies, chosen because their accounts value them cheaply relative to their share price                                                                                                                                                                                  |
| How often it trades       | About once a year, when the list is rebuilt                                                                                                                                                                                                                                                   |
| What you need             | A spreadsheet, share prices, share counts, and each company's book value                                                                                                                                                                                                                      |
| Where the rules come from | [QuantConnect strategy library, book to market value anomaly](https://www.quantconnect.com/tutorials/strategy-library/book-to-market-value-anomaly) and the [Quantpedia entry](https://quantpedia.com/strategies/value-book-to-market-factor)                                                 |
| The underlying research   | Fama and French, [The Cross-Section of Expected Stock Returns](https://www.bengrahaminvesting.ca/Research/Papers/French/The_Cross-Section_of_Expected_Stock_Returns.pdf), and Asness, Frazzini, Israel and Moskowitz, [Fact, Fiction, and Value Investing](https://ssrn.com/abstract=2595747) |
| How well it held up       | Mixed: one of the longest-documented patterns, with a positive average over almost a century, but the reward is small next to the risk, several credible studies find it weak over the last three decades, and the library's own test lost to the index                                       |
| Also appears in           | [The price to earnings ratio](../price-earnings-anomaly/README.md) and [the Fama-French five-factor model](../fama-french-five-factors/README.md), which use the same idea under other names                                                                                                  |

## The idea in one paragraph

A company's accounts contain a figure for what it owns minus what it owes, which is the money that
would be left for its owners if it sold everything and paid its debts. Divide that figure by the
number of shares and you get the book value of one share. Divide the book value of one share by the
share price and you get the book-to-market ratio, which says how many cents of accounting value you
are buying for each dollar you pay. This strategy buys the companies with the highest such ratio,
which look cheap by their own accounts, and holds them for a year. The bet is that the market has
been too gloomy about these companies and will eventually price them closer to their accounts.

## Why anyone believed it

The idea goes back to Benjamin Graham in the 1930s: buy a dollar of assets for fifty cents and you
have a margin of safety. Later, Eugene Fama and Kenneth French showed that over long periods the
cheapest fifth of shares, ranked this way, earned more on average than the most expensive fifth.

The person on the other side is an investor who is excited about growth. Companies that grow fast
have high share prices relative to their accounts, because buyers pay for profits they expect in the
future rather than for what exists now. If enough buyers chase those expectations, the price can run
ahead of the business, and the companies with modest accounts but steady profits are left behind. A
second explanation says the cheap shares are cheap for a reason: they are firms in trouble, and the
higher average return is simply payment for bearing the risk that they fail. Which of the two stories
is right is still argued about, and the honest position is that both may be partly true.

## An everyday comparison

Think of two houses on the same street. One is a plain house and one has been newly extended and
repainted. The plain house is offered at the value of its land plus materials, while the extended
house is offered well above what its bricks and land are worth because buyers imagine the rent it
could earn. A buyer who insists on paying close to land-and-bricks value will sometimes end up with
the plain house in a rising street and do very well, and sometimes end up with the plain house
because nobody wants it, which is the risk. Buying the cheapest houses on the street is not the same
as buying the best ones.

## The rules, step by step

1. Take the list of companies whose accounts are available, and compute for each one its market
   value: the share price multiplied by the number of shares.
2. Keep only the largest companies. The library keeps the biggest fifth by market value, because
   small companies are hard to trade and their accounts are noisier. Companies with a negative book
   value, which means debts larger than assets, are dropped first.
3. For the companies that remain, compute the book-to-market ratio: book value per share divided by
   the share price.
4. Rank them by that ratio, largest first, and buy the top fifth, which is the fifth with the highest
   book value relative to price. The library holds only this cheap group, bought outright.
5. Weight each holding by its market value, so a company worth three times another gets three times
   the money, rather than giving every company the same amount.
6. Hold for one year, then recompute everything from step 1 and rebuild the list.
7. The academic version, which the Quantpedia entry describes, is long and short: it also sells short
   the companies with the lowest book-to-market ratio, the expensive ones, so that it is exposed only
   to the difference between cheap and expensive shares rather than to the share market as a whole.
   This tutorial describes the long-only version, which is what the library code runs.

## The maths, with every symbol named

The book value of one share:

```text
B = ( assets - liabilities ) / shares
```

- `B` is the book value per share, in the same currency as the share price.
- `assets` is everything the company owns, valued in its accounts.
- `liabilities` is everything it owes.
- `shares` is the number of shares it has issued.
- Divided the way it is shown, the top of the fraction is the shareholders' equity, sometimes written
  as common shareholders' equity, and this is what is left for the owners after the debts.

The book-to-market ratio of one company:

```text
BTM = ( assets - liabilities ) / market value = B / price
```

- `BTM` is the book-to-market ratio, a plain number with no units.
- `market value` is the share price multiplied by the number of shares, also called market
  capitalisation.
- `price` is the price of one share.
- A `BTM` of 1.00 means you pay one dollar for each dollar of accounting value; a `BTM` of 2.00 means
  you pay fifty cents for each dollar, which looks cheaper; a `BTM` of 0.25 means you pay four dollars
  for each dollar, which looks expensive.

The related price-to-book ratio is simply the same thing upside down:

```text
PBR = price / B = 1 / BTM
```

- `PBR` is the price-to-book ratio, the number usually quoted by finance websites.
- Because it is the inverse, ranking companies by `BTM` from high to low is exactly the same as
  ranking them by `PBR` from low to high.

The market-value weighting and the portfolio's return:

```text
w_i = market value_i / ( sum of market values of the chosen companies )
R_portfolio = sum over held companies of ( w_i * r_i )
```

- `w_i` is the share of the money placed in company `i`, and the weights of the chosen companies add
  up to 1, so the whole account is invested.
- `r_i` is the return of company `i` over the holding year, meaning its share price at the end plus
  any dividend, divided by its price at the start, minus one.

The cost of a rebuild over one year:

```text
Cost = t * c
```

- `t` is the traded fraction of the account. Replacing half the list costs about 1.0 in these terms,
  because selling the names that leave and buying the names that enter are both counted.
- `c` is the cost of one trade as a fraction of the amount traded, covering the gap between the
  buying and selling prices plus commission. For large American shares a realistic figure is 0.0005
  to 0.001, that is five to ten basis points, where one basis point is one hundredth of one percent.

## A worked example

Ten companies, with invented but plausible numbers. Shares are in millions, so market value is in
millions of the currency, and book value is the shareholders' equity in millions.

| Company | Share price | Shares | Market value | Book value | BTM    |
| ------- | ----------- | ------ | ------------ | ---------- | ------ |
| Charlie | 90.00       | 40     | 3600.00      | 2200.00    | 0.6111 |
| Hotel   | 50.00       | 55     | 2750.00      | 700.00     | 0.2545 |
| Foxtrot | 35.00       | 60     | 2100.00      | 600.00     | 0.2857 |
| Alpha   | 40.00       | 50     | 2000.00      | 500.00     | 0.2500 |
| Bravo   | 25.00       | 80     | 2000.00      | 1800.00    | 0.9000 |
| India   | 80.00       | 25     | 2000.00      | 1200.00    | 0.6000 |
| Echo    | 60.00       | 30     | 1800.00      | 400.00     | 0.2222 |
| Golf    | 20.00       | 90     | 1800.00      | 950.00     | 0.5278 |
| Delta   | 15.00       | 100    | 1500.00      | 700.00     | 0.4667 |
| Juliet  | 12.00       | 120    | 1440.00      | 1100.00    | 0.7639 |

The largest half of ten is five companies, so step 2 keeps Charlie, Hotel, Foxtrot, Alpha and Bravo.
Among those five, ranked by `BTM`, the two largest ratios are Bravo at 0.9000 and Charlie at 0.6111,
so the strategy buys those two. With only ten companies, the example keeps half by size and two by
book-to-market; the real library keeps the largest fifth and then the cheapest fifth within it, which
is the same two-stage shape.

The weights come from market value, so Bravo gets 2000 / (2000 + 3600) = 0.3571 and Charlie gets
3600 / 5600 = 0.6429. Suppose the next year brings Charlie a return of 6 percent and Bravo a return of
15 percent, because the cheaper-looking company did better.

| Company held | Weight | Return over the year | Contribution    |
| ------------ | ------ | -------------------- | --------------- |
| Charlie      | 0.6429 | +6 percent           | +3.8574 percent |
| Bravo        | 0.3571 | +15 percent          | +5.3565 percent |
| Total        | 1.0000 |                      | +9.2139 percent |

The portfolio gained about 9.21 percent before costs. If half the list changed, `t` is about 1.0, and
with `c = 0.001` the cost is 0.10 percent, so the net result is about 9.11 percent. Run the same
numbers with the two returns swapped and Charlie's larger weight makes the result much smaller, which
is the point of weighting by size: the example's outcome depends heavily on which company is bigger,
not only on which is cheaper.

## What the research actually found

| Source                                   | What it measured                                                             | Result                                                                                                                                                                                 |
| ---------------------------------------- | ---------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Fama and French, 1993                    | American shares, 1963 onward, book-to-market as one of two sorting variables | The cheapest shares earned more than the most expensive across the sample, and the pattern survived together with company size                                                         |
| Quantpedia, summarising the updated work | The long-and-short value portfolio, monthly, equal weighted, 1926 to 2014    | 3.6 percent a year, volatility 12.02 percent, maximum fall 55.99 percent, reward-to-risk 0.30, and it rates its confidence in the effect as strong                                     |
| Asness, Frazzini, Israel and Moskowitz   | A very long sample and several markets                                       | Value investing survived the long sample once its construction was improved, and they argue many supposed weaknesses are measurement choices                                           |
| Lev and Srivastava                       | American value portfolios up to the late 2010s                               | They conclude the strategy has been unprofitable for almost thirty years, blaming accounting changes and slower reshuffling between cheap and expensive shares                         |
| Kok, Ribando and Sloan                   | Formulaic ratios such as book value and earnings                             | They find little evidence these simple screens beat the market, and argue they often pick up firms with temporarily inflated accounting numbers rather than genuinely underpriced ones |
| Israel and Moskowitz                     | Value by company size                                                        | The value premium decreases with size and is weak among the largest shares, which matters because the library trades only the largest fifth                                            |
| QuantConnect, the library page itself    | Its own long-only backtest                                                   | The page states plainly that the portfolio underperformed the S&P 500 over the test period                                                                                             |
| Cakici and Tan                           | Twenty-three developed markets                                               | Value returns tend to be lower before recessions and in times of poor funding liquidity                                                                                                |

Read together, this is a genuine disagreement rather than a settled fact. The long sample and several
independent markets give the effect a positive average, but the reward is modest, it clusters in the
smaller companies the library avoids, and two credible bodies of work argue that in recent decades it
has become weak or disappeared.

## How this project relates to it

This repository contains no implementation of an equity value factor, and the honest answer is that
the nearest things are measurement studies rather than strategies. The closest is the treatment of
the whole question in
[Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
a brief built from a harvest of academic papers. Its summary of the evidence is that cross-sectional
return predictors of this kind mostly replicate, persist out of sample and carry large test
statistics, so the real danger in a search like this one is the analyst's own choices about which
ratio to use rather than a single statistical threshold. That is exactly the warning the studies
above raise about book value: it works, then it does not, depending on the period and the construction
chosen after seeing the results. The two other tutorials in this collection named in the provenance
table apply the same cheapness idea through different ratios.

## Where it goes wrong

- The definition of book value is soft. It depends on accounting rules for intangible assets, goodwill
  and write-downs, and two firms with the same business can report very different book values.
- It is entangled with size and risk. The effect is strongest among small and financially distressed
  firms, and the library, by keeping only the largest fifth, removes much of the very group where the
  premium was measured.
- Costs and capacity. The cheap names are often the hardest to buy and sell, and a strategy that
  trades them in size pays a wide buying-and-selling gap that a backtest using mid-prices ignores.
- Survivorship and restatement. A backtest built today sees only the companies that survived and the
  accounts as later restated, both of which flatter a screen that looks for cheapness.
- A regime that ends. Value investing was out of favour for long stretches, and the studies above
  disagree over whether that is a temporary cycle or a permanent change in how markets work.
- The number of ways to define it. Book value, earnings, cash flow and sales are all plausible
  versions of cheapness, and choosing the one that worked best in the past is the analyst's own choice
  coming back to haunt the result.

## Try it yourself

You need a spreadsheet and published accounts; company reports and finance websites give book value,
share counts and prices. Pick ten large companies you recognise.

1. Build a sheet with columns: name, share price, number of shares, book value, market value, and
   book-to-market. Compute market value as price times shares and book-to-market as book value
   divided by market value.
2. Sort by market value and keep the largest half. This is your universe, mimicking the rule that
   keeps only big companies.
3. Among those, sort by book-to-market, largest first. The top one or two are what the rule would
   buy.
4. Write down those names, then look up their share prices one year later and compute the return of
   each, including any dividends.
5. Compare the average of those returns with the return of a broad index over the same year.

What to notice: the cheap names are usually companies you have heard of for boring reasons, and in
some years they lag the index badly. A single year tells you almost nothing; the studies above needed
decades, and even then disagreed.

## Where this came from

- [QuantConnect strategy library: book to market value anomaly](https://www.quantconnect.com/tutorials/strategy-library/book-to-market-value-anomaly),
  the rules as implemented, and its statement that the portfolio lost to the index.
- [Quantpedia: value (book-to-market) factor](https://quantpedia.com/strategies/value-book-to-market-factor),
  the performance figures, the instrument count and the underlying papers.
- Fama and French, [The Cross-Section of Expected Stock Returns](https://www.bengrahaminvesting.ca/Research/Papers/French/The_Cross-Section_of_Expected_Stock_Returns.pdf),
  the paper that established book-to-market as a sorting variable.
- Asness, Frazzini, Israel and Moskowitz, [Fact, Fiction, and Value Investing](https://ssrn.com/abstract=2595747),
  the modern restatement the Quantpedia entry is drawn from.
- [Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's brief on how cross-sectional predictors replicate.

## Words used in this tutorial

- book value: what a company owns minus what it owes, as recorded in its accounts, divided by the
  number of shares to give a figure per share.
- market value: the share price multiplied by the number of shares, the price the whole company is
  worth on the market.
- price-to-book: the share price divided by book value per share, the inverse of book-to-market.
- value stock: a company that looks cheap relative to its accounts; the opposite is called a growth
  stock.
- long: owning a share, so the position gains when its price rises.
- short: selling a share you do not own and buying it back later, so the position gains when the
  price falls.
- weight: the share of your money placed in one holding.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
