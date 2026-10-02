# Profiting from sector rotation

Date: 2026-10-02. Revision 2, incorporating external review feedback.

## The question

If sector rotation is random, meaning leadership changes with no memory, what is the best strategy to
profit from it?

## The short answer

Under genuinely memoryless sector returns, no directional sector-selection strategy has an expected
gross forecasting edge. A ranking rule still trades, still pays spreads, and therefore has a negative
expected net return. What remains is portfolio construction rather than prediction: a diversified
sector allocation with cost-controlled rebalancing can convert dispersion and imperfect correlation
into a geometric-growth benefit. That benefit is real, it is small, and it is not the same thing as
outperforming buy-and-hold in expected terminal wealth.

The defensible formulation, which the rest of this document supports, is:

> Under memoryless sector returns, disable directional selection. Then optimise portfolio construction
> from dispersion, correlation, turnover and implementation cost. Do not encode "memoryless means
> rebalancing wins", because that is a conditional mathematical result, not a permanent trading rule.

Directional overlays should be activated only after statistically significant, out-of-sample evidence
of persistence or reversal has been established on the actual traded universe.

## How the evidence here was gathered

Research was assembled with live web search (Perplexity, via OpenRouter) and then checked against the
primary sources wherever they were reachable. Two labels are used throughout:

- **Verified**: read from the primary source. Where the search layer disagreed with the primary, the
  primary value is used and the disagreement is noted inline at the point of use, not in a separate
  appendix.
- **Summary only**: reported by the search layer with a citation, but the primary was not reachable to
  confirm it. Directionally useful, not to be relied on as exact.

Two such corrections were made and are flagged where they appear: the Molchanov and Stangl
outperformance figure, and the Moskowitz and Grinblatt break-even cost, which applies to a
lower-turnover variant rather than the base strategy.

## Section 1: Three different things called sector rotation

Most disputes about sector rotation are disputes between people doing different things. These are
distinct and only one of them is about prediction.

| Term              | Definition                                                                                      | Needs a forecast? |
| ----------------- | ----------------------------------------------------------------------------------------------- | ----------------- |
| Sector rotation   | Changing *relative* sector exposure because you believe future relative returns are predictable | Yes               |
| Sector allocation | Choosing how much capital to hold in each sector as a structural policy                         | No                |
| Rebalancing       | Returning allocations toward predetermined targets after drift                                  | No                |

Under memoryless returns the three separate cleanly:

```
Sector rotation    -> no predictive edge
Sector allocation  -> still meaningful, and is a policy choice
Rebalancing        -> economically useful, but not a free lunch
```

Keeping these apart is the single largest clarification available in this subject, because it prevents
a non-predictive mechanism (rebalancing) from being sold as a predictive one (rotation).

## Section 2: What "random rotation" means, and why it is two questions

"Random" covers several different statistical worlds with different answers.

| World               | Definition of sector relative returns                     | What is eligible             | What is not                                 |
| ------------------- | --------------------------------------------------------- | ---------------------------- | ------------------------------------------- |
| Memoryless          | Independent across time. Today's leader tells you nothing | Rebalancing, diversification | Momentum, trend, cycle rotation, contrarian |
| Negative dependence | Tendency to reverse. Winners lag next period              | Contrarian, rebalancing      | Momentum                                    |
| Positive dependence | Persistence lasts months. Winners keep winning            | Momentum, trend              | Contrarian                                  |
| Uncertain           | Estimate exists but is not significant                    | Neither overlay              | Both overlays                               |

The fourth row is not decoration. Real data will rarely produce a clean zero. A rank autocorrelation
of +0.07 with a p-value of 0.42 is not the same finding as +0.07 with a p-value of 0.003, and only the
second should switch on a momentum overlay. Treating the first as evidence of persistence is exactly
the overfitting failure this document is trying to prevent.

A trap sits in the memoryless row. **Unpredictable does not mean mean-reverting.** An independent
sequence has no expected reversal, so a contrarian strategy has zero expected gross return and loses
after costs, exactly like momentum. Reversal requires negative dependence, which is a stronger
condition than the absence of positive dependence. This should be treated as the foundational
principle of any regime-gated system.

### 2.1 Two independent dimensions

Temporal dependence and cross-sectional structure are separate questions and must be measured
separately. Conflating them is a common error, including in the first revision of this document.

```
Dimension 1: temporal dependence
    Does sector leadership contain exploitable dependence?
        positive  -> test momentum
        negative  -> test reversal
        ~zero     -> do not rank sectors directionally
        unclear   -> do nothing directional

Dimension 2: cross-sectional structure
    What does the dispersion and correlation structure support?
        high dispersion + low correlation -> rebalancing economics improve
        low dispersion or high correlation -> rebalancing economics degrade
```

