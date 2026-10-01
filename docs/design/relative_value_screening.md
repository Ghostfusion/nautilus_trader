# Relative-value screening: a rotation-regime playbook reviewed as a specification

## Source and provenance

The source is an unattributed trading playbook on profiting from sector rotation: mean-reversion on
overextended sectors, dollar-neutral pairs and spread trading, index premium selling, volume dry-up
entries, and a Python screener for cointegrated sector ETF pairs. It carries no author, no
repository and no licence, so it is not a source review in the sense of
[`vectorbt_lessons_design.md`](vectorbt_lessons_design.md) or
[`vnpy_lessons_design.md`](vnpy_lessons_design.md): there is no upstream project to probe, no
revision to pin, and no provenance chain to satisfy.

**Nothing from the source is reproduced here.** The mechanisms below are restated in this project's
own terms, and every number is this project's own measurement on synthetic or documented data.
No source code, fixture, documentation or dependency enters this repository, and no dependency is
added to do it: the measurements in section 5 use only `numpy`, which is already present transitively
through `pandas` and is not declared as a project dependency by this document.

**Status.** Analysis only. No production code changed, and no test was added: the measurements
belong to a throwaway probe, not to the shipped suite. Sections 4 (candidate tranches) are proposals
awaiting the owner's authorisation under the working agreement, which permits code changes only for
defects unless a feature is authorised. Read section 4 as a menu, not as a work order.

**No market-specific semantics.** The source's subject matter is United States equity sector ETFs.
Nothing proposed here touches a market rule: a screen family, an estimator's sample-size floor and a
window declaration are statistical and engineering mechanisms, and the same obligations apply to any
market. Leg sizing, borrow availability and short-sale constraints remain owned by the execution and
instrument layers, and section 2.6 states why the source's sizing rule is a defect rather than a
market convention to be encoded.

### Revision summary

| Revision | Change                                                                                                                                                                                                                         |
| -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1        | Initial review: the source's mechanisms restated, nine defects established with measurements, the transferable obligations mapped onto the existing research contracts, and five candidate tranches proposed as not authorised |

## 1. What the source gets right

Three things, and they are worth stating before the defects, because they are the reason the document
exists at all.

1. **Cointegration is not correlation.** The source separates the two correctly, asks for an
   Engle-Granger two-step test rather than a correlation screen, and hedges with a regression slope
   rather than with equal notional (its position-sizing rule contradicts that, section 2.6). That is
   the right first cut, and the more common mistake in the wild is the opposite one.
2. **A relative-value trade is a set of decision rules, not a signal.** Entry, exit, stop and a
   holding-period bound are each declared, and the source names the conditioning regime explicitly.
   Declared rules are the thing this project's research contracts require (section 3), even though
   the source declares them as prose rather than as data.
3. **A parameter chosen without its estimator is not a parameter.** The source reaches for a
   half-life gate and a Z threshold, which is at least the correct *class* of control. Sections 2.2
   and 2.7 show why the values it suggests do not survive contact with the estimator it names.

## 2. Defects found, with the evidence

Each subsection states the source's claim, the defect, and the measurement that establishes it. The
estimator, parameters and seeds needed to reproduce every number are in section 5.

### 2.1 A screen is a trial family, and the source does not count it

**Claim.** Test every unique combination of eleven sector ETFs for cointegration at `p < 0.05` and
trade the ones that pass.

**Defect.** Eleven series make 55 unique pairs, and the source corrects nothing. At `p < 0.05` the
expected number of false cointegrations is 2.75, so a screen that passes two or three pairs has
demonstrated nothing at all. The sector ETFs also overlap heavily in constituents, so the tests are
not independent and the family is weaker than its nominal size suggests.

| Family size | Expected false positives | P(at least one) | P(at least three) |
| ----------- | ------------------------ | --------------- | ----------------- |
| 55          | 2.75                     | 0.9405          | 0.5232            |

**Reproduction.** Binomial with `n = 55`, `p = 0.05`, exact.

### 2.2 The half-life gate admits the pairs it exists to exclude

**Claim.** Estimate the half-life of mean reversion, keep pairs between 5 and 25 trading days, and
reject pairs outside that band as either too noisy or too slow.

**Defect.** The estimator is an ordinary least squares fit of the spread's first difference on its
lagged level. For a process this close to a random walk, that estimate is biased toward mean
reversion at the sample sizes involved: the least squares coefficient in a near-unit-root
autoregression with a constant is biased away from unity in the stationary direction, and the bias
shrinks only slowly as the window grows. The bias is one-sided: it makes a slow pair look fast,
which is exactly the direction that gets a pair past the gate.

