# Return, risk and drawdown

Date: 2026-10-07. Revision 1.

Money in a market does not arrive on a smooth line; it arrives in fits, and it can fall as well as
rise. This page explains what a return in percent means, why an average return is not the same as
the money you end up with, and what volatility, drawdown and the Sharpe ratio measure about the
bumpy path in between.

## A return, in percent

A **return** is how much an investment gained or lost, written as a percentage of what it was worth
at the start. If 100 dollars becomes 105 dollars, the return is 5 percent; if it becomes 95 dollars,
the return is -5 percent, and the minus sign matters. A return is a pure fraction of the starting
value, so it lets you compare a small account with a large one.

Returns are usually quoted for a fixed period, such as a day, a month or a year. A return of 5
percent over a year and 5 percent over a month are very different things, and a number without the
period beside it is incomplete. When you read a return, ask "over what length of time".

## The average return and the compounded result

There are two ways to combine a series of yearly returns, and mixing them up is one of the most
common errors in finance.

The **average return** adds the yearly returns and divides by the number of years. It answers the
question: what return did a typical year produce?

The **compounded** result lets each year's gain grow in the next year. Compounding is what happens
when the money you earned last year also earns money this year. The two are not the same, and the
gap grows as the returns get larger or more variable.

Take 10 percent a year for three years, starting with 100 dollars. One table shows the two
accounting rules side by side. Here "added" means the 10 dollars is set aside each year and the
starting 100 dollars never grows; "compounded" means each year's 10 percent is applied to the
new, larger amount.

| Year         | Value if 10 dollars is added each year | Value if it compounds |
| ------------ | -------------------------------------- | --------------------- |
| Start        | 100.00                                 | 100.00                |
| After year 1 | 110.00                                 | 110.00                |
| After year 2 | 120.00                                 | 121.00                |
| After year 3 | 130.00                                 | 133.10                |

The added version ends at 130.00 and the compounded version at 133.10, a difference of 3.10. Small
here, but over thirty years the gap is large. This is why analysts are careful to say whether a
figure is an average or a compounded annual result.

The project's brief on allocation reports the size of this error: using the arithmetic mean in place
of the compounded one overstated the terminal value of the S&P 500 from January 1960 to January 2020
by 89 percent `2405.10920v1` (p.8). The arithmetic mean in that sample was only 0.0009 per month
above the compounded one, yet the long-run effect was enormous `2405.10920v1` (p.8). Averages
flatter; compounded results are what you actually keep.

## Volatility, in plain words

**Volatility** is how much a price moves around its own average. If a price sits near the same level
most days, it has low volatility; if it jumps around, it has high volatility. Volatility is usually
reported as the size of the typical move, in percent, over a fixed period such as a year.

Volatility is not the same as loss. A price can be volatile and still end higher, or calm and still
end lower. What volatility measures is uncertainty: a more volatile investment has a wider range of
possible outcomes, in both directions. It is the standard way of putting a number on "how bumpy was
the ride".

Two warnings about the word. First, volatility says nothing about the direction of the moves, so a
large jump upward counts the same as a large jump down. Second, it is estimated from a sample of
prices, and a short or quiet sample can understate the true uncertainty.

## Drawdown

A **drawdown** is the fall from a peak in value to the low point that follows it, measured in
percent. If an account reaches 100 dollars, then falls to 70 dollars before it rises again, the
drawdown is 30 percent, even if the account later recovers fully. The worst such fall over a period
is called the **maximum drawdown**.

Drawdown is different from volatility because it cares about the order of events. A series of returns
that goes up and down and ends where it started can have no drawdown if it only ever rises, or a
large one if it falls first. Reordering the same returns changes the maximum drawdown, which is why
the risk briefs describe it as path-dependent `1403.8125v4` (p.3).

The reason drawdown is the number beginners feel most sharply is the arithmetic of recovery. The gain
needed to get back to the peak is larger than the loss, because the gain is calculated on the smaller
remaining amount. The table below makes the point. The right-hand column is the loss divided by what
is left after the loss.

| Fall from the peak | Gain needed to get back to the peak |
| ------------------ | ----------------------------------- |
| 10 percent         | 11.1 percent                        |
| 25 percent         | 33.3 percent                        |
| 50 percent         | 100 percent                         |
| 75 percent         | 300 percent                         |

A 50 percent loss needs a 100 percent gain, which is why large drawdowns are so damaging: the
recovery is twice as hard as the fall. This is the central lesson of the project's brief on risk and
drawdown, [strategies/books2/22_risk_measures_and_drawdowns.md](../../strategies/books2/22_risk_measures_and_drawdowns.md),
which also reports that a portfolio ranked by drawdown can turn out less risky than a
return-ranked one by several measures `1403.8125v4` (p.8).

