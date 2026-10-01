# 01 - What options are

An option is a contract that gives one side the right, but not the obligation, to buy or
sell an underlying asset at a fixed price, on or before a fixed date. The other side
accepts the obligation if the first side chooses to use that right. This is the whole idea.
Everything else in this manual is detail about how that right is priced, measured, and
traded.

Do not worry about the underlying asset yet. It can be a share, a futures contract, a
crypto coin, or an index. The mechanics are the same.

## The four words you need first

- **Call.** A call gives its owner the right to buy the underlying at a fixed price. You
  buy a call when you think the price will rise.
- **Put.** A put gives its owner the right to sell the underlying at a fixed price. You
  buy a put when you think the price will fall, or when you want to protect something you
  already own.
- **Strike.** The strike is the fixed price written into the contract. A call with strike
  100 lets you buy at 100 no matter what the market price is.
- **Expiry.** The expiry is the date and time after which the right disappears. Before
  expiry you can act; after expiry the contract is worth only what an immediate exercise
  would pay.

Two more words appear in every trade.

- **Premium.** The premium is the price you pay to own the option, or the money you
  receive to take on the obligation. It is quoted per unit of the underlying, so one
  contract usually costs the premium times a multiplier.
- **Exercise.** To exercise is to actually use the right: to buy at the strike or sell at
  the strike.

## Vocabulary

| Term             | Plain meaning                                                                              |
| ---------------- | ------------------------------------------------------------------------------------------ |
| Option           | A contract giving one side a right to trade at a fixed price.                              |
| Call             | The right to buy at the strike.                                                            |
| Put              | The right to sell at the strike.                                                           |
| Strike           | The fixed price in the contract.                                                           |
| Expiry           | The date the right ends.                                                                   |
| Premium          | The price of the option itself.                                                            |
| Holder           | The party that owns the right and paid the premium.                                        |
| Writer           | The party that grants the right and receives the premium.                                  |
| In the money     | An immediate exercise would pay something.                                                 |
| Out of the money | An immediate exercise would pay nothing.                                                   |
| At the money     | The strike is close to the current underlying price.                                       |
| Intrinsic value  | What an immediate exercise pays: `max(S - K, 0)` for a call and `max(K - S, 0)` for a put. |
| European         | The right can be used only on the expiry date.                                             |
| American         | The right can be used at any time up to and including the expiry date.                     |

**European exercise, one sentence.** The holder may exercise the option only on the expiry
date, so the option is a single decision made once.

**American exercise, one sentence.** The holder may exercise the option on any trading day
up to and including expiry, so the option carries an extra early-exercise choice.

American does not mean the United States and European does not mean Europe. They are names
for the two exercise styles. The repository declares the style on the instrument; see the
[options concept page](../../concepts/options.md) for that rule and
[the engine view](02-the-engine-view.md) for the code.

## Where the profit comes from

There are only three honest sources of profit with options.

1. **Direction.** You buy a call and the underlying rises, or you buy a put and it falls.
   The option gains more than the premium you paid.
2. **Time and volatility.** You sell an option and collect the premium. If the underlying
   stays calm and the option expires worthless, the premium is your profit. This is the
   business of accepting risk for a fee.
3. **Differences in price across contracts.** Two contracts that should be worth the same
   trade at different prices, and you buy the cheap one and sell the rich one. This is
   called arbitrage and it is what the surface checks in lecture 06 protect.

If someone tells you there is a fourth source, they have not told you where the risk is.

## The main risks

- A **bought** option can lose one hundred percent of its premium if it expires out of the
  money. That is the maximum loss, and it is known in advance.
- A **sold** option can lose far more than the premium received, because the underlying
  price can move a long way against you. The loss is not capped for the seller.
- **Time decay.** An option loses value as expiry approaches, all else equal. This is
  measured by theta; see [06](06-measure-and-evaluate.md).
- **Implied volatility.** The market's expectation of future movement changes, and that
  changes the option price even when the underlying has not moved. This is measured by
  vega.

## Worked example 1: a bought call at expiry

You buy one call on an underlying trading at 100.00. The strike is 100.00, the expiry is
one month away, and the premium is 5.00 per unit. The multiplier is 1 for this example, so
you pay 5.00.

At expiry, let `S` be the underlying price. The call pays `max(S - 100, 0)`. Your profit is
that payoff minus the 5.00 premium.

