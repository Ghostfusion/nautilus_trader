# Sector Regime Engine: user manual

For readers with no technical or financial background. No previous knowledge is assumed, and every
term is explained where it first appears. If you only read one section, read section 10, the cheat
sheet.

---

## 1. What this app does, in plain words

Stock markets are divided into groups of similar companies, called **sectors**. Banks are one sector.
Oil and gas companies are another. Health care is another. There are eleven of them in this app.

Money does not sit still between these groups. Some months banks do well and oil does badly; other
months it is the reverse. That movement of money from one group to another is called **sector
rotation**. The obvious question is whether you can spot which group is about to do well and move your
money there first.

This app answers a narrower and more useful question: **is there any reliable pattern in that movement
at all, in the data you have given it?**

It measures the data and then gives one of four verdicts. If there is no reliable pattern, it says so,
and it refuses to suggest any strategy that depends on spotting one. That refusal is the point of the
app. Most tools of this kind promise to find patterns; this one is built to tell you honestly when
there is nothing to find.

Think of it as a thermometer, not a doctor. A thermometer tells you the temperature. It does not
decide what you should do about it.

### What you will need

- A web browser (Chrome, Edge, Firefox or Safari).
- Nothing else, to start. The app comes with pretend data so you can see how it behaves.

---

## 2. What this app does not do

Being clear about this saves a great deal of confusion later.

| It does not...                                       | Because...                                                                          |
| ---------------------------------------------------- | ----------------------------------------------------------------------------------- |
| Tell you what to buy or sell                         | It contains no prices feed, no broker connection, and no recommendation of any kind |
| Predict the future                                   | It measures the past and reports how much of it looks like luck                     |
| Place any trades                                     | It never connects to a trading account. It cannot move money                        |
| Produce a complete investing strategy                | It is one tool for one question, not a plan                                         |
| Give financial advice                                | It is research software. Decisions and their consequences remain yours              |
| Work on made-up data and tell you about real markets | The pretend data is pretend. See section 9                                          |

If you are looking for something that tells you which sector to buy next month, this is the wrong tool.
If you want to know whether such a signal even exists in your data, this is exactly the right tool.

---

## 3. The one idea you need to understand

The app sorts data into one of **four worlds**. Everything else in the app follows from this.

Imagine tracking which of two shops is busier each week.

**World 1: no pattern.** Which shop is busier this week tells you nothing about next week. It is
essentially a coin toss. The app calls this **MEMORYLESS**, meaning the data has no memory.

**World 2: a pattern that continues.** Whichever shop is busier tends to stay busier for a while.
Trends persist. The app calls this **POSITIVE_DEPENDENCE** and you will also see the word
**momentum**, which means exactly this: what has been winning keeps winning, for a while.

**World 3: a pattern that flips back.** Whichever shop is busier tends to become the quieter one
next. Trends reverse. The app calls this **NEGATIVE_DEPENDENCE**, and you will see the word
**contrarian**, which means betting against whatever has just been winning.

**World 4: not enough evidence.** The data is too short or too muddled to say. The app calls this
**UNCERTAIN**. This is a real answer, not a failure.

One warning that the app takes seriously: **"no pattern" and "a pattern that flips back" are not the
same thing.** If something is a coin toss, betting against the last winner is just as hopeless as
betting on it. Many people confuse these two, and the app keeps them apart on purpose.

### Why the verdict matters so much

Two of the four verdicts, MEMORYLESS and UNCERTAIN, switch off every strategy that tries to pick
winners. The other two allow such a strategy to be switched on. The app will not let you enable a
winner-picking rule in a world where winners cannot be picked. Section 6, panel 5 shows this directly.

---

## 4. Opening the app

### The easy way

1. Open the folder `implementation/sector-regime-engine`.
2. Double-click the file named `index.html`.

Your browser opens and the app starts working immediately. It will take a few seconds.

### If the page looks empty

You may see a short message:

> The bundle app.js is not present. Run bun run build in this directory, then reload.

This means the app's engine file is missing. It is a single file that is normally built automatically.
If you are not technical, ask whoever set this up for you to run this one command in that folder:

```
bun run build
```

Then reload the page. You should not need to do anything else.

### The alternative way (for a helper)

If someone technical is assisting, they can run `bun run serve` in the same folder and then open
`http://127.0.0.1:8787` in a browser. This gives exactly the same app. It is only needed if the
double-click route runs into trouble.

### What you should see

A dark page with a heading, then twelve numbered panels stacked down the page, beginning with
"1. Data". The app runs itself once on pretend data as soon as it opens, so panels 3 onwards will
already be filled in.

If you see an old, half-finished version of the page, press `Ctrl` and `F5` together (on a Mac,
`Cmd` and `Shift` and `R`) to force a fresh load.

---

## 5. A five-minute guided tour

Follow this once, in order. You cannot break anything.

**Step 1.** Look at panel 1, "Data". Underneath the two small round buttons it says something like:

> Synthetic data with known regime MEMORYLESS. The classifier measured MEMORYLESS. Agreement.

"**Synthetic**" means pretend, made up by the app. "**Known regime**" means the app made the data with
a known answer built in, like a practice test with the answer key. "**Measured**" is what the app
worked out from looking at the data alone, without peeking. "**Agreement**" means it got the right
answer. This is the app checking itself.

**Step 2.** Press the blue **Run analysis** button. Everything recalculates, and the verdict panel
flashes briefly to show that something happened. Because nothing has changed yet, the numbers come out
identical, and the app tells you so in the small line beside the button:

> ran at 5:31:29 PM - 2016 observations across 11 sectors (settings unchanged, so this is the same
> result again)

That sentence matters. Pressing the button with the settings untouched recomputes the same answer, and
without that line it would look exactly as though the button were broken. It is not. The line also
reports the time of the run, so you can always tell a fresh result from a stale one.

**Step 3.** Look at panel 3, "Regime verdict". You should see a large coloured word, one of the four
worlds, followed by "confidence", a number of "observations", and a short list of reasons.

**Step 4.** Read the reasons. They are written in plain English and they tell you exactly which
measurement produced the verdict. One of them, in this default run, says something like:

> Return autocorrelation (pooled) = -0.0208 is statistically detectable (p=0.0097) but sits inside the
> deadband, so it is not treated as signal.

This is the app being careful. A number looked interesting, but it was too small to matter, so the app
ignored it rather than acting on it. You will meet this idea again in section 9.

**Step 5.** Look at panel 5, "Eligibility". This is a list of strategies with the words "eligible" or
"blocked" beside each. In the default run, the two that pick winners are blocked. That is the app
following through on its verdict.

**Step 6.** Scroll to panel 7, "Equity curves". Four coloured lines show how different approaches
would have grown or shrunk over the period. Look at the legend underneath to see which is which.

**Step 7.** Look at panel 8, "Attribution". This splits the final result into parts, so you can see
where the money came from or went. It is the most informative panel in the app, and section 6,
panel 8 explains it line by line.

**Step 8.** In panel 1, change the dropdown **Ground-truth regime** from `memoryless` to `persistent`,
then press **Run analysis** again.

**Step 9.** Look at panels 3 and 5 again. The verdict has changed to POSITIVE_DEPENDENCE, and the
"momentum" row in Eligibility now says "eligible". The app has found a real pattern, because you told
it to build data that contains one.

**Step 10.** Now change the **Overlay** dropdown in panel 2 from `none` to `momentum`, and press
**Run analysis** once more. Look at panel 7: the legend now includes "with overlay (gross)", and
panel 8 shows a line for the overlay's contribution. You have just watched a strategy be allowed and
then used, in a world where it is allowed.

**Step 11.** Finally, change **Ground-truth regime** to `uncertain (short sample)` and run again. The
verdict becomes UNCERTAIN, and both winner-picking rows go back to "blocked", even though you just
switched the overlay on. The app is refusing to act on too little evidence.

You have now seen all four worlds. That is the whole app.

---

## 6. The panels, one by one

### Panel 1: Data

**What it shows.** Where the numbers come from, and (for pretend data) whether the app measured it
correctly.

**Plain meaning.** This is the raw material. Either the app invents it with a known answer, or you
supply a file of real historical prices.

