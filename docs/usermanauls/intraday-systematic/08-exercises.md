# 08 - Exercises

Work each exercise before reading its solution. The solutions are given as the changed lines or as a
complete program, and each shows real output. One exercise asks you to break the program on purpose;
do it, because reading about a failure is not the same as seeing one.

Use the lecture 05 program as your base. Save it as a scratch file, change it, run it, and compare
the output with the numbers already in this manual.

## Exercise 1: change the periods

Set the fast period to 5 and the slow period to 20. Predict whether the rule trades more or fewer
times, then run it and report the fills, positions, PnL, win rate and profit factor.

Solution, the two changed lines in `IntradayRuleConfig`:

```python
            fast_period=5,
            slow_period=20,
```

Output:

```text
fills: 14
positions: 7
PnL (total): 1877.5699999999488
Win Rate: 0.8571428571428571
Profit Factor: nan
```

Faster averages trade more: fourteen fills instead of twelve. On this data the result is a profit,
which is exactly the danger. Nothing about the rule got better; the parameters were moved until the
curve improved, which is curve fitting. The `nan` profit factor is another warning: the engine reports
not-available rather than inventing a number when the statistic cannot be computed
(`docs/concepts/performance_periods.md`).

## Exercise 2: change the size

Set `trade_size` to `Decimal(10_000)` and predict the PnL before you run it.

Solution, the changed line in `engine.add_strategy`:

```python
            trade_size=Decimal(10_000),
```

Output:

```text
fills: 12
positions: 6
PnL (total): -24.180000000051223
Win Rate: 0.16666666666666666
Profit Factor: 0.20303355126792264
```

The loss is one tenth of the original 241.89 USD, because every price and every fee scales with size.
The direction and the win rate are unchanged. Sizing changes how loudly the rule speaks, not what it
says.

## Exercise 3: count the bars

Write a program that reads the committed CSV and counts how many bars closed above their open, how
many below, and how many flat.

Solution:

```python
import csv

PATH = (
    "D:/Users/vince/PycharmProjects/nautilus_trader/docs/usermanauls/"
    "intraday-systematic/sample_data/usdjpy_1min_bars.csv"
)

up = 0
down = 0
flat = 0
with open(PATH, newline="") as f:
    for row in csv.DictReader(f):
        if float(row["close"]) > float(row["open"]):
            up += 1
        elif float(row["close"]) < float(row["open"]):
            down += 1
        else:
            flat += 1

print("up bars:", up)
print("down bars:", down)
print("flat bars:", flat)
```

Output:

```text
up bars: 87
down bars: 78
flat bars: 2
```

The two flat bars are the first bar, whose open equals its close, and one bar inside the sine wave
where the price returned to its starting value. A little over half the bars rose, which is the raw
material the rule is trying to exploit.

## Exercise 4: price the fees

Re-run the lecture 05 program with `maker_rate=Decimal("0")` and `taker_rate=Decimal("0")`. Report
the difference in PnL and explain where the number comes from.

Solution, the changed line in `engine.add_venue`:

```python
    fee_model=MakerTakerFeeModel(maker_rate=Decimal("0"), taker_rate=Decimal("0")),
```

Output:

```text
PnL (total): -217.89000000001397
```

The difference is `-217.89 - (-241.89) = 24.00` USD. That is the sum of the twelve commissions in
the fills report, converted to the account currency. Fees were about ten percent of the loss.

## Exercise 5: add a protective stop

Add a stop to the lecture 05 rule: whenever it opens a position, place a stop 0.050 away in the
other direction. Report the fills, the order types, the PnL and the win rate.

Solution, the two entry branches in `on_bar` gain a second `submit_order`:

```python
            if self.portfolio.is_net_flat(self.config.instrument_id):
                self.submit_order(
                    self.order_factory.market(
                        self.config.instrument_id, OrderSide.BUY, self._quantity(),
                    ),
                )
                trigger = Price.from_str(f"{float(bar.close) - float(self.config.stop_distance):.3f}")
                self.submit_order(
                    self.order_factory.stop_market(
                        self.config.instrument_id, OrderSide.SELL, self._quantity(), trigger,
                    ),
                )
```

and the short branch is the mirror image, with `OrderSide.SELL` becoming `OrderSide.BUY` and the
trigger adding the distance instead of subtracting it. The config also gains `stop_distance`.

Output:

```text
fills: 16
stop orders: {'MARKET': 10, 'STOP_MARKET': 7}
PnL (total): 12.07999999995809
Win Rate: 0.125
```

The stops turned this loss into a small profit and lowered the win rate, because they cut several
losers short at the cost of a few trips that would have recovered. Do not take this as proof that
stops help: the data is a sine wave, the sample is tiny, and the stop distance was chosen by hand.
A different distance gives a different answer, and that is the point of lecture 07.

## Exercise 6: break it on purpose

Delete the two lines that guard on initialization:

```python
        if not self.indicators_initialized():
            return
```

Predict what happens before you run it. Does the program crash?

Solution, the changed lines in `on_bar` (both removed):

```python
    def on_bar(self, bar: Bar) -> None:
        if self.fast_ema.value > self.slow_ema.value:
```

Output:

```text
fills: 14
positions: 7
PnL (total): -83.39000000013039
Win Rate: 0.14285714285714285
Profit Factor: 0.7253803862134941
```

It does not crash, which is the trap. Before an average is initialized, its value is 0.0, so once the
fast average is ready at bar 10 it compares greater than the still-empty slow average and the rule
starts trading early, on a false signal. The result changes silently. This is why the guard matters,
and why a program that runs without error is not the same as a program that is correct.

## Exercise 7: interrogate a good-looking result

The lecture 03 program printed a win rate of 1.0 on three round trips. Write down three reasons this
is not evidence, then check how many trades you would require before believing a result.

Solution, no code. The three reasons:

1. Three trades is a sample far too small for a win rate to be stable; with three trades, a win rate
   of 1.0 is one of eight possible outcomes by chance alone.
2. The data is a smooth sine wave with no news, no gaps and a constant spread, so any trend rule
   looks good on it.
3. The rule never exits except on an opposite crossover, so it held through the entire wave from
   bottom to top, which is the best possible path on that data.

There is no fixed number that guarantees a result is real, but a few hundred trades is a reasonable
floor for a rule of this kind before you treat the statistics as informative rather than decorative.

## Exercise 8: make the rule degenerate

Set `fast_period=10` and `slow_period=10` so the two averages have the same length. Predict how many
orders the rule sends, then run it.

Solution, the changed line in `IntradayRuleConfig`:

```python
            slow_period=10,
```

Output:

```text
orders: 0
fills: 0
positions: 0
PnL (total): 0.0
```

Zero orders. Two equal-length averages always have equal values, so neither `fast > slow` nor
`fast < slow` is ever true, and the rule never wants a direction. A backtest that does nothing looks
perfect in every statistic that divides by the number of trades, because there are none; this is why
you always print the trade count.

Next: where to go from here, in [09-go-further](09-go-further.md).
