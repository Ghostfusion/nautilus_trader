# 01 - What is intraday systematic trading

This lecture is in plain language. It defines the style, explains where the money is supposed to
come from, names the risks, and finishes with three examples you can work with a pen.

## The style in one paragraph

Intraday systematic trading means you write down, in advance, a fixed rule that says when to buy
and when to sell, and you let a program follow that rule without argument. Intraday means the
positions are opened and closed within the same trading day or over a few hours, not held for
months. Systematic means the decision comes from arithmetic on recorded prices, not from a feeling
about the news. Medium frequency means you act on bars of one minute to one hour, not on every
individual quote.

## A rule, not a hunch

A hunch is a decision you cannot repeat. You cannot say exactly why you bought, you cannot check it
later, and you cannot test it on last year's prices. A rule fixes all three problems:

1. It is written down, so you and a program can apply it identically every time.
2. It can be replayed on recorded prices, so its past behaviour is measurable.
3. It can be changed one piece at a time, so you can see which change helped.

A rule also has a cost. A hunch can quietly ignore a losing month; a rule exposes it. If the rule
loses money over the test, you learn that before you risk real money.

## Where the profit is supposed to come from

A rule buys because it expects the price to be higher later, or sells short because it expects the
price to be lower later. Selling short means selling something you do not own yet, in the hope of
buying it back cheaper; in this repository a short is just a position with a negative signed
quantity (`docs/concepts/positions.md`). Profit, in the simplest case, is the difference between
the price at which you entered and the price at which you exited, multiplied by the quantity, minus
costs.

That is the whole idea. A moving-average rule makes no forecast about the news; it merely follows a
smoothed version of the recent price and assumes that recent direction persists for long enough to
pay the costs. Sometimes it does not.

## What a bar is

A bar summarises trading over one fixed interval, for example one minute. It has five numbers and
two timestamps, and they are defined in `docs/concepts/data/bar.md`:

| Field    | Plain meaning                                    |
| -------- | ------------------------------------------------ |
| open     | The first price of the interval                  |
| high     | The highest price during the interval            |
| low      | The lowest price during the interval             |
| close    | The last price of the interval                   |
| volume   | How much traded during the interval              |
| ts_event | When the bar happened, in nanoseconds since 1970 |
| ts_init  | When the bar became known to the system          |

The engine guarantees `high` is at least as large as `open`, `low` and `close`, and `low` is at
most `open` and `close` (`docs/concepts/data/bar.md`). A bar does not record the order in which the
high and the low happened, so it cannot tell you whether the price rose then fell or fell then
rose. That gap matters later, when a stop and a target both sit inside one bar; see
`docs/concepts/backtesting/bar-execution.md`.

## The vocabulary

Every term is used in this manual exactly as the engine uses it.

| Term            | Meaning                                                              |
| --------------- | -------------------------------------------------------------------- |
| Instrument      | The thing being traded, such as the currency pair USD/JPY            |
| Venue           | The marketplace, identified by a short name such as `SIM`            |
| Bid             | The price at which someone will buy from you                         |
| Ask             | The price at which someone will sell to you                          |
| Spread          | The ask minus the bid, the immediate cost of a round trip            |
| Tick            | One quote or trade update, with a nanosecond timestamp               |
| Quote tick      | A bid and an ask with sizes (`docs/concepts/data/quote_tick.md`)     |
| Bar             | An interval summary with open, high, low, close and volume           |
| Indicator       | A number computed from past bars, such as an average                 |
| Moving average  | The average of the last N values, recomputed each bar                |
| EMA             | An exponential moving average, which weights recent values more      |
| Signal          | The rule's current instruction, such as long, short or none          |
| Position        | The amount you currently hold, positive for long, negative for short |
| Flat            | Holding no position                                                  |
| Order           | An instruction sent to the venue to trade                            |
| Market order    | An order to trade now at whatever price is available                 |
| Stop order      | An order that becomes active only after a trigger price trades       |
| Fill            | One execution of an order, with a price and a quantity               |
| Commission      | The fee the venue charges on a fill                                  |
| Slippage        | The gap between the price you expected and the price you got         |
| Backtest        | A replay of a rule on recorded prices                                |
| PnL             | Profit and loss, the money made or lost                              |
| Exposure        | The size of your open positions                                      |
| Drawdown        | How far below a previous peak your account has fallen                |
| Look-ahead bias | Using information the rule could not have known at the time          |
| Overfitting     | Tuning a rule until it fits the past and nothing else                |

