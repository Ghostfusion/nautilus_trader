# Whether sector rules are allowed: the regime verdict

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                          |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Nothing. It decides whether rules that buy and sell groups of shares are allowed to be used on a given set of data                                                                                                                                                                             |
| How often it trades       | Never on its own; it is run again whenever the data or the settings change                                                                                                                                                                                                                     |
| What you need             | Nothing but this page to read, or a web browser to run the tool                                                                                                                                                                                                                                |
| Where the rules come from | [Sector Regime Engine README](../../../implementation/sector-regime-engine/README.md), which states the decision rule                                                                                                                                                                          |
| The underlying research   | [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), this repository's own study of whether sector leadership can be predicted                                                                                                                                 |
| How well it held up       | Mixed: the measuring tool is accurate on invented data whose answer is known, but the real-market patterns it looks for are weak in the studies the project reviewed and several are indistinguishable from chance                                                                             |
| Also appears in           | [Telling which kind of market you are in](../measuring-the-regime/README.md) and [Contrarian reversion](../contrarian-reversion/README.md) in this collection, and the app it describes lives in [implementation/sector-regime-engine](../../../implementation/sector-regime-engine/README.md) |

## The idea in one paragraph

Most rules for trading groups of shares pick the groups that have been doing well. This app asks a
question that comes before any such rule: does the data show that doing well can be picked at all? It
measures the price history of the groups you give it, runs three statistical tests on that history, and
returns one of four verdicts. Two of the verdicts mean that picking winners is not supported, and the
app then switches such rules off; the other two allow a rule that follows winners or a rule that bets
against them. A verdict is more useful than a profit number because a profit number does not say
whether it came from a real pattern, from luck, or simply from the whole market rising. The app refuses
rather than guesses, because on a coin toss a guess is worth nothing, and the trading a guess would
cause still costs money.

## Why anyone believed it

Markets do not behave the same way all the time. There are long stretches when one industry keeps
leading, and other stretches when leadership changes every few weeks for no visible reason. It is
natural to believe that if you could tell which kind of stretch you were in, you would know whether to
follow the leader or to bet against it. That belief is the whole idea behind a regime verdict.

The reason it is easy to keep believing is hindsight. Looking back, any past period can be labelled:
this was a trending market, that was a choppy one. The labels always fit after the fact. The hard part
is deciding the label at the time, from the numbers alone, without knowing what happened next.

The counterparty matters as much here as in any trade. In a market where leaders really do keep
leading, the person selling you the strong group is often someone forced to sell for reasons of their
own, so you can be paid for taking the other side. In a market where leadership is a coin toss, the
person on the other side is simply someone who disagrees, and there is no reason either of you should
win; you just pay the gap between the buying and selling price. That difference is what the verdict is
meant to detect before any money moves.

## An everyday comparison

A doctor does not hand out antibiotics for every sore throat. First a test is run to see whether the
illness is the kind that antibiotics can help. If the test says it is not, the medicine is withheld,
even though the patient is unwell and the medicine exists. Withholding it is not a failure of the
doctor; it is the correct answer to the question actually asked.

This app is the test. The four verdicts are its possible results, and two of them mean "this treatment
does not apply". A tool that always found a pattern would be the doctor who always prescribes.

## The rules, step by step

1. Take a history of prices for a set of groups of shares, called sectors (energy, health care and so
   on), one price per sector per trading day.
2. For each day, rank the sectors by how well they have done over a fixed recent window.
3. On the whole history, compute three tests of whether the ranking has memory: the rank
   autocorrelation, the return autocorrelation, and the variance ratio. Each test returns an effect
   size and a p-value, where the p-value is the chance of seeing a result this extreme purely by luck.
4. Decide the significance threshold. Because three tests are run at once, divide the normal threshold
   of 0.05 by three, giving about 0.0167, so that running three tests does not give three chances of a
   false alarm.
5. For each test, decide whether the effect is large enough to matter. Each of the three has a
   deadband, a zone of effects considered too small to care about: 0.05 for the two autocorrelations
   and 0.20 for the variance ratio's distance from 1.
6. If there are fewer than 252 observations, about one year of trading days, return UNCERTAIN and stop.
7. Combine the results: a test that is both significant and outside its deadband is admitted. If an
   admitted test points to following winners, the verdict is positive; if it points to betting against
   winners, the verdict is negative. If admitted tests point both ways, or an effect is outside the
   deadband without being significant, the verdict is UNCERTAIN. If nothing is admitted and every
   effect is inside its deadband, the verdict is MEMORYLESS.
