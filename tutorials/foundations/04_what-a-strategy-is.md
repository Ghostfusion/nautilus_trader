# What a strategy is

Date: 2026-10-07. Revision 1.

Before you can read a trading strategy you have to know what one is. This page names the four parts
every strategy contains, says what makes a rule precise enough that two people get the same answer
from it, and explains why a strategy with no reason on the other side of the trade is a wish rather
than an idea.

## The four ingredients every strategy has

Every strategy, from the simplest to the most complicated, answers four questions in advance. If any
one of the four is missing, what you have is not a strategy but a hunch, because you still have to
make a decision in the moment and your judgement, not the rules, decides the outcome.

1. **A signal.** Something you can look up in a price or a document that tells you whether now looks
   like a good moment. A signal is observable: it is a number or a yes-or-no fact, not a feeling.
2. **A size rule.** How much to buy or sell when the signal says to act. This is a quantity, such as
   a number of shares, a fixed amount of money, or a fixed fraction of what you have.
3. **An entry rule.** Exactly when the position (the thing you own or owe after a trade) is opened.
   The entry rule is the signal plus any further conditions you set, such as a minimum size or a
   time of day.
4. **An exit rule.** Exactly when the position is closed again. An exit can be triggered by a profit
   target, a loss limit, a time limit, or the signal turning off. Without an exit rule, a strategy
   has no way to end.

You can think of the four as a sentence: when I see this signal, I buy this much, and I hold it
until this happens. Any trade a machine or a person can make from that sentence is repeatable; a
trade made from a feeling is not.

## What makes a rule checkable

A rule is checkable when a stranger with a spreadsheet, who has never met you, computes exactly the
same answer that you do. The test is not whether the rule is clever. The test is whether it removes
the need for judgement.

Compare two entry rules. "Buy when the company looks strong" is not checkable, because two people
disagree about what "strong" means and about how many days count. "Buy when this quarter's reported
profit is higher than the same quarter a year earlier" is checkable, because the two numbers come
from the same filed report and the comparison has one answer.

The same care is needed for every part. "Sell when the price has risen enough" fails the test; "sell
when the price is 20 percent above the price I paid" passes it. "Buy a reasonable amount" fails;
"buy shares worth 5 percent of the money in the account" passes. A rule that a stranger cannot
reproduce cannot be tested, and an untestable strategy is indistinguishable from a lucky guess.

Checkable also means adding the practical details, because they change the answer. You have to say
what price you use (the closing price of the day, for example), how often you check (every day or
every month), and what happens if the signal appears on a day the market is shut.

## A strategy is not a forecast and not an opinion

Three things are easy to confuse.

A **forecast** is a statement about the future: "this company will earn more next year". A forecast
can be right or wrong, but it says nothing about what to do, how much to buy, or when to sell, so it
is not yet a strategy. A forecast is an input that a strategy may use inside its signal.

An **opinion** is a preference held by a person: "this is a good company", "the market is too
expensive". It cannot be checked because it has no fixed rule and no end date, and two people can
hold opposite opinions without either being shown wrong.

A **strategy** is a complete, checkable set of instructions for acting. It contains the forecast or
the signal, but it also commits to a size, an entry and an exit. The moment it is written down this
way, it can be tested on past prices, and the result is a number - for example, what it would have
earned over a stretch of years - rather than a matter of taste.

The practical reason this distinction matters: only a strategy can be shown to be unreliable. A
forecast or an opinion can always be excused by saying the future has not arrived yet. Writing the
forecast into a full strategy takes away that escape.

## Every trade has someone on the other side

When you buy, someone sold to you. When you sell, someone bought. That person had a reason to take
the other side, and if you cannot say what it might have been, you probably do not yet understand
why the price is what it is.

Their reasons are usually ordinary. Some are **insurance**: a company that will receive foreign
money next month trades now to remove the uncertainty of the exchange rate, and accepts whatever
price it gets. Some are a **forced sale**: an account is closed because it borrowed money and the
lender demanded repayment, and the sale goes through whether or not the owner likes the price. Some
are a **need for cash**: a household, a pension fund or a business may sell an investment to pay an
unrelated bill. Some simply have a **different time horizon**: a trader who must show a result by
Friday may sell to a buyer who will hold for ten years. The full-collection glossaries explain these
words; see [GLOSSARY.md](../GLOSSARY.md).

These are the reasons a price can move even when nothing about the business changed. A strategy that
only notices the pattern in the price, and not who is on the other side, is guessing about the
pattern's cause. The table below shows four everyday signals and the counterparty each one quietly
assumes.

| Example signal                        | What it assumes about the counterparty                     |
| ------------------------------------- | ---------------------------------------------------------- |
| Price crosses its recent average      | Sellers expect a reversal, or must sell for other reasons  |
| Company reports unexpected profit     | Sellers have not read the news, or must sell before acting |
| Market falls more than usual in a day | Sellers need cash quickly and accept a lower price         |
| Country pays a higher interest rate   | Buyers want the higher rate and accept currency risk       |

If you cannot name a plausible counterparty reason, the signal is more likely to be a coincidence of
the past than a repeatable opportunity. This is the honest test, and it is the one most published
ideas fail.

## One strategy or many

A single strategy takes one signal and turns it into trades. A **portfolio** of strategies runs
several at once and thinks about how they fit together, which is a different job. Two ideas can each
look modest alone and still be worth holding together, if their bad days tend to happen at different
times; running the same idea twice under two names diversifies nothing. Spreading money across
investments so that no single loss decides the outcome is called **diversification**, and the
estimator error in how weights are chosen is covered in the project's brief
[strategies/books2/10_portfolio_and_allocation.md](../../strategies/books2/10_portfolio_and_allocation.md).
For a beginner, the practical point is smaller: decide whether you are following one rule or several,
and do not let two rules secretly tell you to buy the same thing twice.

## The time horizon decides the rest

The **time horizon** is how long you expect to hold a position before the strategy closes it. This
choice is made first, because it decides almost everything else.

A horizon of minutes forces the signals to be price patterns, because there is little time for news
to be read, and it makes the trading fee a large share of every trade, so a strategy must overcome
that fee many times a day. A horizon of months lets the signal use company reports and interest
rates, and the fee is spread over a longer holding period, so it matters less. A horizon of years
makes almost every day-to-day price move irrelevant.

So the same words take on different meanings: "a big fall" is a different number for a day trader
and for a long-term holder, and "sell soon" means something different in each case. A rule written
for one horizon, then applied at another, will not behave as its author intended.

## Words used in this tutorial

- strategy: a complete, checkable set of instructions for what to trade, how much, when to enter and when to exit.
- signal: an observable price or fact that a strategy uses to decide whether to act.
- position: what you own or owe after a trade, such as a number of shares.
- counterparty: the person or firm on the other side of your trade, who bought what you sold or sold what you bought.
- entry rule: the exact condition that opens a position.
- exit rule: the exact condition that closes a position.
- portfolio: a collection of investments, or of strategies, held together.
- time horizon: how long a strategy expects to hold a position before closing it.

## Where this came from

- [TUTORIAL_TEMPLATE.md](../TUTORIAL_TEMPLATE.md), the authoring contract for this collection.
- [GLOSSARY.md](../GLOSSARY.md), for the shared vocabulary of the collection.
- [strategies/books2/10_portfolio_and_allocation.md](../../strategies/books2/10_portfolio_and_allocation.md),
  the brief that covers how portfolio weights are chosen and how fragile those choices are.
