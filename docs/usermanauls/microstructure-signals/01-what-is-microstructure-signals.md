# What order book and microstructure signals are

## The order book

A market is not a single price. At any instant, a venue holds a list of orders that traders have
placed but that have not traded yet. Some are offers to buy, called **bids**. Some are offers to
sell, called **asks**. The list of those resting orders, grouped by price, is the **order book**.

By convention a book is drawn with bids on the left and asks on the right, and with the most
aggressive prices nearest the centre:

| Side | Price  | Size |
| ---- | ------ | ---- |
| bid  | 99.90  | 300  |
| bid  | 99.80  | 150  |
| bid  | 99.70  | 900  |
| ask  | 100.10 | 400  |
| ask  | 100.20 | 250  |
| ask  | 100.30 | 700  |

Three facts are visible immediately, and all three matter for the rest of this manual.

1. Bids are always at prices *lower* than asks. A buyer never offers more than a seller is asking,
   because if they did the two orders would have already traded.
2. The best bid (the highest bid, here 99.90) and the best ask (the lowest ask, here 100.10) are
   the two prices at which a trade can happen right now.
3. The **size** at each price is how much quantity is resting there. Size 300 at 99.90 means a
   seller can sell up to 300 units at 99.90 before that level is exhausted.

A **bid** is an order to buy. An **ask** is an order to sell; it is also called an **offer**. The
best bid and best ask together are called the **top of book**, or the **BBO** (best bid and offer).

## The spread

The **spread** is the distance between the best ask and the best bid:

```
spread = best ask price - best bid price
```

In the table above:

```
spread = 100.10 - 99.90 = 0.20
```

The **midpoint** is the price halfway between them:

```
midpoint = (best bid price + best ask price) / 2
midpoint = (99.90 + 100.10) / 2 = 100.00
```

Why the spread exists: a resting order is a promise to trade at a fixed price. Whoever places it
takes the risk that the market moves against them before someone trades with them. The spread is
the compensation for that risk and for the work of quoting. A market with a wide spread is one
where quoting is dangerous or expensive; a market with a narrow spread is one where many traders
compete to quote.

### Worked example 1: the cost of crossing the spread

You want to buy 100 units right now. You have two choices.

- Send a **market order**: it trades immediately at the best ask, 100.10. It is called a **taker**
  because it takes liquidity from the book. You pay `100 * 100.10 = 10,010.00`.
- Send a **limit order** to buy at 99.90 and wait. If a seller arrives, you pay
  `100 * 99.90 = 9,990.00`. You are called a **maker** because you added liquidity to the book.
  But the order may never fill.

The difference is `10,010.00 - 9,990.00 = 20.00`, which is exactly `100 * spread`. Half of the
spread, 0.10 per unit, is what you give up by demanding an immediate trade. This is why any
strategy that must trade often has to earn more than the spread to break even.

## Order book imbalance

The **order book imbalance** is a number that compares the size on the two sides. The simplest
form uses only the top of book:

```
imbalance = (bid size - ask size) / (bid size + ask size)
```

The result is always between -1 and +1. Positive means the bid side is heavier; negative means the
ask side is heavier; zero means perfectly balanced.

Another common form uses a ratio instead of a difference:

```
ratio = smaller size / larger size
```

The ratio is always between 0 and 1. It equals 1 when the sides are equal and approaches 0 when one
side is huge and the other is tiny. The NautilusTrader example strategies use this ratio form; see
`docs/tutorials/orderbook_imbalance.py`.

In the table above, bid size is 300 and ask size is 400:

```
imbalance = (300 - 400) / (300 + 400) = -100 / 700 = -0.143
ratio     = 300 / 400 = 0.750
```

### Worked example 2: an imbalanced top of book

Suppose the top of book is bid 99.90 with size 500, and ask 100.10 with size 50.

```
imbalance = (500 - 50) / (500 + 50) = 450 / 550 = 0.818
ratio     = 50 / 500 = 0.100
```

The bid side is much heavier. The idea behind the signal is that many resting buyers, with little
resting supply, means the next trade is more likely to happen at the ask. If a seller crosses the
spread and buys, the ask at 100.10 is consumed, and the price must move up to find the next seller.
The signal says: lean long.