The first dimension governs whether a directional overlay is admissible. The second governs how much
the portfolio-construction layer can be expected to contribute. They are not connected: you can have
strong dispersion with no temporal dependence at all, which is precisely the memoryless world.

## Section 3: What cannot work under memoryless rotation

### 3.1 A terminology convention

Two statements are routinely conflated and should be kept apart:

```
Memoryless rotation:
    Expected directional gross alpha = 0
    Expected directional net alpha   < 0,  because turnover x costs > 0
```

The gross zero is a statement about information. The negative net is a statement about friction. Only
the second is an argument against trading, and it is the stronger one, because it holds even when the
gross edge is zero rather than negative. This convention is used consistently below.

### 3.2 Sector momentum

Momentum needs positive serial dependence in relative sector returns. Under memoryless rotation the
conditional expectation is flat:

```
E[r_next | past leadership] = E[r_next]
```

A ranking rule still trades. It still pays spreads, commissions, slippage and market impact. Its
expected net return is therefore negative. Frequent rotation does not rescue it: frequent but
persistent rotation supports momentum, because winners stay winners for several periods, whereas random
rotation destroys it, because the identity of the next winner is independent of the current one.

### 3.3 Business-cycle rotation

This is the version of sector rotation that most practitioners mean, and it is the most thoroughly
tested. Molchanov and Stangl (**verified**) test it directly and find no evidence of systematic sector
performance where the popular cycle maps predict it.

| Finding                                                                       | Number                              |
| ----------------------------------------------------------------------------- | ----------------------------------- |
| Conventional rotation, perfect foresight of NBER turning points, before costs | 0.11 percent per month              |
| Simple market-timing alternative, equities except early recession             | 0.15 percent per month              |
| Conventional rotation after transaction costs of 0.5 to 1.5 percent           | 0.07 down to 0.01 percent per month |
| Significance after costs                                                      | Indistinguishable from zero         |

The strategy carries a higher standard deviation, higher beta and lower Sharpe ratio than the market.
The paper then relaxes every assumption, testing 10 sectors with two-stage cycles and all 1,022
possible rotation rules:

| Measure                                      | Value                           |
| -------------------------------------------- | ------------------------------- |
| Mean return of all 1,022 rotation strategies | 0.86 percent per month          |
| Buy and hold over the same sample            | 0.89 percent per month          |
| Strategies beating buy and hold              | 132 of 1,022, or 12.9 percent   |
| Strategies beating simple market timing      | 35 of 1,022, or 3.4 percent     |
| Authors' conclusion                          | Outperformance is data snooping |

The test that bears most directly on the question asked here ignores the cycle entirely and regresses
every sector's excess return on every other sector's at lags of 1 to 24 months. Across 2,160
t-statistics, 6 percent are significantly positive at the 10 percent level against 5 percent expected
by chance, mostly at the one-month lag. Their conclusion is that cross-sector predictability "occurs
only randomly".

A practitioner datapoint from the same paper: the Sector Rotation Fund (NAVFX) returned 7.34 percent
from 2010 to 2018 against 12.23 percent for the S&P 500.

Note on the correction: this result is reported in places as 0.16 percent per month before costs. The
paper states 0.11 percent risk-adjusted with perfect NBER timing, falling to 0.07 down to 0.01 percent
with costs. The primary value is used above.

### 3.4 What this evidence does and does not establish

It does not prove that sector rotation is random. It establishes two narrower and more defensible
things:

1. The tested business-cycle rotation rules did not produce reliable excess performance in that
   sample, across a wide range of specifications.
2. Cross-sector predictive regressions in that sample were consistent with chance.

Either statement is consistent with the memoryless interpretation. Neither is a mathematical proof
that future sector returns are independent and identically distributed. Throughout this document,
"consistent with memoryless behaviour" is the phrase used, and it should not be upgraded to "proves
rotation is random" when this material is turned into a trading specification.

## Section 4: What remains economically useful under memoryless rotation

The title of this section is deliberate. Strictly speaking, nothing here demonstrates a magical alpha
source. What it demonstrates is that a portfolio-construction rule can transform dispersion and
volatility into a geometric-growth benefit under suitable conditions. That is a real but narrower
claim than "rebalancing is the winner".

### 4.1 Four different quantities that get confused

A large part of the confusion in this literature comes from comparing different things and calling the
comparison one thing.

| Comparison                                           | Question it answers                            | Typically favours                                             |
| ---------------------------------------------------- | ---------------------------------------------- | ------------------------------------------------------------- |
| A. Geometric growth versus weighted component growth | Does rebalancing add to compound growth?       | Fixed weight, whenever correlations are below one             |
| B. Constant-weight versus drifting portfolio         | Does rebalancing beat letting weights drift?   | Depends on autocorrelation and on concentration               |
| C. Expected terminal wealth versus buy and hold      | Which ends with more money on average?         | Buy and hold, unless autocorrelation is sufficiently negative |
| D. Risk-adjusted performance                         | Which earns more per unit of risk or drawdown? | Usually the rebalanced portfolio                              |