## Worked example 1: a moving average crossing by hand

A moving average of period N is the sum of the last N closing prices divided by N. A short average
reacts quickly to new prices; a long average moves slowly. The rule "buy when the short average
crosses above the long average" is the rule this manual builds.

Take five made-up closing prices: 10, 10, 10, 12, 14. Use a 2-period average as the fast one and a
3-period average as the slow one.

| Bar | Close | Fast, 2-period  | Slow, 3-period  | Fast above slow? |
| --- | ----- | --------------- | --------------- | ---------------- |
| 1   | 10    | not enough data | not enough data | no               |
| 2   | 10    | 10.0            | not enough data | no               |
| 3   | 10    | 10.0            | 10.0            | equal, so no     |
| 4   | 12    | 11.0            | 10.667          | yes              |
| 5   | 14    | 13.0            | 12.0            | yes              |

Read the arithmetic: at bar 4 the fast average is `(10 + 12) / 2 = 11.0`, and the slow average is
`(10 + 10 + 12) / 3 = 32 / 3 = 10.667`. At bar 3 they were equal at 10.0, so bar 4 is the first bar
at which the fast average is strictly above the slow one. The rule buys at bar 4 and stays long at
bar 5, because the fast average is still above the slow one. Nothing here involves a forecast; the
rule simply observes the two averages.

## Worked example 2: reading one real bar

The file in `sample_data/` contains this row (it is the second data row of the file):

```text
ts_event_ns,open,high,low,close,volume
1546383660000000000,109.506,109.565,109.506,109.565,60000000
```

Read it as follows. During the one minute ending at 1546383660000000000 nanoseconds after 1970, the
USD/JPY mid price opened at 109.506, reached a high of 109.565, fell to a low of 109.506, and closed
at 109.565. The volume field is 60000000, which is the accumulated quote size the engine recorded
for that minute. The bar rose over the minute: the open equals the low and the close equals the
high. If you wanted to trade that minute you would have had to act while it was forming; the bar
only tells you the summary after the fact.

## Worked example 3: one round trip, with costs

This example uses prices from a real run later in the manual (lecture 07). A rule bought 100,000
USD/JPY at 109.344 and was stopped out, that is sold, at 109.289.

1. The price moved against the position by `109.344 - 109.289 = 0.055` yen per unit.
2. The loss before costs is `0.055 * 100,000 = 5,500` yen.
3. Each of the two market orders paid a commission of 219 yen, so costs are `2 * 219 = 438` yen.
4. The total loss is `5,500 + 438 = 5,938` yen.

Note that the rule lost money on this trip even though the stop worked exactly as designed. A stop
limits the size of a loss; it does not prevent losses. Note also that a round trip always pays the
spread twice, once on entry and once on exit; with a spread of 0.010 on 100,000 units that is
`2 * 0.010 * 100,000 = 2,000` yen per round trip before commissions.

## The main risks

- Costs. Commissions and the spread are charged on every trip. A rule that looks profitable before
  costs can be unprofitable after them.
- Whipsaw. A price that oscillates around a slow average makes the rule buy near a local top and
  sell near a local bottom, repeatedly.
- Too few trades. Five or ten trades is not evidence. The result is dominated by luck.
- Bad data timing. If the rule reads a bar before that bar could have existed, the backtest is
  fantasy. Bar timestamps are covered in `docs/concepts/backtesting/bar-execution.md`.
- Session time. Markets open and close at local exchange times, and daylight saving shifts those
  times in UTC. The engine resolves sessions in exchange local time through a trading calendar
  (`docs/concepts/trading_calendars.md`).
- Overfitting. Changing the periods until the past looks good produces a rule that fails in the
  future.
- Execution reality. A backtest assumes your order fills; a real venue may not fill it, or may fill
  it worse.

Next: how this repository represents the style, before any code, in
[02-the-engine-view](02-the-engine-view.md).
