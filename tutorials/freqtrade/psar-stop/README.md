# A stop that trails the price and flips to the other side: the parabolic stop and reverse

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                           |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | A cryptocurrency pair on the spot market, such as Bitcoin priced in United States dollars                                                                                                                                       |
| How often it trades       | On one-hour candles; the placeholder buy rule fires whenever the stop has moved up, which in a trend is often                                                                                                                   |
| What you need             | Nothing but this page and a pencil; the arithmetic is three multiplications per candle                                                                                                                                          |
| Where the rules come from | [CustomStoplossWithPSAR.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/CustomStoplossWithPSAR.py)                                                                                         |
| The underlying research   | J. Welles Wilder Jr., New Concepts in Technical Trading Systems (1978), which introduced the parabolic stop and reverse; the method is described at [Investopedia](https://www.investopedia.com/terms/p/parabolicindicator.asp) |
| How well it held up       | Weak: the idea is old and widely reproduced, but the file is an untested example whose own docstring calls the buy rule nonsensical and whose stop arithmetic does not do what its name says, as the worked example below shows |
| Also appears in           | [Risk boundaries and stops](../../project/risk-boundaries-and-stops/README.md), which describes trailing stops of the same family                                                                                               |

## The idea in one paragraph

A stop is a price at which a position is closed to limit a loss. A trailing stop is a stop that moves
as the price moves: it follows the price up, so a position that has risen keeps more of its gain, and
it never moves back down. This strategy's stop is a special trailing stop called the parabolic stop
and reverse, or PSAR. It sits below the price while the price is rising, creeping closer and closer to
it as the rise continues, and when the price finally drops through it the stop flips to above the
price. The same line, flipped, then serves a position betting on a fall. The faster and longer the
rise, the faster the stop catches up, which is the behaviour that gives it its name.

## Why anyone believed it

A trailing stop answers a real question: how much of a rise should be given back before admitting the
rise has ended? A fixed stop gives back a fixed amount whether the rise was large or small. A trailing
stop gives back a percentage of the best price reached, so a trade that has climbed far can afford to
give back more, and a trade that has barely moved is closed quickly. The PSAR adds a second idea: the
longer a move has run without reversing, the more likely it is to be a real move, so the stop should
hurry towards the price as the move extends. The "acceleration factor" in its formula does exactly
that, and it is why the line is a curve rather than a straight trail.

The counterparty is the trader who keeps holding a position that has already turned, waiting for the
price to come back. If reversals tend to continue once they start, the person who leaves on the flip
keeps a gain the person who stayed gives up. When reversals do not continue, the flip exits a position
that then recovers, and the stop has sold at the worst moment.

## An everyday comparison

Think of a boat anchored in a rising tide. The anchor line is paid out so that the boat can float
wherever the water takes it, but the line is also shortened whenever the tide rises, so the boat
cannot drift far from where it last was. To begin with there is plenty of slack; as the water keeps
rising, the slack is taken in faster and faster until the line is nearly taut. The moment the tide
turns, the taut line stops the boat from drifting back the way it came. That is the shape of a
parabolic stop: slack while a move starts, tightening as it runs, and a hard stop at the turn.

## The rules, step by step

1. Use one-hour candles of a crypto pair.
2. On every candle, compute the parabolic stop and reverse value from the recent highs and lows, using
   the recipe in the next section. The starting acceleration factor is 0.02 and it may rise to no more
   than 0.20.
3. When the stop is below the price, the market is counted as rising; when it is above the price, the
   market is counted as falling. The stop changes side only when price closes through it.
4. The buy signal in this file is a placeholder and is not a strategy: it buys whenever the stop value
   is lower than it was on the previous candle, that is, whenever the rising stop has just moved up.
5. The sell signal is switched off entirely. Nothing in the file ever sells because of a signal.
6. Instead, a trade is closed by the moving stop. The strategy asks the bot for the pair's data, reads
   the latest stop value, and turns it into a stop level as described in the next section.
7. The ordinary stop set in the file is 20 percent. It applies until the moving stop is first
   calculated and as a lower bound afterwards.
8. A trade is also closed by a profit ladder, if the file's ladder is reached first; the file as
   written has no ladder set, so in this file the stop is the only exit.

One honest point before the arithmetic. The file is written as an example, not as a strategy. Its own
comment says the buy rule is nonsensical and that the file is meant to be copied into a real strategy.
The stop conversion, as the next section shows, is also wrong in a way that matters.

## The maths, with every symbol named

The parabolic stop and reverse is computed from the highest high reached during a rise, or the lowest
low during a fall, and an acceleration factor.

While the market is rising:

```text
SAR_t = SAR_{t-1} + AF_{t-1} * (EP_{t-1} - SAR_{t-1})
```

While the market is falling:

```text
SAR_t = SAR_{t-1} - AF_{t-1} * (SAR_{t-1} - EP_{t-1})
```

- `SAR_t` is the stop value at candle `t`, the price at which the trend would be called broken.
- `EP_{t-1}` is the extreme point so far: the highest high seen during the current rise, or the lowest
  low seen during the current fall.
- `AF_{t-1}` is the acceleration factor. It starts at 0.02, and each time the extreme point sets a new
  high or low it rises by 0.02, up to a maximum of 0.20. It resets to 0.02 whenever the stop flips.
- The factor in front of the bracket means the stop moves a fraction of the distance between itself
  and the extreme point, so it never jumps on top of the price and it speeds up as the factor grows.

The stop flips sides when the price crosses it: in a rise, if the low of a candle goes below the stop,
the trend is called down, and the new stop starts at the previous extreme point. In a fall, if the high
goes above the stop, the trend is called up, and the new stop starts at the previous extreme point.

Now the conversion the file performs, and the problem with it. The bot expects a stop expressed as a
fraction of the current price, so that a return of 0.02 means a stop 2 percent below the price. To put
the stop at the stop value, the fraction is:

```text
correct_fraction = (current_rate - sar) / current_rate
```

- `current_rate` is the price now.
- `sar` is the parabolic stop value, an absolute price.

The file instead computes:

```text
file_fraction = (current_rate - sar) / current_rate - 1
```

The subtraction of one is the defect. The bot ignores the sign of the returned number and uses its
size, so the file's value has size `1 - correct_fraction`, not `correct_fraction`. When the stop sits
10 percent below the price, the correct fraction is 0.10 and the stop is placed 10 percent below; the
file returns a number of size 0.90 and the stop is placed 90 percent below, which is to say almost
nowhere. The correct conversion is what the freqtrade documentation calls `stoploss_from_absolute`, and
a strategy that follows this file's intent should use it. The worked example shows both numbers.

## A worked example

The table computes the stop on a rising market with a starting stop of 9.80 and an extreme point of
10.50, using the recipe above. Each row adds the movement of one candle, prices in dollars.

| Candle | High  | Low   | New extreme | AF   | Stop this candle | Arithmetic                                      |
| ------ | ----- | ----- | ----------- | ---- | ---------------- | ----------------------------------------------- |
| A      | 10.60 | 10.30 | 10.60       | 0.02 | 9.8140           | 9.8000 + 0.02 * (10.5000 - 9.8000)              |
| B      | 10.80 | 10.45 | 10.80       | 0.04 | 9.8454           | 9.8140 + 0.04 * (10.6000 - 9.8140)              |
| C      | 11.00 | 10.60 | 11.00       | 0.06 | 9.9027           | 9.8454 + 0.06 * (10.8000 - 9.8454)              |
| D      | 11.10 | 10.80 | 11.10       | 0.08 | 9.9905           | 9.9027 + 0.08 * (11.0000 - 9.9027)              |
| E      | 11.20 | 10.90 | 11.20       | 0.10 | 10.1015          | 9.9905 + 0.10 * (11.1000 - 9.9905)              |
| F      | 11.05 | 10.60 | 11.20       | 0.12 | 10.2333          | 10.1015 + 0.12 * (11.2000 - 10.1015)            |
| G      | 10.70 | 10.10 | 11.20       | 0.12 | 10.2333          | no new high; the low of 10.10 is below the stop |

Read the table. The stop starts 0.69 behind the extreme point of 10.50 and finishes 0.97 behind it,
because the acceleration factor grows as each candle sets a new high, so the stop closes the gap. At
candle G the low of 10.10 falls below the stop of 10.2333, which is the flip: the trend is called down
and the stop jumps to above the price at the previous extreme point of 11.20.

Now the conversion, at candle D, with the stop at 9.9905 and a price of 11.10:

```text
correct_fraction = (11.10 - 9.9905) / 11.10 = 1.1095 / 11.10 = 0.09995, about 0.1000
correct stop price = 11.10 * (1 - 0.1000) = 9.9905, which is the stop value, as intended

file_fraction = 0.09995 - 1 = -0.90005
stop price the file produces = 11.10 * (1 - 0.90005) = 1.1109
```

So the file, as written, sets the stop a little above one dollar when the price is eleven, which is
just over 90 percent below the price. A position would have to lose almost everything before the stop
acted. The number is consistent and reproducible, and it is not the trailing stop the name promises.
Anyone copying this file should replace the conversion with the documented helper.

## What the research actually found

The parabolic stop and reverse was published in Wilder's 1978 book, which is the origin cited by every
later description. It is one of the oldest technical indicators still in use, and it is embedded in
most charting software. That is evidence that it circulated, not evidence that it earned a return.

There is no published out-of-sample measurement attached to this file, and the repository publishes no
result for it. The Investopedia description of the method lists the failure modes rather than a
performance figure: the stop is always on and always generating signals whether or not a real trend
exists; a reversal is generated eventually even when the price has not reversed, because the
acceleration factor drags the stop into the price; and in a market that drifts sideways the stop
reverses repeatedly and produces many small losses. Those are properties of the rule, stated by the
source that documents it.

What the general trend-following literature measures is described in the sibling tutorial on the
[supertrend line](../supertrend/README.md); it applies to trend following at the level of a portfolio
of markets and does not test this stop on one crypto pair.

## How this project relates to it

This repository's own treatment of stops is
[Risk boundaries and stops](../../project/risk-boundaries-and-stops/README.md), the tutorial that
places a trailing stop, a chandelier stop that follows the highest price, and a fixed stop side by
side in a single risk layer. It makes the point this file illustrates by accident: a stop does not
change the average outcome of a fair bet, it changes the shape, cutting both the large losses and the
large gains. Read it after this page to see the family this stop belongs to.

[Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md) is the document
behind that tutorial. Its Section 7 lists volatility-scaled and trailing stops as admissible risk
boundaries but requires the whole layer to stay switched off by default until it proves itself on
held-out data, which is the rule that a copied example file cannot satisfy on its own.

## Where it goes wrong

- The conversion is wrong in this file. As shown in the worked example, the stop is set far below the
  price rather than at the stop value, so the file does not behave as its name suggests.
- The buy rule is a placeholder. The file's own comment calls it nonsensical; it fires whenever the
  stop has moved up, which in a long trend is almost every candle, and it has no reason behind it.
- The stop is always on. Even in a market with no trend, the formula keeps moving and eventually
  flips, generating a trade where there was no signal. That is a property of the method, not a bug.
- It gives back part of every gain. The stop trails below the price, so a position that turns is
  closed after the price has fallen back to the stop, which is never at the top.
- Acceleration cuts both ways. The factor that tightens the stop during a long move also makes it
  reach the price, so a long, slow grind upward eventually forces an exit through sheer proximity.
- Costs are unmodelled in the example. Buying on every stop increment and selling on every flip pays
  the fee and the gap between prices many times over, and none of that appears in the file.

## Try it yourself

You need nothing but a spreadsheet and a public source of hourly prices.

1. Put five candles in a sheet with columns for the high and the low, and add a column for the stop.
2. Fill the first stop with the lowest low of those five candles and set the starting acceleration
   factor to 0.02.
3. For each next row, add the factor multiplied by the distance from the stop to the highest high so
   far, and raise the factor by 0.02 each time a new high is set, up to a ceiling of 0.20.
4. Add a column that flags the flip: it records a break when the candle's low is below the current
   stop, at which point the stop jumps to above the price at the highest high so far.
5. Add a last column for the stop price as a fraction of the close: take the close minus the stop,
   divided by the close. Compare it with the number the file would return, which is that fraction
   minus one, and note how far apart the two stop prices are.

What to notice: the stop climbs faster and faster the longer the run continues, so a trade that has
run a long time is closed on a very small pullback. Change the ceiling from 0.20 to 0.05 and repeat:
the stop stays further away, flips later, and gives back more when it does. That dial is the same
trade-off as the supertrend's multiplier, and it has no answer that is right for every market.

## Where this came from

- [CustomStoplossWithPSAR.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/CustomStoplossWithPSAR.py),
  the file whose rules, placeholder signal and stop arithmetic are quoted above.
- J. Welles Wilder Jr., New Concepts in Technical Trading Systems (1978), the book that introduced the
  method, and the [Investopedia description](https://www.investopedia.com/terms/p/parabolicindicator.asp)
  of its formula and its limitations.
- The freqtrade documentation on
  [custom stoploss](https://www.freqtrade.io/en/stable/strategy-callbacks/), which states that the
  return value is a fraction of the current price and that the sign is ignored, and offers the
  conversion helper this file should use.
- [Risk boundaries and stops](../../project/risk-boundaries-and-stops/README.md) and
  [Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md), this
  repository's own account of trailing stops and why the whole layer defaults to off.

## Words used in this tutorial

- acceleration factor: the number that sets how fast a parabolic stop closes on the price, from 0.02 rising to 0.20.
- extreme point: the highest high in a rise or the lowest low in a fall, the reference the stop closes towards.
- long: owning something, so that a rise is a gain.
- short: selling something you do not own, so that a fall is a gain.
- stop and reverse: a system whose stop, once hit, becomes the signal to trade the other way.
- trailing stop: a stop that moves with the price and never loosens.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