These are not interchangeable. A result stated in terms of A is regularly read as if it were a result
about C, which is how "rebalancing beats buy and hold" gets asserted on evidence that does not support
it. The remainder of this section keeps them apart.

### 4.2 The rebalancing premium: mechanism

Bouchey, Nemtchinov, Paulsen and Stein (**verified**) give the clean decomposition. The growth rate of a
constant-weight portfolio equals the weighted-average growth of its components plus a term depending
only on dispersion:

```
g_p = sum(w_i * g_i) + (1/2) * [ sum(w_i * sigma_i^2) - portfolio_variance ]
```

The second term is the rebalancing premium, also called the diversification return (Booth and Fama,
1992; Willenbrock described it as the only free dessert). It is positive whenever correlations are
below one and rises with volatility and with falling correlation. For two equally weighted assets with
equal volatility at correlation rho, the approximation is:

```
premium ~= sigma^2 * (1 - rho) / 4
```

For N equally weighted assets with equal volatility and average pairwise correlation rho, the general
equal-weight form is:

```
premium ~= (1/2) * sigma^2 * (1 - 1/N) * (1 - rho)
```

The decisive argument for the memoryless case is the authors' coin-flipping experiment: returns have no
serial correlation, so the concepts of momentum and reversal do not apply, and the premium is still
present, with a half-invested coin flip compounding at about 6 percent while full investment compounds
at zero. The premium comes from trading a volatile, imperfectly correlated set of assets at fixed
weights, not from predicting anything.

### 4.3 The premium is small for a sector universe

Section 4.2 explains why the mechanism exists. Section 2.1 explains why it is smaller here than the
headline figures suggest, and the size of the effect for exactly this universe should be stated before
any strategy is built on it.

Applying the equal-weight form above to assumed inputs:

| Sectors | Volatility | Average pairwise correlation | Implied premium before costs |
| ------- | ---------- | ---------------------------- | ---------------------------- |
| 11      | 20 percent | 0.5                          | about 0.91 percent per year  |
| 11      | 20 percent | 0.7                          | about 0.55 percent per year  |
| 11      | 20 percent | 0.9                          | about 0.18 percent per year  |
| 11      | 25 percent | 0.7                          | about 0.85 percent per year  |

These are formula outputs for stated inputs, not empirical results. Their purpose is to establish the
sensitivity that matters: **the premium is inversely proportional to sector correlation**, and sector
ETFs are a comparatively correlated universe. At a plausible 0.7 average correlation it is worth
well under one percent a year before costs, and every rebalance trades against a spread. A strategy
built on this must measure average and median pairwise correlation, correlation dispersion,
cross-sectional volatility and cross-sectional return dispersion, rather than assuming that eleven
sectors provide enough independent volatility.

### 4.4 The counter-result: expected terminal wealth

El Bernoussi and Rockinger (**verified**) analyse fixed weight against buy and hold directly. For two
assets over two periods:

```
W_fixed - W_buyandhold = -a1 * a2 * W0 * (r1,1 - r2,1) * (r1,2 - r2,2)
```

Taking expectations with a risk-free alternative available:

```
E[D] = -a1 * a2 * { rho * sigma^2 + (mu - rf)^2 }
```

With zero autocorrelation this is negative, so buy and hold has the higher expected wealth. Their
general statement is that fixed weight beats buy and hold in expected wealth only when
autocorrelation is more negative than minus the squared Sharpe ratio of the return differential.

This does not contradict Section 4.2, and the reason is the whole point of Section 4.1:

- Section 4.2 is a comparison of type A. It measures the growth rate of a constant-weight portfolio
  against the weighted average of its components' own growth rates.
- Section 4.4 is a comparison of type C. It measures expected terminal wealth against a portfolio left
  to drift toward its winners.

Both are correct. Reading either as the other is the error. Under memoryless rotation, rebalancing buys
risk control and a modest growth improvement against a drifting portfolio, and it does **not** provide
a large expected-wealth edge over buy and hold.

### 4.5 Empirical magnitudes, labelled by what they measure

| Result                                                        | Source                                                        | Comparison type       | Value                                                                     |
| ------------------------------------------------------------- | ------------------------------------------------------------- | --------------------- | ------------------------------------------------------------------------- |
| Rebalanced 60/40 beat buy and hold, after costs, 1926 to 2010 | Anderson, Bianchi and Goldberg 2012, quoted in Bouchey et al. | C, after costs        | 74 basis points per year, with lower volatility                           |
| US equal-weight stocks, 1997 to 2012                          | Bouchey et al., Russell Global data                           | A and B, before costs | Rebalancing premium 1.42 percent per year                                 |
| Global, developed ex-US, emerging                             | Bouchey et al.                                                | A and B, before costs | 0.72, 0.34 and 1.41 percent per year                                      |
| Random 100-stock portfolios, 1 million trials                 | Bouchey et al.                                                | A, before costs       | About 2.80 percent per year total excess over cap weighting               |
| Random 100-stock portfolios, hit rates                        | Bouchey et al.                                                | B                     | 74 percent for drifting equal weights, over 90 percent rebalanced monthly |

