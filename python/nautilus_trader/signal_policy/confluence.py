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
Scoring a set of conditions into one entry decision, with the gates stated.

A strategy that needs three of five signals to agree before it enters has made a decision, and that
decision is usually expressed in code as a chain of `if` statements whose thresholds and counts are
spread across the file. `ConfluenceCard` states the same thing as data: named conditions, weighted
rules, the number that must match, the score that must be reached, conditions that are mandatory,
and conditions that veto the entry outright. The card is evaluated against a mapping of indicator
values and returns a score, a qualified flag and a record of which rules matched and which inputs
were missing.

Three boundaries are enforced rather than documented:

- **A missing input is not a false one.** A condition over an input that was not supplied evaluates
  to unknown, and unknown is never treated as satisfied: a rule that cannot be evaluated is not
  counted as matched, an `AllOf` containing an unknown is unknown rather than true, and `AtLeast`
  counts how many unknowns could still change the answer before it decides. The names of the rules
  whose inputs were missing are recorded, so an entry refused for want of data is distinguishable
  from one refused on the data.
- **A veto and a mandatory condition are different statements.** A mandatory condition must hold for
  a qualified entry; a veto refuses the entry when it holds at all. Both are evaluated before the
  score can qualify anything, so a high score cannot buy past them.
- **The points of a rule carry their own sign.** A positive rule must not have negative points and a
  negative rule must not have positive points, so a card cannot be read as though every rule added
  to the score.

