# 01 - What is cross-venue relative value?

This lecture assumes you have never seen a price chart and have never written a program. Every
term is defined the first time it appears, and every number is worked out by hand before any code
is shown.

## 1. The idea in one paragraph

A **venue** is a place where an instrument is bought and sold: an exchange, a broker, a pool, a
market. The same economic thing is often listed on more than one venue. A share of one company,
a kilogram of one metal, one unit of one crypto asset: the thing is identical, but the price is
not, because the venues have separate order books, separate participants and separate rules.

**Relative value** trading buys the cheap one and sells the expensive one at the same time. You do
not care whether the asset goes up or down. You care only that the difference between the two
prices moves in your favour. That difference is the **spread**.

**Cross-venue** relative value is the version of this where the two prices come from two different
venues. The same idea also covers two different instruments on one or two venues that should move
together: a perpetual future against the spot price it tracks, two perpetual futures on the same
asset at two venues, one company against another company in the same industry.

## 2. Who does this, and why it exists

Relative value is one of the oldest styles in trading. It is used by:

- **Market makers** who quote on two venues and quote the rich one a little wider.
- **Hedge funds and prop firms**, whose relative value desks run hundreds of pairs at once.
- **Crypto desks**, which arbitrage a perpetual future against spot, and one venue's perpetual
  against another venue's perpetual.
- **Anybody with an inventory problem**: if you already hold the asset on venue A and want the
  same exposure on venue B, the pair trade moves the exposure without selling the position.

The profit comes from **convergence**: the difference between two prices of the same thing tends to
shrink, because a trader can buy at one venue and sell at the other, which pushes the two prices
together. Your trade is placed to be paid when that happens.

The style exists because the market is not perfectly connected:

- Moving money between venues takes time, and during that time the price can move.
- Some accounts can trade on one venue but not the other.
- The two venues have different fee schedules and different liquidity.
- The two instruments are similar but not identical (a perpetual future and spot have different
  funding, margin and settlement rules).

## 3. Vocabulary

| Term                    | Plain meaning                                                                                             |
| ----------------------- | --------------------------------------------------------------------------------------------------------- |
| Venue                   | A place where an instrument is traded, for example a crypto exchange.                                     |
| Instrument              | The thing being traded, for example "BTCUSDT-PERP on BINANCE".                                            |
| Order book              | The list of resting buy and sell orders on one venue for one instrument.                                  |
| Bid                     | The highest price a buyer is currently offering. You can sell to the bid.                                 |
| Ask                     | The lowest price a seller is currently asking. You can buy from the ask.                                  |
| Bid-ask spread          | `ask - bid`. It is a cost: you buy at the ask and sell at the bid.                                        |
| Mid price               | `(bid + ask) / 2`. A reference price, not a price you can trade at.                                       |
| Last price              | The price of the most recent completed trade.                                                             |
| Basis                   | The difference between two prices of the same economic thing.                                             |
| Spread                  | In this manual, the difference you are trying to capture, usually the basis.                              |
| Basis point (bps)       | One hundredth of one per cent. `1 bps = 0.01%`. `10,000 bps = 100%`.                                      |
| Leg                     | One half of a pair trade. A two-leg trade has a long leg and a short leg.                                 |
| Long                    | You own the instrument. You profit when the price rises.                                                  |
| Short                   | You have sold an instrument you did not own, and must buy it back later. You profit when the price falls. |
| Notional                | `price * quantity`. The money value of a position.                                                        |
| Perpetual future (perp) | A futures contract with no expiry date, kept in line with spot by periodic payments.                      |
| Funding rate            | The periodic payment between longs and shorts in a perpetual future.                                      |
| Index price             | An external reference price for one asset, used by the venue to compute mark price and funding.           |
| Mark price              | The price the venue uses to value positions and to decide liquidations.                                   |
| Hedge ratio             | How many units of one leg you hold for each unit of the other leg.                                        |
| Beta                    | The regression slope of one instrument's returns on another's; a common way to choose a hedge ratio.      |
| Dollar-neutral          | Long notional equals short notional, so a broad market move creates no net exposure.                      |
| Maker                   | An order that rests in the book. Usually a lower fee, sometimes a rebate.                                 |
| Taker                   | An order that crosses the bid-ask spread and trades immediately. Usually a higher fee.                    |
| Slippage                | The difference between the price you expected and the price you got.                                      |
| Netting                 | Combining all fills for one instrument into one position.                                                 |
| Hedging                 | Keeping long and short positions in the same instrument separate instead of combining them.               |
| Leg risk                | The risk that one leg fills and the other does not.                                                       |
| Convergence             | The two prices moving back together. The source of the profit.                                            |
| Carry                   | Profit or cost that accrues while a position is held, for example funding.                                |