Labelling note on the last two rows, which matters for implementation: **the random 100-stock result is
supporting evidence for the mechanism, not evidence of a sector ETF premium.** A 100-stock universe has
more idiosyncratic dispersion, different correlation structure and far greater cross-sectional
heterogeneity than eleven sector ETFs. The mechanism transfers; the magnitude does not.

The cost caveat is verified and explicit in the source: unconstrained rebalancing can generate
transaction costs that exceed the benefit. Their recommendations are to allow drift within bands,
rebalance at the sector level rather than the stock level, and use weights between cap weight and equal
weight.

## Section 5: Contrarian reversion

A contrarian strategy buys recent sector losers and sells recent winners. It earns a positive expected
gross return only when sector returns show negative serial dependence.

Three levels must be distinguished, because evidence at one level does not transfer to another:

| Level                    | Question                             | Transfer risk                                                        |
| ------------------------ | ------------------------------------ | -------------------------------------------------------------------- |
| Time-series reversal     | Does sector A's own return reverse?  | Does not imply cross-sectional reversal                              |
| Cross-sectional reversal | Does relative sector rank reverse?   | This is what a rotation rule needs                                   |
| Stock-level reversal     | Do individual stock returns reverse? | Does not transfer to sectors: aggregation removes much of the effect |

**Methodological rule for this project: never transfer a stock-level anomaly to sector ETFs without
independently testing it at the sector level.**

| Claim                                                                                                                                                      | Label        | Notes                                                              |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | ------------------------------------------------------------------ |
| Hameed and Mian, Industries and Stock Return Reversals: pervasive intra-industry monthly reversals, stronger after market declines and in volatile periods | Summary only | Direction credible and widely cited; magnitudes not confirmed here |
| Industry-adjusted stock reversal around 0.53 percent per month, Sharpe near 0.74                                                                           | Summary only | A stock-level result, not sector-level                             |
| Long-horizon reversal in the 1.4 to 2.5 percent per month range                                                                                            | Summary only | Not a tradable sector rotation estimate                            |

Two consequences follow from the verified material. A stock-level reversal result is partly
idiosyncratic and does not survive aggregation to sector level. And the effect must exceed spread plus
turnover: a monthly sector rotation at 10 to 20 basis points a trade consumes much of a 0.5 percent
monthly gross signal.

## Section 6: Dispersion and relative value

Instead of forecasting which sector leads, a relative-value book trades the spread between sectors with
market exposure neutralised. This monetises dispersion rather than direction.

| Claim                                                                                                                             | Label          | Notes                                                                                      |
| --------------------------------------------------------------------------------------------------------------------------------- | -------------- | ------------------------------------------------------------------------------------------ |
| Gatev, Goetzmann and Rouwenhorst: about 12 percent excess annual return, 4 to 6 percent annualised volatility                     | Summary only   | Historical sample, classic result                                                          |
| Cointegration-based sector pairs: 1.01 percent total profit, Sharpe 0.28, versus Sharpe 0.81 for a lower-volatility specification | Summary only   | Shows extreme sensitivity to specification                                                 |
| Option-based dispersion: sector volatility against index volatility                                                               | Mechanism only | Profits when realised correlation diverges from implied; loses badly on correlation spikes |

The risks are specific and not diversifiable: correlation spikes in crises turn a neutral book into a
directional one, borrow and financing costs are real, and cointegration relationships break permanently
when index composition or business models change. No peer-reviewed net-of-cost sector-level dispersion
estimate was located in this research.

## Section 7: The other side, if rotation is persistent

If the data show persistence, the ranking reverses and momentum becomes the best documented signal.
Moskowitz and Grinblatt (**verified**), 20 value-weighted SIC industries, July 1963 to July 1995:

| Measure                                                                     | Value                                                 |
| --------------------------------------------------------------------------- | ----------------------------------------------------- |
| (6,6) strategy: long top 3 industries, short bottom 3, held 6 months        | 0.43 percent per month                                |
| Same, size and book-to-market adjusted                                      | 0.29 percent per month, t = 3.34                      |
| Same, DGTW-adjusted                                                         | 0.20 percent per month, t = 2.27                      |
| Individual stock (6,6) momentum, raw                                        | 0.43 percent per month, t = 4.65                      |
| Individual stock momentum minus industry                                    | 0.13 percent per month, t = 2.04                      |
| Individual stock momentum, size and book-to-market adjusted, minus industry | 0.08 percent per month, t = 0.91                      |
| Equal-weighted industry portfolios, (6,6)                                   | 0.81 percent per month, 10.2 percent a year, t = 7.71 |