Simulating a known autoregressive coefficient and applying the source's own estimator over 2000
paths:

| True coefficient | Window | True half-life | Mean estimate | Median estimate | Share below 5 | Share above 25 |
| ---------------- | ------ | -------------- | ------------- | --------------- | ------------- | -------------- |
| 0.95             | 252    | 13.86          | 11.98         | 11.10           | 0.015         | 0.021          |
| 0.95             | 60     | 13.86          | 8.35          | 5.71            | 0.399         | 0.029          |
| 0.98             | 252    | 34.66          | 25.47         | 19.63           | 0.001         | 0.335          |

The third row is the damaging one: 33.5 per cent of pairs whose true half-life is 34.7 days, well
outside the gate, are estimated at 25 days or less and therefore pass it. The second row shows the
other failure: over 60 bars, 39.9 per cent of pairs that do satisfy the band are rejected as too
fast. A gate applied to this estimator at 252 daily bars separates little more than noise from noise.

### 2.3 The continuous half-life formula and the discrete one disagree, and the source uses the continuous one

**Claim.** Half-life is `-ln(2) / theta` from the fitted coefficient.

**Defect.** That is the continuous-time solution for `dS = -theta S dt`, which is not what was
estimated. For an estimated discrete autoregressive coefficient `phi` the exact half-life is
`ln(0.5) / ln(phi)`. The continuous form is biased high, one-sidedly, by an amount that grows as the
process gets faster:

| `phi` | Continuous `-ln(2) / (phi - 1)` | Exact `ln(0.5) / ln(phi)` | Continuous bias |
| ----- | ------------------------------- | ------------------------- | --------------- |
| 0.90  | 6.931                           | 6.579                     | +5.3 per cent   |
| 0.95  | 13.863                          | 13.513                    | +2.6 per cent   |
| 0.98  | 34.657                          | 34.310                    | +1.0 per cent   |
| 0.99  | 69.315                          | 68.968                    | +0.5 per cent   |

Small, but in the same direction as the gate's ceiling: an estimate nudged upward is more likely to
be rejected as too slow, and a pair near the boundary flips on the formula rather than on the market.
The convention has to be declared once and used consistently, in the style of the divisor convention
the statistical contract already requires for a Sharpe ratio.

### 2.4 The prose and the code disagree about the sign convention of the parameter

**Claim.** The text writes the dynamics as `dS = -theta S dt` and then gives the half-life as
`-ln(2) / theta`.

**Defect.** In that convention the autoregressive coefficient is `1 - theta`, so the half-life is
`+ln(2) / theta` and the source's formula is wrong by a sign. The code, separately, estimates the
coefficient on the lagged level, where the same formula is correct under the opposite naming. Each
half is self-consistent; the two halves together are not, and a reader who implements the prose gets
a negative half-life, or takes the guard branch for an explosive process and reports "not
mean-reverting" for a pair that is. This is precisely the class of claim a specification review is
for: re-derived, never restated.

### 2.5 The cointegration test is documented as unsafe on exactly the inputs a screen produces

**Claim.** Screen sector ETFs for cointegration.

**Defect.** The library's own documentation states that the augmented Engle-Granger test
(`statsmodels.tsa.stattools.coint`, null hypothesis of no cointegration, MacKinnon's approximate
p-values from MacKinnon 1994 and 2010) *assumes no missing values and no gaps in the time series* and
carries an open item to drop them in the auxiliary regressions. Both scripts hide the problem with
`dropna(axis=1)`, which for these eleven ETFs is harmless and for a real universe silently drops
series with any gap: a series is removed for a data problem and the screen reports the survivors as
the universe. The same documentation notes that two nearly collinear series produce an infinite test
statistic and a p-value of zero, so a near-duplicate pair arrives as certain evidence.

Missing data is a first-class concern in this project, down to a capability code for a range that is
not covered and one for a range with gaps. A screen built on a test that documents gaps as
unsupported, over a universe it filters silently, is not a screen this project could record honestly.

### 2.6 The rule labelled dollar neutral is not dollar neutral

**Claim.** Size the pair so that `shares_long * price_long = beta * shares_short * price_short`,
described as dollar neutral, ensuring broad-market shocks create no net directional exposure.

**Defect.** That equation sets `notional_long = beta * notional_short`. At `beta = 2` with a long leg
at 50 and a short leg at 100, the long notional is 20,000 against 10,000 short: a net directional
exposure of 10,000 with a spread label on it.