8. Read the verdict beside the list of permitted approaches. Momentum and contrarian rules are allowed
   only in the matching world; everything else is switched off.

## The maths, with every symbol named

The adjusted threshold, after allowing for three tests:

```text
alpha_used = alpha / 3
```

- `alpha` is the significance level, 0.05 by default, meaning a one-in-twenty tolerance for a false
  alarm.
- `alpha_used` is the stricter level actually used, about 0.0167 here.

A test is significant when its p-value falls below that level:

```text
significant = (p < alpha_used)
```

- `p` is the p-value of the test: the chance of a result this extreme if there were no pattern at all.
- A small `p` means "unlikely to be luck", but says nothing about the size of the effect.

A test is material when its effect lies outside the deadband:

```text
material = |effect| > deadband
```

- `effect` is the measured size of the pattern.
- `deadband` is the threshold below which an effect is treated as too small to act on.
- For the variance ratio the effect is measured as the distance from 1, so the rule reads
  `|variance_ratio - 1| > 0.20`.

A rule is admitted only when both conditions hold:

```text
admitted = significant AND material
```

- `admitted` is true only if the pattern is both hard to explain by luck and big enough to be worth
  acting on. This double condition is the heart of the app.

The verdict then follows:

```text
any admitted positive  -> POSITIVE_DEPENDENCE
any admitted negative  -> NEGATIVE_DEPENDENCE
both directions        -> UNCERTAIN
outside deadband only  -> UNCERTAIN
none admitted, all small -> MEMORYLESS
too few observations   -> UNCERTAIN
```

## A worked example

Three tests are run on a history of eleven sectors. The return autocorrelation line is taken from the
app's own default run, quoted in its manual; the other two values are invented, but they are of the
size such runs produce.

| Test                                 | Effect  | p-value | Deadband | Significant? | Material? |
| ------------------------------------ | ------- | ------- | -------- | ------------ | --------- |
| Cross-sectional rank autocorrelation | +0.021  | 0.310   | 0.05     | no           | no        |
| Return autocorrelation (pooled)      | -0.0208 | 0.0097  | 0.05     | yes          | no        |
| Variance ratio (five-day)            | +1.060  | 0.220   | 0.20     | no           | no        |

The adjusted threshold is `0.05 / 3 = 0.0167`. Only the return autocorrelation clears it, at 0.0097.
But its size, -0.0208, is far inside the deadband of 0.05, so it is not treated as signal. The app
says so plainly:

```text
Return autocorrelation (pooled) = -0.0208 is statistically detectable (p=0.0097)
but sits inside the deadband, so it is not treated as signal.
```

No test is admitted. Nothing sits outside the deadband without significance either, so the verdict is
MEMORYLESS, and the eligibility list blocks both the momentum and the contrarian rules. The honest
reading is that on this data, a rule that picks winners has nothing to pick.

Notice what happened: a result that would be reported as a genuine finding in a simpler tool was
measured, acknowledged, and then set aside, because being real and being worth trading are different
things.

## What the research actually found

The tool was tested against invented data whose answer is set in advance. Before the deadband was
added, the classifier got 27 of 30 seeded universes right; with the deadband added it reached 60 of
60. That is a statement about the measuring instrument, not about markets.

When it is pointed at real sector data, the underlying study reports that the usual answer is that
there is nothing to find. Molchanov and Stangl tested 1,022 different rotation rules on American
sectors from 1948 to 2018: the average rule returned 0.86 percent a month against 0.89 percent for
simply holding the market, and only 132 of the 1,022 beat buying and holding, which the authors
attributed to trying so many rules. Their cross-sector regressions produced 2,160 test statistics, of
which 6 percent were significantly positive at the 10 percent level against the 5 percent expected by
chance.

A later pre-registered test on 2010 to 2026 sector data, quoted in the same study, checked four popular
rotation states and found none of 24 cells confirmed; the one effect visible in the first decade shrank
to nothing in the data held back for the honest test. Read together, these results say that a verdict
of POSITIVE_DEPENDENCE on your own data is a finding about your data, and that the common outcome on
real markets is MEMORYLESS or UNCERTAIN.

## How this project relates to it

The whole verdict lives in [src/engine.js](../../../implementation/sector-regime-engine/src/engine.js).
The function `classifyRegime` is the decision rule above, written once; it reads the three primary
tests, applies the corrected threshold and the deadband, and returns the label with a list of plain-English
reasons. The function `buildTests` marks each measurement as primary, secondary or diagnostic, and only
the primary ones are allowed a vote.