## 4. Worked example 1: the same asset on two venues

You watch one asset on two venues. Both venues quote a two-sided market.

| Venue   | Bid    | Ask    | Mid    |
| ------- | ------ | ------ | ------ |
| Venue A | 99.99  | 100.01 | 100.00 |
| Venue B | 100.19 | 100.21 | 100.20 |

Step 1: the basis in money.

```
mid_A  = (99.99 + 100.01) / 2 = 100.00
mid_B  = (100.19 + 100.21) / 2 = 100.20
basis  = mid_B - mid_A = 100.20 - 100.00 = +0.20 per unit
```

Step 2: the basis in basis points. Basis points make the number comparable across instruments,
because `0.20` on a `100.00` asset is a very different trade from `0.20` on a `5.00` asset.

```
basis_bps = basis / mid_A * 10,000
          = 0.20 / 100.00 * 10,000
          = 20 bps
```

Step 3: the basis you can actually trade. The mid is not a price you can reach. To take the trade
you buy at the ask of the cheap venue and sell at the bid of the expensive venue.

```
executable = bid_B - ask_A = 100.19 - 100.01 = 0.18 per unit = 18 bps
```

Two basis points are already gone, before any fee, because you crossed two bid-ask spreads.

Step 4: the fees. Suppose each venue charges a taker fee of 4 bps of the traded notional, which is
`0.0004`.

```
fee_A = 0.0004 * 100.01 = 0.040004 per unit
fee_B = 0.0004 * 100.19 = 0.040076 per unit
fees  = 0.080080 per unit
```

Step 5: what is left.

```
net_before_transfer = 0.18 - 0.080080 = 0.099920 per unit = 9.99 bps
```

Step 6: the cost that eats it. The two positions are on two different venues, and at some point
you want the money on the venue that owes you. A withdrawal and a deposit have a fixed fee and a
delay. If the withdrawal costs 0.10 per unit, the whole trade is negative:

```
net_after_transfer = 0.099920 - 0.10 = -0.000080 per unit
```

The lesson of this example is the whole of this manual in one line: **a 20 bps paper basis became
a loss of 0.00008 per unit once two spreads, two fees and one transfer were paid.** The transfer
time also matters: while the money is in transit, which can be twenty minutes on a blockchain and
several days for a bank wire, the basis can move against you.

## 5. Worked example 2: a perpetual future against spot

A **perpetual future** is a contract that tracks an asset with no expiry date. Because it never
expires, nothing forces its price to equal the spot price at a known date. Instead the venue makes
a **funding payment** between the two sides of the contract, normally every eight hours.

Say spot trades at 100.00 and the perpetual trades at 100.30.

```
basis = 100.30 - 100.00 = +0.30 = 30 bps
```

The perpetual is expensive, so you sell the perpetual and buy the spot (you are long spot, short
perp). If the funding rate is positive, longs pay shorts, so as a short you **receive** funding.
With a funding rate of `0.0001` (which is 0.01%, or 1 bps) every eight hours:

```
funding per payment  = 0.0001 * 100.30 = 0.0100300 per unit
payments per day     = 3
carry per day        = 0.0300900 per unit
carry in bps per day = 0.0300900 / 100.30 * 10,000 = 3.0 bps per day
```

Against that carry you pay the entry and exit cost of the example above, roughly 10 bps to get in
and 10 bps to get out. So this trade needs about seven days of funding to break even, and it is a
trade only if you believe the 30 bps premium will not collapse first. The number that decides
the trade is not the basis, it is **basis minus cost, divided by the carry**.

## 6. Worked example 3: two instruments and a hedge ratio

Relative value is not limited to one asset on two venues. Two different instruments can be linked
by an economic relationship. Suppose a regression over history says instrument Y moves 2 units for
every 1 unit of instrument X, so the hedge ratio is 2.

|              | Price  |     |
| ------------ | ------ | --- |
| Instrument X | 100.00 |     |
| Instrument Y | 50.00  |     |

**Dollar-neutral sizing** means the two notionals are equal:

```
long X:  100 units * 100.00 = 10,000 notional
short Y: 200 units *  50.00 = 10,000 notional
net exposure = 10,000 - 10,000 = 0
```

**Beta-neutral sizing looked at wrongly** means the long notional is multiplied by the hedge ratio
as well:

```
long X:  notional = 2 * 10,000 = 20,000
short Y: notional =          10,000
net exposure = 20,000 - 10,000 = +10,000
```

The second version carries a large directional exposure while wearing the label "hedged". The
repository's own design record for relative-value screens states this failure precisely and
measures it: see the table in `docs/design/relative_value_screening.md`, section 2.6, where the
rule as written shows a net exposure of `+10000` against a flat-notional net of `0`. Dollar
neutrality and market neutrality are different things, and a sizing rule that claims to remove
beta but does not is worse than no rule at all, because the risk report will say the position is
hedged.

## 7. Where the profit comes from, honestly

Three sources, in order of size for a beginner:

1. **Convergence.** The two prices move together again and you exit at a smaller basis.
2. **Carry.** While you hold the pair, funding, interest, or the dividend of one leg against the
   other pays you.
3. **Liquidity provision.** If you can quote the spread instead of taking it, you avoid the two
   bid-ask spreads of section 4 and may earn a maker rebate.

The costs are:

1. Two taker fees, if you take both legs.
2. Two bid-ask spreads, if you take both legs.
3. Transfer, withdrawal and deposit costs, if the money must move between venues.
4. Funding, if the carry is against you.
5. Borrow cost, if the short leg is a real short and somebody charges you for the loan.
6. The risk that the basis does not converge, which is not a cost but a loss.

## 8. The main risks

- **Basis risk**: the basis widens instead of narrowing. It can widen arbitrarily far.
- **Leg risk**: one leg fills and the other does not, so you are left directional.
- **Venue risk**: one venue halts, goes down, or freezes withdrawals, and one leg of your hedge
  becomes untradeable exactly when you need it.
- **Liquidation risk**: each leg sits in its own margin account. A large move can liquidate one
  leg before the other leg's profit arrives.
- **Transfer risk**: the asset is in transit, and the price moves while it is.
- **Model risk**: the hedge ratio was estimated on history. The relationship can change, and the
  design record in `docs/design/relative_value_screening.md` shows how easily an estimated
  half-life or a fitted slope is mistaken for a stable law.

## 9. What you will build

In the rest of this manual you build, in this order:

- A backtest engine holding **two venues** (`BINANCE` and `BYBIT`), each with its own account.
- Two perpetual instruments, one per venue, on the same underlying: `BTCUSDT-PERP.BINANCE` and
  `BTCUSDT-PERP.BYBIT`.
- Two committed CSV files with identical timestamps, one per venue.
- A strategy that computes the basis, decides when it is wide enough, and submits **both legs**.
- Reports that tell you what each leg did, why the pair still lost money in the default
  configuration, and what a funding payment does while you hold.

Continue to [02-the-engine-view.md](02-the-engine-view.md).