| Rule          | Long notional | Short notional | Net exposure | Ratio |
| ------------- | ------------- | -------------- | ------------ | ----- |
| As written    | 20000         | 10000          | +10000       | 2.00  |
| Flat notional | 10000         | 10000          | 0            | 1.00  |

Dollar neutrality is the second row. Market neutrality is a third thing again: it requires each leg's
sensitivity to the market, which the rule does not mention and the regression does not estimate. A
sizing rule that is described as removing beta but does not remove it is worse than no rule, because
the risk report will say the position is hedged.

### 2.7 If a recursive estimator is used, its memory is chosen by the noise parameter, not the one the tuning table varies

**Claim.** Replace the static regression slope with a Kalman filter, tune the process-noise
parameter between 1e-5 and 1e-3, and warm the filter up for 30 to 60 bars.

**Defect.** For the scalar random-walk state the filter's gain converges to a fixed value, and with
it so does its effective memory, which is set jointly by the state noise `Q` and the observation
noise `R`: `P = (Q + sqrt(Q^2 + 4QR)) / 2` and `K = P / (P + R)`, so the memory is roughly `1 / K`.
The tuning table varies only the state noise. At the recommended observation-noise default the same
table spans a memory of 32 to 317 bars:

| State noise | Observation noise | Converged gain | Effective memory |
| ----------- | ----------------- | -------------- | ---------------- |
| 1e-4        | 1.0               | 9.95e-3        | 100.5 bars       |
| 1e-4        | 0.01              | 9.51e-2        | 10.5 bars        |
| 1e-5        | 1.0               | 3.16e-3        | 316.7 bars       |
| 1e-3        | 1.0               | 3.11e-2        | 32.1 bars        |

So the advice to warm up for 30 to 60 bars is shorter than the effective memory of its own
recommended default, which is 100.5 bars, and a tenfold smaller observation-noise value shortens the
memory tenfold with no change to the parameter the tuning table discusses. Measured on the two-state
filter with the source's structure and defaults, the covariance settles (largest change over the last
500 of 3000 steps is 1.5e-4 with a bounded regressor, and the slope gain settles at 7.6e-3, a memory
of about 131 bars), and the residual series over the first 200 bars has 36.8 times the standard
deviation of the settled series (4.1781 against 0.1135). Two things inflate the early series: the
state starts at zero while the pair's true intercept is around 3, and the covariance starts at the
identity.

The consequence is not that the filter is wrong; it is that the "dynamic hedge ratio" is a recursive
least squares with exponential forgetting whose memory the user never chose, and that any statistic
computed over a window reaching into the transient measures the filter's calibration rather than the
pair. Adaptivity is real only if the state noise or the observation noise is varied with the regime,
which is a declaration, not a default.

### 2.8 Three windows are mixed into one signal, and the data conventions are assumed

**Claim.** The screen fits the hedge ratio on the last 252 bars, computes a rolling Z-score over a
30-bar window of the spread, and estimates the half-life on a 252-bar slice of the same spread.

**Defect.** Three windows, one signal. The spread is built over the full history from a slope fitted
on a subset, the Z-score's dispersion is estimated over 30 bars while the slope that defines the
spread was fitted over 252, and the half-life is estimated over a third slice. Whether the pair is
"2 standard deviations from its mean" therefore depends on three unrecorded choices, and none of them
is stated in the output. This project has a contract for exactly this problem in the label layer:
the window, the entry offset and the measured reach are separate, declared quantities, and the reach
is measured from the produced series rather than derived from the definition.

Two data conventions are also assumed rather than checked. The price download defaults to
adjusting for dividends and splits, which is what makes a pair's spread comparable at all; the same
script against a version whose default was the opposite carries a systematic yield drift between a
high-yielding and a low-yielding sector, which reads as a spread that never reverts. The requested
period is also outside the set the library documents. Neither is an argument against the source's
pipeline in particular; they are the reason a dataset identity exists in this project.

### 2.9 A performance claim that its own code contradicts

The source describes its implementation as using vectorized `numpy` and `scipy` operations. The
listed code iterates a Python loop over every bar. This is trivial in consequence and not trivial in
kind: it is an unverified claim in a specification, which is the failure the parity protocol's
"re-derive, never restate" rule exists to catch.

## 3. What transfers to this repository

Five obligations, each already partly owned by an existing contract. The pattern is the same in each
case: the source names the right control and then specifies it in a way the control cannot survive.

