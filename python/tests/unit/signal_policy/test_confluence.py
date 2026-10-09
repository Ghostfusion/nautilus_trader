# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2015-2026 Nautech Systems Pty Ltd. All rights reserved.
#  https://nautechsystems.io
#
#  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
#  You may not use this file except in compliance with the License.
#  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.
# -------------------------------------------------------------------------------------------------

"""
Tests for the confluence card.

The three-valued logic is checked on its own, because a missing input that reads as a satisfied
condition is the failure that would matter: an entry taken on data that was never supplied. The card
is then checked on the case that separates the gates, where a high total score cannot buy past a
primary group that did not match enough rules.
"""

import json

import pytest

from nautilus_trader.signal_policy import AllOf
from nautilus_trader.signal_policy import AnyOf
from nautilus_trader.signal_policy import AtLeast
from nautilus_trader.signal_policy import Comparator
from nautilus_trader.signal_policy import Condition
from nautilus_trader.signal_policy import ConfluenceCard
from nautilus_trader.signal_policy import Not
from nautilus_trader.signal_policy import RuleGroup
from nautilus_trader.signal_policy import ScoreKind
from nautilus_trader.signal_policy import ScoreRule
from nautilus_trader.signal_policy import confluence_card
from nautilus_trader.signal_policy import evaluate


def _entry_card() -> ConfluenceCard:
    """
    Return a three-rule entry card with a two-match primary floor and a seven-point total floor.
    """
    primary = RuleGroup(
        name="primary",
        rules=(
            ScoreRule(
                "rsi-cross",
                AllOf(
                    (
                        Condition("rsi_prev", Comparator.LESS_OR_EQUAL, 50.0),
                        Condition("rsi", Comparator.GREATER, 50.0),
                    )
                ),
                3.0,
            ),
            ScoreRule(
                "macd-cross",
                AllOf(
                    (
                        Condition("macd_prev", Comparator.LESS_OR_EQUAL, 0.0),
                        Condition("macd", Comparator.GREATER, 0.0),
                    )
                ),
                3.0,
            ),
            ScoreRule("oversold", Condition("stoch", Comparator.LESS, 20.0), 2.0),
        ),
        minimum_matches=2,
        minimum_score=5.0,
    )
    evidence = RuleGroup(
        name="confirmation",
        rules=(ScoreRule("above-average", Condition("close", Comparator.GREATER, 10.0), 2.0),),
        minimum_matches=1,
        minimum_score=2.0,
    )

    return ConfluenceCard(
        name="entry",
        primary=primary,
        evidence=(evidence,),
        minimum_score=7.0,
    )


def test_a_condition_compares_the_input_it_names() -> None:
    """
    Test each comparator against a value on either side of its threshold.
    """
    assert Condition("x", Comparator.GREATER_OR_EQUAL, 1.0).evaluate({"x": 1.0}) is True
    assert Condition("x", Comparator.GREATER, 1.0).evaluate({"x": 1.0}) is False
    assert Condition("x", Comparator.LESS_OR_EQUAL, 1.0).evaluate({"x": 1.0}) is True
    assert Condition("x", Comparator.LESS, 1.0).evaluate({"x": 1.0}) is False
    assert Condition("x", Comparator.EQUAL, 1.0).evaluate({"x": 1.0}) is True
    assert Condition("x", Comparator.NOT_EQUAL, 1.0).evaluate({"x": 1.0}) is False


def test_a_condition_over_a_missing_input_is_unknown() -> None:
    """
    Test an input that was not supplied, or is not finite, does not read as satisfied.
    """
    condition = Condition("x", Comparator.GREATER, 0.0)

    assert condition.evaluate({}) is None
    assert condition.evaluate({"x": float("nan")}) is None
    assert condition.evaluate({"x": float("inf")}) is None


def test_a_threshold_must_be_finite() -> None:
    """
    Test a condition whose threshold is not a number is refused at the boundary.
    """
    with pytest.raises(ValueError, match="not finite"):
        Condition("x", Comparator.GREATER, float("nan"))