The technical description in the [engine README](../../../implementation/sector-regime-engine/README.md)
states the decision rule in the same form used here and explains why the magnitude gate took the
classifier from 27 of 30 to 60 of 60. The [user manual](../../../implementation/sector-regime-engine/USER_MANUAL.md)
describes the same thing from the reader's side, in section 3 and in the panel descriptions for the
verdict and the eligibility list, and its section 2 lists everything the app deliberately does not do.

## Where it goes wrong

- The pretend data is pretend. The app opens on invented numbers with a known answer. A correct verdict
  there proves the instrument works; it says nothing about any real market.
- One run is not evidence. A single pass through one stretch of history can produce almost any verdict.
  Two datasets that cover different years can disagree completely.
- "Real" is not "worth doing". A pattern can be unmistakable and still far too small to cover the cost
  of acting on it. The deadband exists exactly for this, and the sentence "detectable but inside the
  deadband" is easy to misread as a bug.
- The verdict is about the past. It summarises what the history you supplied shows, not what will
  happen next month.
- MEMORYLESS is not a proof. It means no pattern was found, and absence of evidence is not evidence of
  absence. The app prints that caution itself.
- Settings can be tuned until the past looks good. Every dial in the configuration is a chance to fit
  the history; the settings are meant to be frozen before the results are read, not adjusted afterward.
- The cost figures are placeholders. Until the trading cost and the yearly fund charge are replaced
  with real figures, every "after costs" number is optimistic.

## Try it yourself

You need nothing but this page. Apply the decision rule by hand to the four rows below. The threshold
is `0.05 / 3 = 0.0167`; the deadbands are 0.05 for the two autocorrelations and 0.20 for the variance
ratio's distance from 1. For each row, write down the verdict and whether a winner-picking rule would
be allowed.

| Row | Rank autocorrelation | Return autocorrelation | Variance ratio  | Observations |
| --- | -------------------- | ---------------------- | --------------- | ------------ |
| 1   | +0.120, p 0.001      | +0.010, p 0.400        | +1.030, p 0.500 | 2,016        |
| 2   | +0.020, p 0.400      | -0.090, p 0.002        | +0.950, p 0.300 | 2,016        |
| 3   | +0.010, p 0.030      | -0.030, p 0.020        | +1.010, p 0.700 | 2,016        |
| 4   | +0.300, p 0.001      | +0.250, p 0.001        | +1.400, p 0.001 | 140          |

What to notice: row 1 admits a positive test, so it is positive and momentum is allowed. Row 2 admits a
negative one, so it is negative and contrarian is allowed. Row 3 has one significant test and one
outside its deadband, but not the same test, so it is UNCERTAIN. Row 4 looks the strongest by its
numbers and is still UNCERTAIN, because 140 observations is below the minimum of 252 and no verdict is
allowed on that little history. The row that looks most exciting is the row the app refuses.

## Where this came from

- [Sector Regime Engine README](../../../implementation/sector-regime-engine/README.md), the decision
  rule as implemented, including the corrected threshold, the deadbands, and the 60-of-60 validation.
- [Sector Regime Engine user manual](../../../implementation/sector-regime-engine/USER_MANUAL.md),
  section 3 (the four worlds), the panels for the verdict and eligibility, and section 2 (what the app
  does not do).
- [src/engine.js](../../../implementation/sector-regime-engine/src/engine.js), `classifyRegime` and
  `buildTests`, the code that produces the verdict and its evidence tiers.
- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), sections 8 and 9,
  the evidence hierarchy and the eligibility framing.
- Molchanov and Stangl, [The Myth of Sector Rotation](https://acfr.aut.ac.nz/__data/assets/pdf_file/0005/294287/The-Myth-of-Sector-Rotation-non-blind.pdf),
  the 1,022-rule experiment and the cross-sector regressions.
- Quant Data, [Does sector momentum persist?](https://quantdata.uk/research/does-sector-momentum-persist),
  the pre-registered test of four rotation states.

## Words used in this tutorial

- deadband: a zone of effects considered too small to matter, whatever their p-value.
- eligibility: whether the app permits a given approach, given the verdict.
- observations: individual pieces of data, usually one trading day each.
- p-value: the chance of seeing a result this extreme purely by luck; small means unlikely to be luck.
- primary test: the highest tier of evidence, and the only one allowed to decide the verdict.
- regime: which of the four worlds the data is judged to be in.
- significance: a verdict that a result is unlikely to be pure luck, which says nothing about whether it
  is large enough to act on.
- verdict: the app's single answer about which world the data is in.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
