# Orders and how they execute

Date: 2026-10-07. Revision 1.

This page is for someone who has never placed an order and wants to know exactly what each kind of
order promises and what it leaves to chance. It walks through the bid and the ask, the three common
order types, the journey an order takes through an exchange, and the reasons a filled price differs
from the price on the screen.

## The bid, the ask and the spread

At any moment, the people watching a market are willing to do two different things. The bid is the
highest price a buyer is currently offering. The ask is the lowest price a seller is currently
accepting. They are almost never equal, because a buyer wants to pay little and a seller wants to
receive a lot.

The spread is the gap between them, ask minus bid. It is the immediate cost of trading in and
straight back out, and someone earns it. When you buy at the ask and later sell at the bid, the
difference goes to whoever was standing on the other side, usually a dealer quoting both prices all
day. A seat at the best price is not free either: to capture the spread as a resting limit order you
must accept the risk that the market moves against you while you wait. Spreads are measured in
ticks, the smallest allowed price step; on one set of Paris stocks the average spread was 1.43 and
1.99 ticks (`1312.0563v2`).

## Market orders

A market order says: trade now, at whatever price is available. It promises speed and almost
certainly a fill, and it gives up control of the price. A market order is useful when getting in or
out matters more than the exact number, for example when a position must be closed quickly.

It fails when the market is thin. Because you accept whatever prices are standing, a market order in
an illiquid thing can fill far away from the last price you saw, and in a fast-moving market the
prices you were looking at may be gone before your order arrives.

## Limit orders

A limit order says: trade only at this price or better, and never worse. If you are buying, the
price on your order is a maximum; if you are selling, it is a minimum. You give up speed for control:
you will never pay more than your limit, but you may not trade at all.

It fails in the opposite way to a market order. A limit order that is away from the current price
may never fill, and while you wait the market can move without you. Fill is not guaranteed even when
the price touches your level, because other orders may be ahead of yours. In a study of one
electronic order book, 99.9 percent of limit orders were eventually cancelled rather than executed,
more than 90 percent of executions happened at the best prices, and fill probability was negligible
more than one tick away from the best price (`2403.02572v2`). A limit order also tends to fill
preferentially when the market is about to move against you, a cost known as adverse selection: on a
live treasury future, the average price move after a passive fill was about half a tick in the wrong
direction (`2407.16527v1`).

## Stop orders

A stop order says: do nothing until the price reaches a level you set, then send an order. A stop to
sell sits below the current price and is often used to limit a loss. A stop to buy sits above the
current price and is often used to join a rising market. The level is called the trigger.

It fails because the trigger is crossed once and then hands over to an ordinary order, usually a
market order, which then fills at whatever prices are there. If the market jumps over the trigger in
one step, you can be filled well past it. If many people placed stops near the same level, their
orders can arrive together and push the price further, so a stop is not a guarantee of the price you
set.

## What happens when an order reaches an exchange

An exchange does not simply hand your order to a buyer. It runs a fixed sequence.

1. Your order arrives with a price, a size and a direction, and the exchange checks it against the
   rules and against the account it came from.
2. The exchange stamps the order with a time and sends it to the matching engine.
3. The matching engine compares it with the resting orders already waiting in the order book (the
   list of all standing buy and sell orders, sorted by price).
4. Matching follows a priority rule, normally best price first and, at the same price, earliest
   arrival first.
5. Each match produces a trade. If your order is only partly filled, the rest stays in the book as a
   resting order, or is cancelled, depending on the type you chose.
6. A confirmation travels back to you with the price and size of every piece that traded.

Price is not imposed from outside. It is the by-product of which orders arrive and cancel, and how
they meet; across 50 large US stocks, the net imbalance of incoming orders explained about 65
percent of the variation in the price over ten-second windows (`1011.6402v3`).

## Why the price you get is not the price on screen

The price on the screen is a snapshot, and your order reaches the exchange a moment later. If other
orders arrived in between, the best prices may have moved. The difference between the price you
expected and the price you got is called slippage.

An order can fill in several pieces at different prices, because there may not be enough size at the
best price to fill it all. The engine fills what it can at the best price, then moves to the next
price, then the next, so the average you receive is worse than the best price you saw. Each price
level holds a limited amount of size, so this walk through the book is exactly what makes a large
order move the price; because new sellers appear as the price rises, the total impact grows with
size but less than proportionally, a pattern measured across Euro Stoxx futures and more than 100
large US stocks (`1808.09677v2`). The gap between the plan for an order and the fills it actually
receives is the central problem of execution, and it is covered in this repository's brief on
optimal execution and liquidation.

## The promises and the risks, side by side

| Order type   | The promise it makes                              | The risk it leaves                              |
| ------------ | ------------------------------------------------- | ----------------------------------------------- |
| Market       | Trade now at the best prices available            | The fill may be far from the price you saw      |
| Limit        | Trade only at your price or better                | It may never fill, and you miss the move        |
| Stop to sell | Wait, then sell when a set lower price is reached | A jump can skip the trigger and fill much lower |
| Stop to buy  | Wait, then buy when a set higher price is reached | A brief spike can trigger a fill much higher    |

No order gives both certainty of price and certainty of execution. Choosing an order means choosing
which of those two you are willing to give up.

## Words used in this tutorial

- bid: the highest price a buyer is currently offering.
- ask: the lowest price a seller is currently accepting.
- spread: the gap between the ask and the bid, the immediate cost of trading both ways.
- market order: an instruction to trade now at whatever prices are available.
- limit order: an instruction to trade only at a chosen price or better.
- stop order: an instruction that waits, then sends an order once a chosen price is reached.
- order book: the list of all standing buy and sell orders, sorted by price.
- slippage: the difference between the price you expected and the price you actually got.

For the rest of the words used across this collection, see [GLOSSARY.md](../GLOSSARY.md).

## Where this came from

- [Limit Order Book Dynamics](../../strategies/books/03_limit_order_book_dynamics.md), this
  repository's brief on the order book, fill probability and adverse selection of passive fills.
- [Optimal Execution and Liquidation](../../strategies/books/01_execution_and_liquidation.md), the
  brief on how a large order is scheduled and how positions are filled and forced to close.
- [Fill Probabilities in a Limit Order Book](https://arxiv.org/abs/2403.02572), the study of how
  often limit orders fill and how often they are cancelled (`2403.02572v2`).
- [The Negative Drift of a Limit Order Fill](https://arxiv.org/abs/2407.16527), the measurement of
  the small loss that follows a passive fill (`2407.16527v1`).
- [How does latent liquidity get revealed in the limit order book?](https://arxiv.org/abs/1808.09677),
  the study of how the price impact of a large order grows with its size (`1808.09677v2`).
- [The Price Impact of Order Book Events](https://arxiv.org/abs/1011.6402), the study linking
  incoming order imbalance to short-horizon price moves (`1011.6402v3`).