Industry momentum therefore accounts for roughly two thirds of individual stock momentum in this
sample, and the residual stock-level effect is not statistically significant once industry is removed.

Three qualifications decide whether any of it is tradable:

- **Costs.** Turnover is about 200 percent a year for the (6,6) version, with an estimated break-even
  one-way cost of about 75 basis points. Extending the holding period to 12 months halves turnover and
  raises the break-even to about 150 basis points. Correction: the 150 basis point figure is sometimes
  quoted without qualification; it applies only to that lower-turnover variant.
- **Horizon.** The strongest version is (1,1), one-month formation and one-month holding, but skipping a
  single month eliminates its profitability entirely, which points to lead-lag and microstructure
  effects rather than a durable signal. Industry momentum also decays after 12 months and reverses at
  long horizons.
- **Era, and this one dominates.** The sample ends in July 1995. Historical academic evidence is not a
  current tradable edge, and a 2026 system cannot inherit a 1963 to 1995 result. Nothing in this
  document establishes that industry momentum still pays.

### 7.1 The conflicting short-horizon evidence

A pre-registered test on 2010 to 2026 data (**verified**) contradicts the short-horizon reading of that
literature. Quant Data defined four popular rotation states on 11 SPDR sectors and, separately, 12
industry ETFs, froze the definitions and verdict rules before reading data, and tested whether the
following 10, 20 and 60 trading days differed from ordinary days. Result: 24 of 24 cells NOT CONFIRMED,
across 4,482 sector events and 5,173 industry events, with a block bootstrap of 2,000 draws and 20-day
blocks, alpha 0.01, and 2022 onward held out entirely. Expected false positives at that alpha across
24 cells: 0.24.

| State                                           | Difference in hit rate, 2010 to 2021          | Difference, 2022 onward            | Held-out p |
| ----------------------------------------------- | --------------------------------------------- | ---------------------------------- | ---------- |
| Relative rotation graph enters Leading, 10 days | +1.7 points, 95 percent interval -1.6 to +5.2 | -0.4 points, interval -5.8 to +4.7 | 0.566      |
| Crowded flag turns on, 20-day drawdown          | -2.0 points, interval -3.1 to -1.1            | -0.4 points, interval -1.4 to +0.5 | 0.351      |
| Confirmed entry rule turns on, 20 days          | -3.1 points, interval -6.2 to +0.3            | +1.7 points, interval -3.1 to +7.0 | 0.243      |
| Money leaving rule turns on, 20 days            | +1.1 points, interval -1.5 to +3.8            | -1.1 points, interval -5.3 to +3.0 | 0.308      |

The one effect visible in the first decade, a deeper pullback after a crowded flag, shrank to nothing
in the held-out window. The authors' own explanation is horizon: at a 10-day window a sector moved from
Improving into Leading only 1.3 times as often as it fell back to Lagging, and a Leading spell lasted
4.7 trading days on average. They note that academic sector momentum uses 6 to 12 month formation
windows and skips the most recent month precisely because one-month relative returns tend to reverse.

Scope of this evidence, stated precisely: it is the strongest directly relevant evidence in this
document **against the specific short-horizon rotation states that were tested**. It is not a test of
every conceivable sector predictability mechanism, and it should not be cited as though it were.

## Section 8: Measuring which regime you are in

The regime decision is a measurement, not a preference. Every test below should be run on the
instrument set actually traded, using total-return series, and the verdict rules should be frozen in
advance.

### 8.1 Evidence hierarchy

Not all diagnostics deserve equal weight in a decision. Suggested ordering:

```
PRIMARY
    out-of-sample predictive return tests
    cross-sectional rank autocorrelation
    return autocorrelation
    variance ratio
    transaction-cost-adjusted backtest

SECONDARY
    rank turnover
    dispersion
    correlation

DIAGNOSTIC
    Hurst exponent
```

The Hurst exponent is deliberately in the last group. Estimates are sensitive to estimator, sample
length, non-stationarity, volatility clustering, structural breaks and aggregation frequency, and a
value above 0.5 does not by itself establish a tradable momentum edge.

### 8.2 The tests

1. **Cross-sectional rank autocorrelation.** The correlation of today's sector ranking with the
   ranking one decision period later:

   ```
   IC_t = Corr(Rank_t, Rank_t+1)
   ```

   This is more directly relevant to a ranking strategy than raw return autocorrelation, because it
   asks the question the strategy actually depends on: do rankings predict rankings? Report it with a
   p-value, and route an insignificant estimate to UNCERTAIN rather than to an overlay.