The card is a decision aid and nothing more: it computes a score and a flag, submits nothing and
knows nothing about orders, positions or risk.
"""

from __future__ import annotations

import math
import operator
from dataclasses import dataclass
from dataclasses import field
from enum import Enum
from typing import TYPE_CHECKING


if TYPE_CHECKING:
    from collections.abc import Callable
    from collections.abc import Iterable
    from collections.abc import Mapping


CONFLUENCE_CARD_VERSION = 1


class Comparator(Enum):
    """
    How a condition compares an input against its threshold.
    """

    GREATER_OR_EQUAL = "greater_or_equal"
    GREATER = "greater"
    LESS_OR_EQUAL = "less_or_equal"
    LESS = "less"
    EQUAL = "equal"
    NOT_EQUAL = "not_equal"


# The comparisons themselves come from the standard library, so a condition is the language's own
# comparison rather than a re-implementation of it, and adding a comparator is one table entry
# rather than one more branch in an evaluation.
_COMPARISONS: dict[Comparator, Callable[[float, float], bool]] = {
    Comparator.GREATER_OR_EQUAL: operator.ge,
    Comparator.GREATER: operator.gt,
    Comparator.LESS_OR_EQUAL: operator.le,
    Comparator.LESS: operator.lt,
    Comparator.EQUAL: operator.eq,
    Comparator.NOT_EQUAL: operator.ne,
}


class ScoreKind(Enum):
    """
    Whether a matched rule adds to or subtracts from the score.
    """

    POSITIVE = "positive"
    NEGATIVE = "negative"


class _Unknown:
    """
    The three-valued logic of a condition whose input is absent.
    """


UNKNOWN = _Unknown()


@dataclass(frozen=True)
class Condition:
    """
    One comparison between a named input and a threshold.

    Parameters
    ----------
    name : str
        The key the input is supplied under.
    comparator : Comparator
        How the input is compared with the threshold.
    threshold : float
        The value compared against.
    label : str, optional
        A readable name for the condition, used in the record.

    Raises
    ------
    ValueError
        If the threshold is not finite.

    """

    name: str
    comparator: Comparator
    threshold: float
    label: str | None = None

    def __post_init__(self) -> None:
        """
        Refuse a threshold that is not a number to compare against.
        """
        if not math.isfinite(self.threshold):
            raise ValueError(f"threshold {self.threshold!r} is not finite")

    @property
    def display(self) -> str:
        """
        Return the readable name of the condition.
        """
        return self.label or f"{self.name} {self.comparator.value} {self.threshold:g}"

    def evaluate(self, values: Mapping[str, float]) -> bool | None:
        """
        Evaluate the condition against the supplied inputs.

        Parameters
        ----------
        values : Mapping[str, float]
            The inputs by name.

        Returns
        -------
        bool or None
            The comparison's result, or None when the input was not supplied or is not finite.

        """
        raw = values.get(self.name)

        if raw is None or not math.isfinite(raw):
            return None

        return _COMPARISONS[self.comparator](float(raw), self.threshold)


@dataclass(frozen=True)
class AllOf:
    """
    A conjunction: every member must hold.
    """

    members: tuple[object, ...]

    def evaluate(self, values: Mapping[str, float]) -> bool | None:
        """
        Evaluate the conjunction, treating an unknown member as unknown.
        """
        unknown = False

        for member in self.members:
            outcome = evaluate(member, values)

            if outcome is False:
                return False

            if outcome is None:
                unknown = True

        return None if unknown else True


@dataclass(frozen=True)
class AnyOf:
    """
    A disjunction: at least one member must hold.
    """

    members: tuple[object, ...]

    def evaluate(self, values: Mapping[str, float]) -> bool | None:
        """
        Evaluate the disjunction, treating an unknown member as unknown.
        """
        unknown = False

        for member in self.members:
            outcome = evaluate(member, values)

            if outcome is True:
                return True

            if outcome is None:
                unknown = True

        return None if unknown else False


@dataclass(frozen=True)
class Not:
    """
    A negation.
    """

    member: object

    def evaluate(self, values: Mapping[str, float]) -> bool | None:
        """
        Evaluate the negation, treating an unknown member as unknown.
        """
        outcome = evaluate(self.member, values)

        return None if outcome is None else not outcome


@dataclass(frozen=True)
class AtLeast:
    """
    A counting rule: at least `minimum` members must hold.

    Parameters
    ----------
    minimum : int
        The number of members that must hold.
    members : tuple of conditions
        The members counted.

    Raises
    ------
    ValueError
        If `minimum` is not positive or exceeds the number of members.

    """

    minimum: int
    members: tuple[object, ...]

    def __post_init__(self) -> None:
        """
        Refuse a count floor that cannot be reached.
        """
        if self.minimum < 1:
            raise ValueError(f"minimum must be at least 1, was {self.minimum}")

        if self.minimum > len(self.members):
            raise ValueError(
                f"minimum {self.minimum} exceeds the {len(self.members)} members counted"
            )

    def evaluate(self, values: Mapping[str, float]) -> bool | None:
        """
        Evaluate the count, which is unknown while enough members remain unevaluable.
        """
        matched = 0
        unknown = 0

        for member in self.members:
            outcome = evaluate(member, values)

            if outcome is True:
                matched += 1
            elif outcome is None:
                unknown += 1

        if matched >= self.minimum:
            return True

        if matched + unknown < self.minimum:
            return False

        return None


def evaluate(condition: object, values: Mapping[str, float]) -> bool | None:
    """
    Evaluate any condition, composed or not.

    Parameters
    ----------
    condition : object
        A `Condition`, `AllOf`, `AnyOf`, `Not` or `AtLeast`.
    values : Mapping[str, float]
        The inputs by name.

    Returns
    -------
    bool or None
        The outcome, or None when the inputs do not determine it.

    Raises
    ------
    TypeError
        If the object is not one of the condition types this module composes.

    """
    if isinstance(condition, Condition | AllOf | AnyOf | Not | AtLeast):
        return condition.evaluate(values)

    raise TypeError(
        f"{condition!r} is not a condition; compose a Condition with AllOf, AnyOf, Not or AtLeast"
    )


@dataclass(frozen=True)
class ScoreRule:
    """
    One scored rule.

    Parameters
    ----------
    name : str
        The rule's name, used in the record.
    condition : object
        The condition the rule matches on.
    points : float
        The points the rule contributes when it matches. A positive rule's points must not be
        negative and a negative rule's points must not be positive.
    kind : ScoreKind
        Whether the rule adds to or subtracts from the score.

    Raises
    ------
    ValueError
        If the points carry the wrong sign for the rule's kind, or are not finite.

    """

    name: str
    condition: object
    points: float
    kind: ScoreKind = ScoreKind.POSITIVE

    def __post_init__(self) -> None:
        """
        Refuse points that are not a number, or that read as the opposite of the rule's kind.
        """
        if not math.isfinite(self.points):
            raise ValueError(f"points {self.points!r} are not finite")

        if self.kind is ScoreKind.POSITIVE and self.points < 0.0:
            raise ValueError(f"a positive rule cannot carry negative points, was {self.points}")

        if self.kind is ScoreKind.NEGATIVE and self.points > 0.0:
            raise ValueError(f"a negative rule cannot carry positive points, was {self.points}")


@dataclass(frozen=True)
class RuleGroup:
    """
    A named set of scored rules with its own gates.

    Parameters
    ----------
    name : str
        The group's name.
    rules : tuple of ScoreRule
        The rules in the group.
    minimum_matches : int, default 1
        How many rules must match for the group to pass.
    minimum_score : float, default 0.0
        The points the matched rules must reach for the group to pass.

    """

    name: str
    rules: tuple[ScoreRule, ...]
    minimum_matches: int = 1
    minimum_score: float = 0.0


@dataclass(frozen=True)
class GroupOutcome:
    """
    What one group produced.

    Parameters
    ----------
    name : str
        The group's name.
    matched : int
        How many rules matched.
    score : float
        The points the matched rules contributed.
    minimum_matches : int
        The group's declared match floor.
    minimum_score : float
        The group's declared score floor.

    """

    name: str
    matched: int
    score: float
    minimum_matches: int
    minimum_score: float

    @property
    def passed(self) -> bool:
        """
        Return whether the group met both of its floors.
        """
        return self.matched >= self.minimum_matches and self.score >= self.minimum_score

    def to_record(self) -> dict[str, object]:
        """
        Return a flat record of the group's outcome.
        """
        return {
            "matched": self.matched,
            "score": self.score,
            "minimum_matches": self.minimum_matches,
            "minimum_score": self.minimum_score,
            "passed": self.passed,
        }


@dataclass(frozen=True)
class ConfluenceResult:
    """
    What a card produced for one set of inputs.

    Parameters
    ----------
    card : str
        The card's name.
    score : float
        The points every matched rule contributed.
    minimum_score : float
        The card's declared score floor.
    qualified : bool
        Whether the entry qualified.
    primary : GroupOutcome
        The primary group's outcome.
    evidence : tuple of GroupOutcome
        The evidence groups' outcomes.
    conditions_met : bool
        Whether every mandatory condition held.
    vetoed : bool
        Whether any veto triggered.
    matched_rules : tuple of str
        The names of the rules that matched.
    missing_rules : tuple of str
        The names of the rules whose inputs did not determine an outcome.

    """

    card: str
    score: float
    minimum_score: float
    qualified: bool
    primary: GroupOutcome
    evidence: tuple[GroupOutcome, ...]
    conditions_met: bool
    vetoed: bool
    matched_rules: tuple[str, ...]
    missing_rules: tuple[str, ...]

    @property
    def margin(self) -> float:
        """
        Return how far the score is above or below the floor.
        """
        return self.score - self.minimum_score

    @property
    def decision(self) -> str:
        """
        Return the decision as a word.
        """
        return "qualified" if self.qualified else "rejected"

    def to_trace(self) -> dict[str, object]:
        """
        Return a flat, JSON-serialisable record of the decision.

        The record carries the decision, the score and its floor, each group's outcome, and the
        names of the rules that matched and of those whose inputs were missing, so a refusal for
        want of data is distinguishable from a refusal on the data.
        """
        return {
            "trace_version": CONFLUENCE_CARD_VERSION,
            "card": self.card,
            "decision": self.decision,
            "score": self.score,
            "minimum_score": self.minimum_score,
            "margin": self.margin,
            "primary": self.primary.to_record(),
            "evidence": [outcome.to_record() for outcome in self.evidence],
            "conditions_met": self.conditions_met,
            "vetoed": self.vetoed,
            "matched_rules": list(self.matched_rules),
            "missing_rules": list(self.missing_rules),
        }


@dataclass(frozen=True)
class ConfluenceCard:
    """
    A scored entry decision with its gates stated.

    Parameters
    ----------
    name : str
        The card's name.
    primary : RuleGroup
        The group that must match for the entry to be possible at all.
    evidence : tuple of RuleGroup, default ()
        Further groups, each with its own floors.
    conditions : tuple of conditions, default ()
        Conditions that must all hold, whatever the score.
    vetoes : tuple of conditions, default ()
        Conditions that refuse the entry when they hold.
    scoring : tuple of ScoreRule, default ()
        Rules scored by the card but not belonging to a group.
    minimum_score : float, default 0.0
        The total score the entry must reach.

    """

    name: str
    primary: RuleGroup
    evidence: tuple[RuleGroup, ...] = field(default_factory=tuple)
    conditions: tuple[object, ...] = field(default_factory=tuple)
    vetoes: tuple[object, ...] = field(default_factory=tuple)
    scoring: tuple[ScoreRule, ...] = field(default_factory=tuple)
    minimum_score: float = 0.0

    def evaluate(self, values: Mapping[str, float]) -> ConfluenceResult:
        """
        Evaluate the card against the supplied inputs.

        Parameters
        ----------
        values : Mapping[str, float]
            The inputs by name.

        Returns
        -------
        ConfluenceResult
            The score, the qualified flag and the record of which rules matched.

        Examples
        --------
        >>> card = ConfluenceCard(
        ...     name="entry",
        ...     primary=RuleGroup(
        ...         name="trend",
        ...         rules=(ScoreRule("above", Condition("close", Comparator.GREATER, 10.0), 2.0),),
        ...     ),
        ...     minimum_score=2.0,
        ... )
        >>> card.evaluate({"close": 11.0}).qualified
        True

        """
        named: list[tuple[str, object]] = []
        for group in (self.primary, *self.evidence):
            named.extend((rule.name, rule.condition) for rule in group.rules)

        named.extend((rule.name, rule.condition) for rule in self.scoring)

        outcomes = {name: evaluate(condition, values) for name, condition in named}

        matched = tuple(sorted(name for name, outcome in outcomes.items() if outcome is True))
        missing = tuple(sorted(name for name, outcome in outcomes.items() if outcome is None))

        primary = _group_outcome(self.primary, outcomes)
        evidence = tuple(_group_outcome(group, outcomes) for group in self.evidence)

        score = primary.score + sum(outcome.score for outcome in evidence)
        score += sum(rule.points for rule in self.scoring if outcomes.get(rule.name) is True)

        conditions_met = all(evaluate(condition, values) is True for condition in self.conditions)
        vetoed = any(evaluate(condition, values) is True for condition in self.vetoes)

        qualified = (
            primary.passed
            and all(outcome.passed for outcome in evidence)
            and conditions_met
            and not vetoed
            and score >= self.minimum_score
        )

        return ConfluenceResult(
            card=self.name,
            score=score,
            minimum_score=self.minimum_score,
            qualified=qualified,
            primary=primary,
            evidence=evidence,
            conditions_met=conditions_met,
            vetoed=vetoed,
            matched_rules=matched,
            missing_rules=missing,
        )


def _group_outcome(
    group: RuleGroup,
    outcomes: Mapping[str, bool | None],
) -> GroupOutcome:
    """
    Return a group's outcome from the evaluated rules.
    """
    matched = [rule for rule in group.rules if outcomes.get(rule.name) is True]
    score = math.fsum(rule.points for rule in matched)

    return GroupOutcome(
        name=group.name,
        matched=len(matched),
        score=score,
        minimum_matches=group.minimum_matches,
        minimum_score=group.minimum_score,
    )


def confluence_card(  # noqa: PLR0913 - the card's declared fields
    name: str,
    *,
    primary: RuleGroup,
    evidence: Iterable[RuleGroup] = (),
    conditions: Iterable[object] = (),
    vetoes: Iterable[object] = (),
    scoring: Iterable[ScoreRule] = (),
    minimum_score: float = 0.0,
) -> ConfluenceCard:
    """
    Return a card built from iterables, for callers that compose it programmatically.

    Parameters
    ----------
    name : str
        The card's name.
    primary : RuleGroup
        The group that must match for the entry to be possible.
    evidence : Iterable of RuleGroup, optional
        Further groups with their own floors.
    conditions : Iterable of conditions, optional
        Conditions that must all hold.
    vetoes : Iterable of conditions, optional
        Conditions that refuse the entry when they hold.
    scoring : Iterable of ScoreRule, optional
        Rules scored by the card but not belonging to a group.
    minimum_score : float, default 0.0
        The total score the entry must reach.

    Returns
    -------
    ConfluenceCard
        The card.

    """
    return ConfluenceCard(
        name=name,
        primary=primary,
        evidence=tuple(evidence),
        conditions=tuple(conditions),
        vetoes=tuple(vetoes),
        scoring=tuple(scoring),
        minimum_score=minimum_score,
    )
