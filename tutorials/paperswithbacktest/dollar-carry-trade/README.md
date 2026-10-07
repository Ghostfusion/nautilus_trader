# The dollar carry trade: borrowing where money is cheap and holding dollars where money is dear

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                         |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | The American dollar against a basket of other currencies, through futures contracts, with an interest-rate decision each month                                                                                                                                                                |
| How often it trades       | About once a month, sometimes switching from long dollars to long the foreign basket                                                                                                                                                                                                          |
| What you need             | A spreadsheet and the short-term interest rates of several countries                                                                                                                                                                                                                          |
| Where the rules come from | [The awesome-systematic-trading list](https://github.com/paperswithbacktest/awesome-systematic-trading), which keeps the coded rule, and the [Quantpedia dollar carry entry](https://quantpedia.com/strategies/dollar-carry-trade/) it cites                                                  |
| The underlying research   | Lustig, Roussanov and Verdelhan, [Countercyclical Currency Risk Premia](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1541230)                                                                                                                                                          |
| How well it held up       | Mixed: the underlying carry idea is one of the better-replicated effects in currencies, with an independent historical sample, but this specific rule to time the dollar is one vendor's backtest, it relies on borrowing to multiply the position, and its profits cluster in a few episodes |
| Also appears in           | [Forex carry trade](../../quantconnect/forex-carry-trade/README.md) and [Risk premia in forex markets](../../quantconnect/risk-premia-in-forex-markets/README.md) in this collection                                                                                                          |

## The idea in one paragraph

Every currency has a short-term interest rate, the rate a bank earns for parking money overnight. Some
countries pay almost nothing and some pay a lot. This strategy works out the average short-term rate of a
basket of other countries and compares it with the American short-term rate. If America pays more than the
average, it holds dollars and sells the foreign basket short; if America pays less, it does the reverse.
It rebuilds the decision every month. The bet is that the currency which pays the higher interest tends to
keep its value, so the difference in interest is earned without a matching loss on the exchange rate.

## Why anyone believed it

The income from the interest difference is called the carry, and it is real money that arrives every day.
The question is whether the exchange rate moves against you by more than the carry pays. Economists long
believed that it should: a country with high interest rates should have a currency that is expected to
fall, so that the two effects cancel. In practice the high-rate currencies have fallen less than the
interest difference suggested, which is the puzzle this trade is built on.

The counterparty is anyone who wants or must hold the low-rate currency. A Japanese pension fund that
must hold yen to pay yen pensions, or an exporter paid in the low-rate currency, keeps selling the
high-rate currency and buying the low-rate one, which holds the low-rate currency up. A second
counterparty is the cautious investor who is willing to pay for the comfort of a low-rate currency in a
panic, when money rushes back to the safest one. The trade earns its income by taking the other side of
that comfort.

## An everyday comparison

Imagine two friends. One can borrow money from the bank at one percent a year; the other is saving and the
bank pays them five percent. If the first friend borrows at one percent and lends to the second at five,
they earn four percent a year for standing in the middle. It works perfectly until the second friend cannot
repay, or until the first friend's bank demands its money back early. The dollar carry trade is exactly
this, with whole countries instead of friends, and the dollar as the world's savings account: when the
American rate is high, holding dollars earns the most.

## The rules, step by step

The coded rule uses eight currencies, and the vendor describes ten developed-market currencies. Here is the
rule with eight.

1. Choose a basket of currencies. The code uses the Australian dollar, the British pound, the Canadian
   dollar, the euro, the Japanese yen, the Mexican peso, the New Zealand dollar and the Swiss franc.
2. For each currency, look up its three-month interbank rate, the rate banks charge each other for
   three-month loans. The vendor calls the average of these the average forward discount, and says the
   three-month rate may be used in its place, which is what the code does.
3. Average them with equal weights to get one number for the basket.
4. Look up the three-month American Treasury rate, the rate the American government pays to borrow for
   three months.
5. If the American rate is higher than the basket average, hold the American dollar and sell short the
   whole foreign basket, one equal piece per foreign currency.
6. If the American rate is lower than the basket average, do the opposite: sell the dollar short and hold
   the foreign basket, one equal piece each.
7. Rebuild the decision about once a month, and hold the position in between.
8. Money set aside as a deposit for the contracts is placed in an overnight deposit so that it earns the
   short-term rate.

## The maths, with every symbol named

The whole rule is an average, a subtraction, and a choice of which way round to hold the position.

The basket's average rate, which the vendor calls the average forward discount:

```text
AFD = (r_1 + r_2 + ... + r_N) / N
```

- `AFD` is the average short-term interest rate of the foreign currencies, as a decimal per year: 0.03 means
  three percent.
- `r_i` is the three-month interbank rate of currency `i`.
- `N` is the number of currencies in the basket, eight in the code.

The gap between the American rate and the basket average:

```text
D = r_US - AFD
```

- `r_US` is the three-month American Treasury rate.
- `D` is the difference. A positive `D` means the dollar pays more than the basket on average, which the
  rule treats as a reason to hold dollars.

The position, one equal piece per foreign currency:

```text
if D > 0:  hold dollars, and put w_i = -1/N on each foreign currency
if D < 0:  short dollars, and put w_i = +1/N on each foreign currency
```

- `w_i` is the fraction of the account placed in foreign currency `i`.
- A negative weight means the foreign currency is sold short, which is the same as being long dollars
  against it.

The income the position earns, before any exchange-rate move, is the interest difference on the amount
held. For the long-dollar case it is close to `D` per year on the money at risk:

```text
Carry = D (when D > 0)
```

- `Carry` is the interest income over a year, as a fraction of the amount held, if the exchange rates do
  not move.
- The full return is this carry plus the change in the exchange rates of the basket. When the dollar is
  held and the basket falls, that adds to the carry; when the basket rises, it can swallow the carry.

The cost of trading is the traded amount times the cost per trade:

```text
Cost = t * c
```

- `t` is the amount traded, counted on both sides.
- `c` is the cost per trade, about 0.0002 to 0.0005 for liquid futures, that is two to five basis points,
  where one basis point is one hundredth of one percent.

Because the rule can flip from long the basket to short the basket in one month, the traded amount can be
as large as two, meaning the whole account is sold and rebuilt in the opposite direction.

## A worked example

Four currencies, to keep the arithmetic short; the code uses eight. The figures are invented but of the
size these rates take.

| Currency          | Three-month rate |
| ----------------- | ---------------- |
| Australian dollar | 0.0450           |
| Euro              | 0.0200           |
| Japanese yen      | -0.0010          |
| Swiss franc       | -0.0075          |

Average of the four:

```text
AFD = (0.0450 + 0.0200 + (-0.0010) + (-0.0075)) / 4 = 0.0565 / 4 = 0.014125
```

The three-month American rate is 0.0300, higher than the basket average of 0.0141. So `D = 0.0300 - 0.0141
= 0.0159`, which is positive, and the rule holds dollars and sells short each foreign currency with a
weight of -0.25.

Now suppose the next month the foreign currencies move as follows against the dollar (positive means the
foreign currency strengthened, the dollar weakened):

| Currency          | Weight  | Next-month move | Contribution    |
| ----------------- | ------- | --------------- | --------------- |
| Australian dollar | -0.25   | -1.0 percent    | +0.2500 percent |
| Euro              | -0.25   | +0.5 percent    | -0.1250 percent |
| Japanese yen      | -0.25   | -0.8 percent    | +0.2000 percent |
| Swiss franc       | -0.25   | +0.2 percent    | -0.0500 percent |
| Total             | -1.0000 |                 | +0.2750 percent |

The exchange-rate part of the return is +0.2750 percent. The carry for one month is the annual difference
divided by twelve:

```text
Carry for the month = D / 12 = 0.0159 / 12 = 0.001325, that is 0.1325 percent
```

Suppose nothing changed from the previous month except one position, so only a quarter of the account was
traded, on both sides:

```text
t = 2 * 0.25 = 0.50
Cost = 0.50 * 0.0003 = 0.00015, that is 0.015 percent
Total for the month = 0.2750 + 0.1325 - 0.015 = 0.3925 percent
```

That is one invented month. It shows how to apply the rules and how the arithmetic behaves, and it says
nothing about whether the idea works. Notice how the carry, less than a fifth of a percent a month, is the
small part, and the exchange-rate move is the large and unpredictable part.

## What the research actually found

| Source                                                                              | What it measured                                                              | Result                                                                                                                                                                                                                                                       |
| ----------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Lustig, Roussanov and Verdelhan, Countercyclical Currency Risk Premia               | The dollar carry trade against a basket of developed currencies, 1983 to 2009 | The trade delivered large excess returns that were uncorrelated with the usual carry trade; the average forward discount and American industrial production growth together forecast up to 25 percent of the dollar's return variation at a one-year horizon |
| The same paper                                                                      | The reason the returns exist                                                  | They compensate an American investor for the risk of holding a short-dollar position in bad times, when the price of risk in the economy is high                                                                                                             |
| Quantpedia's summary of the source paper                                            | The same rule, 1983 to 2009                                                   | 5.6 percent a year, volatility 8.53 percent, worst fall 31.72 percent, reward-to-risk 0.66; the vendor grades its confidence in the idea as Strong and notes the return could be multiplied with borrowing                                                   |
| Accominotti and Chambers, Out-of-Sample Evidence on the Returns to Currency Trading | Currencies in the 1920s and 1930s                                             | Carry returns from the modern period were also present in this earlier, independent sample and survived the transaction costs of the time                                                                                                                    |
| The awesome-systematic-trading list's replication record, across all its papers     | 4,843 coded strategies                                                        | The median replication returned a Sharpe ratio of 0.37, and only 48 percent cleared a t-statistic of 1.96, so half the published record cannot be distinguished from zero on its own sample                                                                  |

The carry idea as a whole is one of the better-replicated effects in currencies, and the historical sample
from the 1920s is genuine out-of-sample support for its existence. What is less well supported is this
particular way of timing the dollar month by month against the basket: that is one vendor's measurement,
over one window, with the position borrowed up.

## How this project relates to it

This repository's brief on rates and currencies,
[Macro, rates and foreign exchange](../../../strategies/books2/07_macro_rates_and_fx.md), collects the
evidence on how interest rates and the dollar move together, including the finding that roughly 30 percent
of central-bank announcements shift the whole term structure at once, which is the rate risk a carry
position is exposed to. Its Section 3 is the closest material to the interest-rate side of this trade.

The finished tutorials that describe the same family are
[Forex carry trade](../../quantconnect/forex-carry-trade/README.md), which holds the higher-rate currency
against the lower-rate one directly rather than timing the dollar, and
[Risk premia in forex markets](../../quantconnect/risk-premia-in-forex-markets/README.md), which places
carry among the currency styles that researchers have named.

## Where it goes wrong

- Carry crashes. The income arrives steadily and the losses arrive all at once. When a panic comes, money
  rushes into the safest currency and the currencies sold short can jump by ten percent in days, wiping out
  years of carry. The worst fall of 31.72 percent in the vendor's table is one such episode.
- It is leveraged. The reward-to-risk figure of 0.66 is measured before any borrowing. The vendor is
  explicit that the return "could be easily leveraged", which also means the losses are leveraged, and a
  borrowed position can be forced closed at the worst moment.
- It is really a bet on the dollar's mood. The rule says nothing about whether the gap between the
  American rate and the basket average is large or small; it acts on the sign alone. A one-hundredth of a
  percent difference and a four percent difference produce the same position.
- The measure is a stand-in. The vendor uses the average forward discount, a measure built from the prices
  of forward contracts, and says a plain average of three-month rates may be used instead. The coded rule
  does the simpler version, which ignores that the rates differ for reasons beyond the currency's outlook.
- The basket is not the vendor's. The code includes the Mexican peso, which the vendor's description of
  the developed-market basket does not, and uses a Swiss money-market rate from a national source. A
  different basket can give a different answer about which way the trade should lean.
- For the whole idea to be false, it is enough that the interest difference is simply payment for a risk
  that shows up rarely, so that the median outcome looks good while the tail outcome is ruinous. That is
  the reading several researchers give, and it is consistent with everything in the table above.

## Try it yourself

You need a spreadsheet and a public source of short-term interest rates. Any finance website or a central
bank's data page will give you three-month rates for several countries.

1. Build a sheet with one column per currency and one row per month for the last ten years, holding that
   currency's three-month rate at the start of the month.
2. Add a column for the basket average, the equal-weighted mean of the currencies' rates.
3. Add a column for the three-month American rate on the same date.
4. Add a column that is the American rate minus the basket average. A positive number says hold dollars.
5. In the next row down, write the next month's move of each foreign currency against the dollar, and work
   out the average. If the sign from step 4 is positive, the trade earns the negative of that average plus
   one twelfth of the difference; if negative, the opposite.
6. Add the two pieces and subtract about 0.03 percent for the trading cost when the direction flips.

What to notice: the carry piece is small and steady, and the exchange-rate piece is large and jumpy. Look
at the worst month in the whole history and check how many months of carry it erased. That ratio, not the
average return, is what this trade is really about. If your sheet shows the trade losing badly in 2008 or
in a similar episode, you are seeing the reason the reward for taking the other side of everyone's comfort
is positive at all.

## Where this came from

- [Dollar Carry Trade](https://quantpedia.com/strategies/dollar-carry-trade/), the page that states the
  rules and reports the source-paper figures and the confidence grade.
- Lustig, Roussanov and Verdelhan,
  [Countercyclical Currency Risk Premia](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1541230), the
  study that named the dollar carry trade and measured its predictability.
- Accominotti and Chambers,
  [Out-of-Sample Evidence on the Returns to Currency Trading](https://research.mbs.ac.uk/accounting-finance/Portals/0/docs/Out-of-Sample%20Evidence%20on%20the%20Returns.pdf),
  the study of carry, momentum and value in the 1920s and 1930s.
- [The awesome-systematic-trading list](https://github.com/paperswithbacktest/awesome-systematic-trading),
  which holds the coded rule and the project's own replication record.
- [Macro, rates and foreign exchange](../../../strategies/books2/07_macro_rates_and_fx.md), this
  repository's brief on interest rates, the dollar and central-bank announcements.

## Words used in this tutorial

- basis point: one hundredth of one percent, so five basis points is 0.05 percent.
- carry: the return from holding a higher-interest currency and funding it with a lower-interest one.
- forward discount: the amount by which the price agreed today for a future currency exchange sits below
  the current price, which reflects the interest difference between the two currencies.
- leverage: borrowing money to hold a larger position than your own money would allow.
- reserve currency: a currency that many governments and companies hold and use for trade, today mainly
  the American dollar.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- volatility: how much a price moves around its average, measured as a percentage per year.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
