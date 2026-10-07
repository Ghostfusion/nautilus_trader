# What a market is

Date: 2026-10-07. Revision 1.

This page is for someone who has never bought or sold anything in a market and wants to know what a
market actually is. It explains where a price comes from, who the people on the other side are, and
why the price you see on a screen is not always the price you would get.

## A price is an agreement

When people say "the price of a share", they mean the price at which a share last changed hands.
That price exists only because one person agreed to buy and another agreed to sell at the same
number. Nothing forces those two to agree: the buyer was willing to pay at least that much, the
seller was willing to accept at most that much, and the trade happened somewhere in between.

A price therefore moves for two ordinary reasons. First, a new buyer is willing to pay more than
the last one did, and the next seller will not part with the thing below that higher number.
Second, a seller is willing to accept less than the last one did, and has to undercut other sellers
to find a buyer. There is no central scoreboard that decides a correct price. The price is simply
the number at which the most recent pair agreed. This is why a price can drift for reasons a
stranger would not call important, and also fail to move while the news is dramatic.

## An everyday comparison

Think of a farmers market on a Saturday. At eight in the morning a crate of apples sells for one
price. By noon the same apples from the same tree sell for less, because more sellers have set up
and each of them wants to empty the stall before the crowd goes home. By five in the afternoon the
price sits somewhere in between, because only a few sellers are left and they can afford to wait.
The apples did not change between those trades. What changed was how many people wanted them, how
many people had them, and how long each side was willing to wait. A market in shares behaves the
same way across a trading day.

## Trading through an exchange, and trading directly

An exchange is an organised place, physical or electronic, where buyers and sellers send their
orders and the exchange matches them under a single set of published rules. Most shares, futures
and crypto are traded this way. The advantage is that everyone can see the same prices, and the
exchange stands between the two sides so that neither buyer nor seller has to trust a stranger
personally.

The alternative is a trade agreed directly between two parties, often called over-the-counter (a
trade negotiated privately, off any public exchange). A bank buying a large block of bonds from a
fund, or two companies swapping currencies, may do it this way. Direct trading lets the two sides
choose a size, a date and a price freely, but the prices they agree are often not visible to anyone
else, so there is no public record to compare against.

## Who is on the other side

Every trade has a buyer and a seller, and behind each of them is a person or an institution with a
reason to trade.

- Individual investors: people buying and selling for their own savings, usually in modest amounts.
- Funds: pooled money managed on behalf of many people, such as a pension or an index fund, which
  trade in large amounts and often on a schedule.
- Dealers: firms that stand ready to buy and sell the same thing all day, quoting a price on each
  side and earning the small difference when both sides trade.
- Companies: a firm may buy back its own shares, sell new shares to raise money, or trade currencies
  because it does business abroad.
- Governments: they borrow by selling bonds, and central banks buy and sell to steer interest rates
  and the amount of money in circulation.

A dealer who quotes a buying price and a selling price at the same time is not predicting the future
for fun. The dealer takes the risk of holding the thing for a few seconds or minutes and is paid for
that service.

## Liquidity, in plain words

Liquidity is how much you can trade without moving the price. Something is liquid when many willing
buyers and sellers stand at nearby prices, so a large order can be filled without pushing the price
far. Something is illiquid when few people are interested, and even a modest order moves the price a
long way.

Liquidity is not fixed. It varies by time of day: measured across 50 large US stocks, the available
depth was about twice as thin at the open of the trading day as on average, so a given order moved
the price roughly twice as much at the open and about five times as much into the close
(`1011.6402v3`, p.14). It varies by asset as well: a heavily traded government bond and the shares
of a small company are different worlds, even though both trade on an exchange. And it varies with
events. After one large order goes through, the normal quote sizes are gone for a while before they
return; in a study of an electronic order book, the spread and the depth recovered within about
twenty updates of the best prices after a large trade (`1602.00731v2`).

## The price you see and the price you can trade at

The price on a screen is usually the price at which something last traded. That is history, not an
offer. The price you can actually trade at is whatever the people watching right now are willing to
do. At any moment there is a highest price some buyer is offering and a lowest price some seller is
accepting, and those two numbers are almost never the same. The gap between them is what a trade
costs you immediately, before the price even has a chance to move. A later page in this collection
names those two numbers the bid and the ask and shows how an order turns into a trade.

## The main kinds of market

The five common kinds of market differ in what is actually changing hands.

| Market      | What is actually being traded                                                  |
| ----------- | ------------------------------------------------------------------------------ |
| Shares      | A small ownership stake in a company, which may pay a share of the profits     |
| Bonds       | A loan to a company or government, repaid with interest on agreed dates        |
| Currencies  | One country's money exchanged for another country's money                      |
| Commodities | Physical things such as oil, wheat, gold or copper, or contracts about them    |
| Crypto      | Digital tokens recorded on a shared public ledger, with no company behind them |

A share gives the holder a claim on a business. A bond is a promise to repay a borrowed amount plus
interest. A currency trade is a swap of two monies. A commodity trade is an agreement about a
physical good that is often settled in cash rather than by delivery. A crypto token is a unit
recorded on a distributed ledger, and its price rests only on what other people will pay for it.

## Words used in this tutorial

- market: any setting where buyers and sellers agree prices and exchange things of value.
- exchange: an organised venue, usually electronic, that matches buyers and sellers under published
  rules.
- over-the-counter: a trade negotiated directly between two parties rather than on a public
  exchange.
- price: the number at which one buyer and one seller most recently agreed to trade.
- liquidity: how much can be bought or sold without moving the price much.
- dealer: a firm that quotes a buying price and a selling price all day and earns the difference.
- depth: the amount of buying and selling interest standing at prices near the current one.

For the rest of the words used across this collection, see [GLOSSARY.md](../GLOSSARY.md).

## Where this came from

- [Limit Order Book Dynamics](../../strategies/books/03_limit_order_book_dynamics.md), this
  repository's brief on order books and liquidity, the source of the depth-at-the-open and recovery
  figures.
- [The Price Impact of Order Book Events](https://arxiv.org/abs/1011.6402), the study of 50 US
  stocks that measured how depth changes across the trading day (`1011.6402v3`).
- [Limit-order book resiliency after effective market orders](https://arxiv.org/abs/1602.00731),
  the study of how spread and depth recover after a large trade (`1602.00731v2`).
