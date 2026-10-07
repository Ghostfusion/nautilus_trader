# How to read a claim

Date: 2026-10-07. Revision 1.

Anyone who reads about trading will meet confident claims about strategies that "beat the market".
This page gives eight questions to ask of any such claim, then applies them to one plausible
example and reaches an honest verdict.

## The eight questions

1. What exactly is being measured? A total return is not the same as a price change, and "beat the
   market" is not the same as "made money". Ask for the quantity in words before you accept a
   number.
2. On which assets, and over which period? A rule tested on twenty large companies in the 1990s is
   a different claim from the same rule tested on thousands of shares across six decades. The
   answer fixes what the claim can possibly be about.
3. Which costs are included, and which are not? Every trade pays a spread and a commission, and
   the price you receive is not the price printed on the screen. A return quoted before costs is
   not the return a person could have earned.
4. Was it checked on data the rules never saw? If the rules were chosen by looking at the whole
   history, the history cannot also be the test. The clean test is a stretch of prices that was
   set aside in advance.
5. How many variations were tried before this one was reported? If the author tested forty
   versions and showed the best, the reported result is the best of forty, and some of that
   strength is luck dressed up as skill.
6. Who is on the other side of the trade, and why? Every purchase has a seller. A real edge needs a
   counterparty with a reason to take the losing side, such as a forced seller or a slow reader of
   news. If no such party can be named, the story is incomplete.
7. What would have to happen for the claim to be false? A claim that cannot fail is not evidence.
   Ask which measurement, if it came out the other way, would overturn the conclusion.
8. What is left after taxes and borrowing costs? Profits are taxed, and buying with borrowed money
   costs interest. A gross return can shrink a great deal at this step.

## The claim we will test

Here is a plausible claim, of the kind seen in a newsletter. "Over the thirty-one years from 1990
to 2020, a rule that each month bought the twenty shares in the S&P 500 index with the strongest
return over the previous twelve months, and held them for one month before repeating, earned 11
percent a year. The index itself earned 9 percent a year over the same period. The rule therefore
beat the market by 2 percentage points a year."

The S&P 500 is an index, a single number that tracks the value of 500 large American companies. We
will not check whether those returns are real; we will assume the author reported them honestly
and ask instead whether the claim supports the conclusion.

## Applying the checklist

- Measured quantity: the claim says "earned 11 percent a year" but never says whether dividends
  are included. If the 11 percent is a price change and the index figure includes dividends, the
  two numbers are not comparable. The claim fails its first test on ambiguity.
- Assets and period: twenty shares chosen from 500, monthly, from 1990 to 2020. That is a real
  thirty-one-year window, which is a point in the claim's favour.
- Costs: none are mentioned. Monthly rebalancing replaces part of the portfolio every month, so
  turnover is high. Suppose the portfolio trades about three times its value a year, round trip,
  and each one-way trade costs 0.25 percent; that is about 0.75 percentage points a year of cost.
- Out-of-sample data: there is none. The twenty-share rule and the twelve-month look-back were
  chosen after seeing the whole period, so the period is both the training data and the test.
- Variations tried: not stated. Assume only that the author tried the obvious forty combinations
  of look-back length and holding size before reporting this one.
- Counterparty: the story behind such a rule is that prices react to news slowly, so the buyer of
  last year's winners is paid by slower investors. No such party is named, and no forced seller is
  identified, so the reason the edge should exist is asserted rather than shown.
- Falsification: the claim would be false if the 2-point edge were smaller than the noise in the
  measurement, or smaller than the trading costs. Both are checkable, and we check them below.
- Taxes and borrowing: the claim says nothing about them. Dividends, if held, are taxable, and any
  borrowing to enlarge the position adds interest.

Now the arithmetic. The claimed edge is 2 percentage points a year, before costs. Subtract the
estimated 0.75 points of costs, and about 1.25 points remain. Then ask whether 1.25 points is more
than noise. If the portfolio's annual wobble is about 16 percent, then over 31 years the standard
error of its average return is 16 divided by the square root of 31, which is about 2.87 points. The
margin of 1.25 points is well inside one standard error, so it is not distinguishable from zero.

| Step           | Arithmetic                          | Result             |
| -------------- | ----------------------------------- | ------------------ |
| Gross edge     | 11.0 minus 9.0                      | 2.0 points a year  |
| Trading costs  | 3.0 turns times 0.25 percent        | 0.75 points a year |
| Net edge       | 2.0 minus 0.75                      | 1.25 points a year |
| Standard error | 16 divided by the square root of 31 | 2.87 points        |
| Score          | 1.25 divided by 2.87                | 0.44               |

## The verdict

The claim does not survive the checklist. The measured quantity is ambiguous, the costs are
missing, the period was used both to choose the rules and to test them, and the number of
variations is unknown. After a modest cost estimate the claimed edge is 1.25 percentage points a
year, and the noise in a thirty-one-year average is about 2.87 points, so the edge is roughly one
third of its own error bar. The honest conclusion is that this rule cannot be distinguished from
luck on the evidence given, and the 2 points of "beating the market" are more likely to be
measurement noise than skill.

None of this proves the rule loses money. It proves the claim, as stated, does not support its
conclusion. That is the normal outcome of reading a claim carefully, and it is why the questions
come before the excitement. The authoring rules behind this judgement are in
[TUTORIAL_TEMPLATE.md](../TUTORIAL_TEMPLATE.md), and the research brief on overfitting explains the
same failures at book length
([that brief](../../strategies/books2/28_overfitting_and_research_integrity.md)).

## Words used in this tutorial

- Claim: a statement that a trading rule achieved a particular result.
- Costs: the money paid to trade, including the spread between buying and selling prices and any
  commission.
- Turnover: how much of a portfolio is bought and sold over a period, expressed as a multiple of
  its value.
- Out-of-sample: data kept aside and not used when the rules were chosen.
- Standard error: the size of the wobble in an average of many years; a margin smaller than it
  cannot be told apart from chance.
- Counterparty: the person on the other side of your trade, who buys what you sell.
- Index: a single number that tracks the combined value of many shares, such as the S&P 500.

See [GLOSSARY.md](../GLOSSARY.md) for the rest of the vocabulary used in this collection.

## Where this came from

- [TUTORIAL_TEMPLATE.md](../TUTORIAL_TEMPLATE.md), which states the rules every page in this
  collection follows and defines the grades used to judge a strategy.
- [The overfitting and research integrity brief](../../strategies/books2/28_overfitting_and_research_integrity.md),
  on multiple testing, trial counts and why a forecast must be scored on data it never saw.
- [What the evidence says](09_what-the-evidence-says.md), the previous foundations page, for the
  size of these problems across the published record.
- [GLOSSARY.md](../GLOSSARY.md), the shared list of terms.