**What to look for.** If you are studying real markets, make sure you are on the CSV option, not the
pretend data. Look at the grey note under the buttons for the agreement check.

**What it does not mean.** Pretend data tells you nothing about real markets. It only tells you
whether the app is working.

### Panel 2: Frozen configuration

**What it shows.** All the numbers that control how the app measures and calculates, plus a short code
called the **fingerprint**.

**Plain meaning.** "Frozen" means you are supposed to decide these *before* you look at results, and
then leave them alone. The fingerprint is a short label for that exact set of choices, so a result can
always be traced back to the settings that produced it.

**What to look for.** The fingerprint changes whenever you change any setting. If you compare two
results and the fingerprints differ, you changed something.

**What it does not mean.** It does not mean the settings are good ones. It only means they are fixed
and identifiable.

### Panel 3: Regime verdict

**What it shows.** The headline answer: which of the four worlds the data is in, how confident the app
is, and the reasons.

**Plain meaning.** This is the thermometer reading.

**What to look for.** The reasons list. It is written to be read, and it tells you which measurements
drove the answer. If the verdict is MEMORYLESS, the reasons will tell you that everything was too
small to matter. If UNCERTAIN, they will tell you what was missing.

**What it does not mean.** "Confidence medium" does not mean the market will behave that way next
month. It means the app is reasonably sure about what the *past* data shows.

### Panel 4: Two independent axes

**What it shows.** Two separate sets of measurements, side by side. The left card is about *timing*
patterns (does what happened before tell us anything about what happens next?). The right card is
about *spread* (how different are the sectors from each other, and do they move together?).

**Plain meaning.** These are two unrelated questions that people constantly muddle together.

**What to look for.** On the right, "Implied rebalancing premium" is the app estimating how much a
tidy-up habit is worth in this data, before costs. On the left, the four numbers are the raw material
for the verdict.

**What it does not mean.** A large number on the right does not create a timing opportunity. Spread
and timing are separate.

### Panel 5: Eligibility

**What it shows.** Six possible approaches, each marked "eligible" or "blocked", with a reason.

**Plain meaning.** The app's permissions list. In a world with no pattern, the approaches that try to
pick winners are not permitted.

**What to look for.** The two winner-picking rows (Momentum overlay, Contrarian overlay) and whether
the reason says "Regime admits it" or "Blocked".

**What it does not mean.** "Blocked" is not a criticism of the approach. It means the data does not
support using it.

### Panel 6: Metrics with the evidence hierarchy

**What it shows.** Eight measurements in a table, each with a coloured tag: `primary`, `secondary`, or
`diagnostic`, plus its value, a score, a p-value, and how many observations it used.

**Plain meaning.** The evidence hierarchy is a ranking of trustworthiness. Only the three `primary`
rows are allowed to decide the verdict. The others describe the data but do not get a vote.
`diagnostic` rows are the least trustworthy and are labelled that way deliberately.

**What to look for.** The `p` column. In plain terms, a **p-value** is the chance of seeing a result
this extreme purely by luck. Small means "unlikely to be luck". A common threshold is 0.05, which
means "less than a one in twenty chance this is luck".

**What it does not mean.** A small p-value is not the same as a big effect. Something can be
unmistakably real and still far too small to be worth acting on. That is exactly what the deadband
handles. See section 9, trap 3.

### Panel 7: Equity curves

**What it shows.** Four lines tracking the value of a pot of money over the period, all starting at the
same level. The legend below names them: "drifting basket (gross)", "drift-band rebalanced (gross)",
"actual book" (or "with overlay") "(gross)", and "net of all costs".

**Plain meaning.**

- **Basket, drifting** means an equal split across all eleven sectors, left alone to drift.
- **Drift-band rebalanced** means the same split, but tidied back to equal whenever it wanders too
  far. Tidying means selling a little of what has grown and buying a little of what has shrunk.
- **Actual book** is what the app actually ran, which includes the overlay if one is switched on.
- **Net of all costs** is the same thing after paying the tolls of trading and the yearly charges that
  funds deduct.

