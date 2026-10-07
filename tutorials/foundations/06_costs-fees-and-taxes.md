# Costs, fees and taxes: what a trade costs before it can make anything

Date: 2026-10-07. Revision 1.

Every trade costs something, and a strategy only keeps what is left after those costs. This page
names each cost in plain words, then works a small example in which a strategy that looks good
before costs ends the year with nothing.

## The costs, one at a time

Buying and selling is not free, and each cost has a name. The six below are the ones a beginner meets
first. Five are paid on each trade; the last is paid only when a gain is kept.

- The spread. Every asset has two prices at the same moment: a slightly higher one at which you can
  buy, and a slightly lower one at which you can sell. The gap between them is the spread. You buy at
  the higher price and sell at the lower one, so one round trip (one buy and the matching sell) loses
  the whole gap, no matter which way the price moved in between.
- Commission. A broker (the company that places your orders for you) charges a fee for the service.
  It is usually a small percentage of the order's value, and it is charged on each side, so a round
  trip pays it twice.
- Borrow fee. To short sell (to sell something you do not own, planning to buy it back later at a
  lower price), you must first borrow it from someone. The lender charges rent, called the borrow
  fee, which is a percentage of the borrowed value for as long as the short position (the position
  that loses money when the price rises) stays open.
- Funding payment. A perpetual future is a contract that tracks the price of an asset without ever
  expiring. To stop its price drifting away from the asset's price, the venue moves a small payment
  between the two sides at fixed intervals. When the contract trades above the asset, the buyers pay
  the sellers; when below, the sellers pay the buyers. Anyone holding the contract for a while pays
  or receives this funding.
- Market impact. Your own order changes the price. If you buy a large amount, your buying pushes the
  price up as it is filled, so the average price you pay is worse than the price on the screen when
  you started. This cost grows with the size of the order.
- Taxes. Where a gain is a profit you have realised by selling, the government takes a share of it.
  In many places a gain kept for less than a year is taxed at a higher rate than one kept longer, so
  the tax depends on how often you trade.

The first five are the costs a backtest usually forgets; the sixth is real too. The rest of this page
concentrates on the two that arrive on every single trade, the spread and the commission, because
they are the ones you can measure most easily.

## A worked example: twenty trades on a 10,000 unit account

Take a made-up account holding 10,000 units of currency, and a made-up strategy that trades about
twenty times a year. Each time it trades, it buys or sells the whole account, so each round trip
turns over 10,000 units. The assumptions are a spread of 0.1 percent and a commission of 0.05
percent per side. Both numbers are small, and both are plausible for a liquid asset.

The cost of a single round trip is built up line by line.

| Step                   | Calculation           | Cost  |
| ---------------------- | --------------------- | ----- |
| Spread                 | 10,000 x 0.1 percent  | 10.00 |
| Commission on the buy  | 10,000 x 0.05 percent | 5.00  |
| Commission on the sell | 10,000 x 0.05 percent | 5.00  |
| One round trip         | 10.00 + 5.00 + 5.00   | 20.00 |

One round trip costs 20 units, which is 0.2 percent of the account. Twenty of them cost twenty times
that.

| Quantity                         | Calculation  | Cost        |
| -------------------------------- | ------------ | ----------- |
| One round trip                   | see above    | 20.00       |
| Twenty round trips               | 20 x 20.00   | 400.00      |
| As a share of the 10,000 account | 400 / 10,000 | 4.0 percent |

So the strategy must earn more than 4 percent before costs simply to break even. Now give it a year
that would look good on its own: four percent gross, meaning four percent before any cost is
subtracted. Spread the twenty trades over four quarters.

| Quarter | Trades | Gross gain before costs | Cost                | Net after costs |
| ------- | ------ | ----------------------- | ------------------- | --------------- |
| First   | 5      | +120                    | -100                | +20             |
| Second  | 5      | +80                     | -100                | -20             |
| Third   | 5      | +140                    | -100                | +40             |
| Fourth  | 5      | +60                     | -100                | -40             |
| Year    | 20     | +400 (+4.0 percent)     | -400 (-4.0 percent) | 0 (0.0 percent) |

The gross column sums to +400, a four percent year that a beginner would be pleased with. The cost
column sums to exactly the same amount, because 4 percent is what twenty round trips cost at 0.2
percent each. The net after costs is zero. A number that looked like an edge was entirely a fee paid
to other people.

