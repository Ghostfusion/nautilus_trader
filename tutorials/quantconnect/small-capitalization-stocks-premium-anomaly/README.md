# The small company premium: buying the smallest shares and waiting a year

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of the smallest companies listed on American stock markets                                                                                                                                                                                                                                    |
| How often it trades       | About once a year, when the smallest companies are re-selected                                                                                                                                                                                                                                       |
| What you need             | A spreadsheet and a table of share prices and counts of shares                                                                                                                                                                                                                                       |
| Where the rules come from | [QuantConnect strategy library, small capitalization stocks premium anomaly](https://www.quantconnect.com/tutorials/strategy-library/small-capitalization-stocks-premium-anomaly) and the [Quantpedia entry](https://quantpedia.com/strategies/small-capitalization-stocks-premium-anomaly) it cites |
| The underlying research   | Alquist, Israel and Moskowitz, [Fact, Fiction, and Size Effect](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=3177539) and Fama and French, [The Cross-Section of Expected Stock Returns](https://www.jstor.org/stable/2329112)                                                                |
| How well it held up       | Mixed: small shares did earn more in the original and earliest samples, but the plain effect has been weak or absent since it was published in the 1980s, and the papers disagree about whether quality controls bring it back                                                                       |
| Also appears in           | Nothing else in this collection                                                                                                                                                                                                                                                                      |

## The idea in one paragraph

A company's size is measured by what it would cost to buy every share at today's price. The largest
companies cost hundreds of billions and the smallest cost a few million. A famous finding from the
1980s was that the smallest companies earned more for their owners than the largest ones, even after
allowing for how risky they are. This strategy buys the smallest available companies, in equal
amounts, and holds them for a year before picking a fresh set. The puzzle is that a small company
that can double its business is a far smaller company than a giant that can only grow by a few
percent, so the small firms should have more room to grow and more reason to be rewarded.

## Why anyone believed it

Small companies are harder to own. Their shares trade rarely, the gap between what a buyer pays and
what a seller receives is wide, and a fund that manages billions cannot put a meaningful amount into
a company worth fifty million without moving the price. Anyone who does own them has to accept that
they may not be able to sell quickly. A buyer who accepts all of that should demand a reward, just
as a shop discounts goods that are awkward to carry home.

The counterparty, then, is the investor who avoids small companies for reasons that have nothing to
do with how good the company is: an index fund that must hold the largest names, a retirement
account that is not allowed to touch tiny shares, or a manager who cannot be bothered with the
paperwork. If enough people are kept out, the ones who stay can be paid for the effort. The older
story is similar: small companies have more room to grow, they are more flexible when conditions
change, and they fail more often, so the ones that survive earn more.

## An everyday comparison

Think of a corner shop and a supermarket chain. The corner shop can double its takings by adding a
coffee machine and staying open an hour later; the chain would need to open hundreds of new shops to
move its own numbers by the same amount. So the shop has more room to grow. But the shop is also
much more likely to close when a big competitor opens nearby, and someone selling their stake in it
may have to wait months for a buyer. The strategy is to buy a basket of corner shops, spread the
money so that the ones that close do not sink the basket, and be patient.

## The rules, step by step

1. Build a list of every company whose shares trade on the large American markets, as long as basic
   financial information about it exists and its share price is above five dollars. The price floor
   keeps out the very cheapest, most fragile listings.
2. For each company, work out its market capitalisation: the share price multiplied by the number of
   shares in existence. A company with ten million shares at six dollars each has a market
   capitalisation of sixty million dollars.
3. Sort the companies from the smallest market capitalisation to the largest.
4. Buy the ten smallest, in equal amounts. The QuantConnect version then holds them for a year and
   lets the selection run again.
5. At the end of the year, redo steps 1 to 3 with the new prices, sell whatever has grown out of the
   smallest ten, buy whatever has shrunk into it, and hold for another year.
6. There is also a version that buys the smallest group and simultaneously sells the largest group
   short. That version is how the academic literature measures the effect, and it is called small
   minus big. The strategy described on the library page is the long-only version, which buys the
   small companies and does not short anything.

A company is usually called small if its market capitalisation is below about two billion dollars,
and the very smallest, below about three hundred million, are called micro caps.

## The maths, with every symbol named

The first quantity is the size of a company:

```text
Market capitalisation = price per share * number of shares
```

- `price per share` is what one share costs today.
- `number of shares` is how many shares the company has issued.
- The product is what the whole company would cost at today's price.

Sort the companies by this number and take the smallest `n`. Give each one the same share of the
money:

```text
w_i = 1 / n
```

- `w_i` is the fraction of the money placed in company `i`.
- With ten holdings each gets 0.1, that is ten percent.
- The weights add up to one, so the whole account is invested.

The return of the basket over the following year is the average of the holdings' returns:

```text
R_small = (R_1 + R_2 + ... + R_n) / n
```

- `R_1` to `R_n` are the next-year returns of the chosen companies.
- Dividing by `n` is the same as multiplying each by its weight and adding.

The literature usually measures the effect as the gap between the small basket and the large basket:

```text
SMB = R_small - R_big
```

- `SMB` stands for small minus big, the extra return the smallest companies earned over the largest.
- `R_big` is the return of a basket of the largest companies over the same year.
- A positive `SMB` means small companies did better; a negative one means they did worse.

Finally the cost of trading, which matters more here than in most strategies because small shares
are expensive to buy and sell:

```text
Cost = t * c
```

- `t` is the fraction traded: 1.0 when the whole basket is bought from scratch, and less when only
  part of it is replaced.
- `c` is the cost per trade as a fraction of the amount traded. For very small shares, a wide gap
  between the buying and selling price and a thin market can make `c` ten times the figure for a
  large company; 0.005, five tenths of a percent, is a cautious working figure.

## A worked example

Ten invented companies. The QuantConnect rule buys the ten smallest of the whole market; this
example uses ten companies and buys the four smallest, so that the table stays short and the
arithmetic stays visible.

| Company | Shares (millions) | Price | Market cap (millions) | Rank |
| ------- | ----------------- | ----- | --------------------- | ---- |
| ABC     | 2                 | 12.00 | 24                    | 1    |
| DEF     | 3                 | 10.00 | 30                    | 2    |
| GHI     | 4                 | 9.00  | 36                    | 3    |
| JKL     | 5                 | 8.00  | 40                    | 4    |
| MNO     | 20                | 15.00 | 300                   | 5    |
| PQR     | 50                | 20.00 | 1,000                 | 6    |
| STU     | 100               | 25.00 | 2,500                 | 7    |
| VWX     | 200               | 30.00 | 6,000                 | 8    |
| YZA     | 300               | 28.00 | 8,400                 | 9    |
| BCD     | 500               | 32.00 | 16,000                | 10   |

The four smallest are ABC, DEF, GHI and JKL, each getting a quarter of the money. Suppose the next
year brings these returns:

| Company | Weight | Next-year return | Contribution |
| ------- | ------ | ---------------- | ------------ |
| ABC     | 0.25   | +20 percent      | +5.00        |
| DEF     | 0.25   | -10 percent      | -2.50        |
| GHI     | 0.25   | +5 percent       | +1.25        |
| JKL     | 0.25   | +30 percent      | +7.50        |
| Total   | 1.00   |                  | +11.25       |

The basket returned 11.25 percent before costs. The two largest companies, YZA and BCD, returned 6
percent and 8 percent, an average of 7 percent, so the small-minus-big gap was 11.25 minus 7, or
4.25 percent for the year. Buying the four costs half a percent once, so the net result of the first
year is 10.75 percent.

Now extend that to five years, rebuilding at the start of each year and paying half a percent each
time. The returns are invented, but the swings are the size small companies actually show.

| Year  | Small basket | Large basket | SMB   | Cost | Net small |
| ----- | ------------ | ------------ | ----- | ---- | --------- |
| 1     | +11.25       | +7.0         | +4.25 | 0.50 | +10.75    |
| 2     | -8.0         | +3.0         | -11.0 | 0.50 | -8.50     |
| 3     | +22.0        | +12.0        | +10.0 | 0.50 | +21.50    |
| 4     | +5.0         | +9.0         | -4.0  | 0.50 | +4.50     |
| 5     | +35.0        | +15.0        | +20.0 | 0.50 | +34.50    |
| Total | +77.00       | +54.73       |       | 2.50 | +73.05    |

Compounding the net small basket over the five years, 1.1075 times 0.915 times 1.215 times 1.045
times 1.345, gives 1.7305, a gain of 73.05 percent, while the large basket compounded to 54.73
percent. The small basket beat the large one by about eighteen points over five years, roughly three
and a half points a year, which is the size of the effect the research reports. This invented sample
was built to show the arithmetic working; the honest point is that real samples after the 1980s do
not look like it, and the second year of the table shows how quickly a bad year erases the gains of
a good one.

## What the research actually found

The headline claim and the record after it was published pull in different directions, and the
tension is the most important thing to carry away.

| Source                                           | What it measured                                               | Result                                                                                                                                                                                                                                                                        |
| ------------------------------------------------ | -------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Quantpedia, summarising the source papers        | Smallest tenth of the market against the largest, 1926 to 2017 | 6.1 percent a year, volatility 25.6 percent, worst fall 39.68 percent, reward-to-risk 0.24, but its own confidence rating is only moderate                                                                                                                                    |
| Quantpedia's out-of-sample note                  | The same basket tested after the original sample               | The result was slightly negative; Quantpedia writes that the strategy's edge appears to be deteriorating out of sample                                                                                                                                                        |
| Fama and French                                  | Size and price-to-book across American shares                  | Small size was one of two variables that captured the higher returns of certain shares, a finding that made the effect part of the standard toolkit                                                                                                                           |
| Asness, Frazzini, Israel, Moskowitz and Pedersen | The challenges to the effect                                   | The premium is weak historically, weakened after its discovery in the early 1980s, is concentrated in the very smallest shares, sits mostly in January, and is weak outside the United States; they argue it revives once the quality of the company is controlled for        |
| Blitz and Hanauer                                | Whether a tradeable size premium exists                        | The premium has failed to appear since its discovery almost forty years ago; the American result that appears when quality is controlled for cannot be captured in advance and is driven by the short side, while outside the United States it is indistinguishable from zero |
| Herskovic, Kind and Kung                         | The size premium over decades                                  | Large in the 1960s and 1970s, gone in the 1980s and 1990s, and partly back in the 2000s, tied to how uncertain the economy is about individual firms                                                                                                                          |
| Alquist, Israel and Moskowitz, the source paper  | Many claims about the effect                                   | Despite its long history and its wide acceptance, there is still confusion and debate about the size effect; the paper sets out to test the claims on public data rather than settle them                                                                                     |

The honest summary is this. The original finding was real on the data it used. Almost everyone who
has looked since agrees on one thing: the plain version of the effect, buying the smallest
companies and holding them, has not delivered the premium since the early 1980s. What the sources
disagree about is why. One camp argues the effect is still there once you avoid low-quality
companies, though the later work finds that version cannot be captured in advance and does not hold
outside the United States. Another argues the effect comes and goes with the economy. Quantpedia is
blunt about its own out-of-sample test coming out slightly negative. So the headline claim, that
small companies earn more, is not a safe description of the period since it was discovered, and a
tutorial that repeated it without that caveat would be misleading.

## How this project relates to it

The most relevant brief in this repository is
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md).
It records, on the paper `2206.15365v10`, that the cross-sectional findings are mostly real rather
than mined, but that the honest bound on how many are false moves from below ten percent to above
forty percent once the returns are weighted by company size and adjusted for known factors, and that
large, liquid shares show weaker predictability than small ones. That is precisely the tension in
this strategy: the effect lives in shares that are hard to trade, which is both why it can persist
and why it is hard to collect.

The data-integrity side is in
[Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
which reports on `2209.13623v3` that replicated predictors lose about a quarter of their in-sample
return out of sample, and that a decayed strategy is evidence about changing returns rather than
proof that the original finding was false. The construction side is in
[Portfolio construction and allocation](../../../strategies/books2/10_portfolio_and_allocation.md),
which shows how much a result depends on how the test portfolio is built. This repository does not
implement a small-company strategy; the only strategy tool is
[the sector regime engine](../../../implementation/sector-regime-engine/README.md), which measures
market states rather than company sizes.

## Where it goes wrong

- The very smallest shares are where the effect lives and where it is hardest to trade. A fund
  managing real money cannot buy enough of a fifty-million-dollar company to matter without moving
  the price, so the measured premium may be impossible to collect in practice.
- Survivorship and delisting. Small companies fail and disappear from the records. A dataset that
  quietly drops the failures will show small shares doing better than they did, because the losers
  vanished.
- January. Much of the measured effect has historically arrived in the first month of the year, which
  is exactly the kind of detail that makes a strategy look strong on an annual table and impossible
  to rely on.
- The effect weakened right after it was published. That timing is the classic sign of a finding
  being traded away once enough people know it, and every subsequent test that includes the later
  years shows less.
- Cost sensitivity. The gap between buying and selling prices for the smallest shares can be several
  percent. A premium of a few points a year can be entirely consumed by the cost of building and
  rebuilding the basket.
- The rule is not unique. Small can be measured by market capitalisation, by sales, by assets, or by
  the number of shares; the answer moves with the choice, which makes it easy to pick the version
  that looks best after the fact.

## Try it yourself

You need nothing but a public finance website and a spreadsheet.

1. Find the yearly returns of a small-company index and a large-company index over the last twenty
   years, one row per year, two columns.
2. Add a column for the difference: the small-company return minus the large-company return.
3. Count how many of the twenty years have a positive difference and how many negative.
4. Add the differences up, then divide by twenty, to get the average extra return the small index
   gave over the period.
5. Split the twenty years into the first ten and the last ten and repeat step 4 for each half.
6. Finally, subtract half a percent from the small index each year as a rough cost of trading.

What to notice: the average over the whole period is unlikely to be the same as the average in
either half, and the recent half is usually the weaker one. That is the honest picture behind the
headline, and it is why a careful reader asks not just whether an effect existed once, but whether
it has survived since it was found.

## Where this came from

- [QuantConnect strategy library: small capitalization stocks premium anomaly](https://www.quantconnect.com/tutorials/strategy-library/small-capitalization-stocks-premium-anomaly),
  the rules as implemented: a price floor of five dollars, the ten smallest companies by market
  capitalisation, held and rebuilt yearly.
- [Quantpedia: size factor, small capitalization stocks premium](https://quantpedia.com/strategies/small-capitalization-stocks-premium-anomaly),
  the performance figures, the sample period 1926 to 2017, the long-short construction, and the
  out-of-sample note that the result turned slightly negative.
- Alquist, Israel and Moskowitz, [Fact, Fiction, and Size Effect](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=3177539),
  the source paper, which tests the many claims made about the effect.
- Fama and French, [The Cross-Section of Expected Stock Returns](https://www.jstor.org/stable/2329112),
  the paper that put size alongside price-to-book in the standard toolkit.
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  which covers the factor-zoo evidence and the effect of weighting by company size, citing
  `2206.15365v10`.

## Words used in this tutorial

- market capitalisation: the share price multiplied by the number of shares, the cost of buying the
  whole company at today's price.
- micro cap: the very smallest listed companies, often those worth less than a few hundred million.
- out of sample: tested on a period that was not used to discover the effect.
- premium: here, the extra return one group of shares earned over another.
- quantile: a group formed by dividing a ranked list into equal parts; a decile is one of ten.
- survivorship bias: the error introduced when companies that failed are missing from the records.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
