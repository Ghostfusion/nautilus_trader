# News sentiment: trading medicine makers on the tone of their press releases

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                             |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of American drug manufacturers, bought in the morning and sold at the close on the same day                                                                                                                                                |
| How often it trades       | About once a week, on Wednesdays                                                                                                                                                                                                                  |
| What you need             | Python and a data file of press releases with their publication times                                                                                                                                                                             |
| Where the rules come from | [QuantConnect strategy library: using news sentiment to predict the price direction of drug manufacturers](https://www.quantconnect.com/tutorials/strategy-library/using-news-sentiment-to-predict-price-direction-of-drug-manufacturers)         |
| The underlying research   | Shah, Isah and Zulkernine, [Predicting the Effects of News Sentiments on the Stock Market](https://arxiv.org/abs/1812.04199), and Berument and Kiymaz, [The Day of the Week Effect on Stock Market Volatility](https://ssrn.com/abstract=3211399) |
| How well it held up       | Weak: the 70 percent directional accuracy is one small study of Indian shares, and the library's own version on American shares lost money before a day-of-week filter was added, and still fell short of the index                               |
| Also appears in           | nothing else in this collection                                                                                                                                                                                                                   |

## The idea in one paragraph

Drug makers live and die by announcements: a trial either works or it does not, and the company
says so in a press release. Researchers built a hand-made list of words and phrases that appear in
this kind of news, each with a plus or minus score, and used it to add up the tone of everything a
company publishes. The bet is that when the tone of the night's news is positive, the shares drift
up during the next trading day, and when it is negative they drift down. The strategy buys after
such a morning when the tone is positive, sells short when it is negative, and closes the position
at the close of the same day. It only trades on Wednesdays, because that day of the week has
historically been a little stronger than the others. It is a bet on a few hours of drift, not on
whether the drug works.

## Why anyone believed it

A small biotechnology company's whole value can rest on one drug in one trial. When the result is
announced, the news is public immediately, but the market's reaction is not instant. The people who
read press releases for a living are few, and they cannot all act in the same second. Analysts have
to write notes, fund managers have to get approval, and ordinary investors read the news at
breakfast or in the middle of the day. Each of those readers moves the price a little, so the
initial jump is followed by a slow drift in the same direction through the day.

The counterparty, then, is whoever supplies the shares to the buyer. Some of them are index funds
and other accounts that must hold a fixed basket regardless of the news, so they sell when the
price rises. Others are holders who sell to take a profit or to spread their risk, or who simply
have not read the release yet. If those sellers keep appearing through the day, an early buyer can
collect the drift.

## An everyday comparison

A newspaper runs a restaurant review over breakfast, and the restaurant gets a queue at lunch. The
queue does not appear all at once. One reader tells a colleague, a few more walk past and see the
crowd, and by one o'clock the wait is an hour. A diner who arrives at half past eleven hears the
same review as everyone else but gets a table before the crowd forms. The strategy here is the
early diner: it does not know more than the market, it only acts early in the day, on the theory
that the crowd arrives slowly.

## The rules, step by step

1. Pick the companies to watch. Restrict the list to drug manufacturers, which the data vendor
   Morningstar groups under that one label. Among them, keep the ones with the greatest dollar
   volume, meaning the total value of shares traded each day. Among those, keep the ones with the
   highest price-to-earnings ratio, which is the share price divided by the profit per share.
2. Watch the news feed for those companies only, from the previous close until the present moment.
   A close is the moment the market stops trading for the day.
3. Score each press release as described in the maths section: lower-case the text, slide a window
   of one to a fixed number of consecutive words across it, look each window up in a dictionary that
   gives a number to a phrase, and add those numbers.
4. Add each company's article scores together into a running total for that company. Start the total
   at zero at each close, so the total always refers to the news since the last close.
5. About half an hour after the market opens, look at the total. If it is above zero, buy the
   shares. If it is below zero, sell short, which means borrowing shares, selling them now, and
   buying them back later, so that a fall in the price is a gain. If it is exactly zero, do nothing.
6. Only act on a Wednesday. On every other day, note the news and do nothing.
7. Close every position at the market close on the same day, so nothing is held overnight.
8. Put the same amount of money into each position, and do not borrow more than the account holds.

One important detail: rules 1 and 2 must use only information that was public and time-stamped
before the moment you would trade. A press release that appeared after the market closed cannot
change a position taken earlier that morning.

## The maths, with every symbol named

The sentiment of one article is the sum of the scores of the phrases it contains:

```text
S = sum of value(p) over every phrase p found in the article
```

- `S` is the article's tone score, a plain number that can be positive, negative or zero.
- `value(p)` is the number the dictionary gives to phrase `p`: for example +3 for "meets primary
  endpoint", +2 for "approval", +1 for "positive", -2 for "discontinued", -3 for "halted".
- A phrase is one word or a short run of consecutive words. Sliding a window of one to four words
  over the text produces every candidate phrase, and only those present in the dictionary count.

The company's running total is the sum of its articles since the last close:

```text
C = S_1 + S_2 + ... + S_k
```

- `C` is the cumulative tone for the company, over the `k` articles published since the last close.
- `S_1` to `S_k` are the tone scores of those articles, each computed as above.

The direction of the trade is the sign of the total:

```text
d = sign(C):  d = +1 if C > 0,  d = -1 if C < 0,  d = 0 if C = 0
```

- `d` is the intended direction: +1 means buy, -1 means sell short, 0 means stand aside.
- The whole dictionary, window length and threshold are choices made in advance, not judged per
  article.

The return of the position, from the entry price to the close, is:

```text
Long:   r = P_close / P_entry - 1
Short:  r = P_entry / P_close - 1
```

- `P_entry` is the price paid or received when the position is opened, about half an hour after the
  open.
- `P_close` is the price at the close, when the position is closed.
- For a short, the formula reverses, because the position profits when the second price is lower.

Finally the cost. A short round trip involves selling borrowed shares and buying them back, so two
trades pay the gap between the buying and selling price (the spread) and any commission:

```text
Cost = t * c
```

- `t` is the number of sides traded: 2 when a position is opened and closed the same day.
- `c` is the cost of one side as a fraction of the amount traded. For a liquid large drug maker a
  realistic figure is 0.0005 to 0.001, that is five to ten basis points, where one basis point is
  one hundredth of one percent.

The day-of-week rule adds no formula of its own; it is a filter that says "apply all of the above
only when the calendar day is Wednesday".

## A worked example

Two weeks of a made-up but plausible drug maker, using the phrase scores above. The company's total
starts at zero at each close, so the "total" column is only the news since the previous close. The
dictionary scores "meets primary endpoint" as +3, "highly significant" as +1, "approval" as +3,
"positive" as +1, "fails" as -3, "halted" as -3, "discontinued" as -2, "did not meet" as -3 and
"announces conference participation" as 0.

| Day       | News since the last close                                   | Total | Action at 10:00     | Entry price | Close price | Gross return  | Cost         | Net return    |
| --------- | ----------------------------------------------------------- | ----- | ------------------- | ----------- | ----------- | ------------- | ------------ | ------------- |
| Monday    | "trial fails ... primary endpoint" (-3) and "halted" (-3)   | -6    | none, not Wednesday |             |             |               |              |               |
| Tuesday   | "announces conference participation" (0)                    | 0     | none, not Wednesday |             |             |               |              |               |
| Wednesday | "FDA grants approval" (+3) and "positive" (+1)              | +4    | buy                 | 25.00       | 25.40       | +1.60 percent | 0.20 percent | +1.40 percent |
| Thursday  | "discontinued" (-2) and "did not meet" (-3)                 | -5    | none, not Wednesday |             |             |               |              |               |
| Monday    | "meets primary endpoint" (+3) and "highly significant" (+1) | +4    | none, not Wednesday |             |             |               |              |               |
| Tuesday   | no news                                                     | 0     | none, not Wednesday |             |             |               |              |               |
| Wednesday | "fails" (-3) and "no differentiation from placebo" (-3)     | -6    | sell short          | 40.00       | 39.40       | +1.50 percent | 0.20 percent | +1.30 percent |
| Thursday  | "approval" (+3)                                             | +3    | none, not Wednesday |             |             |               |              |               |
| Monday    | no news                                                     | 0     | none, not Wednesday |             |             |               |              |               |
| Wednesday | "positive" (+1) and "encouraging" (+1)                      | +2    | buy                 | 30.00       | 29.70       | -1.00 percent | 0.20 percent | -1.20 percent |

The arithmetic for the first trade: the buy at 25.00 and the close at 25.40 give a gross return of
25.40 divided by 25.00, minus one, which is 0.016, or 1.60 percent. The cost is two sides at ten
basis points, `2 * 0.001 = 0.002`, or 0.20 percent, so the net is 1.40 percent. The second trade is
a short: selling at 40.00 and buying back at 39.40 gains 0.60 on 40.00, which is 1.50 percent, less
0.20 percent, for 1.30 percent net. The third trade is a long that falls, so the net is minus 1.20
percent.

Over the ten days there were three trades. The average net return per trade is
`(1.40 + 1.30 - 1.20) / 3 = 0.50` percent, and the three trades together returned 1.50 percent
before any cost of borrowing shares to sell short. Two things are worth noticing. Only one day in
five produces a trade, which is the whole point of the Wednesday filter. And the cost line is a
fixed 0.20 percent on every trade, so a signal whose edge is a few tenths of a percent is decided
by costs rather than by the news.

## What the research actually found

The honest summary is that the tone of news clearly contains information, that the size of the
prize is uncertain, and that the same headline can be scored differently by two systems, which is a
measurement problem before it is a money problem.

| Source                                               | What it measured                                                                                                                        | Result                                                                                                                                                                                                                                                                                               |
| ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Shah, Isah and Zulkernine, arXiv `1812.04199`        | A hand-built dictionary of financial and pharmaceutical phrases, applied to news about several hand-picked Indian pharmaceutical stocks | A directional accuracy of 70.59 percent in predicting short-term price moves, using only news sentiment. The paper does not report costs, and the sample is small and single-market                                                                                                                  |
| Berument and Kiymaz, 2001                            | Daily movements of an American stock index                                                                                              | Volatility and average return are not the same on every weekday; the library uses this to restrict trading to Wednesday                                                                                                                                                                              |
| QuantConnect's own implementation                    | American drug manufacturers, intraday, 2020 sample including the market crash                                                           | Only after restricting to Wednesday did the strategy make money, and its Sharpe ratio was -1.619 against -0.579 for simply holding the index fund SPY. The authors attribute the shortfall to commissions and the spread                                                                             |
| Budennyy and others, arXiv `2208.07248v2` (p.6)      | 5,436 clinical trial announcements from 681 pharmaceutical companies                                                                    | Negative announcements move prices sharply, but the positive ones are statistically indistinguishable from quiet days: a rank test against non-announcement periods gives p = 0.34 for positive news and p = 2 times 10 to the power -13 for negative news. Good news does not reliably lift a price |
| Aparicio and others, arXiv `2401.11011v1` (p.5)      | FinBERT, a finance-tuned language model, labelling 105 biotech press releases                                                           | 66.86 percent of the releases were labelled with the correct sign. For small and micro companies a blunt rule, entering when the day's open-to-close move exceeded five percent, beat the language-model strategy                                                                                    |
| Glasserman and Lin, arXiv `2309.17322v1` (p.9)       | The same news headlines, once as written and once with the company name replaced by a random string                                     | The anonymised headlines earned 30.97 basis points per day against 25.08 for the originals in the training window, a difference of about 5.9 basis points per day. Removing the company name changed how the same words were scored                                                                  |
| Boustanifar and Mansouri, arXiv `2609.31013v1` (p.4) | Seven language models from seven companies scoring 1,946 earnings calls on identical instructions                                       | The average agreement between two providers was a rank correlation of only 0.52. About 34.4 percent of the score variation came from the transcript and 33.4 percent from which provider read it                                                                                                     |
| Mavillonio and others, arXiv `2607.13968v1` (p.23)   | Transformer language models against dictionary methods on a human-labelled set of 588 articles                                          | The transformer measures reached a macro-F1 of 0.67 to 0.69 against 0.187 for the best dictionary method, so context-aware scoring tracks human judgments better than a word list                                                                                                                    |

Two conclusions follow from reading these together. First, the direction of a price reaction is
genuinely predictable from text in several studies, but the effect is small and most of it is in the
negative news, where the reaction is sharp. Second, the number a scoring system produces is a
property of the system as much as of the text: change the provider, anonymise the company name, or
swap a dictionary for a transformer, and the same headline moves.

## How this project relates to it

This repository keeps a brief on exactly the questions this tutorial raises: how text is turned into
numbers, why two models disagree, and what the disagreement means for a backtest. It is
[strategies/books2/11_language_models_news_and_text.md](../../../strategies/books2/11_language_models_news_and_text.md).
Its first listed finding is the cross-provider result above, a mean rank correlation of 0.52, and
its practical advice is to store the provider, the prompt and the score's spread alongside any
text-derived number, rather than a bare score.

The companion brief,
[strategies/books/14_news_and_language_model_signals.md](../../../strategies/books/14_news_and_language_model_signals.md),
covers the trading side. Its central warning is that every strong headline number in this literature
is measured before costs, and it records a strategy whose Sharpe ratio of 3.41 at zero cost fell to
2.0 once trades cost five basis points. That is the same cost line the worked example above charges
on every trade.

## Where it goes wrong

- Look-ahead with publication timestamps. A daily return is usually stored as the move from one
  close to the next, but a press release arrives at a particular minute. If the scoring step can see
  an article published after 4 p.m., the backtest trades on information the market did not have.
  Getting this right means storing the exact publication time and refusing to let a position see any
  article published later.
- The same headline scores differently in two systems. A dictionary reads words, a language model
  reads context, and a general model also brings outside knowledge of the company. In `2309.17322v1`
  the headline about Pfizer buying Allergan was scored as bad news for Allergan with its name in
  place and good news with the name removed. A signal that changes this much with the wording of the
  scorer is not a property of the news.
- Word lists ignore grammar. "Failed to avoid a loss" contains "failed", and "no evidence of harm"
  contains "harm", yet the meanings are close to the opposite of what the individual words suggest.
  Negation, hedging and clinical jargon are exactly where dictionaries are weakest.
- Costs and the intraday spread. This strategy enters and exits on the same day, so it pays the gap
  between buying and selling prices twice for every trade, plus any commission. The library's own
  result was negative until a filter was added, and its authors blamed commissions and spread.
- Crowding and decay. Once a text signal is published and cheap to compute, more money acts on it
  earlier, the drift shrinks, and the people acting late pay for it. The measurement literature above
  suggests the effect is fragile even before that.
- The Wednesday filter is chosen after the fact. The library added it because that day worked in its
  sample. Searching many weekday, threshold and universe combinations and keeping the best is how a
  rule with no real edge can still look profitable. For the whole idea to be false it is enough that
  the tone of news is already fully in the price by the open, in which case the drift the strategy
  harvests does not exist and only the costs remain.

## Try it yourself

You need nothing but a spreadsheet, a handful of press releases and a printed dictionary.

1. Make a sheet with the columns `Date`, `Time`, `Company`, `Headline`, `Phrase`, `Score`,
   `Article total`, `Day`, `Action`.
2. Write ten short phrases you expect in drug news and give each a number, positive for good news and
   negative for bad. Keep the list to one page.
3. Collect the last twenty press releases from one or two drug makers. For each, write down the exact
   publication time and the headline, then find every phrase from your list that appears in it and
   add the scores into `Article total`.
4. In `Day`, put the weekday. In `Action`, write "buy" only if the day is Wednesday and the article
   total is positive, "sell short" only if the day is Wednesday and the total is negative, and
   "none" otherwise.
5. For the buys and shorts, look up the price half an hour after the open and the price at the close
   on that same day, and compute the gross return with the formulas above. Subtract 0.20 percent for
   costs.
6. Now repeat the whole exercise using a second dictionary, one you write differently on purpose, and
   compare the two sets of totals.

What to notice: on most days the action is "none", and the trades cluster on one or two weekdays. The
same release will often score positive under one list and weaker or negative under the other, which
is the measurement problem this tutorial keeps returning to. And after costs, the small average edge
you computed by hand is easily wiped out.

## Where this came from

- [QuantConnect strategy library: using news sentiment to predict the price direction of drug
  manufacturers](https://www.quantconnect.com/tutorials/strategy-library/using-news-sentiment-to-predict-price-direction-of-drug-manufacturers),
  the rules as implemented: drug manufacturers ranked by dollar volume and price-to-earnings ratio, a
  dictionary that scores n-grams, an intraday long or short, equal weights, and the Wednesday filter.
- Shah, Isah and Zulkernine, [Predicting the Effects of News Sentiments on the Stock
  Market](https://arxiv.org/abs/1812.04199), arXiv `1812.04199`, the source of the dictionary and the
  70.59 percent directional accuracy figure.
- Berument and Kiymaz, [The Day of the Week Effect on Stock Market
  Volatility](https://ssrn.com/abstract=3211399), the source of the Wednesday filter.
- Budennyy and others, arXiv `2208.07248v2`, on clinical trial announcements and the asymmetry between
  positive and negative reactions.
- Aparicio and others, arXiv `2401.11011v1`, on scoring biotech press releases with finance-tuned
  language models.
- Glasserman and Lin, arXiv `2309.17322v1`, on look-ahead bias and the effect of hiding company names
  from the scorer.
- Boustanifar and Mansouri, arXiv `2609.31013v1`, on how much of a text score depends on which model
  produced it.
- [strategies/books2/11_language_models_news_and_text.md](../../../strategies/books2/11_language_models_news_and_text.md),
  this repository's brief on language models and text signals.
- [strategies/books/14_news_and_language_model_signals.md](../../../strategies/books/14_news_and_language_model_signals.md),
  this repository's brief on the trading side of news signals, including the cost collapse.

## Words used in this tutorial

- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- dictionary: a fixed list of words and phrases, each with a number, used to score text by lookup.
- dollar volume: the total value of a company's shares traded in a day, price times number of shares.
- long: owning something, so that a rise in its price is a gain.
- price-to-earnings ratio: the share price divided by the profit per share; a high number often means
  investors expect fast growth.
- short selling: borrowing something you do not own, selling it, and buying it back later, so that a
  fall in the price is a gain.
- spread: the small gap between the price at which you can buy and the price at which you can sell.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