## Why trading more often makes it worse

The cost per round trip is fixed, so the yearly cost is proportional to the number of trades: twice
the trades, twice the cost. A strategy that trades forty times a year, not twenty, pays 40 x 20 = 800
units, or 8 percent of a 10,000 account. A strategy that trades every single day, around 250 round
trips a year, pays 250 x 20 = 5,000 units, which is 50 percent of the account. An edge would have to
be very large to survive a fee that large.

The trap is that the gross gain per trade usually does not grow with the number of trades. Splitting
a yearly move into many small trades does not create extra return; it only multiplies the fixed cost.
This is why a strategy's net result is far more sensitive to how often it trades than to how clever
its rules look. Another way to see it is through the break-even edge: a strategy trading twice a year
must clear 0.2 percent to pay for itself, while the daily one must clear 50 percent. A required edge
that large is a warning sign: the daily strategy is not a smaller version of the twice-a-year one; it
is a different and much harder problem. Market impact makes the picture worse still: frequent large
orders push the price against themselves, as the market impact brief in this repository explains
([strategies/books/02_market_impact_and_trading_cost.md](../../strategies/books/02_market_impact_and_trading_cost.md)).

## Market impact, in numbers

Market impact is the cost that grows with order size. A small order barely moves the price; a large
one is filled against a thinning book of resting orders and pays progressively worse prices. Research
on real trades finds that the average impact of a large order grows roughly with the square root of
the order's size relative to normal daily volume, not in direct proportion to it. One survey of 2,299
stock-datapoints on the Tokyo Stock Exchange measured that exponent at 0.489 (arXiv `2411.13965v3`).
In plain words the relationship is curved: doubling an order's size increases its impact by less than
double, but the impact never disappears, and it adds to the spread and commission rather than
replacing them. A beginner's cost model built only from the spread and the commission therefore
understates the true cost of any order large enough to be noticed. This repository's market impact
brief explains the measurement and its limits in detail
([strategies/books/02_market_impact_and_trading_cost.md](../../strategies/books/02_market_impact_and_trading_cost.md)).

## Gross versus net, and the duty to say which

Two words separate a figure that ignores costs from one that pays them.

- Gross means before costs, the raw gain or loss from price moves alone.
- Net means after costs, what actually reached the account once the spread, commission, borrow fee,
  funding, impact and tax were subtracted.

A gross figure is not wrong on its own, but it is incomplete, and it is the number that makes weak
strategies look strong. The only honest way to state a result is to name which of the two it is and
to list the costs that were included. A claim such as "this made four percent a year" is a defect
unless it says gross or net and says what each cost was assumed to be. When in doubt, ask for the net
number and the list of costs, and treat a missing list as a warning.

Fee schedules themselves are not fixed; venues and regulators change them, and those changes move the
net result without the strategy changing at all. This repository's brief on market design and fees
covers how rule and fee changes reroute activity
([strategies/books2/20_market_design_regulation_and_fees.md](../../strategies/books2/20_market_design_regulation_and_fees.md)).

## Words used in this tutorial

- Spread: the gap between the price at which you can buy and the price at which you can sell at the
  same moment.
- Commission: the fee a broker charges for placing an order, usually a small percentage of its value
  and charged on each side.
- Short sell: to sell something you do not own, borrowing it first, in the hope of buying it back
  later at a lower price.
- Perpetual future: a contract that tracks an asset's price with no expiry date, kept near that price
  by periodic funding payments.
- Funding payment: the periodic transfer between the buyers and sellers of a perpetual future that
  keeps its price aligned with the asset.
- Market impact: the tendency of your own order to move the price against you as it is filled.
- Gross: a result measured before any costs are subtracted.
- Net: a result measured after all costs are subtracted.

## Where this came from

- [GLOSSARY.md](../GLOSSARY.md), the shared list of trading terms used across this collection.
- [strategies/books/02_market_impact_and_trading_cost.md](../../strategies/books/02_market_impact_and_trading_cost.md),
  this repository's brief on how order size moves prices and what that costs.
- [strategies/books2/20_market_design_regulation_and_fees.md](../../strategies/books2/20_market_design_regulation_and_fees.md),
  this repository's brief on how venue and regulatory fee rules change what trading costs.
- The worked example and its 0.1 percent spread and 0.05 percent commission are made up for teaching.
  They are not measurements, and the arithmetic is the only claim being made.