"**Gross**" means before those costs. A line labelled gross is a bit of a fiction: it shows what would
have happened if trading were free. It is shown anyway, because comparing gross with net is how you
see what the tolls are doing.

**What to look for.** The gap between the gross and net lines. That gap is the price of activity, and
it is often the most surprising thing a beginner learns from this app.

**What it does not mean.** Four lines on one chart cannot tell you which approach is best. This is one
run, through one particular stretch of history. See section 9.

### Panel 8: Attribution

**What it shows.** The total result broken into named parts, as a table. Every line is a percentage per
year as well as a raw figure. At the bottom, a line called "Additivity residual" shows a very small
number.

**Plain meaning.** Attribution answers "where did the result actually come from?" instead of
"what was the total?". Without it, a good total could be caused by something entirely different from
what you think.

The lines, in plain terms:

| Line                              | Plain meaning                                                                  |
| --------------------------------- | ------------------------------------------------------------------------------ |
| Underlying sector exposure        | What simply owning the sectors did, with no cleverness at all                  |
| Rebalancing contribution (type B) | What the tidying habit added or cost, compared with just leaving the pot alone |
| Directional overlay contribution  | What the winner-picking extra added or cost                                    |
| Trading cost                      | The tolls paid each time the app bought or sold                                |
| Holding cost                      | The yearly fund charges                                                        |
| Total                             | Everything added together                                                      |

There are two different "rebalancing contribution" lines, labelled type A and type B, and they often
have opposite signs. This is not an error. They answer two different questions: type A compares tidying
against what the sectors did on their own, type B compares it against leaving the pot alone. Both are
correct; they are just not the same question. This app is careful to keep them apart, because most
popular explanations of investing quietly mix them up.

**What to look for.** The residual. It should be a tiny number like `-4.7e-15`. That is the app
proving its own arithmetic adds up. It is the app's honesty check on itself, printed where you can see
it.

**What it does not mean.** A positive "overlay contribution" does not mean the overlay is clever. The
overlay changes what you own, so some of its contribution is simply owning different things. The app
labels that line to say so.

### Panel 9: Cross-sectional structure

**What it shows.** Two small charts and a coloured grid.

- The first chart: how different the sectors were from each other over time.
- The second chart: how closely the sectors moved together over time.
- The grid: a table of colours, one cell per pair of sectors. Warm colours mean they move together;
  cool colours mean they do not.

**Plain meaning.** This is the texture of the market. If all sectors move together, tidying the pot
achieves little. If they move differently, tidying achieves more.

**What to look for.** The number on the diagonal, which is always exactly 1.00, and the numbers off it.
Values around 0.4 to 0.6 are common for sectors.

**What it does not mean.** A colourful grid is not a signal. It describes the market's shape, not its
future.

### Panel 10: Frequency and band grid

**What it shows.** A table of sixteen rows. Each row is one combination of two choices: how often to
check the pot, and how far it is allowed to wander before being tidied.

**Plain meaning.** This is the app running the same idea sixteen ways to show you whether those two
choices matter much. Every figure in the table is after costs.

**What to look for.** The "Rebalances" and "Turnover" columns, which tell you how much activity each
combination creates, and the "Trading cost" column that follows from it. Notice whether the results
change much as you go down the table.

**What it does not mean.** The best-looking row is not a recommendation. With sixteen attempts, one
will look best by luck alone.

### Panel 11: Held-out window

**What it shows.** A button, then a table comparing two stretches of the history.