2. **Conditional persistence.** The probability that a sector is up next period given it is up now:

   ```
   P(R_t+1 > 0 | R_t > 0)
   ```

   Preferably conditioned on state: top-ranked sectors, bottom-ranked sectors, high-dispersion
   environments, low-correlation environments. Persistence that exists only in one state is much more
   useful than a single unconditional number, and it will usually be invisible to an unconditional
   average.

3. **Return autocorrelation.** Lag-1 autocorrelation of each sector's excess return over the basket, at
   monthly and weekly frequency. Negative supports contrarian, positive supports momentum,
   indistinguishable from zero supports neither.

4. **Variance ratio.** The variance of k-period relative returns against k times the one-period
   variance. Above one indicates persistence, below one indicates reversal.

5. **Rank turnover.** The average change in rank between decision dates. High rank turnover is the
   signature of the world in which ranking strategies fail.

6. **Dispersion.** The cross-sectional standard deviation of sector returns. This is what the
   rebalancing premium actually pays on, and it needs no forecast.

7. **Correlation structure.** Average and median pairwise correlation, correlation dispersion, and
   correlation stability over time. The premium is inversely proportional to the first two.

8. **Hurst exponent.** Diagnostic only. The search layer reported 0.627 to 0.671 for sector ETFs
   (**summary only**). Treat as suggestive at most.

## Section 9: Strategy eligibility under memoryless rotation

An earlier revision of this document ranked these strategies from 1 to 4. That was the weakest choice in
it, because the ordering implied a comparability the evidence does not establish. Eligibility is the
right frame: each strategy has a required condition, and under strict independence most of them are
simply not admissible.

| Strategy                     | Required condition                    | Eligible under strict independence? |
| ---------------------------- | ------------------------------------- | ----------------------------------- |
| Diversified rebalancing      | Dispersion plus imperfect correlation | Yes                                 |
| Buy and hold                 | No predictive assumption              | Yes                                 |
| Dispersion and pairs trading | Stable spread relationship            | Potentially, evidence thin          |
| Contrarian                   | Negative dependence                   | No                                  |
| Momentum                     | Positive dependence                   | No                                  |
| Business-cycle rotation      | Reliable cycle to sector relationship | No                                  |

The two "yes" rows are the ones that require no forecast. That is the whole answer to the question
asked at the top of this document, and it is a statement about admissibility rather than about
superiority.

## Section 10: A two-layer design

### 10.1 Layer A, the structural portfolio

Always present, never switched off:

```
eleven sector ETFs
    -> baseline allocation (equal weight, or a blend of equal and cap weight)
    -> drift-band rebalancing
    -> cost-aware execution
```

Rebalancing is triggered by a drift band rather than by the calendar, because the band is the cost
control and the calendar is only a monitoring schedule. Monthly is a reasonable evaluation cadence; it
is not obviously a reasonable trading cadence.

### 10.2 Layer B, the conditional overlay

Enabled only when Section 8 supports it, and off by default:

```
negative dependence and significant -> contrarian overlay
positive dependence and significant -> momentum overlay
neither, or uncertain                -> no directional overlay
```

### 10.3 Rebalancing opportunity

Rather than fixing a calendar frequency, size the rebalancing activity from the economics:

```
RebalancingOpportunity ~= sigma^2 * (1 - rho) - expected_cost
```

where sigma is cross-sectional volatility, rho is average pairwise correlation and expected cost is
the round-trip implementation cost of the trade set. Then:

```
high   -> tighten the drift band
medium -> normal band
low    -> allow drift
```

This is a more useful decision variable than monthly versus quarterly, because it moves with the thing
that actually pays for the trade.

### 10.4 What to test, in a fixed grid

The interesting empirical question is the interaction of frequency and band, and specifically whether
frequency matters much once the band is set. Freeze a grid before running anything:

```
frequencies: weekly, monthly, quarterly, event-driven
bands:       5, 10, 15, 20 percent
universe:    eleven sector ETFs, total return
costs:       modelled per trade in basis points
```

### 10.5 Cost model, stated as two separate things

```
TradingCost  = spread + commission + slippage + market impact
HoldingCost  = ETF expense ratio
TotalImplementationDrag = TradingCost + HoldingCost
```

Expense ratio is a continuous drag and must not be modelled as a trading cost. Mixing them makes both
the attribution and the break-even analysis wrong.

### 10.6 Return attribution is mandatory

A backtest that reports only a total return cannot distinguish a working mechanism from an accidental
exposure. Every run should decompose:

```
Total return                    +X

Underlying sector exposure      +A
Rebalancing contribution        +B
Directional overlay alpha       +C
Trading costs                   -D
ETF expenses                    -E
```

Without B separated from C, a positive total return is not evidence that any rotation mechanism worked.

