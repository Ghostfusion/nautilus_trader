# What you can buy and sell

Date: 2026-10-07. Revision 1.

This page is for someone who has never owned anything a market trades and wants a plain description
of each kind of thing that can be bought and sold. It ends with the two words that confuse every
beginner, long and short, and shows how borrowing money multiplies both gains and losses.

## Shares: a slice of a company

A share is a small slice of ownership in a company. If a company has issued one million shares and
you own one of them, you own one millionth of the business. What that slice is worth is not written
anywhere; it is whatever the next buyer and the next seller agree, as the previous page explained.

Owning a share can pay you in two ways. The company may pay out part of its profit as a dividend (a
cash payment per share, decided by the company's board), and the share price may rise if buyers
later value the business more highly. It can also fall, and a company can fail, in which case the
shares can become worth nothing. A share is a claim that ranks behind almost everyone else if the
company is wound up: lenders and employees are paid before shareholders.

## Bonds: a loan you make

A bond is a loan. When you buy a bond, you are lending money to the company or government that
issued it. In return the issuer promises to pay you a fixed rate of interest, usually on fixed
dates, and to repay the original amount on an agreed date in the future.

Because a bond pays a set amount of interest, its price moves in the opposite direction to interest
rates in the wider economy. If new bonds start paying more interest, an old bond paying less looks
less attractive, so its price falls until its effective return matches. A bond is generally safer
than a share of the same company, because the lender is paid before the owners, but a borrower can
still fail to pay.

## Funds and ETFs: a basket in one order

A fund is a pool of many people's money used to buy a collection of shares, bonds or other things
all at once. Instead of buying fifty companies separately, you buy one unit of the fund and own a
slice of the whole basket. A fund run by a manager who picks the holdings is called actively
managed; one that simply copies a published list, such as an index, is called passive.

An exchange-traded fund (ETF) is a fund whose units themselves trade on an exchange like a share,
so you can buy and sell it during the trading day. A plain fund usually trades only once a day, at a
price calculated after the market closes. The attraction of both is instant variety: one order gives
you dozens of holdings, which spreads your risk across many businesses rather than one.

## Futures and options: agreements about a later date

A futures contract is an agreement to trade something at a fixed price on a fixed date in the
future. Both sides are committed: one must buy and the other must sell at that price when the date
arrives, or settle the difference in cash before then. Futures are used by farmers and airlines to
lock in a price, and by speculators who want to bet on the direction without owning the thing.

An option is different because it gives a right, not an obligation. A call option lets its owner buy
something at a fixed price before a set date, and a put option lets its owner sell at a fixed price.
The owner chooses whether to use that right; the seller of the option must act if asked. Because the
owner can walk away, an option costs money up front, and it can expire worthless if the price never
reaches the agreed level. The value of a derivative often depends on where the two sides stand on
collateral (the assets each side must set aside to guarantee the promise), not only on the price of
the underlying thing (`2604.19604v6`).

## Currencies and crypto

Foreign exchange, usually shortened to FX, is the market for swapping one country's money for
another. Every exchange rate is a price like any other: the euro price of the dollar rises when
people want more dollars. FX is the largest market in the world by daily turnover, and almost all of
it happens over-the-counter rather than on a single exchange.

Crypto tokens are digital units recorded on a shared public ledger that many computers keep in
agreement. Some settle payments, some run software, and some are simply a scarce thing people trade.
There is no company or government behind a token, so its price rests entirely on what the next buyer
will pay. Crypto markets trade every day of the week, unlike most share markets, and many of them
are thinly traded, so prices can move a long way on small orders.

## Positions, long and short, and selling what you do not own

Your position is everything you currently hold in one thing, counted net: if you bought two hundred
shares and later sold fifty, your position is one hundred and fifty shares.

Being long means you own the thing and profit if its price rises. That is the ordinary case. Being
short means the opposite: you profit if the price falls. A short sale works by borrowing something
you do not own, from someone who does own it, and selling it straight away. Later you must buy it
back and return it to the lender. If the price has fallen you buy it back for less than you sold it
for and keep the difference. If the price has risen you must buy it back for more, and your loss can
grow without any fixed limit, because a price can rise indefinitely while it can only fall to zero.
Short selling is therefore riskier than it first appears. The costs of borrowing and the risk of
being forced to return the thing early are part of why positions get closed in a hurry, a topic
treated in the execution brief linked at the end of this page.

## Margin and leverage

Margin is money you borrow from your broker to buy more than your own cash would allow, and leverage
is the size of the resulting position measured against your own money. If you put in 1,000 of your
own money and borrow 2,000 to hold 3,000 of shares, your leverage is three to one.

Leverage multiplies the change in the value of the position, because the borrowed amount does not
change while the position does. The table below uses made-up numbers to show the arithmetic, not a
forecast.

| What the price does | Value of the holding | Still owed | Your own money | Change on your own money |
| ------------------- | -------------------- | ---------- | -------------- | ------------------------ |
| Start               | 3,000                | 2,000      | 1,000          | 0 percent                |
| Rises 10 percent    | 3,300                | 2,000      | 1,300          | +30 percent              |
| Falls 10 percent    | 2,700                | 2,000      | 700            | -30 percent              |
| Falls by one third  | 2,000                | 2,000      | 0              | -100 percent             |

The rule is simple to state and easy to forget: a 10 percent move in the price of the holding
becomes a 30 percent move in your own money, up or down, because the whole 300 of gain or loss lands
on the 1,000 you supplied. At three to one, a one-third fall in the price wipes out your entire
contribution. A broker who sees your own money shrinking will demand more cash, and will sell the
holding if it does not arrive. That forced sale is called liquidation, and it is why leveraged
positions can be closed at the worst possible moment. How such selling is scheduled and how it
interacts with the market is the subject of the execution brief linked below.

## Words used in this tutorial

- share: a slice of ownership in a company, which may pay dividends and may rise or fall in value.
- bond: a loan to a company or government that pays interest and is repaid on an agreed date.
- exchange-traded fund: a fund whose units trade on an exchange like a share, giving one order a
  whole basket of holdings.
- futures contract: an agreement to trade something at a fixed price on a fixed future date, binding
  on both sides.
- option: a right, but not an obligation, to buy or sell something at a fixed price before a set
  date.
- long: holding something so that you gain when its price rises.
- short: having sold something you borrowed, so that you gain when its price falls.
- leverage: the size of a position measured against the money you supplied yourself.

For the rest of the words used across this collection, see [GLOSSARY.md](../GLOSSARY.md).

## Where this came from

- [Options and derivative instruments](../../strategies/books2/12_options_and_derivatives.md), this
  repository's brief on futures, options and the collateral that backs them, the source of the point
  that a derivative's value depends on both sides' collateral.
- [Optimal Execution and Liquidation](../../strategies/books/01_execution_and_liquidation.md), the
  brief on how positions are filled and forced to close.
- [The Cost of a Free Lunch: Evidence from U.S. Derivatives Markets](https://arxiv.org/abs/2604.19604),
  the study of the carry cost hidden in derivative prices (`2604.19604v6`).