## The Sharpe ratio

The **Sharpe ratio** is a single number that combines return and bumpiness. In words, it divides the
return earned above a safe alternative (such as keeping the money in cash, which earns a small
interest rate) by the volatility of the returns. A higher ratio means more return for each unit of
bumpiness. Its formula is:

```text
Sharpe ratio = (average return - cash return) / volatility
```

- average return: the return of the investment over the period, in percent.
- cash return: the return of a safe alternative over the same period, in percent.
- volatility: how much the return moved around its average, in percent.

Because both the top and the bottom are in percent, the ratio is a plain number with no units. A value
of 1 means the return above cash was about as large as the volatility.

The ratio is only comparable between similar strategies, measured over the same period and the same
market. A strategy that trades often, or that is measured monthly, can show a different ratio from a
strategy that tracks the same idea over years, even when both made the same money. Comparing a
crypto ratio with a government-bond ratio is meaningless, because the bumpiness in the two markets
is not measured on the same scale.

## What risk-adjusted means

A **risk-adjusted** return is a return judged together with the risk taken to get it, not on its own.
Ten percent a year sounds better than five, until you learn that the ten came with a 50 percent
drawdown and the five with a 5 percent one. Risk-adjusted measures, of which the Sharpe ratio is the
most common, exist to make that comparison fair.

The table below shows three hypothetical strategies with the same average return but very different
paths. The drawdown is the worst fall from a peak; the last column is the gain needed to recover, as
calculated above.

| Strategy | Average yearly return | Worst fall from a peak | Gain needed to recover |
| -------- | --------------------- | ---------------------- | ---------------------- |
| A        | 8 percent             | 10 percent             | 11.1 percent           |
| B        | 8 percent             | 30 percent             | 42.9 percent           |
| C        | 8 percent             | 50 percent             | 100 percent            |

All three earn 8 percent on average, but a reader could actually hold A, might hold B through a bad
year, and would find C hard to sit through, because the account would halve before it recovered.

The figures in this table are made up to show the arithmetic, not taken from any market sample. The
numbers in the earlier sections carry their sources. When several strategies are held together, their
drawdowns can overlap or cancel, which is the subject of
[strategies/books2/10_portfolio_and_allocation.md](../../strategies/books2/10_portfolio_and_allocation.md);
that brief finds that a drawdown-aware allocation policy had a shallower maximum drawdown than an
unmanaged benchmark, 21.28 percent against 21.39 percent, with a higher Sharpe ratio of 0.94 against
0.87 `2510.19271v2` (p.18).

A long-run return that arrives smoothly, with no drawdowns, is rare. Markets that have produced
strong average returns have also produced deep falls along the way, so a plan that assumes a smooth
20 percent a year is a plan that has never looked at a chart. The honest way to read a return is to
ask what the worst fall on the way there was.

## Words used in this tutorial

- return: how much an investment gained or lost, as a percentage of its starting value.
- compounded: letting each period's gain grow in the following periods rather than setting it aside.
- volatility: how much a price moves around its own average, usually in percent per year.
- drawdown: the fall from a peak in value to the following low, in percent.
- maximum drawdown: the worst such fall over a chosen period.
- Sharpe ratio: return above cash divided by volatility, a unitless measure of return per unit of bumpiness.
- risk-adjusted: a return judged together with the risk taken to produce it.

## Where this came from

- [TUTORIAL_TEMPLATE.md](../TUTORIAL_TEMPLATE.md), the authoring contract for this collection.
- [GLOSSARY.md](../GLOSSARY.md), for the shared vocabulary of the collection.
- [strategies/books2/22_risk_measures_and_drawdowns.md](../../strategies/books2/22_risk_measures_and_drawdowns.md),
  the brief on drawdown, value at risk and tail measures.
- [strategies/books2/10_portfolio_and_allocation.md](../../strategies/books2/10_portfolio_and_allocation.md),
  the brief on how weights are chosen and on drawdown-aware allocation.
- `1403.8125v4`, Maximum drawdown, recovery, and momentum (2014), https://arxiv.org/abs/1403.8125.
- `2405.10920v1`, Data-generating process and time-series asset pricing (2024), https://arxiv.org/abs/2405.10920.
- `2510.19271v2`, Managing Portfolios Across the Return Distribution (2025), https://arxiv.org/abs/2510.19271.