def test_the_composed_conditions_read_missing_inputs_as_unknown() -> None:
    """
    Test the three-valued logic of the conjunctions, disjunctions and counts.
    """
    true_condition = Condition("a", Comparator.GREATER, 0.0)
    false_condition = Condition("b", Comparator.GREATER, 100.0)
    unknown_condition = Condition("missing", Comparator.GREATER, 0.0)

    assert AllOf((true_condition, false_condition)).evaluate({"a": 1.0, "b": 1.0}) is False
    assert AllOf((true_condition, unknown_condition)).evaluate({"a": 1.0}) is None
    assert AllOf((true_condition, true_condition)).evaluate({"a": 1.0}) is True

    assert AnyOf((false_condition, true_condition)).evaluate({"a": 1.0, "b": 1.0}) is True
    assert AnyOf((false_condition, unknown_condition)).evaluate({"b": 1.0}) is None
    assert AnyOf((false_condition, false_condition)).evaluate({"b": 1.0}) is False

    assert Not(true_condition).evaluate({"a": 1.0}) is False
    assert Not(unknown_condition).evaluate({}) is None

    # One match and one unknown among three: the count cannot be decided yet.
    assert (
        AtLeast(2, (true_condition, unknown_condition, false_condition)).evaluate(
            {"a": 1.0, "b": 1.0}
        )
        is None
    )
    # One match and two known failures: the count is decided, and it fails.
    assert (
        AtLeast(2, (true_condition, false_condition, false_condition)).evaluate(
            {"a": 1.0, "b": 1.0}
        )
        is False
    )
    assert (
        AtLeast(2, (true_condition, true_condition, false_condition)).evaluate({"a": 1.0, "b": 1.0})
        is True
    )


def test_a_count_that_cannot_be_reached_is_refused() -> None:
    """
    Test a count floor with no way to reach it is refused when the rule is built.
    """
    condition = Condition("a", Comparator.GREATER, 0.0)

    with pytest.raises(ValueError, match="at least 1"):
        AtLeast(0, (condition,))

    with pytest.raises(ValueError, match="exceeds the"):
        AtLeast(2, (condition,))


def test_an_object_that_is_not_a_condition_is_refused_with_its_name() -> None:
    """
    Test a rule built over something else fails where it is evaluated, naming the object.
    """
    with pytest.raises(TypeError, match="is not a condition"):
        evaluate("close > 10", {})


def test_the_points_of_a_rule_carry_its_own_sign() -> None:
    """
    Test a card cannot hold a rule whose points read as the opposite of its kind.
    """
    condition = Condition("a", Comparator.GREATER, 0.0)

    with pytest.raises(ValueError, match="cannot carry negative points"):
        ScoreRule("bad", condition, -1.0, ScoreKind.POSITIVE)

    with pytest.raises(ValueError, match="cannot carry positive points"):
        ScoreRule("bad", condition, 1.0, ScoreKind.NEGATIVE)

    with pytest.raises(ValueError, match="not finite"):
        ScoreRule("bad", condition, float("nan"))


def test_the_card_qualifies_when_every_gate_holds() -> None:
    """
    Test the documented case: two primary crossings and the confirmation score eight of seven.
    """
    result = _entry_card().evaluate(
        {
            "rsi_prev": 45.0,
            "rsi": 55.0,
            "macd_prev": -0.1,
            "macd": 0.2,
            "stoch": 15.0,
            "close": 11.0,
        }
    )

    # The total is the primary group's eight points plus the confirmation's two.
    assert result.score == 10.0
    assert result.minimum_score == 7.0
    assert result.margin == 3.0
    assert result.primary.matched == 3
    assert result.primary.passed
    assert result.qualified
    assert result.decision == "qualified"
    assert result.matched_rules == (
        "above-average",
        "macd-cross",
        "oversold",
        "rsi-cross",
    )


def test_a_high_total_score_cannot_buy_past_the_primary_floor() -> None:
    """
    Test one primary match with a passing confirmation is refused despite its points.
    """
    result = _entry_card().evaluate(
        {
            "rsi_prev": 60.0,
            "rsi": 55.0,
            "macd_prev": 0.2,
            "macd": 0.1,
            "stoch": 15.0,
            "close": 11.0,
        }
    )

    assert result.primary.matched == 1
    assert result.primary.score == 2.0
    assert not result.primary.passed
    assert result.score == 4.0
    assert not result.qualified


