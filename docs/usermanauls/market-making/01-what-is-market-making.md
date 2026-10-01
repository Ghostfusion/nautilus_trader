# 01 - What is market making?

## The one-sentence version

A market maker posts a price to buy and a price to sell at the same time, and earns the small
difference between those two prices whenever both of them trade.

That is the whole idea. Everything else in market making is about the three ways it goes wrong.

## The two prices

Every market has two prices at any moment:

- The **bid** is the highest price anyone is currently willing to pay.
- The **ask** is the lowest price anyone is currently willing to sell for.

The gap between them is the **spread**. If the best bid is `1.20000` and the best ask is
`1.20020`, the spread is `0.00020`.

A market maker does not wait for someone else to publish those prices. A market maker *publishes*
them. You send a **limit order** to buy at `1.20000` and a limit order to sell at `1.20020`. Both
orders rest in the market. Each is called a **quote**. Together they are your **two-sided quote**.

If a seller arrives and hits your bid, you buy at `1.20000`. If a buyer arrives and lifts your ask,
you sell at `1.20020`. Do both and you have bought low and sold high.

## Who does this

- **Professional trading firms** run market making as their main business on exchanges, foreign
  exchange, and crypto venues. They measure profit in fractions of a basis point.
- **Exchange operators** run market making programs to keep new markets liquid.
- **Systematic traders** use market making as one strategy among several, often on crypto
  perpetual futures because those venues trade continuously.

A **basis point** (bps) is one hundredth of one percent, that is `0.0001` as a fraction, or
`0.01%`. A spread of `0.00020` on a price of `1.20010` is about `1.67` bps.

## Where the profit comes from

Three sources, in order of importance for a beginner:

1. **The spread.** You buy at the bid and sell at the ask. The difference is gross profit.
2. **Rebates.** Some venues pay you a small fee when your resting order provides liquidity instead
   of taking it. This is a **maker rebate**.
3. **Fees you avoid.** A **taker** order crosses the spread and pays the taker fee. A **maker**
   order rests and pays the maker fee, which is usually lower. A market maker tries to be a maker.

The venue charges a **commission** on every fill. In this repository the model is
`MakerTakerFeeModel`: you give it a `maker_rate` and a `taker_rate`, both expressed as a fraction of
the traded value. See `docs/concepts/backtesting/fill-models.md`.

## The three risks

### 1. Adverse selection (the important one)

You do not choose which of your two quotes fills first. The market chooses for you.

Your bid fills when the market is falling. Your ask fills when the market is rising. So most of
your fills arrive *just before* the price moves against the position those fills created. The
people who trade against you often know something, and you are the one quoting to them.

This is **adverse selection**: your resting orders are selected by informed traders precisely when
they are wrong. It is why a filled passive order can be a losing trade. Worked example 2 below
shows the arithmetic.

### 2. Inventory risk

After one side fills you hold a **position**: a quantity you own (long) or owe (short). That
position is exposed to price moves until the other side fills. Market makers call this
**inventory risk**. The larger the position and the longer you hold it, the more one bad move costs.

### 3. Fee and cost risk

Every fill costs commission. If the spread you earn is smaller than the fees you pay on both legs,
a "successful" round trip still loses money. On thin markets the spread can be smaller than the
round-trip fee.

## Vocabulary

| Term              | Plain meaning                                                               |
| ----------------- | --------------------------------------------------------------------------- |
| Bid               | The price you are willing to buy at.                                        |
| Ask (offer)       | The price you are willing to sell at.                                       |
| Spread            | Ask minus bid. Your gross profit per round trip.                            |
| Quote             | A resting limit order that advertises a price and size.                     |
| Maker             | The order that rests and provides liquidity.                                |
| Taker             | The order that crosses the spread and consumes liquidity.                   |
| Fill              | The event of your order trading.                                            |
| Inventory         | The position you hold after fills, long or short.                           |
| Position          | The net quantity of an instrument you own or owe.                           |
| Long              | You own the instrument; profit if the price rises.                          |
| Short             | You owe the instrument; profit if the price falls.                          |
| Adverse selection | Your resting orders fill just before the price moves against you.           |
| Basis point (bps) | One hundredth of one percent, `0.0001`.                                     |
| Skew              | Shifting both quotes in one direction to discourage more fills on one side. |
| Max position      | The hard cap on how large your inventory may grow.                          |
| Mark-to-market    | Valuing your position at the current market price.                          |
| Realized PnL      | Profit or loss booked when a position closes.                               |
| Unrealized PnL    | Profit or loss on a still-open position at the current price.               |
| Queue position    | Your place in line among orders resting at the same price.                  |
| Tick              | The smallest price increment a market allows.                               |
| Post-only         | An order the venue rejects or cancels if it would cross the spread.         |

## Worked example 1: both sides fill

You quote a bid at `1.20000` and an ask at `1.20020` for `1,000,000` units. A seller hits your bid:
you buy `1,000,000` at `1.20000`. Later a buyer lifts your ask: you sell `1,000,000` at `1.20020`.
The venue charges `0.00002` on each side.

