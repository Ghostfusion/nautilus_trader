# 06 - Measuring and evaluating options

This lecture explains the numbers you will read on a chain and on a surface, what range is
healthy, and the three mistakes beginners make most often. It ends with the Rust tests that
prove the surface refuses bad data.

## The Greeks in plain words

A greek is a sensitivity: how much the option's price changes when one input changes. All
five are reported per contract by the venue, and the local calculator can compute them from
cached prices. The definitions below are from [greeks.md](../../concepts/greeks.md).

| Greek | Field   | Plain meaning                                                            | Typical sign for a long option           |
| ----- | ------- | ------------------------------------------------------------------------ | ---------------------------------------- |
| Delta | `delta` | How much the option price moves when the underlying moves by one unit.   | Positive for a call, negative for a put. |
| Gamma | `gamma` | How much delta itself moves when the underlying moves.                   | Positive for a long option.              |
| Vega  | `vega`  | How much the price moves per one percentage point of implied volatility. | Positive for a long option.              |
| Theta | `theta` | How much the price moves per day of passage of time.                     | Usually negative for a long option.      |
| Rho   | `rho`   | How much the price moves per change in the interest rate.                | Small; often reported as zero.           |

Read a delta of 0.5135 on a call and say: a one unit rise in the underlying is expected to
raise the option price by about 0.5135 units. A delta of -0.30 on a put means a one unit
rise in the underlying lowers the put by about 0.30. Delta is also a rough probability of
finishing in the money; `itm_prob` reports that probability directly.

Gamma of 0.0393 means delta changes by about 0.0393 when the underlying moves one unit. High
gamma means your exposure is unstable: a hedge that is neutral now will not stay neutral. A
short-option position has negative gamma and is hurt by large moves in either direction.

Theta of -0.0210 means the option loses about 0.0210 of value per day if nothing else
changes. This is time decay. It accelerates as expiry approaches, which is why holding a
bought option into a quiet week feels like the price bleeds away.

Vega tells you how exposed you are to the market's expectation, not to the market itself. A
vega of 0.1967 per percentage point means a two point rise in implied volatility adds about
0.39 to the option price, with no move in the underlying at all.

## Good and bad values

There is no universal good number, because these are sensitivities and they depend on the
contract. What matters is whether the numbers are consistent.

| Reading                               | Healthy                                                            | Warning                                                           |
| ------------------------------------- | ------------------------------------------------------------------ | ----------------------------------------------------------------- |
| Delta                                 | Between -1 and +1 for a single contract.                           | Outside [-1, 1] suggests a convention problem, not a market move. |
| Delta of a call                       | Positive, and near 1 deep in the money, near 0 far out.            | A call with a large negative delta is wrong.                      |
| Delta of a put                        | Negative, and near -1 deep in the money, near 0 far out.           | A put with a large positive delta is wrong.                       |
| Gamma                                 | Positive for a long option, higher near the money and near expiry. | Negative gamma on a position you believe is long options.         |
| Theta                                 | Negative for a long option, more negative near expiry.             | A large positive theta on a bought option.                        |
| Vega                                  | Positive for a long option, larger for longer expiries.            | Vega that does not fall as expiry shortens.                       |
| Implied volatility                    | A positive decimal such as 0.60.                                   | Zero, negative, or an implausibly large number such as 5.0.       |
| Call mid versus put mid at one strike | The call is richer when the forward is above the strike.           | A gap that implies a different forward on every strike.           |

## The engine's own reports

The backtest engine produces three reports through
`generate_account_report`, `generate_order_fills_report`, and
`generate_positions_report`. In the program from [03](03-first-run.md) the account report
shows the starting balance and the fills and positions frames are empty because the strategy
subscribes to data and does not trade. That is the correct result for a data-only pipeline:
an empty fills report means no order was sent, not that the run failed.

| Report             | What it answers                                    | How to read a data-only run                        |
| ------------------ | -------------------------------------------------- | -------------------------------------------------- |
| Account report     | What is the balance and margin state at each step? | One row with the starting balance and zero locked. |
| Order fills report | What orders filled, at what price and size?        | Empty, since no orders were sent.                  |
| Positions report   | What positions were open, at what cost?            | Empty, since nothing was bought or sold.           |

When you add trading to the pipeline, these three reports become the first place to check
whether the strategy did what you think it did.

## The volatility surface: what the checks protect

The Rust module `crates/model/src/data/volatility_surface.rs` builds a surface from chain
observations. It is not callable from Python. Its job is to turn many prices into a smooth
function of strike and expiry that does not contain a free lunch. The checks exist because a
fitted curve can create prices no real market would show.

The surface works in total implied variance `w = sigma^2 * T` against log moneyness
`k = ln(K / F)`, with time to expiry as the second axis. Three conditions are validated, and
a surface that fails any of them is refused, never repaired:

- **Monotonicity and convexity (the butterfly condition).** Interpolating total variance
  piecewise linearly against log moneyness preserves the observations' ordering and
  convexity, so the interpolant cannot introduce a butterfly arbitrage the data did not
  already contain. A butterfly arbitrage is a combination of three strikes at one expiry
  that costs nothing and can only gain. It appears as a negative implied density, checked
  through Durrleman's function.
- **Calendar condition.** For each log moneyness the total variance must not decrease as
  time to expiry grows. If a later expiry were cheaper in total variance than an earlier
  one, a calendar spread would be a free lunch. The surface refuses that data.
- **Extrapolation honesty.** Beyond the quoted moneyness and expiry ranges, total variance
  is held flat. A continued curve would extrapolate local shape and can invent arbitrage,
  while a flat value keeps the boundary and cannot. Every query outside the quoted region
  reports `extrapolated: true`, so a user can tell an interpolated value from an extrapolated
  one.

