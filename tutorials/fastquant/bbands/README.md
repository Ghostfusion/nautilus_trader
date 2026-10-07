# Bollinger bands: buying the price when it stretches below its own average

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                         |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | One company's shares, or one fund that tracks an index, bought and sold whole                                                                                                                                                 |
| How often it trades       | A few times a year; the rule waits for the price to stretch unusually far from its recent average                                                                                                                             |
| What you need             | A spreadsheet                                                                                                                                                                                                                 |
| Where the rules come from | The strategy table in the [fastquant](https://github.com/enzoampil/fastquant) README (alias `bbands`) and its [source file](https://github.com/enzoampil/fastquant/blob/master/python/fastquant/strategies/bollinger_band.py) |
| The underlying research   | John Bollinger, who developed the bands in the 1980s, described in the [canonical entry](https://en.wikipedia.org/wiki/Bollinger_Bands)                                                                                       |
| How well it held up       | Disputed: one study finds the lower band a good place to buy, another finds the bands no better than a simpler envelope, and a third finds them profitable only during one crash                                              |
| Also appears in           | Nothing else in this collection describes the bands, though the lower band is the mirror image of the relative strength index in [rsi](../rsi/README.md)                                                                      |

## The idea in one paragraph

Draw a plain average of the last twenty closing prices, then draw two more lines above and below it,
each one the width of the recent wobble of the price away from that average. The lines form a channel
that widens when the price is jumpy and narrows when it is calm. This strategy buys when the closing
price falls below the bottom line of the channel, on the theory that a price which has moved further
below its own recent average than is usual is likely to come back, and it sells when the price rises
above the top line.

## Why anyone believed it

The bands are a ruler for stretching, and the economic story is over-reaction. When a piece of bad
news arrives, some holders sell more than the news warrants: they sell because they are frightened,
because a broker calls in a loan, or because a fund is losing customers and must raise cash. The
buyer at the lower band is taking the other side of that over-reaction, buying from someone who is
selling for reasons that have little to do with the value of the business.

The counterparty keeps appearing for the same reason in any market with leverage and with people who
own more than they can comfortably hold. The mirror image is the seller at the upper band, who sells
to buyers arriving late out of envy. Both trades assume that the price usually returns towards its own
recent average rather than continuing away from it, which is exactly the assumption the research
below puts under pressure.

## An everyday comparison

A dog on a long lead walks beside its owner. The owner walks at a steady pace, which is the average,
and the lead is the band: it stretches when the dog gets excited. Touching the end of the lead does
not tell you that the dog is about to turn around. A calm dog that drifts to the end of the lead and
comes back is the kind of day the strategy is betting on, but a strong dog that keeps pulling drags
the owner along, and the end of the lead stays taut while both of them travel a long way. The lead
measures how far the dog is from the owner; it says nothing about where either of them is going.

## The rules, step by step

1. Pick one thing to trade: one company's shares, or one fund that tracks an index.
2. Get the daily closing price.
3. Choose the length of the average, called `period`. The fastquant default is 20 days.
4. Choose how many standard deviations wide to make the channel, called `devfactor`. The fastquant
   default is 2.0. The library's source itself notes that the implementation is naive and deserves
   closer study, which is a fair warning about how much weight to put on the defaults.
5. Each day, compute the average of the last `period` closes and the standard deviation of the same
   closes, as in the next section.
6. Draw the upper line at the average plus `devfactor` standard deviations, and the lower line at the
   average minus the same amount.
7. Buy when the closing price is below the lower line.
8. Sell when the closing price is above the upper line.
9. Hold at every other time. The rule is checked each day at the close, and both the buy and the sell
   use the whole of the available cash or position by default.

Note that this is the opposite of what many traders do with the upper line. Here a price above the top
of the channel is a sell signal, whereas some users of the bands read it as a sign of strength and buy.
That disagreement is not a detail; it is the subject of the research section.

## The maths, with every symbol named

The middle line is a plain moving average:

```text
mid_t = (P_t + P_(t-1) + ... + P_(t-period+1)) / period
```

- `mid_t` is the middle line on day `t`.
- `P_t` is today's closing price, and the sum runs back `period` days.
- `period` is the length chosen, 20 by default, so the average is of the last twenty closes.

The standard deviation measures how far the last `period` prices sit from their own average:

```text
sd_t = sqrt( ( (P_t - mid_t)^2 + ... + (P_(t-period+1) - mid_t)^2 ) / period )
```

- `sd_t` is the standard deviation, in the same units as the price: a value of 1.20 means the typical
  distance of the recent closes from their average was about 1.20.
- Each term squares the distance of one price from the middle line, so prices above and below the
  average add to the width in the same way.
- The division is by `period`, not by `period - 1`, which is the version the library's engine uses;
  the difference matters only for very short windows.

The two bands are then placed one width above and below:

```text
upper_t = mid_t + devfactor * sd_t
lower_t = mid_t - devfactor * sd_t
```

- `upper_t` and `lower_t` are the top and bottom of the channel on day `t`.
- `devfactor` is the number of standard deviations, 2.0 by default.

Finally the two rules:

```text
Buy  when P_t < lower_t
Sell when P_t > upper_t
```

- `P_t` is compared with the line on the same day, using the line computed from the closes up to and
  including today.
- Nothing in either band refers to a value other than the price itself; the channel is the price,
  rearranged.

## A worked example

Ten made-up closes, with `period` set to 5 and `devfactor` to 1, so that the arithmetic fits on the
page. The library's defaults are 20 and 2.0, and the rule is the same.

| Day | Close | Mid, the average | Standard deviation | Upper line | Lower line | Signal                 |
| --- | ----- | ---------------- | ------------------ | ---------- | ---------- | ---------------------- |
| 1   | 100   | -                | -                  | -          | -          |                        |
| 2   | 101   | -                | -                  | -          | -          |                        |
| 3   | 100   | -                | -                  | -          | -          |                        |
| 4   | 101   | -                | -                  | -          | -          |                        |
| 5   | 100   | 100.40           | 0.49               | 100.89     | 99.91      |                        |
| 6   | 101   | 100.60           | 0.49               | 101.09     | 100.11     |                        |
| 7   | 100   | 100.40           | 0.49               | 100.89     | 99.91      |                        |
| 8   | 92    | 98.80            | 3.43               | 102.23     | 95.37      | buy: 92 below 95.37    |
| 9   | 100   | 98.60            | 3.32               | 101.92     | 95.28      |                        |
| 10  | 112   | 101.00           | 6.39               | 107.39     | 94.61      | sell: 112 above 107.39 |

Days 1 to 7 are calm: the price moves between 100 and 101, the channel is less than a unit wide, and
the price never leaves it. On day 8 the price falls to 92. Two things happen at once. The average is
dragged down, to 98.80, and the standard deviation jumps from 0.49 to 3.43, because one price is now
far below the average. The lower line falls to 95.37, but the price has fallen further, to 92, so the
buy condition is met and the position is bought at 92.

On day 10 the price jumps to 112. The standard deviation widens again, to 6.39, because the window now
contains both the low of 92 and the high of 112, and the upper line is at 107.39. The price is above
it, so the position is sold at 112.

Now the money. Buy 100 shares at 92, costing 9,200.00, and sell them at 112, bringing in 11,200.00.
The gross gain is 2,000.00. At 0.10 percent per side the costs are 9.20 and 11.20:

```text
Net gain = 2,000.00 - 20.40 = 1,979.60
Return on the money used = 1,979.60 / 9,200.00 = 21.5 percent
```

The numbers were invented to put a signal at each end of the table, so they prove nothing about the
strategy. They do show why a band touch is not, by itself, an event: on the day the price fell to 92,
the band moved down to meet it, and the same thing happens in reverse on the way up. The price does
not break through a fixed wall; it stretches a rubber sheet that is made of the price itself.

## What the research actually found

The findings genuinely disagree, which is why the grade above is disputed rather than weak.

| Source                          | What it measured                                                           | What came out                                                                                                                                                                                               |
| ------------------------------- | -------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Ni, Day, Huang and Yu (2020)    | The 50 largest Taiwanese shares, buying when the price hits the lower band | Positive abnormal returns on those buys; at the upper band the profitable side was continuing to hold, not selling, which contradicts the rule this library uses                                            |
| Leung and Chong (2003)          | Bollinger bands against a simpler fixed-width envelope                     | The bands did not outperform the simpler envelope, despite adapting to sudden price changes                                                                                                                 |
| Lento and Gradojevic (2022)     | Many rules on five markets during the crash of early 2020                  | Only Bollinger bands and trading range breakouts stayed profitable after costs; the moving average rules did not                                                                                            |
| Day, Cheng, Huang and Ni (2023) | Bitcoin futures, buying at the lower band and selling at the upper         | An average holding-period return above 20 percent, rising above 50 percent when the average was lengthened from 20 to 60 days; these are leveraged contracts and the sample is one asset in one bull market |

Read together, the picture is not that the bands work or do not work. It is that the answer depends on
the market, the period and the way the rule is turned around. One study finds the lower band a good
place to buy, another finds the whole family no better than a simpler channel, and a third finds it
profitable only in a violent crash. On the statistical question, the canonical description reports
that with the 20-day, 2-standard-deviation default, only about 88 percent of prices stay inside the
bands, not the 95 percent a bell-shaped distribution would give, because prices are not bell-shaped.
That single fact is worth more than the parameter defaults: the channel is a ruler for stretching, not
a probability statement.

And the defaults. The fastquant defaults of 20 and 2.0 are the numbers John Bollinger popularised, and
the library's README reports them turning 100,000 into 97,060.30 on one Philippine stock over 2018, a
loss of about 3 percent with a commission of zero. The source file itself carries a note that the
implementation is naive. None of this is a finding about the bands; it is one year of one share.

## How this project relates to it

The repository implements the bands, the middle average and the standard deviation in Rust, in
[crates/indicators/src/momentum/bb.rs](../../../crates/indicators/src/momentum/bb.rs). That file
contains the same square-root formula as the maths section, and comparing it with the simple average
in [crates/indicators/src/average/sma.rs](../../../crates/indicators/src/average/sma.rs) shows that the
only extra ingredient is the size of the wobble. The repository's survey of what technical rules have
delivered, and of how quickly a published rule decays once it is crowded, is
[08_predictability_and_trading_strategies.md](../../../strategies/books2/08_predictability_and_trading_strategies.md).

## Where it goes wrong

- A touch is not a signal. The bands are computed from the price, so a sharp fall lowers the average
  and widens the channel at the same time as it lowers the price. The price and the line move towards
  each other, which is why the same rule that buys a small dip will also buy a collapse, again and
  again, on the way down.
- The two sides contradict each other in practice. Buying the lower band is a bet on the price coming
  back; selling the upper band is a bet on the same thing. A trader who expects strength to persist
  would do the opposite at the top, and one of the studies above finds exactly that.
- The shape of the price is not bell-shaped. Two standard deviations do not contain 95 percent of the
  future, and do not even contain 95 percent of the past; the measured figure is about 88 percent.
  The bands are a summary of recent wobble, not a probability of anything.
- The parameters decide the answer. The length and the width are free choices, and the same study that
  found a profit with one pair found a much larger one with a different pair. That is a description of
  the search, not of the market.
- The sample chooses the answer. The clearest positive result comes from a single crash, which is the
  one period when prices move furthest from their own averages. A rule that works only when the market
  is panicking is a rule that is almost never available.
- What would have to be true. A touch of the lower band would have to mean the sellers have finished,
  rather than that the price is falling fast. If most touches are simply the continuation of a fall,
  the rule buys all the way down and pays a cost at each step.

## Try it yourself

You need a spreadsheet and about forty daily closes.

1. Put dates in column A and closes in column B.
2. In C20 enter `=AVERAGE(B1:B20)` for the middle line and drag it down.
3. In D20 enter `=STDEV.P(B1:B20)`, the standard deviation dividing by 20, and drag it down. The `.P`
   tells the spreadsheet to divide by the number of items rather than by one less.
4. In E20 enter `=C20+2*D20` for the upper line, and in F20 `=C20-2*D20` for the lower line, and drag
   both down.
5. In G20 enter `=IF(B20<F20,1,IF(B20>E20,-1,0))`, so a 1 marks a close below the lower line and a -1
   a close above the upper line.
6. Add a column that writes out the width of the channel, `=E20-F20`, and look at its smallest and
   largest values over the whole sheet.

What to notice: the channel is narrow at exactly the calm moments and wide at exactly the wild ones,
so it never gives you a still ruler to measure against. Count the buy marks in a falling stretch of
prices. If there are several on the way down, before the bottom, that is the failure mode the
research is describing, and it is visible in a few minutes with a spreadsheet.

## Where this came from

- The [fastquant](https://github.com/enzoampil/fastquant) README, for the alias `bbands`, the parameter
  names `period` and `devfactor`, and the example result.
- The [Bollinger bands source](https://github.com/enzoampil/fastquant/blob/master/python/fastquant/strategies/bollinger_band.py),
  for the exact buy and sell conditions and for the note in the source that the implementation is
  naive.
- The [backtest documentation](https://github.com/enzoampil/fastquant/blob/master/docs/docusaurus/docs/backtest.md),
  for the parameters that control how much is bought and sold.
- The [canonical description of the bands](https://en.wikipedia.org/wiki/Bollinger_Bands), for their
  origin with John Bollinger, the default of 20 days and 2 standard deviations, and the finding that
  roughly 88 percent of prices stay inside the bands.
- Ni, Day, Huang and Yu (2020), [The profitability of Bollinger Bands: Evidence from the constituent stocks of Taiwan 50](https://doi.org/10.1016/j.physa.2020.124144).
- Leung and Chong (2003), [An empirical comparison of moving average envelopes and Bollinger Bands](https://doi.org/10.1080/1350485022000041032).
- Lento and Gradojevic (2022), [The Profitability of Technical Analysis during the COVID-19 Market Meltdown](https://www.mdpi.com/1911-8074/15/5/192).
- Day, Cheng, Huang and Ni (2023), [The profitability of Bollinger Bands trading bitcoin futures](https://doi.org/10.1080/13504851.2022.2060494).
- The repository's own survey,
  [08_predictability_and_trading_strategies.md](../../../strategies/books2/08_predictability_and_trading_strategies.md).

## Words used in this tutorial

- band: one of the two lines drawn above and below a moving average.
- deviation: the distance of one price from the average of a group of prices.
- envelope: a channel drawn around a moving average, of which the bands are one kind.
- leverage: borrowing money to hold more than you could afford with your own cash.
- standard deviation: a measure of how far a group of numbers typically sits from their own average.
- volatility: how much a price moves around its average, measured as a percentage per year.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
