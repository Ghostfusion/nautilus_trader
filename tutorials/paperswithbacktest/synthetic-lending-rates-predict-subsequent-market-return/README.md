# Synthetic lending rates: reading the price of borrowing a share to guess tomorrow's market

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                            |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | One American share-market fund, bought or sold short each day depending on a signal built from the cost of borrowing shares                                                                                                                                                                                      |
| How often it trades       | Every day; each position is opened in the afternoon and closed the next afternoon                                                                                                                                                                                                                                |
| What you need             | Python and a data file, because the signal comes from option prices rather than a single published number                                                                                                                                                                                                        |
| Where the rules come from | [The awesome-systematic-trading list](https://github.com/paperswithbacktest/awesome-systematic-trading), which keeps the coded rule, and the [Quantpedia synthetic lending rates entry](https://quantpedia.com/strategies/synthetic-lending-rates-predict-subsequent-market-return/) it cites                    |
| The underlying research   | Padysak, [Synthetic lending rates predict subsequent market return](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=3976307)                                                                                                                                                                                 |
| How well it held up       | Mixed: one study finds a statistically strong effect in one market over five years, but the profit is concentrated in two crisis episodes and the rule lags a simple holding of the market during calm periods, so the effect appears in some periods and not others                                             |
| Also appears in           | No other tutorial in this collection uses lending rates; the nearest market-timing pages are [Vix predicts stock index returns](../../quantconnect/vix-predicts-stock-index-returns/README.md) and [Short-term reversal strategy in stocks](../../quantconnect/short-term-reversal-strategy-in-stocks/README.md) |

## The idea in one paragraph

To bet that a share will fall, a trader normally borrows the share from its owner and pays a fee for the
loan. That fee, and sometimes the absence of anyone willing to lend at all, is what makes betting against
a share expensive. This strategy does not look at the fee directly. It works it out from the prices of
options, group by group across thousands of shares and funds, and averages the result into one number each
day. When that number rises, meaning it has become cheaper to bet against shares, the strategy buys the
market for a day; when it falls, the strategy sells the market short for a day. The bet is that the price
of borrowing tells you the mood of investors, and the mood carries into tomorrow.

## Why anyone believed it

Borrowing a share is a transaction between two people, and its price moves with their willingness. When
many investors want to bet against a share, the queue of borrowers grows, the fee rises, and sometimes the
lenders pull their shares back entirely. When few want to bet against it, the fee is low and shares are
easy to borrow. So the fee is a running opinion poll of the people who are willing to put money behind
their pessimism.

The claim in the paper is that this price also carries a message about the whole market, one day ahead.
When it becomes cheaper to borrow, the crowd that wanted to bet against the market is thinning, and the
mood brightens; when it becomes dearer, pessimism is building. The counterparty is the investor who is
being paid to lend shares and is therefore willing to be short-term bearish on the borrower's behalf, and
who does not act on the information the price reveals.

## An everyday comparison

Think of a pawnbroker on a high street. When many people come in to pawn their watches, the pawnbroker can
charge a high interest rate, and a visitor who saw the queue would guess that times are hard in that town.
When the shop is quiet, the rate is low and things are easier. Nobody publishes the town's mood, but the
price the pawnbroker charges carries it, and a rise or fall in that price is itself news. The strategy
treats the cost of borrowing shares the same way: not as a cost to be paid, but as a signal to be read.

## The rules, step by step

The coded rule uses one tradable instrument and a daily signal built from a whole market.

1. Take the fund that tracks the American share market, which the paper and the code use because it is easy
   to trade and easy to sell short.
2. Each day, look at the borrowing data published by the options exchange for several thousand American
   shares and funds. For each one it reports a number that is high when the share is cheap to borrow and
   low when it is dear.
3. Average those numbers across all the shares and funds, giving equal weight to each. This one average is
   the day's reading of how easy it is, in general, to bet against the market.
4. Subtract yesterday's reading from today's. The difference, not the level, is the signal.
5. If the difference is positive, buy the market fund at the close of trading and hold it.
6. If the difference is negative, sell the market fund short and hold it.
7. Close the position at the close of the next trading day, then repeat from step 2. Nothing is held for
   more than one day.
8. The signal is computed at 15:57 and the trade is placed two minutes later, at 15:59, with the close of
   the following day at 15:58.

## The maths, with every symbol named

The reading is an average, and the signal is a change in that average.

The borrow intensity of one share or fund:

```text
BI_j = r_f - f_j
```

- `BI_j` is the borrow intensity of asset `j`, as a decimal per year.
- `r_f` is the risk-free rate, the rate you earn on money that is safe for a short time, as a decimal.
- `f_j` is the lending fee, the annual charge for borrowing asset `j` from its owner, as a decimal.
- Because the fee is subtracted, a high `BI_j` means a low fee, so the asset is cheap to borrow.

The average across the market:

```text
A_t = (BI_1 + BI_2 + ... + BI_M) / M
```

- `A_t` is the aggregate borrow intensity on day `t`.
- `M` is the number of shares and funds in the sample, more than four thousand in the paper.
- Each asset counts equally, so a large share does not carry more weight than a small one.

The signal is the day's change:

```text
d_t = A_t - A_(t-1)
```

- `d_t` is the change in the aggregate reading from one day to the next.
- A positive `d_t` means borrowing became cheaper, on average, that day; a negative `d_t` means it became
  dearer.

The position and its return:

```text
if d_t > 0:  hold the market fund long,   return = +r_market_(t+1)
if d_t < 0:  hold the market fund short,  return = -r_market_(t+1)
```

- `r_market_(t+1)` is the change in the market fund's price on the following day.
- The position is held for exactly one day, so each signal is used once.

The cost, as always, is the traded amount times the cost per trade:

```text
Cost = t * c
```

- `t` is the amount traded. Because the position is closed and reopened every day, the whole account is
  round-tripped, so the traded amount is 2 each day, counting both sides.
- `c` is the cost per trade, about 0.0001 to 0.0002 for a large, cheaply traded market fund, that is one to
  two basis points, where one basis point is one hundredth of one percent.

A short position in the fund also pays a borrow fee, which for a large market fund is small, often a few
tenths of a percent a year, but which is charged on every day the position is short.

## A worked example

Six trading days. The aggregate borrow intensity is the invented but plausible level of the published
data; the signal is its daily change; the position is what the rule would take; and the last column is
what the market actually did the next day. Figures are in percent.

| Day | Aggregate reading | Change d | Position | Next-day market | Contribution |
| --- | ----------------- | -------- | -------- | --------------- | ------------ |
| 0   | 0.4210            | +0.0010  | long     | +0.40           | +0.40        |
| 1   | 0.4205            | -0.0005  | short    | -0.30           | +0.30        |
| 2   | 0.4200            | -0.0005  | short    | +0.20           | -0.20        |
| 3   | 0.4220            | +0.0020  | long     | +0.60           | +0.60        |
| 4   | 0.4215            | -0.0005  | short    | -0.10           | +0.10        |
| 5   | 0.4230            | +0.0015  | long     | +0.30           | +0.30        |

The position column makes the arithmetic clear. On day 0 the reading rose, so the fund was held long and
the next day it gained 0.40 percent. On day 1 the reading fell, so the fund was held short; the next day
the fund lost 0.30 percent, which is a gain for a short position. On day 2 the reading fell again, so the
fund was held short; the next day the fund gained 0.20 percent, which a short position loses.

Adding the contributions gives the total before costs:

```text
Total = 0.40 + 0.30 - 0.20 + 0.60 + 0.10 + 0.30 = 1.50 percent
```

Now the costs. The account round-trips every day, so six days is a traded amount of six times two:

```text
t = 6 * 2 = 12
Cost = 12 * 0.0001 = 0.0012, that is 0.12 percent
```

Then the borrow fee on the three days the position was short. At a quarter of a percent a year, one day
costs about 0.0001 of the position:

```text
Borrow = 3 * 0.25 percent / 252 = 0.0030 percent
Net = 1.50 - 0.12 - 0.003 = 1.377 percent
```

That is one invented six days. It shows how to apply the rules and how the arithmetic behaves, and it says
nothing about whether the idea works. Notice how large the trading cost line is: because the position is
rebuilt every single day, the account pays the gap between buying and selling prices twelve times in six
days, and that line is the first thing to check on any daily rule.

## What the research actually found

| Source                                                                          | What it measured                                                                                                    | Result                                                                                                                                                                                      |
| ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Padysak, Synthetic lending rates predict subsequent market return               | CBOE borrow intensity for more than 4,000 shares and funds, 2016 to 2021, against the next day's market fund return | A positive and statistically strong link: a correlation of 0.137, with a t-statistic of 4.17, between the day's change in the aggregate reading and the next day's market return            |
| The same paper                                                                  | How long the effect lasts                                                                                           | It is short-lived: the effect is largely reversed over the following two days, so the trade must be held for a day and closed                                                               |
| The same paper                                                                  | When the profit arrives                                                                                             | The strategy does best in crises, such as the end of 2018 and the start of the coronavirus pandemic in 2020, and lags a simple holding of the market in calm periods                        |
| Quantpedia, summarising the paper                                               | The coded rule, 2016 to 2021                                                                                        | 15.47 percent a year, volatility 19.52 percent, worst fall 28.51 percent, reward-to-risk 0.79; the vendor grades its confidence in the idea as Strong                                       |
| Quantpedia's comment on the mechanism                                           | The reading against the market's own fear index                                                                     | A rise in the reading, meaning cheaper borrowing, goes with a fall in the market's fear index, which supports the reading as a mood indicator                                               |
| The awesome-systematic-trading list's replication record, across all its papers | 4,843 coded strategies                                                                                              | The median replication returned a Sharpe ratio of 0.37, and only 48 percent cleared a t-statistic of 1.96, so half the published record cannot be distinguished from zero on its own sample |

The whole story rests on one paper. The finding is statistically clean on its own sample, but the sample is
five years, one market, one tradable fund, and the profit is concentrated in two episodes. A reader should
treat a strong grade on one study as a reason to look closely, not a reason to relax.

## How this project relates to it

This repository's brief on derivatives,
[Options, hedging and derivative instruments](../../../strategies/books/16_options_and_derivative_instruments.md),
explains what a synthetic position is and how funding and margin are charged on one; its Section 5 is the
closest material to the idea that a cost implied by prices can be read as a signal.

The brief on lending markets,
[Crypto, DeFi and perpetuals](../../../strategies/books2/03_crypto_defi_and_perpetuals.md), collects
evidence on how borrowing rates move and what they reveal about demand, which is the same reading of a
lending price in a younger market. Nothing in this repository computes a synthetic lending rate from
options; the closest finished tutorial is
[Short-term reversal strategy in stocks](../../quantconnect/short-term-reversal-strategy-in-stocks/README.md),
which the paper itself suggests may be related, because falling prices in the recent past and cheap
borrowing can be two views of the same thing.

## Where it goes wrong

- One study, one sample. The entire result comes from a single paper over 2016 to 2021. Nothing in the
  vendor's page reports an independent replication, and a five-year window can produce a clean statistic
  by luck.
- The profit lives in crises. The paper is explicit that the strategy beats the market mainly during two
  crisis episodes and lags it in calm periods. That means most of the measured reward comes from a few
  months, which is the kind of result that rarely repeats in the same way.
- A daily rule pays daily costs. The position is closed and reopened every day, so the account pays the
  gap between buying and selling prices twice a day, every day. A small edge can be eaten entirely by that
  line, and the reader must check it before believing any gross figure.
- The signal is a step, not a size. The rule acts on whether the reading rose or fell, so a change of one
  hundredth of a percent and a change of one percent are treated the same, even though the paper measures a
  continuous relationship.
- The data is not a published number. The signal comes from option prices for thousands of instruments,
  assembled and cleaned by a vendor. Any error in that assembly, or any change in how the reading is
  computed, changes the signal without changing the market.
- For the whole idea to be false, it is enough that cheap borrowing is simply a measure of recent calm,
  and that the market's tendency to rise after calm days is the well-known short-term reversal effect
  wearing a new hat. The paper itself raises this possibility, which is the honest place to leave it.

## Try it yourself

This one is hard to do by hand, because the required borrowing data is not published in a simple table.
Instead, do the part that is accessible: test the claim with a market data source and a proxy for the cost
of shorting.

1. Build a spreadsheet with one row per trading day for the last two years, holding the closing value of a
   market index.
2. Add a column for the market's own fear index, a published measure of how much investors are paying to
   insure against a fall. This is a public stand-in for the cost of betting against the market.
3. Add a column for the daily change in that fear measure, and a column for the next day's change in the
   market index.
4. Split the rows into those where the fear measure fell and those where it rose. Average the next-day
   market change in each group.
5. Finally, add a column that subtracts about 0.02 percent per day for trading, applied to every row.

What to notice: the two averages in step 4 may be different, and if they are, the difference is the whole
trade. Now look at the cost column in step 5: even a small difference between the two averages can be
smaller than the cost of acting on it every day. That gap between the gross difference and the net cost is
the single most important number in the exercise. If your two averages are almost the same, you have just
seen why a clean statistic in a paper does not guarantee a tradable rule.

## Where this came from

- [Synthetic Lending Rates Predict Subsequent Market Return](https://quantpedia.com/strategies/synthetic-lending-rates-predict-subsequent-market-return/),
  the page that states the rules and reports the source-paper figures and the confidence grade.
- Padysak, [Synthetic lending rates predict subsequent market return](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=3976307),
  the study behind the numbers.
- [The awesome-systematic-trading list](https://github.com/paperswithbacktest/awesome-systematic-trading),
  which holds the coded rule and the project's own replication record.
- [Options, hedging and derivative instruments](../../../strategies/books/16_options_and_derivative_instruments.md),
  this repository's brief on synthetic positions, funding and margin.
- [Crypto, DeFi and perpetuals](../../../strategies/books2/03_crypto_defi_and_perpetuals.md), this
  repository's brief on borrowing rates in lending markets.

## Words used in this tutorial

- basis point: one hundredth of one percent, so two basis points is 0.02 percent.
- borrow fee: the annual charge paid by someone who borrows a share in order to sell it.
- lending rate: here, the same fee seen from the owner's side, or the rate inferred from option prices when
  the market moves it out of sight.
- reversion: the tendency of something that has moved away from an average to come back to it.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- synthetic: a position built from other contracts that copies the behaviour of one you did not take
  directly, such as a short sale copied with options.
- volatility: how much a price moves around its average, measured as a percentage per year.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