### 3.1 A search must carry its trial family

A screen over a universe of `n` series is a trial family of `n * (n - 1) / 2` tests before any
filtering, and the family grows with every gate applied afterwards. This is the problem the
statistical contract was written for: the deflated Sharpe ratio exists because a selected maximum is
not a statistic until its trial count is declared. `nautilus_trader.optimization.significance`
already declares the contract, models trial dependence, derives provenance from runs, and refuses an
undeclared dependence; `nautilus_trader.optimization.capability` already answers
`EFFECTIVE_TRIALS_UNDECLARED`, `EFFECTIVE_TRIALS_UNEXPECTED` and `EFFECTIVE_TRIALS_OUT_OF_RANGE`.

The gap is that a *screen* has no shape: nothing in the repository enumerates a family of pairs, and
nothing forces the family size into the result. Feeding a screen's trials into the existing contract
is the whole of candidate `RV1`.

### 3.2 An estimator must declare its sample-size floor

The statistical contract already treats a minimum observation count as a declared parameter
(`DEFAULT_MINIMUM_OBSERVATIONS`) with an unavailable answer below it, and the significance capability
reports `INSUFFICIENT_OBSERVATIONS` with the number of periods still required. Section 2.2 shows the
same obligation for any persistence estimator: below some window the estimate is not merely noisy, it
is biased in a known direction and the bias passes the filter that reads it. The repository's version
of that statement is a floor plus a refused answer, not a caveat in prose.

### 3.3 A window is three quantities

Fit window, measurement window and reach. The label layer already separates them (`wait`, `horizon`,
alignment convention) and measures the reach from the produced series so a leakage policy shorter
than the reach is refused with the shortfall in nanoseconds. A relative-value signal needs the
identical treatment: which bars the slope was fitted on, which bars the dispersion was measured
over, and how far past the decision bar the estimate reaches.

### 3.4 A recursive estimator must declare its memory

Section 2.7. A Kalman filter, a rolling regression, an exponentially weighted mean and an expanding
window are all the same object with different memory, and the memory is a study parameter whether or
not it is written down. The repository's convention is that a policy is a value with a digest and a
declared version, and that an undeclared parameter which changes the answer is a configuration
error. A recursive estimator that does not declare its memory should be refused rather than
defaulted.

### 3.5 Not applicable is an answer

The source's "patience as a position" is portfolio advice, but its research-grade form is real and
already present: a statistic that cannot be computed reports a status and a reason rather than a
zero, and a request that cannot be served returns an unavailable capability with the requirements it
did not meet. A screen over a gapped, short or non-cointegrated universe should be able to answer
"nothing qualifies", with the reason, instead of returning the least-bad candidate.

## 4. Candidate tranches

All five are **not authorised, not implemented and not scheduled**. They are stated with acceptance
criteria so that the owner can authorise one without a second design round, and so that a refusal is
equally cheap. The listed order is cheapest first, and `RV1` is the only one that reuses an existing
contract end to end.

### RV1. A declared screen family behind the statistical contract

**Specification.** A screen declares its universe, its pair enumeration and its gates; the enumerated
family is counted before the gates run; the count and the surviving trials are carried into the
significance contract. A screen whose family cannot be declared returns an unavailable capability
rather than a result.

**Acceptance.** A screen over `n` declared series reports the family size `n * (n - 1) / 2` and the
number of tests actually evaluated; a screen against an undeclared universe answers the capability
with `EFFECTIVE_TRIALS_UNDECLARED`; a screened result recomputes the correction from the recorded
family; and a screen whose family is larger than the declared maximum answers
`EFFECTIVE_TRIALS_OUT_OF_RANGE` with the bound in the requirements.

### RV2. A persistence estimator with a declared convention and a floor

**Specification.** A single estimator of mean-reversion speed that declares its convention (discrete
or continuous) and its minimum observation count, converts once, and refuses below the floor.

**Acceptance.** A fit reports the convention it used, and the half-life it reports is the conversion
of its own fit under that convention, so the two conventions are distinguishable in the output
rather than agreeing by accident (section 2.3 shows they differ by more than a rounding); a fit below
the floor answers unavailable with the periods still required; and an explosive fit is a distinct
answer from an insufficient one.

### RV3. A relative-value window declaration

**Specification.** A signal declares its fit window, its measurement window and its alignment, and
reports the reach it covers, in the shape the label definition already uses.

