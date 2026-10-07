# News sentiment: buying when the news reads positive

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                              |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of one company, held only while the news about that company reads positive                                                                                                                                                  |
| How often it trades       | Whenever the measured mood crosses a threshold, which can be a few times a month or almost not at all                                                                                                                              |
| What you need             | Python and a data file of prices, plus a stream of news text                                                                                                                                                                       |
| Where the rules come from | [fastquant strategy library table](https://github.com/enzoampil/fastquant) and its [sentiment.py](https://github.com/enzoampil/fastquant/blob/master/python/fastquant/strategies/sentiment.py)                                     |
| The underlying research   | [VADER, the rule-based sentiment model the library calls](https://github.com/cjhutto/vaderSentiment), and this collection's [language models, news and text brief](../../../strategies/books2/11_language_models_news_and_text.md) |
| How well it held up       | Weak: the library publishes one worked example that it says cannot be reproduced, and the wider sentiment literature is split on whether text predicts prices                                                                      |
| Also appears in           | nothing else in this collection                                                                                                                                                                                                    |

## The idea in one paragraph

Every day, read the news stories about one company. Judge each story on a scale from clearly bad to
clearly good, and average the day's scores into a single number. When that number is positive enough,
buy the shares; when it turns negative enough, sell. The belief is that good news arrives gradually
and is not fully reflected in the price at once, so the traders who read the news first can buy
before the slow money catches up. The library measures the mood automatically with a word-based
scoring tool, then trades on the daily average.

## Why anyone believed it

Company news is not a single announcement. A quarter's results arrive as a press release, then a
call with analysts, then a dozen articles that interpret both, then social-media reactions. Each of
those steps takes hours or days, and each one can move the price a little. If some readers act
immediately and others act late, the price can drift in the direction of the news for a while after
the first headline.

The counterparty, then, is the reader who is slow, distracted, or not looking at that company yet.
There is also a forced-seller story: a fund that must trim a position for reasons of its own, or a
household that sells to pay a bill, can push the price down even when the news is good. If the
positive news is real and the sellers are not reacting to it, buying on the news is buying from
someone who is not paying attention.

There is a second, less flattering reason the idea is popular: news text is easy to collect and the
tools are free, so it looks like a way to use information without buying expensive market data. That
convenience is not evidence, and this tutorial returns to it in the section on what the research
found.

## An everyday comparison

Think of a restaurant that has just won an award. The prize is announced on a Tuesday, but most
diners only hear about it over the following weeks, when a friend mentions it or a review appears.
A person who books a table on the Wednesday may eat in a busy, cheerful place at a normal price,
while the crowd arrives later. The award is public on day one, yet the effect on bookings builds for
weeks. News sentiment is a bet that this slow arrival is regular enough to trade.

## The rules, step by step

1. Pick one company and collect its daily closing prices for the period you want to study.
2. Collect news stories about that company from a publisher. The library's helper takes a keyword,
   such as the company name, and a number of search pages, and reads the articles it finds.
3. Score every article. The library reads the full text of each article and produces one number
   between about -1 and +1. The scoring is described below.
4. Build a daily score. For each date the library averages the scores of all articles found for that
   date, and it moves the result to the next calendar day, because a story published late cannot be
   acted on that morning. That shift is the library's own look-ahead guard: the score for a story
   dated today is attached to tomorrow.
5. Buy when the day's score is at least the threshold, which the library calls `senti` and defaults
   to 0.2.
6. Sell when the day's score is at most minus the threshold, so with the default, at -0.2 or below.
   Between those two values the strategy holds whatever it already has.
7. Trade at the next day's closing price, because the library's orders are filled on the close of
   the day after the signal. Buy with all the cash by default, and sell the whole holding.
8. The parameters the library names for this strategy are `keyword`, `page_nums` and `senti`. The
   first two belong to the news-collecting helper; `senti` is the strategy's threshold.

Two things must be said plainly. First, the strategy only ever buys the one company it is pointed
at, so it has no way to tell good news from a good market. Second, the library joins each price date
to the score for that date, and if no article was found, the score is missing and no signal fires.

## The maths, with every symbol named

A sentiment score is a number that tries to stand for "how positive the text reads". The library's
score is the VADER compound score, which is built from a fixed list of words, each with a
hand-assigned value, plus a small set of rules about punctuation and capital letters.

A simplified version of the published formula is:

```text
score = x / sqrt(x * x + 15)
```

- `score` is the output, which stays between -1 and +1.
- `x` is the sum of the values of the words found in the text.
- `sqrt` is the square root, and 15 is the constant the paper uses to keep the result in range.
- `x * x` means `x` multiplied by itself.

For example, if a headline contains one strongly positive word worth +2.0 and one mildly positive
word worth +1.0:

```text
x = 3.0
score = 3.0 / sqrt(9 + 15) = 3.0 / 4.899 = 0.612
```

The word values above are an illustration of the method, not the exact entries in the lexicon; the
exact values live in the model's published word list. The important property is that the score comes
from a fixed, published rule, not from a person's judgement.

The daily score is the average of the articles found for that date:

```text
S_day = (score_1 + score_2 + ... + score_k) / k
```

- `S_day` is the score the strategy uses for that date.
- `score_1` to `score_k` are the individual article scores.
- `k` is the number of articles found for that date.

The strategy's rule is then a pair of thresholds:

```text
buy when S_day >=  senti
sell when S_day <= -senti
```

- `S_day` is the day's average score.
- `senti` is the threshold, 0.2 by default, so a mild positive average triggers a buy and only a
  clearly negative one triggers a sale.

Finally, the cost of each round trip, exactly as in any other strategy:

```text
Cost = 2 * c
```

- `2` counts the buy side and the sell side.
- `c` is the cost per side as a fraction, covering the gap between the buying and selling price plus
  any commission.

## A worked example

Six days of a made-up company. The scores and prices are invented, but they are of the size real
news-and-price data takes. The threshold is the default 0.2.

| Day | Close price | News score | Signal                     | Action taken                                |
| --- | ----------- | ---------- | -------------------------- | ------------------------------------------- |
| 1   | 100.00      | +0.55      | buy (0.55 is at least 0.2) | none yet; the order fills the next close    |
| 2   | 102.00      | +0.30      | buy                        | buy the whole account at 102.00             |
| 3   | 104.00      | +0.05      | none                       | hold                                        |
| 4   | 106.00      | -0.45      | sell (-0.45 is below -0.2) | none yet; the order fills the next close    |
| 5   | 108.00      | -0.10      | none                       | sell the whole holding at 108.00            |
| 6   | 107.00      | -0.60      | sell                       | nothing to sell; the account is already out |

The arithmetic of the round trip, with 0.1 percent slippage and 0.1 percent commission on each side:

```text
buy price = 102.00 * 1.001 = 102.102
shares = 10,000.00 / (102.102 * 1.001) = 97.8435
buy value = 97.8435 * 102.102 = 9,990.01
buy commission = 9.99
sell price = 108.00 * 0.999 = 107.892
sell proceeds = 97.8435 * 107.892 = 10,556.53
sell commission = 10.56
final value = 10,556.53 - 10.56 = 10,545.97
return = 10,545.97 / 10,000.00 - 1 = 0.0546, that is 5.46 percent
```

The strategy gained 5.46 percent because the price happened to rise between the two fills. That
number was chosen by the author of this page, not produced by the library, and it proves nothing
about news sentiment. It only shows how the rules turn scores into trades. Notice the invisible
assumption: the buy filled at 102 because the signal appeared on day 1 and the price on day 2 was
still available. If the good news had already moved the price to 106 by the time the order filled,
the same rule would have bought higher and the result would be different.

## What the research actually found

The library publishes no test of this strategy. Its own documentation shows one run on Tesla shares
from January to July 2020 with a final portfolio value of 313,198.37 from a starting 100,000, and
then states in the next sentence that the scenario cannot be recreated, because the dates and the
scraped sentiments vary from run to run. A result that cannot be repeated is not evidence, whatever
its size, and 2020 was an unusual year for that particular company.

What the wider research measures is the harder question: does text predict prices at all, and how
reliable is the measurement. Three findings matter here.

| Source                                                                             | What it measured                                                        | Result                                                                                                                                                                                     |
| ---------------------------------------------------------------------------------- | ----------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Same Text, Different Numbers: The Divergence of LLM-Based Measures, `2609.31013v1` | Seven models scoring the same 1,946 earnings calls on the same scale    | The average agreement between any two models was a rank correlation of only 0.52, and the provider mattered as much as the text                                                            |
| Measuring Sentiment News with Transformer-Based Language Models, `2607.13968v1`    | Transformer models against word-list models on 588 human-rated articles | The transformer models reached a macro-F1 of 0.67 to 0.69, while the word-list models reached only 0.187 on the same articles, so the choice of method changes the answer by a wide margin |
| Assessing Look-Ahead Bias in GPT Sentiment Analysis, `2309.17322v1`                | Long-short strategies built from scored headlines                       | Removing company names changed the results, showing that a text strategy can quietly use information it would not have had at the time                                                     |

The first two findings are about measurement, not profit. They say that two systems reading the same
headline can disagree substantially, and that a word list and a trained model are not interchangeable.
The third is about honesty: text data is full of ways to know something slightly too early. None of
these studies tests the library's exact rule, and none of them promises a profit.

## How this project relates to it

The measurement questions above are collected in this repository's
[language models, news and text brief](../../../strategies/books2/11_language_models_news_and_text.md),
which reads twelve papers on turning text into numbers and explains why an automated score is a
measurement with a provider attached rather than an objective reading. Its section on look-ahead
bias and its table of provider disagreement are the honest background for any news-sentiment rule,
including the simple one in this tutorial.

## Where it goes wrong

- The score is a third-party construction. The strategy reads a number produced by a fixed word list
  and formula, and the user cannot inspect or adjust the thousands of entries behind it. The score is
  also only as good as the publisher's idea of which words matter, and it has no way to know that
  "the loss narrowed" is good while "the profit warning" is bad.
- Same headline, different number. As the provider study shows, two reasonable systems can score the
  same text differently enough to change a trade. A threshold of 0.2 is therefore not a physical
  constant; it is calibrated to one model, and it means something else in another.
- Look-ahead risk. A story published after the close cannot be traded at that day's open. The library
  shifts each score one day forward, which helps, but a scraped date can be the date of an update
  rather than the date of first publication, and the collection order of the pages can hide that.
- No control for the market. The strategy buys one company and compares the mood to zero, so a
  cheerful day for the whole market is read as cheerful news for the company.
- The sample is chosen after the fact. It is easy to pick a keyword and a period where the news and
  the price happened to line up, and the library's own Tesla example is exactly that kind of
  unrepeatable illustration.
- The publisher can change underneath you. Wherever a page layout or a search order changes, the set
  of articles collected changes, so the measured score changes even when the news does not.

## Try it yourself

You need no money and no code, only a small table and a search engine.

1. Choose one company and one recent month.
2. For each weekday, write down the closing price and the headline of the most prominent news story
   about that company.
3. Score each headline by hand on a scale from -1 (clearly bad) to +1 (clearly good). Do this before
   you look at that day's price movement, or you will cheat without meaning to.
4. Put the scores in one column and the next day's price change in the next column, and add up the
   score for each day.
5. Ask two questions: do the positive-score days tend to be followed by rises, and would a rule that
   buys above +0.2 and sells below -0.2 have made more than the price alone after two trading costs?

What to notice: the hard part is step 3. Two people scoring the same headline often disagree, and the
disagreement is largest exactly on the ambiguous stories that carry the most information. That gap
between two human scorers is the same gap the provider study found between two machine scorers, and
it is why a single number should not be treated as the truth about a story.

## Where this came from

- [fastquant](https://github.com/enzoampil/fastquant), whose strategy table lists the sentiment
  strategy with the parameters `keyword`, `page_nums` and `senti`, and its
  [sentiment.py](https://github.com/enzoampil/fastquant/blob/master/python/fastquant/strategies/sentiment.py),
  which buys at or above `senti` and sells at or below minus `senti`.
- The library's news collector,
  [businesstimes.py](https://github.com/enzoampil/fastquant/blob/master/python/fastquant/data/web/businesstimes.py),
  which scrapes Business Times articles by keyword, scores each with VADER, and attaches each date's
  average to the following day.
- [VADER sentiment analysis](https://github.com/cjhutto/vaderSentiment), the published word-list
  model and the compound formula used for the score.
- [`2609.31013v1`](https://arxiv.org/abs/2609.31013) Same Text, Different Numbers, and
  [`2607.13968v1`](https://arxiv.org/abs/2607.13968) Measuring Sentiment News with Transformer-Based
  Language Models, the provider-agreement and lexicon-versus-transformer measurements.
- [`2309.17322v1`](https://arxiv.org/abs/2309.17322) Assessing Look-Ahead Bias in Stock Return
  Predictions Generated By GPT Sentiment Analysis, on text strategies that can see the future.
- [Language models, news and text](../../../strategies/books2/11_language_models_news_and_text.md),
  this collection's brief, and the local [GLOSSARY.md](../../GLOSSARY.md).

## Words used in this tutorial

- sentiment: a number standing for how positive or negative a piece of text reads.
- lexicon: a fixed list of words with an assigned value, as opposed to a model trained on examples.
- threshold: the value a signal must cross before the strategy acts.
- look-ahead bias: accidentally using information that would not have been available at the time of
  the trade.
- round trip: one buy and the matching sell of the same position.
- rank correlation: a number between -1 and 1 for how closely two rankings agree; 0.52 means the
  agreement is weak.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
