# Regimes, and why nothing lasts

Date: 2026-10-07. Revision 1.

A regime is a stretch of time in which a market behaves in one recognisable way, followed by a
switch to a different way. This page explains what a regime is, gives three examples a newcomer can
see, and shows why a strategy tuned to one regime tends to stop working in the next.

## What a regime is

A market does not behave the same way every year. Sometimes prices move gently and drift upward
for a long stretch; sometimes they jump around violently and fall. Sometimes borrowing money is
cheap and almost everything rises together; sometimes borrowing is expensive and different assets
go their separate ways. A regime is a period during which one short description keeps being
roughly true, followed by a switch to a different description.

Regimes are not announced. Nobody rings a bell. Looking backward you can point to the year the
character of the market changed; looking forward you cannot know that it is happening. The
research brief for this topic puts the technical version plainly: change-point methods assume the
whole series is already observed, so they locate a change in the past rather than signal one in
the present ([the regimes brief](../../strategies/books2/24_regimes_and_change_points.md)).

A useful test is to describe a year in one sentence and see whether the sentence still fits the
next year. "Prices rose gently and the wobble was small" fits some years and fails in others. When
the sentence stops fitting, the regime has changed, even though no label was ever posted.

## Three regime changes a newcomer can see

- Calm years and crisis years. The size of the typical daily move, which traders call volatility,
  is small in calm years and large in crisis years. The bubbles and crashes brief describes how a
  single episode can reset the whole market ([that brief](../../strategies/books2/05_bubbles_crashes_and_criticality.md)).
- High interest rates and near-zero rates. The interest rate is the price of borrowing money. When
  it is near zero, saving pays nothing and borrowing is cheap, which changes what investors are
  willing to hold. When it is high, the same choices look very different.
- Moving together and moving apart. Correlation is the tendency of two prices to move in the same
  direction. In some periods almost every share falls together and picking carefully does not
  help; in others individual company stories dominate and prices separate.

## Why a strategy tuned to one regime stops working

A trading rule is a set of instructions chosen because it worked under particular conditions. If
the instructions were selected during a calm, rising market, they were sized for calm, rising
markets. When volatility doubles, the same instructions take the same bets, but the typical loss
on each bet doubles with it, so a rule that looked safe can become dangerous without changing a
single line.

The regimes brief gives a concrete example of a relationship that flips with the regime. Across
724 monthly American market observations from 1953 to 2013, the link between risk and reward was
positive and significant only in the calm, low-volatility state, and it was absent or negative in
the high-volatility state
([the regimes brief](../../strategies/books2/24_regimes_and_change_points.md)). A rule that
learned the pooled behaviour of the whole sample learned an average that is close to true in
neither state.

## Crowding: the same edge shrinks as more money uses it

An edge is a small, repeatable advantage. Crowding is what happens when many people discover the
same edge and trade it. Each of them buys a little earlier, so the favourable move is partly
already in the price by the time the last trader arrives, and the advantage left over is smaller.
The edge does not have to disappear completely; it only has to become smaller than the cost of
trading.

Capacity is the related idea: the largest amount of money a strategy can hold before its own
buying and selling start to move the price against it. A rule that works beautifully with a
thousand dollars can fail with a hundred million, because the act of buying is itself the thing
that destroys the opportunity.

Costs make both effects concrete. The regimes brief reports that, in one study of crude oil
futures, the estimated round-trip cost, meaning the total cost of buying and later selling once,
was 53.71 basis points on the Shanghai contract against 5.80 basis points on Brent
([the regimes brief](../../strategies/books2/24_regimes_and_change_points.md)). A basis point is
one hundredth of one percentage point. The same idea can therefore be roughly ten times as
expensive in one venue as in another, which decides whether a thin edge survives at all.

## What publication does to a strategy

When a strategy is published it stops being private. Anyone can read the rules, and each new
reader who adopts them adds to the crowd. The natural expectation is that a published edge decays
quickly, and some researchers argue exactly that. The largest public replication of the published
record, reported on [the next page](09_what-the-evidence-says.md), looked for that decay across
2,838 papers and could not measure it once the general market period was controlled for. The
honest summary is that publication is a reason to expect crowding, not a proven cause of decay,
and the measurement itself has limits.

## Why "it worked for thirty years" describes the past

A long record sounds reassuring, and it is worth something: it shows the rule survived several
different years. But thirty years may contain only one or two regimes, and a rule chosen after
looking at all thirty years is a rule fitted to those particular thirty years. If the market's
character was calm for most of them, the record describes calm markets.

This is why a past record is a description of the past and not a forecast. The next thirty years
are not a rerun of the last thirty; they are a new sample, and the whole question is whether the
reason the rule worked is still present. The evidence page reports the size of that problem across
the published literature.

## Words used in this tutorial

- Regime: a stretch of time in which a market behaves in one recognisable way, before switching to
  a different way.
- Volatility: the size of the typical up-and-down move in a price; a measure of how unsettled the
  market is.
- Correlation: the tendency of two prices to move in the same direction at the same time.
- Edge: a small, repeatable advantage that a trading rule is built to capture.
- Capacity: the largest amount of money a strategy can hold before its own trading moves the price
  against it.
- Crowding: the shrinking of an edge as more people trade the same idea.
- Basis point: one hundredth of one percentage point, written as 0.01 percent.
- Out-of-sample: data that was not used when the rule was chosen, kept aside to test it honestly.

See [GLOSSARY.md](../GLOSSARY.md) for the rest of the vocabulary used in this collection.

## Where this came from

- [The regimes, breakpoints and state switching brief](../../strategies/books2/24_regimes_and_change_points.md),
  which reports the low-volatility-only risk-reward relationship, the retrospective nature of
  change-point detection, and the crude oil and Brent cost figures.
- [The bubbles, crashes and criticality brief](../../strategies/books2/05_bubbles_crashes_and_criticality.md),
  on how a single regime can end abruptly.
- [The overfitting and research integrity brief](../../strategies/books2/28_overfitting_and_research_integrity.md),
  for why a strategy selected after looking at the data is hard to trust.
- [The next foundations page](09_what-the-evidence-says.md), which reports the replication record.
- [GLOSSARY.md](../GLOSSARY.md), the shared list of terms.
