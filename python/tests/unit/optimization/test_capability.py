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
Tests for the research capability probes and for the rule that no caller reads a detail.

The probes are asserted on the code they report for a refusal, because the code is the canonical
half of the answer: the detail is prose for a human, and the last test scans the source to keep it
that way.
"""

import re
from pathlib import Path

import pytest

from nautilus_trader.core import Capability
from nautilus_trader.optimization import LabelDefinition
from nautilus_trader.optimization import LabelKind
from nautilus_trader.optimization import LeakagePolicy
from nautilus_trader.optimization import ResearchCapabilityCode
from nautilus_trader.optimization import SplitContract
from nautilus_trader.optimization import StatisticalContract
from nautilus_trader.optimization import TrialDependence
from nautilus_trader.optimization import label_series
from nautilus_trader.optimization import leakage_capability
from nautilus_trader.optimization import significance_capability
from nautilus_trader.optimization import split_capability


DAILY_NS = 86_400_000_000_000
REPO_ROOT = Path(__file__).resolve().parents[4]
PROBE_SOURCES = ("crates/*/src/**/*.rs", "python/nautilus_trader/**/*.py")

# The ways a value can be classified by its prose: an equality, a membership test, or a prefix or
# substring search. A construction, a rendering or an emptiness check is not a match.
_DETAIL_MATCHES = (
    re.compile(
        r"\.detail\(\)\s*(==|!=|\.eq\(|\.ne\(|\.contains\(|\.starts_with\(|\.ends_with\(|\.matches\()",
    ),
    re.compile(r"\.detail\s*(==|!=|\bin\b|\.startswith\(|\.endswith\(|\.find\(|\.index\()"),
    re.compile(r"\bdetail\s+in\b"),
)

FORWARD = LabelDefinition(label_id="forward", kind=LabelKind.FORWARD_RETURN, horizon=2)


def daily(closes: list[float]) -> list[tuple[int, float]]:
    """
    Return a bar series of one observation per day from the given closes.
    """
    return [(index * DAILY_NS, close) for index, close in enumerate(closes)]


def zero_leakage() -> LeakagePolicy:
    """
    Return the declared zero policy, which applies no exclusion.
    """
    return LeakagePolicy(purge_before=0, zero_interval_justification="test")


def two_sets(n_splits: int) -> SplitContract:
    """
    Return a contract of two absolute sets with the given split count.
    """
    return SplitContract(
        sets=("train", "test"),
        lengths={"train": 100, "test": 100},
        leakage=zero_leakage(),
        n_splits=n_splits,
    )


def test_a_label_reach_the_policy_does_not_cover_is_a_capability() -> None:
    """
    Test the leakage probe reports the uncovered reach as a code and a shortfall.
    """
    series = label_series(FORWARD, daily([100.0, 101.0, 103.0, 102.0, 104.0]))
    short = LeakagePolicy(purge_before=DAILY_NS, zero_interval_justification="test")
    covering = LeakagePolicy(purge_before=2 * DAILY_NS, zero_interval_justification="test")

    refused = leakage_capability(series, short)

    assert not refused.is_available
    assert refused.code == ResearchCapabilityCode.LEAKAGE_NOT_COVERED.value
    assert refused.requirements == [f"cover {DAILY_NS} ns more before the evaluation set"]

    assert leakage_capability(series, covering).is_available


def test_a_study_below_the_contract_minimums_is_refused_by_count() -> None:
    """
    Test the significance probe reports the contract minimums before the declaration.
    """
    starved = significance_capability(5, 3)

    assert starved.code == ResearchCapabilityCode.INSUFFICIENT_OBSERVATIONS.value
    assert starved.requirements == ["15 more contributing periods"]

    few_trials = significance_capability(25, 3)

    assert few_trials.code == ResearchCapabilityCode.INSUFFICIENT_TRIALS.value
    assert few_trials.requirements == ["7 more trial estimates"]

    assert significance_capability(25, 12).is_available


def test_a_dependence_declaration_is_required_and_checked() -> None:
    """
    Test the significance probe reports an incomplete dependence declaration.
    """
    declared = TrialDependence.DEPENDENT
    undeclared = significance_capability(25, 12, dependence=declared)

    assert undeclared.code == ResearchCapabilityCode.EFFECTIVE_TRIALS_UNDECLARED.value
    assert undeclared.requirements == ["declare the effective trial count"]

    unexpected = significance_capability(25, 12, effective_trials=4)

    assert unexpected.code == ResearchCapabilityCode.EFFECTIVE_TRIALS_UNEXPECTED.value

    out_of_range = significance_capability(25, 12, dependence=declared, effective_trials=13)

    assert out_of_range.code == ResearchCapabilityCode.EFFECTIVE_TRIALS_OUT_OF_RANGE.value
    assert out_of_range.requirements == ["an effective count of at least 1 and at most 12"]

    assert significance_capability(25, 12, dependence=declared, effective_trials=4).is_available


def test_a_stricter_contract_moves_the_minimums() -> None:
    """
    Test the significance probe reads the contract it is given.
    """
    strict = StatisticalContract(minimum_observations=50, minimum_trials=40)
    refused = significance_capability(25, 12, contract=strict)

    assert refused.code == ResearchCapabilityCode.INSUFFICIENT_OBSERVATIONS.value
    assert refused.requirements == ["25 more contributing periods"]
    assert significance_capability(50, 40, contract=strict).is_available


def test_a_split_layout_the_period_cannot_hold_is_refused() -> None:
    """
    Test the split probe reports a layout the period cannot hold.
    """
    refused = split_capability(two_sets(5), start=0, end=400)

    assert not refused.is_available
    assert refused.code == ResearchCapabilityCode.SPLIT_LAYOUT_UNSATISFIABLE.value
    assert refused.requirements == ["a layout of 5 split(s) inside the period"]

    assert split_capability(two_sets(2), start=0, end=400).is_available


def test_the_research_codes_are_closed_and_canonical() -> None:
    """
    Test every research code is a canonical token.
    """
    for code in ResearchCapabilityCode:
        assert re.fullmatch(r"[A-Z][A-Z0-9_]*", code.value), code.value
        assert Capability.unavailable(code.value, "detail").code == code.value


def test_a_probe_answer_is_the_shared_capability_shape() -> None:
    """
    Test the research domain reports the shared shape rather than one of its own.
    """
    answer = significance_capability(5, 3)

    assert isinstance(answer, Capability)
    assert type(answer).__module__ == "nautilus_trader.core"
    assert type(answer).__name__ == "Capability"


def test_a_code_that_is_prose_is_refused_by_the_shape() -> None:
    """
    Test the shared shape refuses a code that is not canonical.
    """
    with pytest.raises(ValueError, match="is not canonical"):
        Capability.unavailable("two words", "detail")


def test_no_source_classifies_an_answer_by_its_detail() -> None:
    """
    Test that no source reads a capability detail, in either language.

    Only the code may be compared, matched or used for control flow. The scan covers the compiled
    and the Python sources and skips this file, whose patterns are not uses.
    """
    offenders = []
    for pattern in PROBE_SOURCES:
        for path in sorted(REPO_ROOT.glob(pattern)):
            if path.resolve() == Path(__file__).resolve():
                continue
            source = path.read_text(encoding="utf-8")
            for match in _DETAIL_MATCHES:
                found = match.findall(source)
                if found:
                    offenders.append(f"{path.relative_to(REPO_ROOT)}: {found}")

    assert not offenders, f"source classifies a capability by its detail: {offenders}"