def test_a_rule_over_a_missing_input_is_recorded_rather_than_counted() -> None:
    """
    Test a refusal for want of data is distinguishable from a refusal on the data.
    """
    result = _entry_card().evaluate({"close": 11.0})

    assert result.primary.matched == 0
    assert not result.qualified
    assert result.missing_rules == ("macd-cross", "oversold", "rsi-cross")
    assert result.matched_rules == ("above-average",)


def test_a_veto_refuses_an_entry_that_would_otherwise_qualify() -> None:
    """
    Test a veto is checked before the score can qualify anything.
    """
    card = ConfluenceCard(
        name="entry",
        primary=RuleGroup(
            name="primary",
            rules=(ScoreRule("breakout", Condition("close", Comparator.GREATER, 10.0), 5.0),),
            minimum_matches=1,
            minimum_score=5.0,
        ),
        vetoes=(Condition("atr", Comparator.LESS, 0.5),),
        minimum_score=5.0,
    )

    assert card.evaluate({"close": 11.0, "atr": 1.0}).qualified
    assert not card.evaluate({"close": 11.0, "atr": 0.2}).qualified
    assert card.evaluate({"close": 11.0, "atr": 0.2}).vetoed


def test_a_mandatory_condition_is_separate_from_a_veto() -> None:
    """
    Test a condition must hold, and an unevaluable condition does not hold.
    """
    card = ConfluenceCard(
        name="entry",
        primary=RuleGroup(
            name="primary",
            rules=(ScoreRule("breakout", Condition("close", Comparator.GREATER, 10.0), 5.0),),
            minimum_matches=1,
            minimum_score=5.0,
        ),
        conditions=(Condition("spread", Comparator.LESS, 1.0),),
        minimum_score=5.0,
    )

    assert card.evaluate({"close": 11.0, "spread": 0.5}).qualified
    assert not card.evaluate({"close": 11.0, "spread": 2.0}).conditions_met
    assert not card.evaluate({"close": 11.0}).conditions_met


def test_a_negative_rule_subtracts_from_the_score() -> None:
    """
    Test a penalty rule lowers the total while still being reported as matched.
    """
    card = ConfluenceCard(
        name="entry",
        primary=RuleGroup(
            name="primary",
            rules=(ScoreRule("breakout", Condition("close", Comparator.GREATER, 10.0), 5.0),),
            minimum_matches=1,
            minimum_score=5.0,
        ),
        scoring=(
            ScoreRule(
                "already-extended",
                Condition("run_up", Comparator.GREATER, 0.05),
                -2.0,
                ScoreKind.NEGATIVE,
            ),
        ),
        minimum_score=5.0,
    )

    plain = card.evaluate({"close": 11.0})
    penalised = card.evaluate({"close": 11.0, "run_up": 0.1})

    assert plain.score == 5.0
    assert plain.qualified
    assert penalised.score == 3.0
    assert not penalised.qualified
    assert "already-extended" in penalised.matched_rules


def test_the_helper_builds_the_same_card_as_the_dataclass() -> None:
    """
    Test the builder is a convenience and not a second implementation.
    """
    primary = RuleGroup(
        name="primary",
        rules=(ScoreRule("breakout", Condition("close", Comparator.GREATER, 10.0), 5.0),),
    )

    built = confluence_card("entry", primary=primary, minimum_score=5.0)
    constructed = ConfluenceCard(name="entry", primary=primary, minimum_score=5.0)

    values = {"close": 11.0}

    assert built.evaluate(values).to_trace() == constructed.evaluate(values).to_trace()


def test_the_trace_states_the_decision_and_the_gates() -> None:
    """
    Test a decision can be read back without the classes.
    """
    result = _entry_card().evaluate({"close": 5.0})

    trace = result.to_trace()

    assert trace["trace_version"] == 1
    assert trace["card"] == "entry"
    assert trace["decision"] == "rejected"
    assert trace["score"] == 0.0
    assert trace["minimum_score"] == 7.0
    assert trace["primary"] == {
        "matched": 0,
        "score": 0.0,
        "minimum_matches": 2,
        "minimum_score": 5.0,
        "passed": False,
    }
    assert trace["missing_rules"] == ["macd-cross", "oversold", "rsi-cross"]
    assert json.loads(json.dumps(trace)) == trace