```python
from decimal import Decimal

fee_rate = Decimal("0.00002")
size = Decimal("1000000")

# Example 1: both sides of the quote fill, round trip.
buy_px = Decimal("1.20000")
sell_px = Decimal("1.20020")
buy_notional = size * buy_px
sell_notional = size * sell_px
gross = sell_notional - buy_notional
fee_buy = buy_notional * fee_rate
fee_sell = sell_notional * fee_rate
print("Example 1: both sides fill")
print(f"  buy notional  = {size} * {buy_px} = {buy_notional} USD")
print(f"  sell notional = {size} * {sell_px} = {sell_notional} USD")
print(f"  gross spread  = {gross} USD")
print(f"  fees          = {fee_buy} + {fee_sell} = {fee_buy + fee_sell} USD")
print(f"  net profit    = {gross - fee_buy - fee_sell} USD")
```

Output:

```text
Example 1: both sides fill
  buy notional  = 1000000 * 1.20000 = 1200000.00000 USD
  sell notional = 1000000 * 1.20020 = 1200200.00000 USD
  gross spread  = 200.00000 USD
  fees          = 24.0000000000 + 24.0040000000 = 48.0040000000 USD
  net profit    = 151.9960000000 USD
```

Read the arithmetic in words. You bought `1,000,000` units for `1,200,000` dollars. You sold them
for `1,200,200` dollars. Gross profit is `200` dollars. The fees are `1200000 * 0.00002 = 24` and
`1200200 * 0.00002 = 24.004`. Net profit is about `152` dollars.

Notice how thin this is. The round trip earned `152` dollars on `1.2` million of turnover, which is
about `0.0127%`. A single tick of adverse move can erase it.

## Worked example 2: one fill, then the market moves

Your bid at `1.20000` fills for `1,000,000` units. You now own `1,000,000` units at `1.20000`. Before
your ask fills, the market falls: the new bid is `1.19900`. You decide to sell out at `1.19900`.

```python
from decimal import Decimal

fee_rate = Decimal("0.00002")
size = Decimal("1000000")

# Example 2: only the buy fills, and the market falls before you sell.
entry_px = Decimal("1.20000")
exit_px = Decimal("1.19900")
entry_notional = size * entry_px
exit_notional = size * exit_px
loss = entry_notional - exit_notional
fee_entry = entry_notional * fee_rate
fee_exit = exit_notional * fee_rate
print("Example 2: buy fills, market falls to 1.19900")
print(f"  paid          = {entry_notional} USD")
print(f"  received      = {exit_notional} USD")
print(f"  gross loss    = {loss} USD")
print(f"  fees          = {fee_entry} + {fee_exit} = {fee_entry + fee_exit} USD")
print(f"  net loss      = {loss + fee_entry + fee_exit} USD")
```

Output:

```text
Example 2: buy fills, market falls to 1.19900
  paid          = 1200000.00000 USD
  received      = 1199000.00000 USD
  gross loss    = 1000.00000 USD
  fees          = 24.0000000000 + 23.9800000000 = 47.9800000000 USD
  net loss      = 1047.9800000000 USD
```

One adverse move of `0.00100` produced a `1,047.98` dollar loss. That is roughly seven good round
trips from example 1 destroyed by one bad one. **This is why a filled passive order can be a losing
trade.** The order did exactly what it was told. The market simply moved the wrong way after it
filled.

## Worked example 3: how big is the spread?

The spread in price terms means little until you compare it to the market move that can erase it.

```python
from decimal import Decimal

size = Decimal("1000000")
buy_px = Decimal("1.20000")
sell_px = Decimal("1.20020")
spread = sell_px - buy_px
mid = (buy_px + sell_px) / 2
spread_bps = spread / mid * Decimal("10000")
print("Example 3: spread size")
print(f"  spread        = {spread} USD per unit")
print(f"  mid           = {mid}")
print(f"  spread        = {spread_bps} basis points")
print(f"  one 0.01 move against inventory = {size * Decimal('0.01')} USD")
print(f"  round trips of 151.996 USD needed to absorb it = "
      f"{(size * Decimal('0.01')) / Decimal('151.996'):.1f}")
```

Output:

```text
Example 3: spread size
  spread        = 0.00020 USD per unit
  mid           = 1.20010
  spread        = 1.666527789350887426047829348 basis points
  one 0.01 move against inventory = 10000.00 USD
  round trips of 151.996 USD needed to absorb it = 65.8
```

The message: on a `1.67` bps market, a single `0.01` move against inventory on `1,000,000` units
costs `10,000` dollars, which is about 66 of the profitable round trips from example 1. Market
making is a high-frequency, low-margin business where a few bad minutes decide the year.

## Why the repository example quotes both sides

The canonical example in this repository, `examples/backtest/fx_market_maker_gbpusd_bars.py`, uses
the shipped `GridMarketMaker` strategy. It places a ladder of buying levels below the mid and
selling levels above it, and it repegs that ladder whenever the mid moves far enough. You will run
it, unchanged in shape, in lecture `03`.

## The two habits that separate a surviving market maker from a failing one

1. **Cap the inventory.** Decide the largest position you will ever hold, and refuse quotes that
   would exceed it. The strategy field is called `max_position`.
2. **Skew away from the risk.** When you are long, move both quotes down so the sell side is easier
   to hit and the buy side is harder to hit. The strategy field is called `skew_factor`.

Both are covered with running code in lecture `07`.

Continue to [02-the-engine-view.md](02-the-engine-view.md) to see how this repository represents all
of the above.

Previous: [README.md](README.md) | Next: [02-the-engine-view.md](02-the-engine-view.md)
