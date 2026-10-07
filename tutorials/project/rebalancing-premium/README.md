# The rebalancing premium: getting paid to tidy up

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                  |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Baskets of sector funds, held at fixed target weights and returned to those weights as they drift                                                                                                                                      |
| How often it trades       | Whenever a weight wanders past its band, typically a few times a year                                                                                                                                                                  |
| What you need             | A spreadsheet and the prices of the sectors you hold                                                                                                                                                                                   |
| Where the rules come from | [Sector Regime Engine user manual](../../../implementation/sector-regime-engine/USER_MANUAL.md), panels 7 and 8, the drift-band pot and its attribution                                                                                |
| The underlying research   | Bouchey, Nemtchinov, Paulsen and Stein, [Volatility Harvesting](http://www.snifferquant.com/gyantal/Incode/papers/Volatility%20Harvesting_JWM_Fall_2012.pdf), the decomposition of the effect                                          |
| How well it held up       | Mixed: the growth arithmetic is a proven identity and independent samples show a positive after-cost benefit, but the size is small for a correlated sector universe and rebalancing loses to buy and hold on expected terminal wealth |
| Also appears in           | [Whether sector rules are allowed](../regime-verdict/README.md) in this collection, and the pot it describes lives in [implementation/sector-regime-engine](../../../implementation/sector-regime-engine/README.md)                    |

## The idea in one paragraph

Split your money equally across a set of sectors and decide to keep it that way. Over time some sectors
grow faster, so their slice of the pot becomes bigger than the rest. Once a year, or whenever a slice
drifts past a limit you chose in advance, sell a little of the large slices and use the money to refill
the small ones until the pot is equal again. That habit makes you sell what has been rising and buy
what has been falling, with no forecast of any kind. On closely related sectors the extra growth is
small, under about one percent a year before costs, and it is easily eaten by the cost of trading. Even
when it helps, it does not usually beat simply leaving the pot alone as far as how much money you end
up with on average.

## Why anyone believed it

Most of this collection is about predicting something. This idea predicts nothing, which is exactly why
it was believed: it appears to be free money that survives even in a world where nothing can be
predicted. The mechanism is arithmetic, not information. Two investments that each bounce up and down
can be combined into one that grows more steadily than the average of the two, because the bouncing
itself creates an opportunity to sell high and buy low relative to your own fixed weights.

The counterparty is not a person you outsmart but a habit you avoid. A buy-and-hold investor lets the
weights drift, so the pot slowly becomes a bet on whatever has already gone up. When that winner then
falls, the drifting investor has more of it and takes the fall on a bigger holding. The rebalancer has
already trimmed it. The seller in the trade is often a fund or investor forced to act for reasons of
their own, which is what creates the price swings in the first place; the rebalancer is simply willing
to take the other side of those swings without needing to know why they happened.

The idea was also believed because it is genuinely old and genuinely mathematical. It does not rely on
a pattern continuing, so it cannot be killed by a pattern ending. That is a real, if narrow, advantage.

## An everyday comparison

Two neighbours each set aside money for a joint greenhouse. The rule is that they always hold equal
shares of the greenhouse fund. One autumn the tomato crop is worth a lot and the fund's tomato half
swells; the rule says sell a little of the tomato share and buy more of the herb share, so the two are
equal again. Next year herbs do well and the process runs the other way. Neither neighbour knows which
crop will do better, and neither needs to. The tidying itself is what forces them to sell whatever has
just become expensive and buy whatever has just become cheap.

## The rules, step by step

1. Choose the sectors to hold. Use a set that is related, such as the industries of one market, and
   decide the target share of each. Equal shares are the simplest choice.
2. Put the money in. With two sectors the target is half in each; with eleven, one eleventh each.
3. Decide a drift band in advance, such as 5 percent. The band is relative to the target: a weight
   allowed to drift 5 percent from a target of 0.0909 may reach 0.0955 before anything happens.
4. Decide how often to check. Checking is only a monitoring schedule; it is not a reason to trade.
5. At each check, compute the current share of the pot in each sector. If every share is inside its
   band, do nothing.
6. For any share outside its band, sell a little of the sectors that grew too large and buy a little
   of the sectors that shrank, until every share is back at its target. Only the amount needed to
   restore the weights is traded.
7. Pay the stated cost per trade on everything bought and sold, and subtract the yearly charge that the
   funds deduct for holding them.
8. Repeat from step 5 at the next check.

## The maths, with every symbol named

The growth of a pot kept at fixed weights splits into two parts:

```text
g_p = sum(w_i * g_i) + premium
```

- `g_p` is the growth rate of the fixed-weight pot, in percent per year.
- `w_i` is the target weight of sector `i`, so with eleven equal sectors each `w_i` is about 0.0909.
- `g_i` is the growth rate of sector `i` on its own.
- `sum(w_i * g_i)` is the weighted-average growth of the parts. The first half of the formula is simply
  what the parts did.

The `premium` is the extra, and it depends only on how much the parts move and how together they move:

```text
premium ~= (1/2) * sigma^2 * (1 - 1/N) * (1 - rho)
```

- `sigma` is the volatility, the size of the up-and-down swings, written as a percentage per year.
- `N` is the number of sectors held.
- `rho` is the average pairwise correlation, between -1 and 1: how closely the sectors move together.
- The symbol `~=` means "is approximately". The exact expression is given in the source.
- The premium is larger when volatility is high, when there are more sectors, and, above all, when
  correlation is low. When `rho` is 1, everyone moves together and the premium is zero for practical
  purposes.

For two equally weighted sectors the formula simplifies to:

```text
premium ~= sigma^2 * (1 - rho) / 4
```

The rebalancing opening used to size the activity is the same idea in a different dress:

```text
rebalancing_opening ~= sigma^2 * (1 - rho) - expected_cost
```

- `expected_cost` is the round-trip cost of the trades the tidy-up would need, in the same units.
- When the opening is large, tidy up often and tightly; when it is small, let the pot drift.

There is also a counter-result about which portfolio ends with more money on average. For two assets
over two periods, the difference in terminal wealth between the fixed-weight and drifting pots is:

```text
W_fixed - W_buyandhold = -a1 * a2 * W0 * (r1,1 - r2,1) * (r1,2 - r2,2)
```

- `W_fixed` and `W_buyandhold` are the final pot values of the two approaches.
- `a1` and `a2` are the fractions of wealth allocated to each asset.
- `W0` is the starting pot.
- `r1,1` is asset 1's return in period 1, and so on.

Taking expectations, with a safe asset available, gives:

```text
E[D] = -a1 * a2 * { rho * sigma^2 + (mu - rf)^2 }
```

- `E[D]` is the expected difference in terminal wealth, fixed weight minus buy and hold.
- `mu` is the average return of the return difference between the assets, and `rf` is the safe rate.
- With no memory in the returns, this is negative, so buy and hold has the higher expected ending
  wealth. The source's general statement is that fixed weight beats buy and hold in expected wealth
  only when the returns reverse strongly enough, more strongly than minus the squared reward-to-risk
  ratio of the return difference.

## A worked example

Two sectors, Energy and Health care, each starting at 100.00, so the pot starts at 200.00. The returns
alternate exactly: whichever sector rose this period falls by the same amount next period, and the
other does the opposite change.

| Period | Energy return | Health care return | Rebalanced pot | Buy-and-hold pot |
| ------ | ------------- | ------------------ | -------------- | ---------------- |
| Start  |               |                    | 200.00         | 200.00           |
| 1      | +10 percent   | -10 percent        | 200.00         | 200.00           |
| 2      | -10 percent   | +10 percent        | 200.00         | 198.00           |
| 3      | +10 percent   | -10 percent        | 200.00         | 198.00           |
| 4      | -10 percent   | +10 percent        | 200.00         | 196.02           |

Walk through the rebalanced pot. At the start of each period it holds 100.00 in each sector. In period
1 Energy returns +10 percent, taking its 100.00 to 110.00, and Health care returns -10 percent, taking
its 100.00 to 90.00; the pot is back to 200.00, and the tidy-up restores 100.00 to each. Every later
period repeats this, because the average of the two returns is always `(+10 - 10) / 2 = 0` percent, and
the pot holds them equally. It finishes at 200.00, exactly where it began.

Walk through the buy-and-hold pot. It never tidies. After period 1 it holds 110.00 in Energy and 90.00
in Health care, the same 200.00, but now it is 55 percent Energy. In period 2 Energy falls 10 percent,
so 110.00 becomes 99.00, and Health care rises 10 percent, so 90.00 becomes 99.00; the pot is 198.00,
down one percent. Period 3 leaves it at 198.00, and period 4 takes it to 196.02.

Now the part that matters. Each sector's four returns are +10, -10, +10, -10, and they average exactly
zero. The average return available in this market is zero, and both pots are fed by the very same
returns. Neither pot predicts anything. The average return of the two sectors is identical for both
pots. Yet the rebalanced pot finishes at 200.00 and the drifting pot at 196.02, a gap of 3.98, or about
2 percent of the starting pot. The whole of that gap is the rebalancing premium from the formula above,
and it comes from arithmetic, not from foreknowledge.

Then pay the cost. Restoring the weights in this example means selling 10.00 of the sector that rose
and buying 10.00 of the one that fell, three times in four periods. At 5 basis points per trade, where
one basis point is 0.01 percent, the cost is about 0.01 each time, so 0.03 in total against a gain of
3.98. In this deliberately extreme example the cost is trivial, but it is not always: a real sector
universe has smaller and more correlated moves, and there the cost line is the one that decides.

## What the research actually found

The source of the decomposition, Bouchey and co-authors, gives the mechanism and the magnitudes. On
United States equal-weight stocks from 1997 to 2012 the measured premium was 1.42 percent a year before
costs; on global, developed ex-United States and emerging universes it was 0.72, 0.34 and 1.41 percent a
year. In one million trials on random 100-stock portfolios the total excess over a size-weighted
portfolio was about 2.80 percent a year before costs, and the tidied portfolios beat the drifting ones
over 90 percent of the time against 74 percent for drifting equal weights.

Those numbers are for large, diverse, single-stock universes. The same source warns that the mechanism
transfers but the magnitude does not: a hundred individual shares have far more independent movement
than eleven related sector funds. Applying the formula to realistic sector inputs gives much less.
Eleven sectors, 20 percent volatility and an average correlation of 0.5 imply about 0.91 percent a year
before costs; push the correlation to 0.7, which is plausible for sectors, and it falls to about 0.55
percent; at 0.9 it is 0.18 percent. These are formula outputs for stated inputs, not measurements.

The cost warning is explicit in the source: unconstrained rebalancing can generate transaction costs
that exceed the benefit. Its recommendations are to allow drift within bands, to tidy at the sector
level rather than the stock level, and to hold weights between a size-weighted and an equal split.

The counter-result comes from El Bernoussi and Rockinger, who compare fixed weight with buy and hold
directly and find that buy and hold has the higher expected terminal wealth unless the returns reverse
strongly. Both results are correct; they answer different questions. The formula in the previous
section is a comparison of compound growth against the weighted growth of the parts. The counter-result
is a comparison of expected ending money against a portfolio left to drift. A result stated in the
first form is regularly read as if it were the second, and that is the confusion the source is at pains
to separate.

## How this project relates to it

The app builds the pot this tutorial describes and reports where its result came from. In
[src/engine.js](../../../implementation/sector-regime-engine/src/engine.js) the attribution keeps two
different rebalancing lines apart: one compares tidying against what the sectors did on their own, the
other compares tidying against simply leaving the pot alone. Both are correct, and they often have
opposite signs, which is exactly the distinction in the four-quantities table in the research.

The [user manual](../../../implementation/sector-regime-engine/USER_MANUAL.md) shows these as the type A
and type B lines in panel 8, and panel 7 draws the drifting pot and the tidied pot as two of its four
equity curves, gross and after costs. Section 4 of
[Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md) is the full
treatment, with the decomposition, the sensitivity table for sector inputs, and the counter-result.

## Where it goes wrong

- Costs can exceed the benefit. Every tidy-up trades against the gap between the buying and selling
  price, and the source says plainly that unconstrained rebalancing can cost more than it earns. This
  is the single most common way the idea fails in practice.
- Correlated sectors kill it. The premium is proportional to one minus the correlation. Eleven sector
  funds that largely move together offer far less than a hundred unrelated shares, and at high
  correlation the whole effect approaches zero.
- The four quantities get mixed up. A gain of the first kind (compound growth against the parts) is
  routinely reported as though it were a gain of the third kind (expected ending wealth against buy and
  hold). The first is often true and the third often false.
- Too little dispersion. When the sectors differ little in a period, there is nothing to buy low and
  sell high, and the formula says the premium shrinks toward zero.
- The measuring matters. The correlation and volatility used to size the trades come from the past,
  and both change. A premium estimated from a calm decade can disappear in a wild one, and vice versa.
- The counter-result is real. If your question is which pot ends larger on average, the honest answer
  from the sources is that buy and hold usually does, unless returns reverse strongly.

## Try it yourself

You need a spreadsheet and four periods of two sectors' returns; the numbers below are the worked
example, and you can replace them with any two sectors you like.

1. Columns: `Period`, `Energy return`, `Health care return`, `Rebalanced pot`, `Buy-and-hold pot`.
2. Put 200.00 in both pot columns on the `Start` row.
3. For each period, the rebalanced pot's new value is the old value times
   `1 + (Energy return + Health care return) / 2`, because it holds them equally.
4. The buy-and-hold pot needs two helper columns, `Energy value` and `Health care value`, starting at
   100.00 each. Each period multiply by that period's return, and let the columns drift; the pot is
   their sum.
5. At the bottom, compute the average of the two return columns. Then compute each pot's own average
   return, and its final value.
6. Now change the costs: subtract 5 basis points on every 10.00 traded and see how the gap moves.

What to notice: the two sectors' returns average zero, and the rebalanced pot's average return is zero,
yet its final value is higher than the drifting pot's. Then notice how quickly the gap shrinks if you
make the two sectors move together instead of oppositely. That sensitivity, not the headline number, is
the honest lesson.

## Where this came from

- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), section 4: the
  four quantities, the mechanism, the sector sensitivity table, the empirical magnitudes and the
  counter-result.
