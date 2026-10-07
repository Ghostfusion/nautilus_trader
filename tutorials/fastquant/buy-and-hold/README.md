# Buy and hold: buying once and never selling

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of a broad stock-market fund, or of one company, bought on a single day and kept                                                                                                              |
| How often it trades       | Almost never: one purchase at the start, one sale at the end, and nothing in between                                                                                                                 |
| What you need             | A spreadsheet and the yearly price history of one fund                                                                                                                                               |
| Where the rules come from | [fastquant strategy library table](https://github.com/enzoampil/fastquant) and its [buy_and_hold.py](https://github.com/enzoampil/fastquant/blob/master/python/fastquant/strategies/buy_and_hold.py) |
| The underlying research   | [Damodaran, Historical Returns on Stocks, Bonds and Bills](https://pages.stern.nyu.edu/~adamodar/New_Home_Page/datafile/histretSP.html), the long-run measurement of the benchmark                   |
| How well it held up       | Strong: roughly a century of published index returns measured with dividends, though this is the yardstick rather than an edge                                                                       |
| Also appears in           | nothing else in this collection                                                                                                                                                                      |

## The idea in one paragraph

Put all the money into one broad stock-market fund on a single day, then do nothing. Do not sell when
the news is frightening, do not buy more when everyone is cheerful, and do not move into cash because
next year looks risky. The shares are held until the money is genuinely needed. This is the simplest
possible way to own a piece of the economy, and because it is so simple it is the yardstick that every
other strategy has to beat. If a clever rule cannot do better than buy and hold after its costs, the
rule has added work and risk for nothing.

## Why anyone believed it

Most strategies need to explain who is on the other side of the trade and why that person keeps
appearing. Buy and hold does not need that story, because it is not trying to win a trade against
anyone. It owns a slice of companies that sell goods and services at a profit. Part of that profit
is paid out to shareholders as a dividend (a cash payment out of profit), and part is kept and
reinvested to grow the business. A shareholder owns a piece of both. Over decades the profits of the
whole market tend to grow, and the prices of the shares follow the profits.

The person who sold you the shares is usually someone who wants cash now, or who prefers a different
investment, or who must sell for a reason unrelated to the outlook. You are not taking money from
that person by being clever; you are agreeing to be patient on their behalf in exchange for a share
of the long-run growth. That is why buy and hold needs no special insight and no forecasting, and
why it is the honest thing to compare everything else against.

## An everyday comparison

Think of a fruit tree you plant in a garden. For the first years it produces nothing worth
harvesting, and in some years a late frost destroys the crop. A gardener who digs the tree up after
the first bad spring never gets the years of heavy fruit that follow. The tree is not a secret trick
and it does not beat the neighbouring trees; it simply takes many seasons to repay the planting. Buy
and hold is the decision to leave the tree in the ground and to ignore the weather forecasts.

## The rules, step by step

1. Choose one broad fund that tracks a whole market, such as a fund of American large-company
   shares. One fund is enough; a single company's shares behave far more like a gamble.
2. On the first day of the test, buy once, using all the money you have set aside. In the fastquant
   library this is the whole account, because the buy fraction defaults to 1.
3. Hold. Do not look at the price in between, and do not react to any news.
4. Reinvest the dividends. The library does this by default: each dividend is added to your cash, and
   when that cash is enough to buy a whole share the strategy buys more. This is written as
   `invest_div` set to true.
5. Sell nothing until the test ends. The library's only exit rule is a sale on the last day of the
   data, which is there to close the books rather than to make a decision.
6. Use a real trading cost. The library's default commission is exactly 0 and its slippage is 0.1
   percent, so a backtest run with the defaults understates the cost of buying and selling. Set the
   commission to a realistic figure, such as 0.1 percent, when you compare strategies.

A note on the library source: the strategy class is short, and its "sell" rule fires only when the
end of the data is two rows away. So the library version genuinely buys once and does nothing, which
is what makes it a fair benchmark.

## The maths, with every symbol named

A return is a gain expressed as a fraction of what was put in. If something cost 100 and is now
worth 115, the return is 0.15, which is 15 percent.

```text
R = (V_end - V_start) / V_start
```

- `R` is the total return over the whole period, written as a decimal.
- `V_start` is the value of the account at the start.
- `V_end` is the value at the end, counting the shares, any cash that has accumulated, and any
  dividends that were not reinvested.

The growth multiple simply says how many times the money became itself:

```text
G = V_end / V_start
```

- `G` is the multiple, so 2.0 means the money doubled.
- The two are related by `R = G - 1`.

To turn a total return over `n` years into an average yearly rate, the compounding has to be undone,
because each year's gain also grew in the following years:

```text
r = G ** (1 / n) - 1
```

- `r` is the compounded yearly rate, as a decimal.
- `n` is the number of years.
- `**` means "raised to the power of", so `G ** (1 / n)` is the number whose `n`th power is `G`.

The final value is the shares at the final price, plus the dividends, plus any cash left over:

```text
V_end = S * P_end + D_total + C
```

- `S` is the number of shares held.
- `P_end` is the price per share at the end.
- `D_total` is the sum of all dividends received, if they are held as cash rather than reinvested.
- `C` is any leftover cash.

Finally, the cost of trading. If a fraction `t` of the account is traded and the cost per side is
`c`, then:

```text
Cost = t * c
```

- `t` counts both sides, so selling a whole holding and buying a new one in the same month gives
  `t = 2.0`.
- `c` covers the gap between the buying and selling price plus any commission; 0.001 means 0.1
  percent per side.

## A worked example

Ten thousand dollars are put into a fund at 50.00 a share. The buying price after slippage is 50.05,
and the commission is 0.1 percent. The example allows fractional shares so the arithmetic stays
visible, and it holds the dividends as cash rather than reinvesting them, which makes the final
number a little conservative.

Buying:

```text
shares = 10,000.00 / (50.05 * 1.001) = 199.6006 shares
share value = 199.6006 * 50.05 = 9,990.01
commission = 9.99
total spent = 10,000.00, cash left = 0.00
```

The price then moves up and down, and the fund pays a dividend of 1.00 per share each year.

| Year | Price | Dividend per share | Shares   | Shares value | Dividend cash (cumulative) | Total value |
| ---- | ----- | ------------------ | -------- | ------------ | -------------------------- | ----------- |
| 0    | 50.00 | -                  | 199.6006 | 9,990.01     | 0.00                       | 9,990.01    |
| 1    | 52.00 | 1.00               | 199.6006 | 10,379.23    | 199.60                     | 10,578.83   |
| 2    | 49.00 | 1.00               | 199.6006 | 9,780.43     | 399.20                     | 10,179.63   |
| 3    | 55.00 | 1.00               | 199.6006 | 10,978.03    | 598.80                     | 11,576.83   |
| 4    | 60.00 | 1.00               | 199.6006 | 11,976.04    | 798.41                     | 12,774.45   |
| 5    | 66.00 | 1.00               | 199.6006 | 13,173.64    | 998.00                     | 14,171.64   |

Then the position is sold on the last day, at 66.00 with 0.1 percent slippage and 0.1 percent
commission:

```text
sale price = 66.00 * 0.999 = 65.934
proceeds = 199.6006 * 65.934 = 13,160.47
commission = 13.16
net proceeds = 13,147.31
final value = 13,147.31 + 998.00 = 14,145.31
return = 14,145.31 / 10,000.00 - 1 = 0.4145, that is 41.45 percent over five years
yearly rate = 1.4145 ** (1 / 5) - 1 = 0.0719, that is 7.19 percent a year
```

The arithmetic above is one number, computed from an invented price path; it says nothing about
whether buy and hold is a good idea. What it shows is that the dividends and the one-time costs are
small parts of the total, which is the opposite of what happens to a strategy that trades often.

Now the arithmetic of twenty years, at three fixed yearly rates. These are pure multiplications, so
they are exact:

| Yearly rate | Multiple after 20 years | 100.00 becomes |
| ----------- | ----------------------- | -------------- |
| 4 percent   | 2.1911                  | 219.11         |
| 7 percent   | 3.8697                  | 386.97         |
| 10 percent  | 6.7275                  | 672.75         |

The published long-run figure for American shares is close to the bottom row. According to the
Damodaran series, 100 dollars invested in the S&P 500 at the start of 1928, with dividends included,
had grown to 1,157,598.95 dollars by the end of 2025, which is 98 years and an average of about 10
percent a year. That is the number a strategy has to beat, and it is a high bar.

The last piece of arithmetic is the cost of trading often. A strategy that sells the whole account
and buys it back every month pays two sides a month, or 24 sides a year:

```text
yearly drag = 2 * 0.001 * 12 = 0.024, that is 2.4 percent a year
```

At 0.1 percent per side, that is 2.4 percentage points taken off the return every year. Compared
with a 7 percent gross return, the net 4.6 percent over twenty years grows to 245.84 rather than
386.97, so the busy strategy ends with 64 percent of what buy and hold kept. Costs are not a
footnote; they are often the whole difference.

## What the research actually found

There is no experiment that tests "buy and hold" against buy and hold, because it is the benchmark.
What the literature measures is how often other rules beat it.

| Source                                                        | What it measured                                                         | Result                                                                                                                                     |
| ------------------------------------------------------------- | ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------ |
| Damodaran, Historical Returns on Stocks, Bonds and Bills      | 100 dollars in the S&P 500 with dividends, 1928 to 2025                  | Grew to 1,157,598.95 dollars, about 10 percent a year over 98 years, including the four worst years when it fell 37, 26, 22 and 18 percent |
| paperswithbacktest, awesome-systematic-trading                | 4,843 published strategies re-coded and run over their own price history | The median strategy earned a Sharpe ratio of 0.37, and 48 percent could not be told apart from luck on their own sample                    |
| The same list, reported in this collection's foundations page | The same run, after removing the market's contribution                   | The median exposure to the market was a beta of +0.17, and the margin left over was an information ratio of 0.21, which costs can erase    |

Read together, these say something modest and important. Owning the whole market has produced a
large long-run return, and the typical published strategy, tested over decades, has not clearly
beaten it. Roughly half of the strategies were indistinguishable from luck, and much of what looked
like skill was simply the market itself rising. None of this proves that buy and hold will produce
10 percent a year in the future; it reports what the past measured.

## How this project relates to it

This repository does not implement buy and hold as a product, because there is nothing to implement
beyond one purchase. The two useful places to read are the foundations pages that explain the
machinery this tutorial depends on: [return, risk and drawdown](../../foundations/05_return-risk-and-drawdown.md)
explains the difference between an average return and a compounded one, and
[what the evidence says](../../foundations/09_what-the-evidence-says.md) reports the 4,843-strategy
replication run and why the median strategy is not distinguishable from chance. For the research
discipline that a comparison like this requires, see the
[overfitting and research integrity brief](../../../strategies/books2/28_overfitting_and_research_integrity.md).

## Where it goes wrong

- It is not a guarantee, only an average. The same series that averaged 10 percent a year contains
  losses of more than a third in a single year. A holder who sells during such a year converts a
  temporary fall into a permanent loss.
- The past is a small sample. Ninety-eight years is one country's history, and the twentieth century
  in America was unusually kind. A Japanese investor buying at the peak of 1989 waited decades to
  break even, so "the long run" is not always as long as a life.
- Costs and taxes are real. Frequent trading is punished, but even one purchase and one sale pay a
  gap and a commission, and a taxable account pays tax on dividends each year whether or not it
  wants them.
- The benchmark is easy to misquote. Dividends are most of the difference: the price-only version of
  the index looks much weaker than the total-return version, and a comparison that ignores dividends
  flatters the strategy it is testing.
- It carries the full market's risk. Buying one company rather than a fund is not buy and hold in
  this sense; it is a bet on one business that can fail entirely.
- The behaviour, not the arithmetic, is the hard part. The rule is trivial to write and difficult to
  follow, which is why so much money leaves the market during panics.

## Try it yourself

You need a spreadsheet and a public source of yearly prices and dividends for one broad fund.

1. Make a sheet with the columns Year, Price, Dividend per share.
2. Add a fixed number of shares, say 100, and a column "Position value" that multiplies shares by
   price.
3. Add a column "Dividend cash" that multiplies shares by the dividend and keeps a running total.
4. Add a column "Total" that adds the position value to the running dividend cash.
5. Fill in the price and dividend for as many years as you can find, and compute the total return and
   the compounded yearly rate with the formulas from this tutorial.
6. Now add a second column that subtracts 2.4 percent a year, the cost of trading the whole account
   every month, and watch how far the two columns separate over twenty or thirty years.

What to notice: the second column is not slightly lower, it is a different shape entirely, because
the loss repeats every year and compounds. Also notice how many of the years are negative in the
price column. If a rule you read elsewhere claims to have avoided all of them, ask what it paid to
do so.

## Where this came from

- [fastquant](https://github.com/enzoampil/fastquant), whose strategy table lists buy and hold with
  no parameters, and the
  [buy_and_hold.py source](https://github.com/enzoampil/fastquant/blob/master/python/fastquant/strategies/buy_and_hold.py),
  which buys once and sells only at the end.
- [Damodaran historical returns](https://pages.stern.nyu.edu/~adamodar/New_Home_Page/datafile/histretSP.html),
  the 1928 to 2025 series used for the long-run figures, including the dividends-included S&P 500.
- [awesome-systematic-trading](https://github.com/paperswithbacktest/awesome-systematic-trading),
  the replication run of 4,843 strategies, quoted here through this collection's foundations page.
- [What the evidence says](../../foundations/09_what-the-evidence-says.md) and
  [return, risk and drawdown](../../foundations/05_return-risk-and-drawdown.md), the local pages
  that explain the Sharpe ratio, the beta adjustment, and compounding.

## Words used in this tutorial

- return: a gain or loss expressed as a fraction of the money put in; 0.10 means a gain of 10
  percent.
- compounded: letting each period's gain grow in the following periods, rather than setting it
  aside.
- dividend: a cash payment a company makes to its shareholders out of profit.
- benchmark: a simple, well-known holding used as a yardstick to judge whether a strategy added
  anything.
- slippage: the difference between the price you expected and the price you actually got.
- Sharpe ratio: average return divided by the size of the typical wobble; a small number means the
  return is small next to the risk taken.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
