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
Tests for the discovery allocation contract and its cross-sectional selection.

The allocation and the selection are pure functions, so they carry the declared rules and are tested
directly and exhaustively, one case per rule, including the case that would fail if the rule were
reversed: a tie, a missing feature value, a halted or ineligible holding, both caps binding
together, a partial fill, both cash policies, fewer eligible instruments than the requested
portfolio size, and a held position leaving the selection.

The interpreter is not tested here. It is a strategy, so it runs inside an engine, and its tests
live where the repository keeps engine runs, in `tests/integration/test_discovery_round_trip.py`.
"""

from __future__ import annotations

from decimal import Decimal
from typing import Any

import pytest

from nautilus_trader.discovery.schema import SelectionBlock
from nautilus_trader.discovery.strategy import AllocationRequest
from nautilus_trader.discovery.strategy import CrossSectionRow
from nautilus_trader.discovery.strategy import allocate_targets
from nautilus_trader.discovery.strategy import rebalance_period_key
from nautilus_trader.discovery.strategy import select_cross_section
from nautilus_trader.model import Target


# The allocation fixture's declared values, named so a case reads as the decision it exercises.
A = "AAPL.XNAS"
B = "MSFT.XNAS"
C = "NVDA.XNAS"
D = "AMZN.XNAS"
OUTSIDER = "TSLA.XNAS"
FULL_WEIGHT = "1"

CASH_POLICY_LEAVE_IDLE = "leave_idle"
CASH_POLICY_REALLOCATE = "reallocate"
EQUAL = "equal"
RANK_PROPORTIONAL = "rank_proportional"
MODE_RANK = "rank"
MODE_THRESHOLD = "threshold"
MODE_NONE = "none"
TIE_FEATURE_DESC = "feature_desc"
TIE_INSTRUMENT_ID_ASC = "instrument_id_asc"

BASE_REQUEST: dict[str, Any] = {
    "ts_event": 1_700_000_000_000_000_000,
    "ts_init": 1_700_000_000_000_000_001,
    "selected": (A, B, C),
    "held": (),
    "held_weights": {},
    "ineligible": (),
    "sector_of": {},
    "weighting": EQUAL,
    "position_count": 3,
    "max_position_weight": FULL_WEIGHT,
    "max_sector_weight": None,
    "max_gross_exposure": None,
    "cash_policy": CASH_POLICY_LEAVE_IDLE,
    "flatten_ineligible": True,
}

# The clock periods the rebalance schedule's weekly and monthly period keys are read from.
SECOND_NS = 1_000_000_000
DAY_NS = 86_400 * SECOND_NS


def request(**overrides: Any) -> AllocationRequest:
    """
    Return the allocation fixture's request, with the given fields overridden.
    """
    return AllocationRequest(**{**BASE_REQUEST, **overrides})


def weights_of(targets: tuple[Target, ...]) -> dict[str, float]:
    """
    Return the returned targets' weights, by instrument identifier.
    """
    return {str(target.instrument_id): float(target.value.weight or 0.0) for target in targets}


def fields_of(targets: tuple[Target, ...]) -> tuple[tuple[str, str, float, int, int], ...]:
    """
    Return every field of every target, which is what determinism is compared by.
    """
    return tuple(
        (
            str(target.instrument_id),
            target.kind,
            float(target.value.weight or 0.0),
            target.ts_event,
            target.ts_init,
        )
        for target in targets
    )


def row(instrument_id: str, value: float, *, member: bool = True) -> CrossSectionRow:
    """
    Return one cross-section row, a member unless the case says otherwise.
    """
    return CrossSectionRow(instrument_id, value, member)


def selection_block(**overrides: Any) -> SelectionBlock:
    """
    Return a rank selection block, with the given fields overridden.
    """
    declared: dict[str, Any] = {
        "mode": MODE_RANK,
        "feature": "trend",
        "count": Decimal(2),
        "tie_break": TIE_FEATURE_DESC,
    }

    return SelectionBlock(**{**declared, **overrides})


# The nine declared allocation decisions, each with the case that would fail if it were reversed.


def test_equal_weighting_gives_every_selected_instrument_the_same_weight() -> None:
    """
    Decision 1: the declared weighting method decides how a rank becomes a weight.
    """
    targets = allocate_targets(request(weighting=EQUAL))
    weights = weights_of(targets)

    assert weights == {A: pytest.approx(1 / 3), B: pytest.approx(1 / 3), C: pytest.approx(1 / 3)}


def test_rank_proportional_weighting_follows_the_selection_order() -> None:
    """
    Decision 1, other method: the weight is proportional to `count - index`.

    The weight is read from the selection's order, so a reversed selection produces reversed
    weights, which is how the case fails if the rule were reversed.
    """
    forward = weights_of(allocate_targets(request(weighting=RANK_PROPORTIONAL)))
    reversed_selection = weights_of(
        allocate_targets(request(weighting=RANK_PROPORTIONAL, selected=(C, B, A))),
    )

    assert forward[A] == pytest.approx(3 / 6)
    assert forward[B] == pytest.approx(2 / 6)
    assert forward[C] == pytest.approx(1 / 6)
    assert reversed_selection[A] == pytest.approx(1 / 6)
    assert reversed_selection[C] == pytest.approx(3 / 6)


def test_an_unknown_weighting_method_is_refused_rather_than_defaulted() -> None:
    """
    Decision 1: a weighting method the function does not declare is refused.

    It is not silently treated as the equal method, which would be a rule no candidate declared.
    """
    with pytest.raises(ValueError, match="weighting must be one of"):
        allocate_targets(request(weighting="inverse_volatility"))


def test_the_selection_order_is_preserved_and_a_repeated_instrument_is_weighted_once() -> None:
    """
    Decision 2: ties are broken by the selection's own order, which the allocation preserves.
    """
    weights = weights_of(
        allocate_targets(
            request(weighting=RANK_PROPORTIONAL, selected=(B, A, B, C)),
        ),
    )

    assert tuple(weights) == (A, B, C)
    assert weights[B] == pytest.approx(3 / 6)
    assert weights[A] == pytest.approx(2 / 6)
    assert weights[C] == pytest.approx(1 / 6)


def test_a_selection_shorter_than_the_portfolio_size_is_not_padded() -> None:
    """
    Decision 3: the allocation never stretches the selection to fill the declared count.

    An instrument with no eligible value is not in the selection at all, so a run with one eligible
    instrument holds one position however large its declared portfolio is.
    """
    targets = allocate_targets(request(selected=(A,), position_count=5))

    assert weights_of(targets) == {A: pytest.approx(1.0)}


def test_a_selection_longer_than_the_portfolio_size_keeps_only_the_ranked_leaders() -> None:
    """
    Decision 3: the declared portfolio size is a pre-trade constraint the allocation enforces.

    The names beyond it are the ones the selection ranked last, so the leaders keep their weight.
    """
    targets = allocate_targets(
        request(selected=(C, B, A), position_count=2, weighting=RANK_PROPORTIONAL),
    )
    weights = weights_of(targets)

    assert tuple(weights) == (B, C)
    assert weights[C] == pytest.approx(2 / 3)
    assert weights[B] == pytest.approx(1 / 3)


def test_a_held_ineligible_instrument_is_flattened_when_the_rule_says_so() -> None:
    """
    Decision 4: an ineligible holding is flattened by the declared flatten rule.
    """
    targets = allocate_targets(
        request(selected=(A,), held=(A, B), ineligible=(B,), flatten_ineligible=True),
    )

    assert weights_of(targets) == {A: pytest.approx(1.0), B: 0.0}


def test_a_held_ineligible_instrument_is_left_untouched_when_the_rule_says_so() -> None:
    """
    Decision 4, the other way: with the flatten rule false the holding emits no target at all.

    A position the allocation does not name is left exactly where it is, which is what distinguishes
    the two declared resolutions of an ineligible holding.
    """
    targets = allocate_targets(
        request(selected=(A,), held=(A, B), ineligible=(B,), flatten_ineligible=False),
    )

    assert tuple(weights_of(targets)) == (A,)


def test_a_held_instrument_that_leaves_the_selection_is_flattened() -> None:
    """
    Decision 5: a held instrument that is not in the selection and not ineligible is flattened.
    """
    targets = allocate_targets(request(selected=(A, B), held=(A, B, C)))

    assert weights_of(targets) == {A: pytest.approx(0.5), B: pytest.approx(0.5), C: 0.0}


def test_an_instrument_that_is_neither_selected_nor_held_gets_no_target() -> None:
    """
    Decision 5: the allocation emits a target only for a selected or a held instrument.
    """
    targets = allocate_targets(
        request(selected=(A,), held=(B,), sector_of={OUTSIDER: "mega", A: "mega"}),
    )

    assert set(weights_of(targets)) == {A, B}


def test_the_position_cap_clamps_each_weight_and_redistributes_nothing() -> None:
    """
    Decision 6: the position cap is applied first, and the clamp itself redistributes nothing.

    The clamped weight is simply lost to the portfolio, which is why the total falls below one and
    the cash policy is what decides whether it is reinvested.
    """
    targets = allocate_targets(
        request(
            weighting=RANK_PROPORTIONAL,
            selected=(A, B, C),
            max_position_weight="0.25",
        ),
    )
    weights = weights_of(targets)

    assert weights[A] == pytest.approx(0.25)
    assert weights[B] == pytest.approx(0.25)
    assert weights[C] == pytest.approx(1 / 6)
    assert sum(weights.values()) == pytest.approx(2 / 3)


def test_the_sector_cap_scales_a_sector_proportionally() -> None:
    """
    Decision 7: the sector cap is applied second, scaling a sector's weights proportionally.

    The sector's total meets the cap, no instrument in it is left above it, and the sector of one
    is scaled to the cap as well rather than left above it.
    """
    targets = allocate_targets(
        request(
            held=(),
            sector_of={A: "mega", B: "mega", C: "chips"},
            max_sector_weight="0.25",
        ),
    )
    weights = weights_of(targets)

    assert weights[A] == pytest.approx(0.125)
    assert weights[B] == pytest.approx(0.125)
    assert weights[C] == pytest.approx(0.25)
    assert weights[A] + weights[B] == pytest.approx(0.25)


def test_an_undeclared_sector_is_the_instrument_itself_and_cannot_exceed_the_cap() -> None:
    """
    Decision 7: an instrument whose sector the request does not declare is its own sector.

    A sector of one instrument is scaled to the cap rather than left above it, so the sector cap is
    also a per-instrument cap for an instrument the mapping does not name.
    """
    targets = allocate_targets(
        request(selected=(A, B), held=(), sector_of={A: "mega"}, max_sector_weight="0.25"),
    )
    weights = weights_of(targets)

    assert weights[A] == pytest.approx(0.25)
    assert weights[B] == pytest.approx(0.25)


def test_the_gross_exposure_cap_scales_every_weight_down_proportionally() -> None:
    """
    Decision 8: the gross exposure cap is applied last, scaling every weight proportionally.

    The total meets the cap rather than exceeding it, and every weight keeps its proportion to the
    others.
    """
    targets = allocate_targets(
        request(selected=(A, B), held=(), max_gross_exposure="0.25"),
    )
    weights = weights_of(targets)

    assert weights[A] == pytest.approx(0.125)
    assert weights[B] == pytest.approx(0.125)
    assert sum(weights.values()) == pytest.approx(0.25)


def test_the_position_cap_and_the_sector_cap_bind_in_the_declared_order() -> None:
    """
    Decisions 6 and 7: the caps bind in the declared order rather than emergently.

    A sector whose clamped weights still exceed its own cap is scaled down to it, and the sector
    scaling wins inside that sector because it is applied second.
    """
    targets = allocate_targets(
        request(
            weighting=RANK_PROPORTIONAL,
            selected=(A, B, C),
            sector_of={A: "mega", B: "mega", C: "chips"},
            max_position_weight="0.25",
            max_sector_weight="0.25",
        ),
    )
    weights = weights_of(targets)

    assert weights[A] == pytest.approx(0.125)
    assert weights[B] == pytest.approx(0.125)
    assert weights[A] + weights[B] == pytest.approx(0.25)
    assert weights[C] == pytest.approx(1 / 6)
    assert weights[A] < weights[C]


def test_the_position_and_gross_caps_both_binding_scale_the_clamped_weights() -> None:
    """
    Decisions 6 and 8: a gross exposure cap tighter than the clamped total is applied last.

    It scales the weights the position cap already produced, rather than replacing either cap.
    """
    targets = allocate_targets(
        request(
            weighting=RANK_PROPORTIONAL,
            selected=(A, B, C),
            max_position_weight="0.25",
            max_gross_exposure="0.5",
        ),
    )
    weights = weights_of(targets)

    assert weights[A] == pytest.approx(0.1875)
    assert weights[B] == pytest.approx(0.1875)
    assert weights[C] == pytest.approx(0.125)
    assert sum(weights.values()) == pytest.approx(0.5)


def test_leave_idle_leaves_the_capped_residual_uninvested() -> None:
    """
    Decision 9: the leave-idle policy does not redistribute the weight the caps left unused.
    """
    targets = allocate_targets(
        request(
            weighting=RANK_PROPORTIONAL,
            selected=(A, B, C, D),
            position_count=4,
            max_position_weight="0.25",
            cash_policy=CASH_POLICY_LEAVE_IDLE,
        ),
    )
    weights = weights_of(targets)

    assert weights[A] == pytest.approx(0.25)
    assert weights[B] == pytest.approx(0.25)
    assert weights[C] == pytest.approx(0.2)
    assert weights[D] == pytest.approx(0.1)


def test_reallocate_redistributes_the_residual_without_exceeding_a_cap() -> None:
    """
    Decision 9, other policy: the residual the caps left is redistributed.

    It goes over the instruments still below the caps, proportionally to their declared share, and
    never pushes any of them above a cap: here four names all reach the position cap and the
    portfolio is fully invested.
    """
    targets = allocate_targets(
        request(
            weighting=RANK_PROPORTIONAL,
            selected=(A, B, C, D),
            position_count=4,
            max_position_weight="0.25",
            cash_policy=CASH_POLICY_REALLOCATE,
        ),
    )
    weights = weights_of(targets)

    assert set(weights) == {A, B, C, D}
    assert all(weight == pytest.approx(0.25) for weight in weights.values())
    assert sum(weights.values()) == pytest.approx(1.0)


def test_reallocate_never_exceeds_a_cap_when_no_instrument_has_headroom() -> None:
    """
    Decision 9: a residual nothing can absorb is left uninvested rather than pushed above a cap.
    """
    targets = allocate_targets(
        request(
            weighting=EQUAL,
            selected=(A, B),
            position_count=2,
            max_position_weight="0.25",
            cash_policy=CASH_POLICY_REALLOCATE,
        ),
    )

    assert weights_of(targets) == {A: pytest.approx(0.25), B: pytest.approx(0.25)}


def test_an_unknown_cash_policy_is_refused_rather_than_defaulted() -> None:
    """
    Decision 9: a cash policy the function does not declare is refused.

    It is not silently treated as leave-idle, which would be a rule no candidate declared.
    """
    with pytest.raises(ValueError, match="cash_policy must be one of"):
        allocate_targets(request(cash_policy="sweep_to_cash"))


def test_a_partial_fill_does_not_change_the_next_allocation() -> None:
    """
    The declared default for a partial or failed fill: the allocation recomputes the selection.

    It is not read from the weight the fill happened to reach, so a position that is only partly
    filled emits the same targets as a fully filled one and the platform tops it up.
    """
    full = request(
        selected=(A, B),
        held=(A, B),
        held_weights={A: 0.5, B: 0.5},
        position_count=2,
    )
    partial = request(
        selected=(A, B),
        held=(A, B),
        held_weights={A: 0.05, B: 0.0},
        position_count=2,
    )

    assert fields_of(allocate_targets(full)) == fields_of(allocate_targets(partial))


def test_the_allocation_is_deterministic_and_mutates_nothing_it_was_given() -> None:
    """
    Criterion 21: the same request produces the same targets, in identity-of-fields terms.

    No path mutates a target, which is why the second call's targets are distinct objects carrying
    the same fields, and no path mutates the request either.
    """
    held_weights = {A: 0.3, B: 0.1}
    sectors = {A: "mega", B: "mega", C: "chips"}
    allocation = request(
        weighting=RANK_PROPORTIONAL,
        selected=(A, B, C, OUTSIDER),
        position_count=4,
        held=(A, B),
        held_weights=held_weights,
        ineligible=(B,),
        sector_of=sectors,
        max_position_weight="0.25",
        max_sector_weight="0.3",
        max_gross_exposure="0.5",
        cash_policy=CASH_POLICY_REALLOCATE,
    )

    first = allocate_targets(allocation)
    second = allocate_targets(allocation)

    assert fields_of(first) == fields_of(second)
    assert first[0] is not second[0]
    assert fields_of(first) == fields_of(allocate_targets(allocation))
    assert dict(allocation.held_weights) == held_weights
    assert dict(allocation.sector_of) == sectors
    assert [field[0] for field in fields_of(first)] == sorted(
        field[0] for field in fields_of(first)
    )
    assert all(field[1] == "WEIGHT" for field in fields_of(first))
    assert all(field[3] == allocation.ts_event for field in fields_of(first))
    assert all(field[4] == allocation.ts_init for field in fields_of(first))


# The selection contract: membership, the declared ordering, the bound, and the three modes.


def test_a_row_that_is_not_a_member_is_never_selected() -> None:
    """
    The membership rule: a non-member is not selected even when its value ranks first.
    """
    rows = (row(A, 9.0, member=False), row(B, 5.0), row(C, 1.0))

    assert select_cross_section(rows, selection_block()) == (B, C)


def test_a_rank_selection_takes_the_declared_count_by_value_descending() -> None:
    """
    The rank mode: the top `count` rows, highest value first.
    """
    rows = (row(A, 1.0), row(B, 3.0), row(C, 2.0))

    assert select_cross_section(rows, selection_block(count=Decimal(2))) == (B, C)


def test_a_rank_selection_takes_the_declared_percentile_rounded_up() -> None:
    """
    The rank mode: a percentile of n gives at least one row and at most n, rounded up.
    """
    rows = (row(A, 4.0), row(B, 3.0), row(C, 2.0), row(D, 1.0))

    def take(share: str) -> tuple[str, ...]:
        """
        Return the selection the percentile takes, with the count bound left undeclared.
        """
        return select_cross_section(rows, selection_block(count=None, percentile=Decimal(share)))

    assert take("0.5") == (A, B)
    assert take("0.4") == (A, B)
    assert take("0.01") == (A,)
    assert take("1") == (A, B, C, D)


def test_a_rank_selection_takes_no_more_rows_than_are_eligible() -> None:
    """
    The rank mode: a bound larger than the eligible set takes the eligible set.
    """
    rows = (row(A, 1.0), row(B, 2.0))

    assert select_cross_section(rows, selection_block(count=Decimal(5))) == (B, A)


def test_a_tie_is_broken_by_the_declared_rule() -> None:
    """
    The tie rule: the feature's order resolves equal values by instrument identifier ascending.

    The case fails if the rule is reversed, because a reversed resolution would return the tied
    identifiers in descending order.
    """
    rows = (row(C, 5.0), row(A, 5.0), row(B, 5.0))

    assert select_cross_section(rows, selection_block(count=Decimal(2))) == (A, B)
    assert select_cross_section(rows, selection_block(count=Decimal(3))) == (A, B, C)


def test_the_identifier_tie_rule_orders_by_the_identifier_alone() -> None:
    """
    The other declared tie rule: the ranking is the declared identifier order.

    A rank selection that declares it therefore takes the identifiers that come first rather than
    the values that rank first, which is what the rule states.
    """
    rows = (row(C, 9.0), row(A, 1.0), row(B, 5.0))
    selection = selection_block(count=Decimal(2), tie_break=TIE_INSTRUMENT_ID_ASC)

    assert select_cross_section(rows, selection) == (A, B)


def test_a_threshold_selection_takes_the_rows_that_satisfy_the_comparison() -> None:
    """
    The threshold mode: the rows whose value satisfies the declared comparison.

    They are ordered by value descending, and a row that is not a member is excluded however it
    compares.
    """
    rows = (row(A, 1.0), row(B, 3.0, member=False), row(C, 2.0), row(D, 3.0))

    assert select_cross_section(
        rows, selection_block(mode=MODE_THRESHOLD, comparison="gt", threshold=Decimal(1))
    ) == (D, C)
    assert select_cross_section(
        rows, selection_block(mode=MODE_THRESHOLD, comparison="ge", threshold=Decimal(3))
    ) == (D,)
    assert select_cross_section(
        rows, selection_block(mode=MODE_THRESHOLD, comparison="lt", threshold=Decimal(2))
    ) == (A,)


def test_a_selection_that_ranks_nothing_takes_every_member_by_identifier() -> None:
    """
    The none mode: every eligible row, ordered by instrument identifier ascending.
    """
    rows = (row(C, 9.0), row(A, 1.0), row(B, 5.0, member=False))

    assert select_cross_section(rows, selection_block(mode=MODE_NONE)) == (A, C)


def test_an_undeclared_mode_tie_rule_comparison_or_bound_is_refused() -> None:
    """
    The selection contract is closed: a value the function does not declare is refused.

    A mode, a tie rule, a comparison or a bound the function does not know is not defaulted to one
    it does, because that default would be a rule no candidate declared.
    """
    rows = (row(A, 1.0),)

    with pytest.raises(ValueError, match="selection mode must be one of"):
        select_cross_section(rows, selection_block(mode="bottom"))
    with pytest.raises(ValueError, match="tie_break must be one of"):
        select_cross_section(rows, selection_block(tie_break="random"))
    with pytest.raises(ValueError, match="comparison must be one of"):
        select_cross_section(
            rows,
            selection_block(mode=MODE_THRESHOLD, comparison="approx", threshold=Decimal(1)),
        )
    with pytest.raises(ValueError, match="requires a comparison and a threshold"):
        select_cross_section(rows, selection_block(mode=MODE_THRESHOLD))
    with pytest.raises(ValueError, match="requires a count or a percentile"):
        select_cross_section(rows, selection_block(count=None, percentile=None))


def test_the_rebalance_period_is_the_declared_frequency_s_period() -> None:
    """
    The rebalance schedule: every bar rebalances under `daily`.

    A periodic frequency groups the decision bars by the clock's own year and week or year and
    month, so the first bar of a period is the bar that rebalances.
    """
    monday = 1_704_067_200_000_000_000
    friday = monday + 4 * DAY_NS
    next_monday = monday + 7 * DAY_NS
    next_month = 1_706_745_600_000_000_000

    assert rebalance_period_key(monday, "daily") is None
    assert rebalance_period_key(monday, "weekly") == rebalance_period_key(friday, "weekly")
    assert rebalance_period_key(monday, "weekly") != rebalance_period_key(next_monday, "weekly")
    assert rebalance_period_key(monday, "monthly") == rebalance_period_key(friday, "monthly")
    assert rebalance_period_key(monday, "monthly") != rebalance_period_key(next_month, "monthly")
    assert rebalance_period_key(monday, "monthly") == (2024, 1)

    with pytest.raises(ValueError, match="rebalance frequency must be one of"):
        rebalance_period_key(monday, "quarterly")