### 10.7 Pre-registration checklist

Freeze, before looking at results:

1. Universe and data source, including whether returns are total return.
2. Signal definitions, thresholds and parameters, and the window used to choose them.
3. Rebalancing rule and drift band.
4. Cost model in basis points, applied per trade, with holding cost separated.
5. Verdict rule: statistic, horizon, significance level, direction.
6. Observation window and a held-out window not touched until the end.

This is the procedure that produced the negative result in Section 7.1, and its value is that the
result stands whatever it says.

## Section 11: Engine shape

The natural implementation is a regime classifier whose output gates a portfolio-construction layer,
not a strategy score that always emits BUY or SELL per sector.

```
Sector return data
  |
  +-- temporal dependence ---------+-- cross-sectional structure
  |     rank autocorrelation       |     dispersion, correlation
  |     return autocorrelation     |     rank turnover, cost
  |     variance ratio             |
  v                                v
Rotation regime                Rebalancing opportunity
  |                                |
  negative | neutral | positive | uncertain
  |     |        |         |        |
  contrarian none  momentum none    |
  +----------------+--------+-------+--> nothing
                   |
                   v
         Portfolio construction
           baseline sector allocation
           drift-band rebalancing
           cost-aware execution
                   |
                   v
              Attribution
```

Output fields:

```
rotation_regime:    MEMORYLESS | NEGATIVE_DEPENDENCE | POSITIVE_DEPENDENCE | UNCERTAIN
confidence
metrics:
    return_autocorr
    rank_autocorr
    variance_ratio
    rank_turnover
    dispersion
    avg_correlation
    rebalancing_opportunity
```

The critical property: UNCERTAIN and MEMORYLESS both select diversified allocation with rebalancing only
and no directional overlay. **Memoryless must not be encoded as "rebalancing is the winner".** It is
encoded as "disable directional selection", after which portfolio construction is optimised on
dispersion, correlation, turnover and implementation cost.

## Section 12: What would change this answer

- Reliable, out-of-sample evidence of negative serial dependence in sector returns at the traded horizon
  would admit a contrarian overlay.
- Reliable evidence of persistence at 6 to 12 months surviving costs on the traded instruments would
  admit a momentum overlay.
- A peer-reviewed study that generates random sector rotation by construction and shows a rotation
  strategy with positive net-of-cost risk-adjusted return would overturn the main conclusion. None was
  located in this research; the rebalancing Monte Carlo results are the closest available approximation.
- Structural change that raises dispersion while leaving correlations low would mechanically raise the
  rebalancing premium and justify a tighter drift band.

## Section 13: Final thesis

Under genuinely memoryless sector returns, directional sector selection has no expected forecasting
edge, and a turnover-bearing implementation of it has a negative expected net edge. The appropriate
baseline is a diversified sector allocation with cost-controlled rebalancing. Rebalancing should be
treated as a portfolio-construction mechanism that can convert dispersion and imperfect correlation
into a geometric-growth benefit, not as guaranteed alpha and not as guaranteed outperformance against
buy and hold. Directional overlays should be activated only after statistically significant,
out-of-sample evidence of persistence or reversal is established on the actual traded universe.

The one line worth carrying into implementation: **separate portfolio-construction economics from
directional prediction, and gate the second on measured dependence.**

## References

| Reference                                                                                                            | Status                         | Link                                                                                                                                                                      |
| -------------------------------------------------------------------------------------------------------------------- | ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Molchanov and Stangl, The Myth of Sector Rotation, International Journal of Finance and Economics                    | Verified at source             | https://acfr.aut.ac.nz/__data/assets/pdf_file/0005/294287/The-Myth-of-Sector-Rotation-non-blind.pdf                                                                       |
| Moskowitz and Grinblatt, Do Industries Explain Momentum?, Journal of Finance, 54(4), 1999                            | Verified at source             | https://paseman.com/Posts/JOF/199908%20-%20JOF%20-%20Moskowitz_old.pdf                                                                                                    |
| Bouchey, Nemtchinov, Paulsen and Stein, Volatility Harvesting, Journal of Wealth Management, Fall 2012               | Verified at source             | http://www.snifferquant.com/gyantal/Incode/papers/Volatility%20Harvesting_JWM_Fall_2012.pdf                                                                               |
| El Bernoussi and Rockinger, Rebalancing with transaction costs, Financial Markets and Portfolio Management, 37, 2023 | Verified at source             | https://link.springer.com/content/pdf/10.1007/s11408-022-00419-6.pdf                                                                                                      |
| Quant Data, Does sector momentum persist? Pre-registered test of four rotation states, 2026                          | Verified at source             | https://quantdata.uk/research/does-sector-momentum-persist                                                                                                                |
| Witte, Volatility Harvesting: Extracting Return from Randomness, arXiv 1508.05241, 2015                              | Abstract verified              | https://arxiv.org/abs/1508.05241                                                                                                                                          |
| Hameed and Mian, Industries and Stock Return Reversals, Journal of Financial and Quantitative Analysis               | Summary only                   | https://www.cambridge.org/core/journals/journal-of-financial-and-quantitative-analysis/article/abs/industries-and-stock-return-reversals/92B5101FBEA7B40717D46DB60913BF41 |
| Gatev, Goetzmann and Rouwenhorst, Pairs Trading: Performance of a Relative Value Arbitrage Rule, NBER WP 7032        | Summary only                   | https://www.nber.org/system/files/working_papers/w7032/w7032.pdf                                                                                                          |
| Anderson, Bianchi and Goldberg, Will My Risk Parity Strategy Outperform?, 2012, quoted in Bouchey et al.             | Quoted, quoted source verified | https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2390614                                                                                                               |
| Numerical perspectives on the rebalancing premium, Quantitative Finance, 2025                                        | Summary only                   | https://www.tandfonline.com/doi/full/10.1080/14697688.2025.2577822                                                                                                        |