The same reasoning in reverse says: ask 500 and bid 50 means the price is more likely to fall.

### Worked example 3: why the signal is weak

Now watch what happens one second later. The market maker who was quoting 50 units at 100.10 sees
the imbalance too, and does two things:

1. Cancels the thin ask at 100.10 and reposts it at 100.50, where it is safer.
2. Adds 400 units to the bid at 99.90 to earn the spread from the imbalance.

The book is now bid 99.90 with size 900 and ask 100.50 with size 450. Recompute:

```
ratio = 450 / 900 = 0.500
```

The signal that a moment ago screamed "long" now says "balanced", and the price never moved. The
traders who *can* read the book fastest are the ones your signal is competing against, and their
response is to remove exactly the imbalance your signal measures. This is the central problem of
microstructure signals, and it is not a bug you can fix: it is the nature of the data.

## Why the signal is noisy

Five reasons, in plain words:

- **Size is a promise, not a trade.** A resting 500-unit bid can be cancelled a microsecond before
  you act on it. It was never a real buyer.
- **Spoofing and layering.** Some participants post large orders they intend to cancel, to make the
  book look heavier on one side. The signal reads the fake size as if it were real.
- **Hidden and iceberg orders.** Many venues let traders hide most of their order. The displayed
  size is a lower bound, not the truth.
- **The book is rewritten constantly.** The top of book may change thousands of times a second.
  A signal computed from one snapshot is obsolete almost immediately.
- **One venue is not the market.** The same instrument trades on several venues at once. A signal
  from one venue's book sees only a slice of the true supply and demand.

The academic literature treats imbalance as a real but small predictor. It is useful, not
decisive, and it needs careful measurement before it earns anything. The honest expectation is a
weak edge that disappears after fees unless you are unusually fast.

## Where the profit comes from

A microstructure signal tries to earn money in one of two ways:

1. **Directional.** Buy when the book leans up, sell when it leans down, and hope the price drift
   is larger than the spread plus fees. This is what the example imbalance strategy does. It is the
   simplest and the least likely to work, because it pays the spread every time.
2. **Market making.** Quote on both sides and earn the spread, using the imbalance to skew your
   quotes away from the dangerous side. This is a market making style; it is covered by its own
   manual. The signal is a risk control there, not a forecast.

## The main risks

- **Adverse selection.** You buy because the book leans up, and the price falls anyway. The other
  side of your trade knew something you did not.
- **Fee and spread drag.** Every entry pays the spread and a fee. A tiny edge can be entirely
  consumed by them.
- **Latency.** Your signal is computed from data that is already slightly old. If you are slower
  than the participants who move the price, you trade at the wrong moment.
- **Stale or broken book state.** If a delta is missed, the book you are reading is not the real
  book. The engine has integrity checks for a reason; lecture 07 explains them.
- **Overfitting.** A threshold that worked on one day of one venue may be meaningless everywhere
  else.

## Vocabulary

| Term              | Plain meaning                                                          |
| ----------------- | ---------------------------------------------------------------------- |
| order book        | The list of resting buy and sell orders, by price.                     |
| bid               | A resting order to buy.                                                |
| ask (offer)       | A resting order to sell.                                               |
| size              | The quantity resting at a price level.                                 |
| top of book (BBO) | The best bid and the best ask.                                         |
| spread            | Best ask price minus best bid price.                                   |
| midpoint          | The average of the best bid and best ask.                              |
| maker             | A trader whose order rests in the book and adds liquidity.             |
| taker             | A trader whose order trades immediately against the book.              |
| liquidity         | Resting size available to trade against.                               |
| delta             | One change to the book, such as adding, updating or deleting a level.  |
| snapshot          | A complete picture of the book at one instant.                         |
| imbalance         | A number comparing size on the bid side with size on the ask side.     |
| adverse selection | Losing money because the other side of your trade was better informed. |
| latency           | The delay between an event happening and you reacting to it.           |

## What comes next

[02](02-the-engine-view.md) shows how this repository represents all of the above: the book types,
the delta and depth data types, and the Python objects you will use. [03](03-first-run.md) gets you
to a running program.