| S at expiry | Call payoff | Profit |
| ----------- | ----------- | ------ |
| 90.00       | 0.00        | -5.00  |
| 100.00      | 0.00        | -5.00  |
| 105.00      | 5.00        | 0.00   |
| 110.00      | 10.00       | 5.00   |
| 120.00      | 20.00       | 15.00  |

The break-even underlying price is the strike plus the premium: `100 + 5 = 105`. Below 100
the option pays nothing and you lose the whole premium. Between 100 and 105 you exercise,
recover part of the premium, and still lose. Above 105 you profit.

Notice the shape: your loss is capped at 5.00, and your gain grows without limit. That
asymmetry is why people buy options.

## Worked example 2: a sold put at expiry

You sell one put on the same underlying at 100.00 with a premium of 4.00. You receive 4.00
now. The put gives the buyer the right to sell at 100.00, so you have agreed to buy at
100.00 if the buyer chooses. Your payoff at expiry is the negative of the holder's payoff:
`-max(100 - S, 0)`, and your profit adds the 4.00 premium.

| S at expiry | Holder payoff | Your profit |
| ----------- | ------------- | ----------- |
| 90.00       | 10.00         | -6.00       |
| 96.00       | 4.00          | 0.00        |
| 98.00       | 2.00          | 2.00        |
| 100.00      | 0.00          | 4.00        |
| 110.00      | 0.00          | 4.00        |

Your maximum profit is the 4.00 premium, collected when the underlying finishes at or above
100.00. The break-even is `100 - 4 = 96`. Below 96 you lose, and the loss grows as the
underlying falls. If the underlying fell to zero, you would be forced to buy at 100 and lose
almost 100, less the 4.00 premium.

This is the trade that makes many beginners uncomfortable: small, capped gain, and a large
loss if the market moves far. Sellers of options must respect this asymmetry.

## Worked example 3: a bought put as protection

You own one unit of the underlying at 100.00 and you are worried about a fall. You buy a
put with strike 100.00 and premium 3.00. You keep the upside, and the put sets a floor on
your combined value.

At expiry, the combined value of the underlying plus the put is `S + max(100 - S, 0)`. Your
profit is that value minus the 100.00 you paid for the underlying and minus the 3.00
premium.

| S at expiry | Underlying value | Put payoff | Profit |
| ----------- | ---------------- | ---------- | ------ |
| 80.00       | 80.00            | 20.00      | -3.00  |
| 90.00       | 90.00            | 10.00      | -3.00  |
| 100.00      | 100.00           | 0.00       | -3.00  |
| 110.00      | 110.00           | 0.00       | 7.00   |
| 120.00      | 120.00           | 0.00       | 17.00  |

Below the strike your loss is capped at the 3.00 premium, because the put pays back every
unit the underlying falls. Above the strike the put expires worthless and you keep the
upside, minus the premium. This combination is called a protective put. It is the simplest
reason to use options without speculating.

## A short chain, read aloud

A list of options on the same underlying with the same expiry but different strikes is an
option chain. Here is a small one, for an underlying at 100.00 with one month to expiry.
The bid is what a buyer offers, the ask is what a seller asks.

| Strike | Call bid | Call ask | Put bid | Put ask |
| ------ | -------- | -------- | ------- | ------- |
| 90.00  | 12.30    | 13.10    | 1.90    | 2.30    |
| 95.00  | 8.60     | 9.20     | 3.20    | 3.60    |
| 100.00 | 5.40     | 5.90     | 5.30    | 5.80    |
| 105.00 | 2.90     | 3.30     | 8.10    | 8.70    |
| 110.00 | 1.20     | 1.50     | 11.40   | 12.10   |

Read it in two directions. Going down the strikes, calls get cheaper and puts get more
expensive, because a call is worth more the lower its strike. At the same time, the calls
and the puts at one strike carry an implied view about the future that the next lecture
turns into numbers.

## What this manual builds

The manual does not try to make you a market maker of options. It shows how the repository
represents options, how it receives a live chain and its Greeks, how it prices and checks
an American option, and how it builds a volatility surface. The program you run in
[03](03-first-run.md) is a complete option-chain backtest with no network. By the end you
will be able to read a chain, explain the numbers, and know which risks the engine enforces
for you and which it does not.