## Appendix A: formulas

```
Rebalancing premium, equal weights, N assets (Bouchey et al.):

    g_p = sum(w_i * g_i) + (1/2) * [ sum(w_i * sigma_i^2) - portfolio_variance ]
    premium ~= (1/2) * sigma^2 * (1 - 1/N) * (1 - rho)

Two assets, the special case quoted in the literature:

    premium ~= sigma^2 * (1 - rho) / 4

Fixed weight versus buy and hold, two assets, two periods (El Bernoussi and Rockinger):

    W_fixed - W_buyandhold = -a1 * a2 * W0 * (r1,1 - r2,1) * (r1,2 - r2,2)
    E[D] = -a1 * a2 * { rho * sigma^2 + (mu - rf)^2 }

    Fixed weight beats buy and hold in expected wealth only when autocorrelation is
    more negative than minus the squared Sharpe ratio of the return differential.

Rebalancing opportunity:

    RebalancingOpportunity ~= sigma^2 * (1 - rho) - expected_cost

Rank information coefficient:

    IC_t = Corr(Rank_t, Rank_t+1)
```

## Appendix B: master evidence table

Every empirical result used above, with the attributes needed to judge transferability.

| Result                                  | Universe                          | Period       | Frequency                       | Cost assumption               | Out-of-sample?        | Label                     |
| --------------------------------------- | --------------------------------- | ------------ | ------------------------------- | ----------------------------- | --------------------- | ------------------------- |
| Business-cycle rotation, perfect timing | 48 FF industries                  | 1948 to 2018 | Cycle stages                    | None, then 0.5 to 1.5 percent | Full sample           | Verified                  |
| All 1,022 rotation rules                | 10 sectors                        | 1948 to 2018 | Two-stage cycles                | None                          | Full sample           | Verified                  |
| Cross-sector predictive regressions     | 10 sectors                        | 1948 to 2018 | Lags 1 to 24 months             | None                          | Full sample           | Verified                  |
| NAVFX versus S&P 500                    | Fund                              | 2010 to 2018 | Live                            | Realised                      | Live                  | Verified                  |
| Industry momentum (6,6)                 | 20 value-weighted industries      | 1963 to 1995 | 6-month formation, 6-month hold | None, 75bp break-even         | Full sample           | Verified                  |
| Industry momentum (1,1)                 | 20 value-weighted industries      | 1963 to 1995 | 1-month, 1-month                | None                          | Full sample           | Verified                  |
| Rebalanced 60/40 versus buy and hold    | Stocks and bonds                  | 1926 to 2010 | Rebalanced                      | After costs                   | Full sample           | Quoted, source verified   |
| Rebalancing premium, US equal weight    | Russell Global stocks             | 1997 to 2012 | Monthly                         | Before costs                  | Full sample           | Verified                  |
| Random 100-stock Monte Carlo            | Random US stocks                  | 1997 to 2012 | Monthly                         | Before costs                  | Simulation            | Verified, supporting only |
| Fixed weight versus buy and hold theory | Two assets                        | Model        | Two periods                     | Modelled                      | Theory                | Verified                  |
| Swiss pension portfolio rebalancing     | Multi-asset                       | 1999 to 2021 | Monthly                         | Modelled                      | Full sample           | Verified                  |
| Four rotation states                    | 11 SPDR sectors, 12 industry ETFs | 2010 to 2026 | 10, 20, 60 days                 | Not applied to returns        | 2022 to 2026 held out | Verified                  |
| Intra-industry reversal                 | US stocks                         | Various      | Monthly                         | Not confirmed                 | Not confirmed         | Summary only              |
| Pairs trading                           | US stocks                         | Historical   | Daily                           | Not confirmed                 | Not confirmed         | Summary only              |