**Acceptance.** Two signals with the same parameters but different alignment report different
finite reaches; a policy whose exclusion is shorter than the reach is refused with the shortfall; and
a declaration whose fit window does not precede the measurement window is a construction error.

### RV4. A recursive estimator that declares its memory

**Specification.** Any adaptive estimate declares its effective memory and its warm-up, and a
warm-up shorter than the memory is refused.

**Acceptance.** The declared memory matches the converged gain of the underlying recursion to a
declared tolerance; a warm-up shorter than the memory raises a construction error naming both; and a
refused estimate never reaches a caller as a number.

### RV5. Relative-value capability codes

**Specification.** Extend the research capability vocabulary for the data-side refusals a screen
produces: a gapped range, a series whose history begins after the screen's window, a pair whose
series are indistinguishable, and a universe too small for the requested family.

**Acceptance.** Each refusal is reachable from a constructed input; each carries the requirement it
did not meet as a requirement string and never as a detail; and the source scan that rejects a
detail match in either language continues to pass.

## 5. Reproduction of the measurements

Every number in section 2 is re-derivable from this section without shipping a script. All of it is
`numpy` only.

**Section 2.1.** Binomial probability, `n = 55`, `p = 0.05`, exact.

**Sections 2.2 and 2.3.** Simulate `s_0 = 0`, `s_t = phi * s_{t-1} + e_t` with `e_t` standard normal,
for `T` bars and 2000 paths under `numpy.random.default_rng(7)`. Regress the first difference of `s`
on a constant and the lagged level; take `theta` as the coefficient on the level; report
`-ln(2) / theta`, marking a non-negative coefficient as not mean-reverting. The exact discrete
conversion in section 2.3 is evaluated analytically at the stated coefficients.

**Section 2.7.** Scalar steady state: solve `Q = P^2 / (P + R)` for the positive root, then
`K = P / (P + R)`, with `Q = delta / (1 - delta)`. Two-state filter: state `[alpha, beta]`,
observation matrix `[1, x_t]`, recursion `P <- P + Q`, `S = H P H' + R`, `K = P H' / S`,
`theta <- theta + K e`, `P <- P - K H P`, initial state zero and initial covariance the identity.
Regressor `x = 50 + cumsum(standard normal * 0.2)` recentred to a mean of 50, observation
`y = 1.2 x + 3 + standard normal * 0.1`, 3000 steps, 1e-4 state noise, 1.0 observation noise,
`numpy.random.default_rng(11)`. The growing-regressor variant uses a step of 1.0 instead of 0.2.

**Section 2.6.** Arithmetic from the stated equation at `beta = 2`, long price 50, short price 100,
100 short shares.

### 5.1 Not measured

- No real market data was used, so no statement here is evidence about any pair's actual behaviour.
  The measurements are properties of the estimators and the arithmetic, which is the point: they hold
  whatever the market does.
- The transaction-cost, borrow-cost and capacity consequences of any strategy in the source were not
  modelled, and no claim is made about whether any of it is profitable.
- The market-neutral sizing in section 2.6 is stated as a condition, not implemented as a rule;
  nothing here says what a correct position size is for a given account.
- Whether older releases of the price download library defaulted to unadjusted prices was not
  verified; section 2.8 states the version-dependence as a hazard, and the current default as
  verified from the library's own documentation.

## 6. Open questions for the owner

1. **Is a relative-value domain in scope at all?** Nothing here needs to be built for the source's
   advice to be understood, and the record may be the entire deliverable.
2. **If it is, is `RV1` first?** It is the cheapest by a wide margin because it reuses the
   statistical contract, and it is the only one whose absence makes the other four unsafe to use: a
   half-life gate without a counted family is the defect in section 2.2 with extra steps.
3. **Where would a persistence estimator live?** The precedents disagree and the choice matters. The
   portfolio statistics are compiled because they are calculations over a return series; the label
   layer is Python because a label is an object over a future path with a definition. A screen's
   estimator is batch and contextual, which points at the Python research surface, but a streaming
   relative-value indicator would point at the indicator crate, which already carries a rolling
   z-score.
4. **Are the source's gates wanted as policy objects?** Entry, exit, stop and holding bound could be
   declared data in the shape of the label definition, or left as caller logic. Declaring them is
   what makes them reviewable, and it is also what makes the leakage relation enforceable against
   them.
5. **Does the source's options material need recording?** This document does not cover index premium
   selling or long-gamma structures: the research surface has no specification for them, no
   measurement was taken, and the source's statements are unfalsifiable as written.
