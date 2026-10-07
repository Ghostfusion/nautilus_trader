# The short interest effect: joining the investors who bet against a company

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                    |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of American companies, bought in the least-bet-against names and borrowed and sold in the most-bet-against ones                                                                   |
| How often it trades       | Once a month, when the whole portfolio is rebuilt                                                                                                                                        |
| What you need             | A spreadsheet and the twice-monthly short interest figures that exchanges publish                                                                                                        |
| Where the rules come from | [Quantpedia, short interest effect, long-short version](https://quantpedia.com/strategies/short-interest-effect-long-short-version/), the page the list's implementation is written from |
| The underlying research   | Akbas, Boehmer, Erturk and Sorescu, [Why Do Short Interest Levels Predict Stock Returns?](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1019309)                                   |
| How well it held up       | Mixed: the effect is documented across several studies, but its size swings between measures and the vendor's own out-of-sample run was slightly negative                                |
| Also appears in           | Nothing else in this collection describes the short interest ratio; the completed tutorials in this group trade size, value and momentum instead                                         |

## The idea in one paragraph

Some investors bet that a share will fall. To do that they borrow the share from its owner, sell it,
and buy it back later, hoping to return it at a lower price. The number of shares currently borrowed
and sold this way is called short interest, and exchanges publish it twice a month. A company whose
shares are heavily borrowed has many people betting against it; one with almost none has almost
nobody. This strategy buys the shares that the fewest people are betting against and sells short the
shares that the most are, refreshing the list monthly. The bet is that the professional short sellers
know something the price has not yet absorbed.

## Why anyone believed it

Short selling is more expensive and more difficult than buying. To bet against a share you must find
an owner willing to lend it, and you pay a fee for the loan. Someone who goes to that trouble and
pays that fee is likely to have a reason more considered than a hunch. The study behind the rules
argues that these sellers are well informed: they anticipate bad news, disappointing profits and cuts
to analyst forecasts several months before the news becomes public.

The counterparty is the optimistic buyer who cannot sell, or who does not bother. A well-known
argument, from Miller in 1977, is that when pessimistic investors are blocked from selling, only the
optimists set the price, so the shares of heavily shorted companies stay too high and fall later.
Whether the effect is caused by that blockage or by the sellers simply knowing more is still debated,
but both stories point to the same trade: the most-shorted shares are the ones most likely to
disappoint.

## An everyday comparison

Think of a used-car lot where, before a car is listed, a mechanic can inspect it for a small fee. If
a car attracts many mechanics, and the lot allows them to bet against the car, the ones who pay the
fee and bet against it have looked underneath. The cars with the most bets against them are the ones
most likely to have a hidden fault, and their asking prices are the ones most likely to fall. The
strategy buys the cars nobody bothered to inspect, and bets against the cars the inspectors turned
down.

## The rules, step by step

1. Start with all shares listed on the New York, American and Nasdaq exchanges.
2. For each share, take the published short interest, the number of its shares that are currently
   borrowed and sold, and divide by the number of shares the company has issued in total. The result
   is the short interest ratio. A ratio of 0.10 means one share in ten has been sold short.
3. Each month, rank all shares by that ratio from highest to lowest and cut them into ten equal
   groups, called deciles.
4. Buy every share in the lowest decile, the ones with the least short interest. Sell short every
   share in the highest decile, the ones with the most.
5. Weight the shares in each decile equally, so a decile of 50 names puts 2 percent of that leg's
   money in each.
6. Hold for a month, then rebuild. Shares that leave the top or bottom decile are closed, and shares
   that enter are opened.

One detail matters for anyone following this with real data. The written rule uses short interest
divided by shares outstanding, which the exchanges publish twice a month and which is a stock of
borrowed shares at a moment in time. The list's own implementation, because it uses a free data
source, substitutes the ratio of short-selling volume to total volume reported daily, which is a flow
of trades over a day rather than a stock of borrowed shares. The two move together loosely; the
daily volume figure is noisier and is not the same measurement, and this is a genuine difference
between the written rule and the code.

## The maths, with every symbol named

The short interest ratio, one number per share:

```text
SIR = SharesSoldShort / SharesOutstanding
```

- `SharesSoldShort` is the number of the company's shares that are currently borrowed and sold, as
  reported by the exchange.
- `SharesOutstanding` is the total number of the company's shares in existence.

Shares are ranked by `SIR`; the lowest tenth go long and the highest tenth go short.

The return of one leg over a month is the equal average of its holdings:

```text
R_leg = (R_1 + R_2 + ... + R_n) / n
```

- `R_1` to `R_n` are the monthly returns of the shares in that decile.
- `n` is the number of shares in the decile, and dividing by it makes every holding count equally.

The account's return is the long leg's return minus the short leg's return:

```text
R = R_long - R_short
```

- `R_long` is the average return of the least-shorted decile.
- `R_short` is the average return of the most-shorted decile; subtracting it means a fall in those
  shares adds to the account.

The monthly cost has two parts, and the second is where this strategy differs from most:

```text
Cost = t * c + b
```

- `t` is the traded fraction, 2.0 when the whole book is replaced, because the sale and the purchase
  both count.
- `c` is the cost of one side, about 0.001 for large American shares, that is 0.10 percent.
- `b` is the borrow fee for the shorted shares. It is charged as a yearly rate on the shorted amount,
  and for the heavily shorted names this strategy holds it can be high, often several percent a year
  because those are exactly the shares that other people also want to borrow.

## A worked example

Ten companies, one month. The short interest figures are invented but of the size they take in real
markets.

| Company | Shares sold short (millions) | Shares outstanding (millions) | Short interest ratio | Side taken | Return next month |
| ------- | ---------------------------- | ----------------------------- | -------------------- | ---------- | ----------------- |
| A       | 1                            | 100                           | 0.01                 | long       | +1.2 percent      |
| B       | 2                            | 100                           | 0.02                 | long       | +0.8 percent      |
| C       | 5                            | 100                           | 0.05                 | no trade   | +0.5 percent      |
| D       | 8                            | 100                           | 0.08                 | no trade   | 0.0 percent       |
| E       | 12                           | 100                           | 0.12                 | no trade   | -0.3 percent      |
| F       | 15                           | 100                           | 0.15                 | no trade   | -0.5 percent      |
| G       | 20                           | 100                           | 0.20                 | no trade   | -0.9 percent      |
| H       | 25                           | 100                           | 0.25                 | no trade   | -1.2 percent      |
| I       | 30                           | 100                           | 0.30                 | no trade   | -1.6 percent      |
| J       | 40                           | 100                           | 0.40                 | short      | -2.0 percent      |

With only ten companies a true decile would hold one name, which is why the real rule needs a large
universe, but the arithmetic is the same. The lowest ratio is A and the highest is J, so A is bought
and J is borrowed and sold. The account holds 1,000 dollars of A and 1,000 dollars of shorted J on
1,000 dollars of capital.

| Position           | Amount   | Return next month | Effect on the account |
| ------------------ | -------- | ----------------- | --------------------- |
| Long A             | 1,000.00 | +1.2 percent      | +12.00                |
| Short J (borrowed) | 1,000.00 | -2.0 percent      | +20.00                |
| Total before costs |          |                   | +32.00                |

The short gains 20.00 dollars because J fell 2.0 percent and the account had sold it short. So the
gross profit is 32.00 dollars, or 3.2 percent of the 1,000 dollars of capital. Now the costs, and the
borrow fee is the interesting line:

```text
Trading cost = 4 * 1000 * 0.001 = 4.00 dollars (four trades of 1,000 dollars at 0.10 percent)
Borrow fee   = 1000 * (1 percent per month) = 10.00 dollars
Net profit   = 32.00 - 4.00 - 10.00 = 18.00 dollars, that is 1.80 percent of capital
```

The borrow fee of 10.00 dollars, charged at 1 percent a month, is more than twice the trading cost.
Heavily shorted shares are exactly the ones that are expensive to borrow, so `b` is not a rounding
error here; it is the single largest cost after the trade itself. Twelve months at 1.80 percent is
about 21.6 percent a year before compounding, which sits near the published figure below and shows
how much of the gross result the borrow fee removes.

## What the research actually found

| Source                                                         | What it measured                                                             | Result                                                                                                                                                                                                             |
| -------------------------------------------------------------- | ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Akbas, Boehmer, Erturk and Sorescu, the paper behind the rules | American shares, 1988 to 2005, monthly                                       | The most-shorted group underperformed, and the long-short portfolio returned 1.51 percent a month, about 19.7 percent a year, with a reward-to-risk ratio of 0.92                                                  |
| The same paper                                                 | The same returns judged against the market, size, value and momentum factors | Roughly 22.70 percent a year survived those adjustments, and the paper's own conclusion is that short sellers are informed rather than that prices are merely overvalued                                           |
| Asquith, Pathak and Ritter, an earlier study                   | American shares, 1988 to 2002                                                | The most-constrained group underperformed by 215 basis points a month when the names are weighted equally, but only 39 basis points a month when they are weighted by size, which is not distinguishable from zero |
| Boehmer, Huszar and Jordan, a later study                      | American shares, 1988 to 2005                                                | The negative returns to heavily shorted shares can be short-lived and are of debatable size, while the positive returns to lightly shorted, heavily traded shares are larger in absolute terms                     |
| Quantpedia, the page the rules come from                       | The same long-short rule                                                     | A reward-to-risk ratio of 0.92 and volatility of 17.14 percent, a worst fall of 30.23 percent, and a note that the vendor's own out-of-sample backtest was slightly negative                                       |

The three studies disagree in a way worth spelling out. The version that weights every share equally
finds a large effect, 215 basis points a month. The version that weights by company size finds only
39 basis points a month, which is indistinguishable from nothing. That gap is the centre of the
disagreement: the strong result lives in the small, hard-to-trade shares, where it is also hardest to
capture, and the shares an ordinary investor can actually borrow and trade show a much smaller one.
The most-shorted decile is also the decile with the highest borrow fees, so the measured returns are
the ones that are most likely to be eaten by costs.

## How this project relates to it

The repository's brief on market design and fees,
[strategies/books2/20_market_design_regulation_and_fees.md](../../../strategies/books2/20_market_design_regulation_and_fees.md),
models short selling under the margin rules and finds a threshold in the short interest ratio: below
it, the price adjusts smoothly as buyers arrive, and above it, the price jumps, which is the
mechanics of a short squeeze. That is the risk this strategy runs when it crowds into the
most-shorted shares, and the brief's recommended handling is to monitor the short interest ratio as a
distinct risk state rather than a signal.

The same repository also records short interest as a data feed in its
[moomoo adapter design](../../../docs/design/moomoo_adapter_design.md), where it is listed as market
data that the platform can display but does not yet treat as a core tradable signal. That is a fair
summary of the idea's status here: the data exists, the rule is written down, and the measurement is
not.

## Where it goes wrong

- Size decides the answer. The equally weighted version finds 215 basis points a month and the size
  weighted version finds 39 basis points, which is the same as finding nothing. The strong result
  lives where trading is hardest.
- Borrow costs. The most-shorted shares are the most expensive to borrow, and the fee can exceed the
  return the short leg is chasing. This is the single largest omission in the published numbers.
- Short squeezes. When a heavily shorted share rises sharply, the short sellers are forced to buy it
  back, which pushes it up further. The margin model in this repository shows a threshold above which
  the price jumps rather than adjusts.
- Short interest is reported with a delay. The exchanges publish it twice a month, so a signal built
  on it is looking at a picture that is already days or weeks old.
- The measurement is not the one in the rule. The implementation substitutes daily short-selling
  volume, a flow, for short interest, a stock of borrowed shares. The two are correlated but not
  identical, and a backtest run on the substitute is not a test of the written rule.
- The effect may be transient. The later study in the table found the negative returns to heavily
  shorted shares can be short-lived, which is a different claim from a durable one.

## Try it yourself

You need a spreadsheet and the short interest figures that the exchanges publish twice a month, which
are available on most finance websites for large companies.

1. Build one row per company, with columns for the shares sold short and the shares outstanding, and
   a column for the ratio of the two.
2. Sort by that ratio and mark the highest tenth and the lowest tenth.
3. Collect the next month's return for every company.
4. Average the returns of the marked low group and the marked high group separately.
5. Subtract the high group's average from the low group's average. That is the strategy's return for
   the month before costs.
6. Now subtract the costs: about 0.10 percent per side on both legs, and a borrow fee of about 1
   percent a month on the short leg.

What to notice: the low group's return and the high group's return will be close to one another in
many months, and the gap between them is often smaller than the borrow fee you subtract. If you sort
by company size as well as by short interest, you will likely find the effect concentrated in the
smaller names, which is the disagreement the studies in the table describe.

## Where this came from

- [Quantpedia, short interest effect, long-short version](https://quantpedia.com/strategies/short-interest-effect-long-short-version/),
  the page the list's implementation is written from, and the source of the 19.7 percent, the 22.70
  percent adjusted figure, the 17.14 percent volatility, the 0.92 ratio and the out-of-sample note.
- Akbas, Boehmer, Erturk and Sorescu, [Why Do Short Interest Levels Predict Stock Returns?](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1019309),
  the source of the 1988 to 2005 sample and the informed-seller conclusion.
- Asquith, Pathak and Ritter, [Short Interest, Institutional Ownership, and Stock Returns](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=525623),
  the source of the 215 and 39 basis point figures.
- Boehmer, Huszar and Jordan, [The Good News in Short Interest](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1405511),
  the source of the finding that the negative returns to heavily shorted shares can be short-lived.
- The implementation the list carries:
  [short-interest-effect-long-short-version.py](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/short-interest-effect-long-short-version.py),
  which shows the daily short-volume substitute for the twice-monthly figure.
- [strategies/books2/20_market_design_regulation_and_fees.md](../../../strategies/books2/20_market_design_regulation_and_fees.md),
  this repository's brief on margin rules and the short interest threshold.

## Words used in this tutorial

- decile: one tenth of a sorted list, used here to name the most-shorted and least-shorted groups.
- long: owning something, so that a rise in its price makes money.
- short interest: the number of a company's shares that are currently borrowed and sold by people
  betting the price will fall.
- short selling: borrowing something you do not own, selling it now, and buying it back later, which
  makes money if the price falls.
- short squeeze: a sharp rise in a heavily shorted share as the sellers are forced to buy it back.
- borrow fee: the yearly charge paid to the owner of a share for lending it to a short seller.
- basis point: one hundredth of one percent, so 215 basis points is 2.15 percent.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