**Plain meaning.** This is the app's version of an honest exam. It divides the history into a part it
is allowed to study ("Observation window") and a part it must not look at until the end ("Held-out
window"). It decides its verdict using only the first part, then applies it unchanged to the second.
The button reveals the results on the part it was not allowed to study.

Do not press the button until you have decided your settings. Once you look, you cannot un-look, and
the honesty of the exercise depends on that self-restraint.

**What to look for.** Whether the two halves tell a similar story. If they disagree sharply, the
verdict was probably luck.

**What it does not mean.** The held-out stretch is short. Short stretches of history can say almost
anything.

### Panel 12: Instrument validation

**What it shows.** A button, then a small grid of counts.

**Plain meaning.** This checks the thermometer against known temperatures. It generates pretend data
with a known answer, in all three patterns, several times each, and counts how often the app got it
right.

**What to look for.** The sentence underneath: "N of 12 classified correctly". Twelve of twelve is
what you should expect.

**What it does not mean.** Perfection here does not mean the app will be right about real markets. It
means the measuring instrument works on data where the answer is known.

---

## 7. The controls, one by one

You are not expected to change most of these. For each, the honest advice is included.

### Panel 1, Data controls

| Control                           | Plain meaning                                                         | Advice                                                               |
| --------------------------------- | --------------------------------------------------------------------- | -------------------------------------------------------------------- |
| Synthetic universe (known regime) | The app invents data with a known answer                              | Use it to learn the app, never to learn about markets                |
| CSV file                          | You supply a file of real historical prices                           | Use this for real questions. Ask a helper to prepare the file        |
| Total-return prices CSV           | The file chooser                                                      | The file needs a first column of dates, then one column per sector   |
| CSV contains                      | Whether your file holds prices or already-computed percentage changes | If you are unsure, it holds prices                                   |
| Ground-truth regime               | Which pattern the app should build into the pretend data              | This is the practice-exam answer key. Change it to see the app react |
| Seed                              | The starting point for the invented numbers                           | Any whole number. The same seed always gives the same pretend data   |
| Years                             | How much history to invent                                            | Leave at 8                                                           |
| Annual volatility                 | How jumpy the invented prices are                                     | 0.2 means 20 percent. Leave it                                       |
| Average correlation               | How closely the invented sectors move together                        | Leave at 0.6                                                         |
| Latent share of variance          | How much of the invented movement comes from the persistent part      | Leave at 0.3                                                         |
| Run analysis                      | Recalculates everything                                               | Press it after any change                                            |
| Export results JSON               | Saves a file of all the numbers                                       | Useful for sharing results without screenshots                       |

### Panel 2, Configuration controls

| Control                         | Plain meaning                                                                                                                | Advice                                                                           |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| Significance level              | How strict the "could be luck" test is. Default 0.05 means one in twenty                                                     | Leave it, unless you have a reason                                               |
| Bonferroni across primary tests | Makes the test stricter because three tests are being run at once                                                            | Leave it ticked. Without it, three attempts means three chances of a false alarm |
| Minimum observations            | How much history is needed before any verdict is allowed. Default 252, about one year of trading days                        | Leave it                                                                         |
| Rolling window (days)           | How much recent history is used for the smooth charts                                                                        | Leave it                                                                         |
| Rebalance check                 | How often the pot is inspected                                                                                               | Try the options and watch the grid                                               |
| Drift band (relative to target) | How far the pot may wander before being tidied. 0.05 means 5 percent away from its intended share                            | Small means more tidying and more tolls; large means less tidying                |
| Trading cost (bps per trade)    | The toll per trade. One **basis point** is one hundredth of one percent, so 5 bps is 0.05 percent                            | Replace with a real figure before believing any cost line                        |
| Expense ratio (bps per year)    | The yearly fund charge, currently a placeholder                                                                              | Replace with the real figures from the fund documents                            |
| Overlay                         | The optional winner-picking extra: `momentum` follows leaders, `contrarian` bets against them                                | Only works when the verdict allows it                                            |
| Overlay frequency               | How often the extra reconsiders its picks                                                                                    | Leave at monthly                                                                 |
| Overlay lookback (days)         | How much history the extra uses to judge who is winning                                                                      | Leave at 126, about six months                                                   |
| Overlay skip (days)             | How much recent history the extra deliberately ignores. Default 21, one month, avoids reacting to the last few days of noise | Leave it                                                                         |
| Overlay top N sectors           | How many sectors the extra picks                                                                                             | 3 is a sensible middle. 1 concentrates risk heavily                              |
| Held-out fraction               | How much history is reserved for the honest exam                                                                             | Leave at 0.3                                                                     |

**A word on basis points.** One basis point is 0.01 percent, written as 0.01%. The term comes from
finance and it is unavoidable here. So 5 bps is 0.05%, and 100 bps is 1%. A useful rough guide: a
round trip in and out of a fund that costs 5 bps each way costs 10 bps, which is 0.1 percent of the
money traded.

---

## 8. Worked examples

### Example 1: Is there any pattern at all?

Set **Ground-truth regime** to `memoryless` and press **Run analysis**.

Expect: **MEMORYLESS**. Panel 5 blocks both winner-picking rows. Panel 8 shows a small positive type A
rebalancing contribution and a small negative type B one.

What to take from it: the app found nothing to exploit and said so, while still reporting what the
tidying habit is worth. That is the ordinary, unexciting outcome, and it is the most common honest
result.

### Example 2: What if trends really do persist?

Set **Ground-truth regime** to `persistent`, **Overlay** to `momentum`, then press **Run analysis**.

Expect: **POSITIVE_DEPENDENCE**, panel 5 marks the momentum row "eligible", the equity chart gains a
"with overlay" line, and panel 8 gains an overlay contribution.

What to take from it: this is what a working signal looks like when it exists. Notice also that panel 8
still shows the trading cost, which the extra activity increases. A pattern can be real and still be
eaten by the tolls. Check the net line.

### Example 3: My history is short

Set **Ground-truth regime** to `uncertain (short sample)` and run.

Expect: **UNCERTAIN**, with the reason "140 observations is below the minimum of 252; no regime can be
claimed". Both winner-picking rows blocked, even if you left the overlay switched on.

What to take from it: the app prefers saying "I do not know" to guessing. If your own data gives
UNCERTAIN, the honest answer is that you need more history, not a different setting.

### Example 4: Does the tidying schedule matter?

Leave the verdict as it is and study panel 10, the grid. Change **Rebalance check** and **Drift band**
in panel 2 and re-run to see the grid change.

Expect: the "Rebalances" and "Turnover" columns move a great deal, while the results columns move much
less. Often, how often you check matters far less than how much activity you generate.

What to take from it: this is a general lesson that transfers to any investing habit. Activity costs
money, and the benefit of activity has to be bigger than its cost.

---

## 9. How to read the numbers without fooling yourself

These are the ways this app is most likely to mislead you. They are listed in the order they usually
catch people.

**Trap 1: believing the pretend data.** The app opens on invented numbers. They are useful for
learning the app and nothing else. If you want to know something about real markets, you must load real
data.

**Trap 2: one line on one chart.** A single run through a single stretch of history is not evidence.
Two approaches can look dramatically different purely because of which years were included.

**Trap 3: confusing "real" with "worth doing".** A pattern can be unmistakably real and still far too
small to cover the cost of acting on it. This is why the app has a **deadband**: a zone of effects too
small to care about. When the app says a number is "statistically detectable but sits inside the
deadband", it is applying exactly this caution. It is one of the most valuable sentences the app can
print, and it is easy to misread as a bug.

**Trap 4: the cost figures are placeholders.** The trading cost and the fund charges that come
pre-set are guesses. Until they are replaced with real figures, every "net of all costs" number is
optimistic.

**Trap 5: sixteen attempts.** The grid in panel 10 shows sixteen combinations. The best of sixteen
will look good by luck alone. Choosing it because it looks best is how people fool themselves most
often.

**Trap 6: peeking at the held-out window.** The whole value of the honest exam depends on deciding
your settings before you press the reveal button. If you peek and then adjust, the exam has become
practice, and practice always looks better than the real thing.

**Trap 7: a short exam is not an exam.** The held-out stretch is short. Take a disagreement between
the two windows seriously, but do not take agreement as proof.

**Trap 8: too many settings.** Every setting is a dial you could turn until the past looks good. That
is called fitting the past, and it tells you nothing about the future. Panel 2 is deliberately
presented as a list to freeze, not a dashboard to tune.

**Trap 9: banks do not run the experiment.** The app's own research notes found that in real markets,
these patterns are usually weak, and that popular cycle-based rotation rules have not survived an
honest test. A verdict of POSITIVE_DEPENDENCE in your data is a finding about your data, not a promise.

**Trap 10: this is not advice.** Nothing in this app knows your circumstances, your goals, your debts,
or your tax position. A measurement tool cannot take responsibility for a decision.

---

## 10. Cheat sheet

Keep this page and ignore the rest until you need it.

| If you want to...                    | Do this                                                                        |
| ------------------------------------ | ------------------------------------------------------------------------------ |
| Open the app                         | Double-click `index.html` in the `sector-regime-engine` folder                 |
| See it work                          | Leave everything alone and press **Run analysis**                              |
| Know whether there is a pattern      | Read panel 3, the big coloured word                                            |
| Know what the app permits            | Read panel 5, "eligible" or "blocked"                                          |
| Know where the result came from      | Read panel 8, especially the type A and type B lines                           |
| Use real data                        | Choose CSV in panel 1 and load your file                                       |
| Check the app is measuring correctly | Press **Run validation sweep** in panel 12                                     |
| Test yourself honestly               | Decide your settings, then press **Reveal held-out results** in panel 11, once |

The four verdicts, in one line each:

| Verdict             | Meaning                 | What it permits                            |
| ------------------- | ----------------------- | ------------------------------------------ |
| MEMORYLESS          | No pattern found        | Tidying the pot only. No winner-picking    |
| NEGATIVE_DEPENDENCE | Leaders tend to reverse | Tidying, plus an optional contrarian extra |
| POSITIVE_DEPENDENCE | Leaders tend to persist | Tidying, plus an optional momentum extra   |
| UNCERTAIN           | Not enough evidence     | Tidying only. No winner-picking            |

---

## 11. Glossary

Plain-language definitions, in the order you are likely to meet them in the app.

**Attribution.** Splitting a total result into the named parts that caused it, so you can see where
the money came from instead of just how much there is.

**Basis point (bps).** One hundredth of one percent, so 0.01%. Fifty basis points is half of one
percent.

**Basket.** An equal split of money across all the sectors, with nothing clever done to it.

**Confidence.** How sure the app is about its own verdict. It is about the past data, not the future.

**Contrarian.** Betting against whatever has recently been doing well, on the theory that it will
reverse.

**Correlation.** A number between -1 and 1 describing whether two things move together. Near 1 means
they move almost identically. Near 0 means they are unrelated. Negative means they move oppositely.

**Deadband.** A zone of effects considered too small to matter. Anything inside it is not treated as a
signal, no matter how clearly it is visible.

**Diagnostic.** The lowest tier in the app's evidence hierarchy. A measurement worth displaying but not
worth deciding on.

**Dispersion.** How spread out the sector results are from each other in a given period. High
dispersion means a lot is happening between sectors.

**Drift.** The natural process by which the shares in a pot change over time, because some parts grow
faster than others. Tidying reverses drift.

**Eligibility.** Whether the app permits a given approach, given the verdict. Blocked approaches cannot
be switched on.

**Equity curve.** A line on a chart showing how a pot of money would have changed in value over time.

**Expense ratio.** The yearly charge a fund deducts for managing your money. Paid continuously, whether
or not anyone trades.

**Fingerprint.** A short code identifying one exact set of settings, so a result can be traced back to
the choices that produced it.

**Frozen configuration.** The set of settings you decide before looking at results, and then leave
alone. Freezing is what makes a test honest.

**Gross.** Before costs. The opposite of net.

**Held-out window.** A stretch of history deliberately not examined while deciding, used only at the
end as an honest test.

**Holding cost.** The continuous cost of owning funds, chiefly the expense ratio. Distinct from the
cost of trading.

**Momentum.** The tendency of whatever has been doing well to keep doing well, for a while.

**Net.** After all costs have been deducted.

**Observations.** Individual pieces of data, usually one per trading day. More observations mean a more
trustworthy measurement.

**Overlay.** An optional extra rule applied on top of the basic pot, such as picking the three
strongest sectors.

**p-value.** The chance of seeing a result this extreme purely by luck. Small means unlikely to be
luck. The app's default threshold is 0.05, one in twenty.

**Primary.** The highest tier in the evidence hierarchy. Only these measurements may decide the
verdict.

**Rebalancing.** Tidying the pot back to its intended shares by selling a little of what has grown and
buying a little of what has shrunk.

**Regime.** Which of the four worlds the data is in. The app's central concept.

**Residual.** The small leftover when a set of figures is supposed to add up exactly. A tiny residual
is proof the arithmetic is sound.

**Rotation.** Money moving from one sector to another over time.

**Secondary.** The middle tier of the evidence hierarchy. Informative, but not permitted to decide.

**Sector.** A group of similar companies. Banks, oil and gas, health care, technology and so on. This
app uses eleven.

**Seed.** A starting number that makes invented data repeatable. The same seed always produces the same
pretend data.

**Significance.** A statistical verdict that a result is unlikely to be pure luck. It says nothing about
whether the effect is large enough to be worth acting on.

**Slippage.** The gap between the price you saw and the price you actually got.

**Synthetic.** Invented by the app for practice, with a known answer built in.

**Trading cost.** The toll paid each time the app buys or sells, covering the gap between buying and
selling prices, and any fees.

**Turnover.** How much trading happened, usually expressed as a proportion of the pot.

**Volatility.** How jumpy a price is. Higher volatility means bigger and more frequent swings. Unlike
in everyday use, the app does not treat volatility as simply bad; it is what makes tidying worth
anything at all.

---

## 12. Troubleshooting

| What you see                                                            | What it means                                                                       | What to do                                                                                                                                                   |
| ----------------------------------------------------------------------- | ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| A message that the bundle `app.js` is not present                       | The engine file has not been built in this folder                                   | Ask a helper to run `bun run build` in the folder, then reload                                                                                               |
| A completely blank page                                                 | The page may be stale, or the browser blocked a local file                          | Press `Ctrl` and `F5` together. If it persists, ask a helper to run `bun run serve` and use the address it prints                                            |
| I pressed Run analysis and nothing seemed to happen                     | If no setting changed, the app recomputes an identical result, so no number moves   | Look at the small line beside the button. It reports the time of the run and says whether any settings changed. If it names settings, the results did change |
| The line beside the button says "Settings changed - press Run analysis" | You have altered a setting since the last run, so what is on screen is out of date  | Press Run analysis, or put the setting back                                                                                                                  |
| Nothing happens when I press Run analysis                               | A setting is outside its allowed range                                              | Hover the field; the browser shows the allowed range. Restore the number from section 7                                                                      |
| Every verdict is UNCERTAIN                                              | There is not enough history in the data                                             | That is the correct answer. Get a longer history rather than changing settings                                                                               |
| Every verdict is MEMORYLESS                                             | No pattern was found                                                                | That is also a correct answer, and the most common one                                                                                                       |
| The verdict changes when I change a setting                             | Settings affect the measurement                                                     | Decide settings first, record the fingerprint, and compare only like with like                                                                               |
| Numbers look far too good                                               | You may still be on the pretend data, or the cost figures may still be placeholders | Check panel 1 for "Synthetic", and replace the cost and expense figures in panel 2                                                                           |
| The validation sweep takes a while                                      | It runs twelve full analyses                                                        | Wait for it. It finishes in a few seconds on a normal computer                                                                                               |
| I loaded a CSV and nothing appears                                      | The file's shape is wrong                                                           | The file needs a first column of dates and then one column per sector, with the sector names in the top row                                                  |
| The page cannot save my results                                         | Browsers restrict saving in some situations                                         | Use **Export results JSON**; if that fails, take a screenshot instead                                                                                        |

---

## 13. Where to go next

- **This folder's `README.md`** is the technical description, aimed at someone who writes software.
- **`strategies/sector_rotation_strategies.md`**, two folders up from here, is the research behind the
  app. It is written for a professional reader, but its section 13 states the central conclusion in a
  few sentences and is worth reading even without a financial background.
- **`strategies/entry_exit_engine_design.md`** sketches where the app would go next if it ever grew an
  execution and risk layer. It is a plan, not a description of anything that exists yet.

The most useful thing to carry away from this manual: the app's most common answer is that there is
nothing to find. Learning to accept that answer, and to distrust the tools that always seem to find
something, is the actual skill this app is built to teach.