- Bouchey, Nemtchinov, Paulsen and Stein, [Volatility Harvesting](http://www.snifferquant.com/gyantal/Incode/papers/Volatility%20Harvesting_JWM_Fall_2012.pdf),
  the decomposition, the empirical magnitudes and the coin-flipping argument.
- El Bernoussi and Rockinger, [Rebalancing with transaction costs](https://link.springer.com/content/pdf/10.1007/s11408-022-00419-6.pdf),
  the expected-terminal-wealth counter-result.
- Anderson, Bianchi and Goldberg, [Will My Risk Parity Strategy Outperform?](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2390614),
  the after-cost 60/40 versus buy-and-hold figure of 74 basis points a year from 1926 to 2010.
- [Sector Regime Engine user manual](../../../implementation/sector-regime-engine/USER_MANUAL.md),
  panels 7 and 8, and [src/engine.js](../../../implementation/sector-regime-engine/src/engine.js), the
  attribution that keeps the two rebalancing lines apart.

## Words used in this tutorial

- basis point: one hundredth of one percent, so 5 basis points is 0.05 percent.
- correlation: a number between -1 and 1 describing whether two things move together; near 1 means they
  move almost identically, near 0 means they are unrelated.
- dispersion: how spread out the sector results are from each other in a given period.
- drift: the natural process by which the shares in a pot change over time, because some parts grow
  faster than others.
- drift band: the limit a weight may wander from its target before the pot is tidied.
- geometric growth: growth measured by the ending value after compounding, as opposed to the simple
  average of the period returns.
- rebalancing premium: the extra compound growth a fixed-weight pot earns over the weighted average of
  its parts, coming from dispersion and imperfect correlation.
- volatility: how much a price moves around its average, measured as a percentage per year.
- weight: the share of your money placed in one holding.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