What this protects you from, in one sentence: it stops a smooth curve from printing a price
that no set of real option quotes could support, and it tells you when a value came from
beyond the data rather than from inside it.

### The filters

Before any fitting, each observation passes through named filters and every rejection is
counted by reason, never silently dropped. The reasons are: an invalid quote, a crossed
market where the bid exceeds the ask, a bid below the configured minimum, a spread above the
configured maximum, a time to expiry below the minimum, a volume below the minimum, a stale
observation by age, and an implied volatility the solver could not produce. If you see many
rejections for one reason, that is information about the feed, not noise.

The declared minimums are three usable observations per expiry slice and two slices for a
calendar statement. Three is the smallest number of points that can carry a convexity claim,
and one slice cannot make a statement about time.

### Fit confidence and contributing observations

Two numbers accompany every query.

- **Contributing observation count (`observation_count`).** How many accepted observations
  fed the slice or the query. A value of 3 means the bare minimum for a convexity claim, so
  treat the result with caution. A value of 30 means the fit is resting on real data. If a
  query reports zero, the surface has no slice there.
- **Fit error (`fit_error`).** The root-mean-square residual of the fit at the observed
  nodes. Small means the fitted curve passes close to the quotes, so the surface faithfully
  represents the market. Large means the curve is smoothing over disagreement in the quotes.
  A large error is a warning that the observations themselves are inconsistent, not that the
  fit is bad at arithmetic. Read fit error together with the observation count: a small
  error on three points is not the same confidence as a small error on thirty.

When a slice cannot be made to satisfy the conditions under straight interpolation, the
surface can fall back to an arbitrage-free SVI family, whose own constraints are then
checked. If that also fails, the surface is refused with a typed error.

## The surface tests, run

From the repository root:

```bash
export CARGO_TARGET_DIR='D:/Users/vince/PycharmProjects/nautilus_trader/target'
cargo nextest run --locked -p nautilus-model --features python -E 'binary(volatility_surface)'
```

The real output is:

```text
    Starting 10 tests across 1 binary (5 binaries skipped)
        PASS [   0.039s] ( 1/10) nautilus-model::volatility_surface test_svi_family_fits_and_satisfies_its_own_conditions
        PASS [   0.059s] ( 2/10) nautilus-model::volatility_surface test_non_convex_slice_selects_the_svi_fallback
        PASS [   0.062s] ( 3/10) nautilus-model::volatility_surface test_query_beyond_moneyness_range_extrapolates_flat_to_boundary
        PASS [   0.143s] ( 4/10) nautilus-model::volatility_surface test_single_slice_refuses_calendar_statement
        PASS [   0.161s] ( 5/10) nautilus-model::volatility_surface test_query_beyond_expiry_range_extrapolates_flat
        PASS [   0.161s] ( 6/10) nautilus-model::volatility_surface test_query_inside_fitted_region_interpolates_and_is_not_flagged
        PASS [   0.170s] ( 7/10) nautilus-model::volatility_surface test_declared_minimums
        PASS [   0.174s] ( 8/10) nautilus-model::volatility_surface test_too_few_observations_refuses_a_slice
        PASS [   0.177s] ( 9/10) nautilus-model::volatility_surface test_calendar_violation_is_rejected
        PASS [   0.188s] (10/10) nautilus-model::volatility_surface test_each_named_filter_rejects_its_own_case_and_is_counted
     Summary [   0.192s] 10 tests run: 10 passed, 0 skipped
```

Each test name is a promise the code keeps:

- `test_query_inside_fitted_region_interpolates_and_is_not_flagged`: a query between nodes
  returns an interpolated value and leaves `extrapolated` false.
- `test_query_beyond_moneyness_range_extrapolates_flat_to_boundary` and
  `test_query_beyond_expiry_range_extrapolates_flat`: outside the quoted region the value is
  flat and `extrapolated` is true.
- `test_calendar_violation_is_rejected`: data where a later expiry has lower total variance
  is refused.
- `test_too_few_observations_refuses_a_slice` and `test_declared_minimums`: fewer than three
  usable observations per slice, or fewer than two slices, is refused with a typed error.
- `test_single_slice_refuses_calendar_statement`: one slice cannot make a calendar claim.
- `test_each_named_filter_rejects_its_own_case_and_is_counted`: every filter reason rejects
  its case and increments its own count.
- `test_svi_family_fits_and_satisfies_its_own_conditions` and
  `test_non_convex_slice_selects_the_svi_fallback`: the SVI fallback is used only when needed
  and passes its own constraints.

## Three common beginner misreadings

1. **Treating delta as the whole story.** Delta is the first-order sensitivity only. It goes
   stale as the underlying moves, and that change is gamma. A delta-neutral position with
   large gamma becomes directional after a move, which is the entire risk of a delta-hedged
   option book.
2. **Reading a small fit error as high confidence.** A small fit error on three observations
   is a curve through three points, not a market. Always read `fit_error` next to
   `observation_count`, and treat a query with `extrapolated: true` as a model statement, not
   a quote.
3. **Confusing implied volatility with realized movement.** Implied volatility is a price
   input solved from the option's market price. It is what the market is paying for movement,
   not what the underlying has done or will do. It can rise while the underlying sits still,
   and that change alone moves your position through vega.

## Check your understanding

Take the chain snapshot from [04](04-sample-data.md). For the near-expiry 65000 call, the
delta should be near 0.5 because the strike is at the money, gamma should be small for a
quarter of a year to expiry, and vega should be larger than for the far-expiry wings. If a
feed reports a call delta of -0.5 at the money, stop and check the convention: venue greeks
carry a `convention` field, and mixing conventions inside one portfolio is a fast way to
build a position with the wrong sign.
