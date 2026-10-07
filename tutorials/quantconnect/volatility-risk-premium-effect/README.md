# The volatility risk premium: selling insurance against big price moves

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                         |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Contracts that pay out if a share price moves, sold each month on one company or one index, with a cheaper crash-protection contract bought alongside                                                                                                         |
| How often it trades       | About once a month, when the old contracts expire and a new set is sold                                                                                                                                                                                       |
| What you need             | Nothing but this page to understand it; a spreadsheet to follow the arithmetic                                                                                                                                                                                |
| Where the rules come from | [QuantConnect strategy library, volatility risk premium effect](https://www.quantconnect.com/tutorials/strategy-library/volatility-risk-premium-effect) and the [Quantpedia entry](https://quantpedia.com/strategies/volatility-risk-premium-effect) it cites |
| The underlying research   | Coval and Shumway, [Expected Option Returns](http://papers.ssrn.com/sol3/papers.cfm?abstract_id=189840)                                                                                                                                                       |
| How well it held up       | Mixed: the same direction is reported in many samples, but this is a tail bet whose realised result depends on whether the sample contains a crash, and one of the cited source papers found the opposite sign for the unhedged version of the trade          |
| Also appears in           | Nothing else in this collection                                                                                                                                                                                                                               |

## The idea in one paragraph

An option is a contract that gives its owner the right to buy or sell something at a fixed price
before a fixed date. People who fear a sharp fall are willing to pay for that right, and the person
who sells them the right collects the payment up front and keeps it if the fall never comes. This
strategy sells two such contracts at once, one that pays out if the price rises and one that pays
out if it falls, so it collects money whenever the price ends the month near where it started. It
then spends a small part of that money buying one cheaper contract that pays out only if the price
falls a long way, so a crash does not wipe out the account. The bet is that the money collected for
this protection is, on average, more than the protection turns out to be worth.

## Why anyone believed it

Most people dislike losing money far more than they enjoy gaining the same amount. A pension fund
that promises a fixed income, a company that owes a payment in a foreign currency, and an investor
who would be ruined by a crash all want to be told that a bad outcome cannot happen. They buy that
reassurance, and the seller of an option is the one who provides it. Because the buyers value the
reassurance itself, not only the mathematical chance of a payout, they can be willing to pay more
than the average payout is worth.

The person on the other side of the trade, then, is the worried buyer of protection. A second
explanation, called the peso problem, is that the rare disaster the protection guards against has
not happened yet in the recorded history, so a sample that contains no crash makes the insurance
look needlessly expensive. If the true odds of a crash are higher than the recorded history
suggests, the apparent profit is partly luck rather than a genuine premium.

## An everyday comparison

Think of the extended warranty a shop offers when you buy a phone. The shop knows from its records
roughly how often phones break and how much repairs cost, and it sets the price of the warranty
above that expected cost, because otherwise it would not offer the warranty at all. Most buyers
never claim, and the shop keeps their money; the buyers who do claim get a repair they would
otherwise have paid for. The shop is selling the same thing this strategy sells: a promise to pay
for a bad outcome. The profit, if there is any, is the gap between the price of the promise and what
the promise is actually worth over many customers.

## The rules, step by step

1. Choose one thing to write the contracts on, such as a single large share or a broad index. The
   QuantConnect example uses the options on Google shares; the Quantpedia version uses the index.
2. Take the contracts that expire about one month from today. The QuantConnect filter accepts any
   expiry up to thirty-one days away and then keeps the nearest one, so the position is roughly one
   month long.
3. Find the strike price closest to today's price of the underlying. That strike is the one the
   contracts are written at; a strike exactly at the current price is called at the money.
4. Sell one call and one put at that strike, both expiring in a month. A contract that pays out if
   the price rises is a call; one that pays out if it falls is a put; the pair held together is a
   straddle. Selling the pair means collecting two payments now.
5. With part of that money, buy one put whose strike is fifteen percent below today's price. This is
   the crash protection. It costs much less than the two contracts just sold.
6. Hold everything until the contracts expire. Do not trade in between.
7. At expiry, keep whatever money was collected and settle whatever the contracts require. Then sell
   a fresh set at the new price and repeat, so the cycle is about once a month.

Both sources state the same shape: sell the at-the-money straddle, buy the out-of-the-money put as
insurance, and roll monthly. Quantpedia adds that the leftover cash and the collected payments are
left invested in the index for the month.

## The maths, with every symbol named

The price of the protection is the premium. For the straddle, it is the sum of the two contracts:

```text
Straddle premium = C + P
```

- `C` is the price of the call, the right to buy at the strike.
- `P` is the price of the put, the right to sell at the strike.
- The premium is what the seller collects now and what the buyer pays now.

What the position is worth when the contracts expire depends on where the price of the underlying
ends up. Call that final price `S_T` and the strike `K`:

```text
Value at expiry = premium_received - |S_T - K| + max(K_put - S_T, 0)
```

- `S_T` is the price of the underlying on the expiry day.
- `K` is the strike of the straddle, the price both contracts are written at.
- `|S_T - K|` is the distance between the final price and the strike; the seller of the straddle pays
  this distance, whichever way the price moved.
- `K_put` is the strike of the cheaper protection put, fifteen percent below the starting price.
- `max(K_put - S_T, 0)` is what the protection put pays; it is zero unless the price falls below
  `K_put`. When it pays, it cancels part of the straddle loss.
- `premium_received` is the money collected in step 4 minus the money spent in step 5.

The premium is large enough that the seller does not lose for small moves. The prices at which the
position breaks even are the strike plus or minus the premium:

```text
Upper break-even = K + premium_received
Lower break-even = K - premium_received
```

Between those two prices the month is a gain; outside them it is a loss, and the protection put caps
how large the loss can become on the downside.

The reason a seller might expect to gain over many months is written as the volatility risk premium,
the gap between the movement the market priced and the movement that actually happened:

```text
Volatility risk premium = implied volatility - realised volatility
```

- `implied volatility` is the yearly percentage movement that the option prices imply, read back out
  of the premium.
- `realised volatility` is the yearly percentage movement the underlying actually showed.
- When the first is bigger than the second, the protection was priced for more drama than occurred,
  and the seller of it keeps the difference.

Quantpedia reports that implied volatility on equity index options averages about 19 percent a year
while realised volatility averages about 16 percent, and that selling put options has produced
average returns of 0.5 to 1.5 percent per day in the studies it collects. Those figures are the size
of the gap the sellers are chasing.

## A worked example

The arithmetic below is invented, but the prices are of the size real one-month options take. One
unit is the position written on one unit of the underlying; in practice one exchange contract covers
one hundred units, so the numbers are simply multiplied by one hundred.

Today the underlying costs 100.00. We sell a straddle at strike 100.00 and buy a put at strike 85.00,
fifteen percent below. The middle, or mid, prices are 2.60 for the call and 2.40 for the put, so the
straddle is worth 5.00. Selling at the lower bid price and buying the protection at the higher ask
price, which is how costs enter, the collection is:

```text
Sell call at bid 2.55
Sell put at bid 2.35
Buy 85 put at ask 0.45
Net received = 2.55 + 2.35 - 0.45 = 4.45
```

The break-evens are 100.00 plus or minus 4.45, that is 95.55 and 104.45. Below the 85.00 strike the
protection put pays 85.00 minus the final price, so the loss stops growing. Here is the whole menu of
outcomes at expiry, per unit:

| Price at expiry | Straddle payoff | Protection payoff | Net result |
| --------------- | --------------- | ----------------- | ---------- |
| 70.00           | -30.00          | +15.00            | -10.55     |
| 85.00           | -15.00          | 0.00              | -10.55     |
| 90.00           | -10.00          | 0.00              | -5.55      |
| 95.00           | -5.00           | 0.00              | -0.55      |
| 100.00          | 0.00            | 0.00              | +4.45      |
| 105.00          | -5.00           | 0.00              | -0.55      |
| 110.00          | -10.00          | 0.00              | -5.55      |
| 115.00          | -15.00          | 0.00              | -10.55     |
| 130.00          | -30.00          | 0.00              | -25.55     |

Two things stand out. The largest loss on the downside is 10.55, reached once the price falls to
85.00 and never exceeded, because the protection put pays 15.00 of the 15.00 gap to the strike. On
the upside there is no protection at all: a large rise loses money one for one, which is why the
upside is where the position can be genuinely dangerous.

Now run the cycle for six months. Each month the strike resets to the price at the start of that
month, the protection put sits fifteen percent below, and the net collection is 4.45 percent of the
starting price, already net of the spread. The figures are percentages of the starting price.

| Month | Price move  | Straddle loss | Protection gain | Net result |
| ----- | ----------- | ------------- | --------------- | ---------- |
| 1     | +2 percent  | -2.00         | 0.00            | +2.45      |
| 2     | -4 percent  | -4.00         | 0.00            | +0.45      |
| 3     | +1 percent  | -1.00         | 0.00            | +3.45      |
| 4     | -19 percent | -19.00        | +4.00           | -10.55     |
| 5     | +1 percent  | -1.00         | 0.00            | +3.45      |
| 6     | -1 percent  | -1.00         | 0.00            | +3.45      |
| Total |             |               |                 | +2.70      |

Five quiet months produced small gains and one crash month produced a loss that the protection cut
from 19.00 to 10.55. The six months together gained 2.70 percent of the position, which is under half
a percent a month after costs. That is the shape the research describes: many small gains and
occasional large losses, with the average depending heavily on whether a crash falls inside the
period you happen to measure.

## What the research actually found

The sources do not tell one story, and the disagreement is instructive.

| Source                                                            | What it measured                                                  | Result                                                                                                                                                                  |
| ----------------------------------------------------------------- | ----------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Quantpedia, reporting the put-selling literature                  | Selling at-the-money options short term                           | Put sellers earned 0.5 to 1.5 percent per day in the collected studies, but the same studies record losses as large as 800 percent for put sellers who were not careful |
| Quantpedia, the simple strategy above                             | Monthly straddle sale plus 15 percent put insurance, 1986 to 1995 | 26 percent a year before adjusting for the sample, volatility 19 percent, worst fall 24.07 percent, reward-to-risk 1.16; the period includes the 1987 crash             |
| Coval and Shumway, the source paper                               | Zero-risk at-the-money straddles on American index options        | The straddles produced average losses of about 3 percent per week; the paper concludes that some extra, unpriced factor sits in option returns                          |
| Bondarenko, Why are Put Options So Expensive, cited by Quantpedia | Selling at- and out-of-the-money index puts                       | Puts were priced too high to be consistent with standard models, and simple selling strategies would have earned large profits                                          |
| Israelov and Nielsen, Still Not Cheap, cited by Quantpedia        | Buying protection on ten global equity indices                    | Even when option prices look low, their expected value tends to be lower still, so the buyer of insurance tends to lose over time                                       |

The direction most of these share is that the seller of protection has been paid, on average, more
than the protection delivered. But the magnitude is where the sources part. Quantpedia's 26 percent a
year comes from one construction over 1986 to 1995 and rests on a sample that includes a crash as
well as calm years. Coval and Shumway, the paper Quantpedia itself lists as the source, found the
opposite sign for the unhedged straddle and pointed to an unpriced factor rather than easy profit.
The reconciliation is that the seller's income is real in calm markets and arrives all at once in a
crash; the 800 percent loss line is what an unhedged seller can face, and the protection put in this
strategy exists precisely to cap that.

It is also worth separating what is proved from what is assumed. The papers measure that prices
implied more movement than occurred. That the gap is compensation for risk, rather than a mistake
that competition will remove, is an assumption. Quantpedia reports that the peso-problem explanation
is considered unlikely because a crash large enough to erase the premium would have to occur every
few years.

## How this project relates to it

This repository contains a brief that collects the option-pricing evidence directly. It is
[Options and derivative instruments](../../../strategies/books2/12_options_and_derivatives.md), and
its fourth section records that index straddle positions carried negative returns for the buyer over
415 weekly cycles from 2011 to 2018, while attributing the effect to an unspanned component it calls
dark matter rather than to a simple forecast of volatility. That is the same trade viewed from the
seller's chair: a negative return for the buyer is income for the seller. The brief also notes that a
clean quoted price can hide a carry cost of about 37 basis points, which is the kind of friction that
eats into a strategy that re-quotes every month.

The repository does not implement an option-selling engine. The only strategy tool in it measures
market regimes, and it is described in
[the sector regime engine](../../../implementation/sector-regime-engine/README.md). The closest
related study to the options question is
[Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), which shares
the underlying theme that a repeated premium paid for protection or for a pattern can vanish once
costs and a proper null model are applied.

## Where it goes wrong

- The tail is the whole story. Most months are quiet and one month can undo years. Quantpedia's own
  summary records losses as large as 800 percent for careless put sellers and warns that the return
  distribution is abnormal, so an account that is fully invested has no room for the bad month.
- The loss is not symmetric. The protection put caps the downside, but a large rise in the
  underlying loses money with nothing to offset it, as the 130.00 row of the example shows.
- Margin and the rules of the venue change the outcome. Selling options requires a deposit that
  grows as prices move against you, so a strategy that looks profitable on paper can be forced to
  close at the worst moment.
- The sample decides the answer. A test that ends before a crash finds a steady income; a test that
  includes the crash finds it erased. Two people can run the same rules and report opposite results
  because of where their sample stops.
- Crowding shrinks the gap. When many sellers chase the same premium, the price of protection falls
  and the payment for taking the risk shrinks toward the cost of the risk.
- The two sources trade different things. The QuantConnect page sells options on a single company,
  where one company's news can move the price violently, while Quantpedia's figures are for index
  options, where a single company matters much less. A result measured on one is not a result
  measured on the other.

## Try it yourself

You need only this page and a table you build by hand, because the exercise is about the shape of
the payoff, not about real prices.

1. Write down the payoff table from the worked example by hand, using a strike of 100 and a net
   collection of 4.45.
2. Extend it to prices of 60, 75, 120 and 150, and fill in the three columns for each.
3. Mark on your table the two break-even prices and the point below which the loss stops growing.
4. Now add a column for the result if the protection put had not been bought, and subtract 0.45 from
   the collection to keep the comparison fair.
5. Finally, add a column for a version that also buys a call at a strike of 115 for protection
   against a large rise, priced at 0.60.

What to notice: without the put, a fall to 60 loses 40.00 while with it the loss is 10.55, so the
insurance is doing real work. With a call added, the upside loss is capped too, but the collection
drops again and the quiet months pay less. Every piece of protection you buy makes the good months
smaller to make the bad months survivable, which is exactly why the sellers of protection have been
paid over time.

## Where this came from

- [QuantConnect strategy library: volatility risk premium effect](https://www.quantconnect.com/tutorials/strategy-library/volatility-risk-premium-effect),
  the rules as implemented: one-month at-the-money straddle sold, a fifteen percent out-of-the-money
  put bought, monthly roll.
- [Quantpedia: volatility risk premium effect](https://quantpedia.com/strategies/volatility-risk-premium-effect),
  the performance figures, the instrument count, the sample period and the list of underlying papers.
- Coval and Shumway, [Expected Option Returns](http://papers.ssrn.com/sol3/papers.cfm?abstract_id=189840),
  the source paper, which measured average losses of about 3 percent per week for zero-risk
  straddles.
- [Options and derivative instruments](../../../strategies/books2/12_options_and_derivatives.md),
  this repository's brief on the option-pricing evidence, including the dark-matter result from
  `2303.16371v1` and the parity carry gap from `2604.19604v6`.

## Words used in this tutorial

- call: a contract giving its owner the right to buy at a fixed price before a fixed date.
- implied volatility: the yearly percentage movement that an option's price implies.
- out of the money: a contract whose strike is away from the current price, so it pays only if the
  price moves a long way.
- premium: the price paid now for an option.
- put: a contract giving its owner the right to sell at a fixed price before a fixed date.
- straddle: a call and a put at the same strike and expiry, held together.
- strike: the fixed price written into a contract.
- volatility risk premium: the gap between the movement options imply and the movement that occurs.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
