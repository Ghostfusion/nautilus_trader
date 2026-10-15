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
The interpreter: the only executor a compiled candidate runs under, and the allocation contract.

Design Section 9.3 fixes the first half. A compiled candidate is a specification, and the thing the
engine is handed is a live `Strategy` object configured from it, so no import path, no generated
module and no second execution path is involved: what ran is the thing that was hashed. Everything
the platform already owns stays the platform's - the target pipeline resolves the signals this
interpreter submits to quantities, reconciles them against the position and the resting orders, and
emits the orders, while the risk engine applies its own caps.

Design Section 9.7 fixes the second half, and it is new functionality rather than wiring: turning a
cross-sectional ranking into target weights. The nine decisions it requires are resolved here, in
`allocate_targets`, and every one of them is exercised by this module's tests.

1. A rank becomes a weight by the declared method: `equal` gives every selected instrument the same
   weight, and `rank_proportional` gives weight proportional to `count - index`, in the order the
   selection produced.
2. Ties are broken by the selection, which is ordered: the allocation preserves that order for
   weighting, for the caps and for the redistribution of decision 9, and an instrument the selection
   names twice is weighted once, at its first position.
3. An instrument whose feature value was missing is not in the selection at all, so it can never
   receive a weight. The allocation never pads the selection to `position_count`, and it does not
   exceed it either: the declared portfolio size is a pre-trade constraint of Section 9.2, which the
   allocation enforces, so the names beyond it are the ones the selection ranked last.
4. A held instrument that is ineligible is flattened with a zero-weight target when
   `flatten_ineligible` is true, and is left untouched (no target is emitted for it) when it is
   false.
5. A held instrument that is not in the selection and not ineligible is flattened with a zero-weight
   target, which is the declared rule for a position leaving the selected set.
6. `max_position_weight` is applied first and clamps each weight, and the clamp itself redistributes
   nothing.
7. `max_sector_weight` is applied second, scaling every weight in a sector proportionally so the
   sector's total does not exceed it. An instrument whose sector the request does not declare is its
   own sector, so a single instrument cannot exceed the sector cap either.
8. `max_gross_exposure` is applied last, scaling every weight down proportionally when the total
   exceeds it. The investable ceiling is one unit of equity, or the declared gross exposure where
   that is smaller; what a cap leaves below the ceiling is the residual the cash policy decides on.
9. `cash_policy == "reallocate"` redistributes that residual proportionally, by the declared share
   rather than by the current weight, over the instruments still below the caps, in a loop bounded
   by the number of instruments, and it never exceeds a cap. `cash_policy == "leave_idle"` leaves
   the residual uninvested.

Every weight is computed in `Decimal` from the request's text fields and converted to `float` only
at the `Target` boundary, because a price and a weight are discrete values and binary floating point
is not. The function reads its arguments and mutates nothing it was given, and the same request
always produces the same tuple, ordered by instrument identifier ascending.

The interpreter's own contract, which follows the same sections:

- One `FeatureStream` per instrument is updated on every bar, and every stream's snapshot at the
  decision instant is the cross-section a decision reads.
- Exits are evaluated on every decision bar, before entries: the holding-period cap counted in the
  declared signal frequency's decision periods since entry, the rank exit against the current
  ranking when `rank_exit_count` is declared, the declared feature exit, and a close-evaluated stop
  or target against the current close.
- On a rebalance bar - every bar for `daily`, and the first decision bar of the period for `weekly`
  and `monthly` - the cross-section is selected, the selection is allocated, and one batch is
  submitted. Exit signals are never suppressed; `risk.runtime.halt_new_entries` suppresses entries
  only.
- The selection mode `none` ranks nothing and reads the universe membership instead, which is what
  the compiler's own check of that mode requires.
- An entry block's declared conditions gate entry: an instrument whose snapshot does not satisfy
  them is not allocated a weight.
- The candidate's declared weight is realised as a signal strength, `weight / max_position_weight`
  clamped to `[0, 1]`. The pipeline sizes a position from risk, so `risk_per_trade` is passed as the
  candidate's own position cap, while the strength carries the declared weight.
"""

from __future__ import annotations

from dataclasses import dataclass
from datetime import UTC
from datetime import datetime
from decimal import ROUND_CEILING
from decimal import ROUND_FLOOR
from decimal import Decimal
from typing import TYPE_CHECKING

from nautilus_trader.discovery.features import FeatureRegistry
from nautilus_trader.discovery.features import FeatureStream
from nautilus_trader.discovery.schema import ConditionAll
from nautilus_trader.discovery.schema import ConditionLeaf
from nautilus_trader.discovery.schema import SelectionBlock
from nautilus_trader.model import BarType
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import SignalDirection
from nautilus_trader.model import Target
from nautilus_trader.model import TradingSignal
from nautilus_trader.model import Venue
from nautilus_trader.trading import Strategy
from nautilus_trader.trading import StrategyConfig
from nautilus_trader.trading import TargetPipelineConfig


if TYPE_CHECKING:
    from collections.abc import Mapping
    from collections.abc import Sequence

    from nautilus_trader.discovery.compile import CompiledPlan
    from nautilus_trader.discovery.features import FeatureValueAt
    from nautilus_trader.discovery.schema import ConditionNode
    from nautilus_trader.model import Bar
    from nautilus_trader.model import OrderFilled
    from nautilus_trader.research import MembershipSeries


# The declared values this module resolves, each a member of a closed set the pre-registered policy
# declares. A value outside them is refused rather than defaulted, because a default would be a rule
# no candidate declared.
SELECTION_MODE_RANK = "rank"
SELECTION_MODE_THRESHOLD = "threshold"
SELECTION_MODE_NONE = "none"

TIE_BREAK_FEATURE_DESC = "feature_desc"
TIE_BREAK_INSTRUMENT_ID_ASC = "instrument_id_asc"

WEIGHTING_EQUAL = "equal"
WEIGHTING_RANK_PROPORTIONAL = "rank_proportional"

CASH_POLICY_LEAVE_IDLE = "leave_idle"
CASH_POLICY_REALLOCATE = "reallocate"

REBALANCE_DAILY = "daily"
REBALANCE_WEEKLY = "weekly"
REBALANCE_MONTHLY = "monthly"

COMPARISON_GT = "gt"
COMPARISON_GE = "ge"
COMPARISON_LT = "lt"
COMPARISON_LE = "le"

# The source every signal this interpreter submits is attributed to, so a fill can be traced back to
# the executor rather than to a strategy the run did not configure.
SIGNAL_SOURCE = "discovery"

# The repository bar type specification each declared signal frequency is read at. The first release
# declares daily signals (decision SD12), and a frequency this table does not hold is refused rather
# than approximated by a bar size it did not declare.
BAR_SPECIFICATION_BY_SIGNAL_FREQUENCY: dict[str, str] = {
    "daily": "1-DAY-LAST-EXTERNAL",
}

# One unit of weight: the ceiling a weighted allocation reaches when no tighter gross exposure is
# declared, and the share an equal weighting gives every selected instrument.
_UNIT = Decimal(1)

_NS_PER_SECOND = 1_000_000_000

_SELECTION_MODES = (SELECTION_MODE_RANK, SELECTION_MODE_THRESHOLD, SELECTION_MODE_NONE)
_TIE_BREAKS = (TIE_BREAK_FEATURE_DESC, TIE_BREAK_INSTRUMENT_ID_ASC)
_WEIGHTINGS = (WEIGHTING_EQUAL, WEIGHTING_RANK_PROPORTIONAL)
_CASH_POLICIES = (CASH_POLICY_LEAVE_IDLE, CASH_POLICY_REALLOCATE)
_REBALANCE_FREQUENCIES = (REBALANCE_DAILY, REBALANCE_WEEKLY, REBALANCE_MONTHLY)
_COMPARISONS = (COMPARISON_GT, COMPARISON_GE, COMPARISON_LT, COMPARISON_LE)


@dataclass(frozen=True)
class AllocationRequest:
    """
    Everything one allocation is computed from, as plain data.

    The fields are the state the platform holds and the decisions the candidate declared, so the
    allocation needs no engine, no cache and no clock, and the same request always produces the same
    targets. The weights and caps are text rather than `Decimal` so the request stays plain data
    that can be recorded, compared and reconstructed.

    Parameters
    ----------
    ts_event : int
        The event timestamp of the decision the targets are stamped with.
    ts_init : int
        The initialization timestamp of the decision the targets are stamped with.
    selected : tuple[str, ...]
        The instrument identifiers the selection produced, in the order it produced them.
    held : tuple[str, ...]
        The instrument identifiers currently held.
    held_weights : Mapping[str, float]
        The weight each held instrument is carried at, marked at the decision's close. No declared
        rule reads it, because the allocation recomputes from the selection rather than from the
        current weight, and it is carried so a partial or failed fill cannot change the next target.
    ineligible : tuple[str, ...]
        The held instruments that are no longer eligible.
    sector_of : Mapping[str, str]
        The instrument identifier to sector mapping, or empty when no sector is declared.
    weighting : str
        The declared weighting method, `equal` or `rank_proportional`.
    position_count : int
        The declared portfolio size the allocation weights at most.
    max_position_weight : str
        The declared per-position cap, as `Decimal` text.
    max_sector_weight : str, optional
        The declared per-sector cap, as `Decimal` text.
    max_gross_exposure : str, optional
        The declared gross exposure cap, as `Decimal` text.
    cash_policy : str
        The declared cash policy, `leave_idle` or `reallocate`.
    flatten_ineligible : bool
        Whether a held ineligible instrument is flattened rather than left untouched.

    """

    ts_event: int
    ts_init: int
    selected: tuple[str, ...]
    held: tuple[str, ...]
    held_weights: Mapping[str, float]
    ineligible: tuple[str, ...]
    sector_of: Mapping[str, str]
    weighting: str
    position_count: int
    max_position_weight: str
    max_sector_weight: str | None
    max_gross_exposure: str | None
    cash_policy: str
    flatten_ineligible: bool


@dataclass(frozen=True)
class _Caps:
    """
    The declared caps, and the mapping the sector cap is applied through.

    The binding order the design fixes is the order the allocation applies them in, so they are
    carried together rather than passed one at a time.

    Parameters
    ----------
    ceiling : Decimal
        The total weight the caps admit.
    position : Decimal
        The declared per-position cap.
    sector : Decimal, optional
        The declared per-sector cap.
    sectors : Mapping[str, str]
        The instrument identifier to sector mapping.

    """

    ceiling: Decimal
    position: Decimal
    sector: Decimal | None
    sectors: Mapping[str, str]

    def sector_of(self, instrument_id: str) -> str:
        """
        Return the instrument's declared sector, or the instrument itself when none is declared.
        """
        return self.sectors.get(instrument_id, instrument_id)

    def sector_totals(self, weights: Mapping[str, Decimal]) -> dict[str, Decimal]:
        """
        Return each declared sector's total weight.

        Parameters
        ----------
        weights : Mapping[str, Decimal]
            The weights to total.

        Returns
        -------
        dict[str, Decimal]

        """
        totals: dict[str, Decimal] = {}
        for instrument_id, weight in weights.items():
            sector = self.sector_of(instrument_id)
            totals[sector] = totals.get(sector, Decimal(0)) + weight

        return totals

    def headroom(self, weights: Mapping[str, Decimal]) -> dict[str, Decimal]:
        """
        Return how much more each instrument may be weighted before it reaches a cap.

        Parameters
        ----------
        weights : Mapping[str, Decimal]
            The current weights.

        Returns
        -------
        dict[str, Decimal]

        """
        room = {
            instrument_id: max(self.position - weight, Decimal(0))
            for instrument_id, weight in weights.items()
        }
        if self.sector is None:
            return room

        totals = self.sector_totals(weights)
        for instrument_id, value in room.items():
            remaining = max(self.sector - totals[self.sector_of(instrument_id)], Decimal(0))
            room[instrument_id] = min(value, remaining)

        return room


def allocate_targets(request: AllocationRequest) -> tuple[Target, ...]:
    """
    Turn a selection and the current positions into the targets the candidate declares.

    The nine declared decisions are resolved in the order the design fixes them: the declared
    weighting method first, then the position cap, then the sector cap, then the gross exposure cap,
    and finally the cash policy's treatment of the weight those caps leave unused. Every weight is a
    `Decimal` until the `Target` boundary.

    Parameters
    ----------
    request : AllocationRequest
        The selection, the current positions, the declared caps and the declared cash policy.

    Returns
    -------
    tuple[Target, ...]
        The selected instruments with a positive weight and the flattened instruments with a
        zero weight, ordered by instrument identifier ascending.

    Raises
    ------
    ValueError
        If the weighting method or the cash policy is not one this function declares.

    """
    caps = _Caps(
        ceiling=_investable_ceiling(_decimal_of_text(request.max_gross_exposure)),
        position=Decimal(request.max_position_weight),
        sector=_decimal_of_text(request.max_sector_weight),
        sectors=request.sector_of,
    )

    order = _selected_order(request.selected, request.position_count)
    weights, shares = _initial_weights(order, request.weighting)
    weights = _apply_position_cap(weights, caps)
    if caps.sector is not None:
        weights = _apply_sector_cap(weights, caps)
    weights = _apply_gross_cap(weights, caps)
    if request.cash_policy == CASH_POLICY_REALLOCATE:
        weights = _reallocate(weights, shares, caps)
    elif request.cash_policy != CASH_POLICY_LEAVE_IDLE:
        raise ValueError(
            f"cash_policy must be one of {', '.join(_CASH_POLICIES)}, was {request.cash_policy}",
        )

    return tuple(
        Target.from_weight(
            InstrumentId.from_str(instrument_id),
            float(weight),
            request.ts_event,
            request.ts_init,
        )
        for instrument_id, weight in _emitted_weights(weights, request)
    )


@dataclass(frozen=True)
class CrossSectionRow:
    """
    One instrument's ranking value at one decision instant.

    Parameters
    ----------
    instrument_id : str
        The instrument identifier.
    value : float
        The ranking feature's value, which the caller read rather than recomputed.
    member : bool
        Whether the instrument was a member of the universe at the decision instant. A row that is
        not a member is never selected.

    """

    instrument_id: str
    value: float
    member: bool


def select_cross_section(
    rows: Sequence[CrossSectionRow],
    selection: SelectionBlock,
) -> tuple[str, ...]:
    """
    Turn one decision instant's cross-section into the ordered selection.

    The selection block is the schema's own declared type rather than a second set of parameters
    spelling out the same fields, and its `feature` names the alias the caller already resolved into
    `rows`: this function ranks values, it does not read a snapshot.

    Parameters
    ----------
    rows : Sequence[CrossSectionRow]
        The rows the cross-section produced, one per instrument the ranking feature had a value for.
    selection : SelectionBlock
        The declared selection: its mode, its ranking feature, its bound, its tie rule, and, for the
        threshold mode, its comparison and threshold.

    Returns
    -------
    tuple[str, ...]
        The selected instrument identifiers, in the declared order.

    Raises
    ------
    ValueError
        If the mode, the tie rule, the comparison or the bound is not usable.

    """
    eligible = tuple(row for row in rows if row.member)

    if selection.mode == SELECTION_MODE_NONE:
        return tuple(sorted(row.instrument_id for row in eligible))

    if selection.mode == SELECTION_MODE_THRESHOLD:
        ordered = _threshold_rows(eligible, selection)
        return tuple(row.instrument_id for row in ordered)

    if selection.mode != SELECTION_MODE_RANK:
        raise ValueError(
            f"selection mode must be one of {', '.join(_SELECTION_MODES)}, was {selection.mode}",
        )

    ranked = _ranked_rows(eligible, selection.tie_break)
    size = _selection_size(len(ranked), selection)
    return tuple(row.instrument_id for row in ranked[:size])


def rebalance_period_key(ts_ns: int, frequency: str) -> tuple[int, int] | None:
    """
    Return the period a decision instant belongs to, for a periodic rebalance frequency.

    A weekly rebalance is keyed by its ISO year and week and a monthly one by its year and month, so
    the rebalance bar is the first decision bar whose key differs from the previous decision's. The
    instant is the decision's own, never the wall clock's, and `daily` has no period because every
    decision bar rebalances.

    Parameters
    ----------
    ts_ns : int
        The decision instant, in nanoseconds since the epoch.
    frequency : str
        The declared rebalance frequency.

    Returns
    -------
    tuple[int, int] or None
        The period key, or nothing when every decision bar rebalances.

    Raises
    ------
    ValueError
        If the frequency is not one this function declares.

    """
    if frequency == REBALANCE_DAILY:
        return None

    moment = datetime.fromtimestamp(ts_ns // _NS_PER_SECOND, tz=UTC)

    if frequency == REBALANCE_WEEKLY:
        calendar = moment.isocalendar()

        return (calendar.year, calendar.week)

    if frequency == REBALANCE_MONTHLY:
        return (moment.year, moment.month)

    raise ValueError(
        f"rebalance frequency must be one of {', '.join(_REBALANCE_FREQUENCIES)}, was {frequency}",
    )


def _decision_period_key(ts_ns: int) -> tuple[int, int, int]:
    """
    Return the decision period a decision instant belongs to, as a UTC calendar date.

    The first release declares daily signal frequencies, so a decision period is a calendar day and
    a holding-period cap declared in periods is a number of sessions rather than a number of bars.

    Parameters
    ----------
    ts_ns : int
        The decision instant, in nanoseconds since the epoch.

    Returns
    -------
    tuple[int, int, int]

    """
    moment = datetime.fromtimestamp(ts_ns // _NS_PER_SECOND, tz=UTC)

    return (moment.year, moment.month, moment.day)


@dataclass(frozen=True)
class CrossSectionEntry:
    """
    One feature value a decision read, which is the cross-section evidence of criterion 19.

    Parameters
    ----------
    instrument_id : str
        The instrument the value describes.
    alias : str
        The alias the specification declared the feature under.
    value : float
        The value that was read.
    observed_at : int
        The observation instant of the value.
    available_at : int
        The earliest instant the value was usable.
    member : bool
        Whether the instrument was a member of the universe at the decision instant.

    """

    instrument_id: str
    alias: str
    value: float
    observed_at: int
    available_at: int
    member: bool


@dataclass(frozen=True)
class SignalRecord:
    """
    One signal a decision submitted.

    Parameters
    ----------
    instrument_id : str
        The instrument the signal names.
    direction : str
        The signal direction's name, as the platform's own enum spells it.
    strength : float
        The strength the signal carried, which is the candidate's weight as a fraction of its cap.
    ts_event : int
        The signal's event timestamp.
    ts_init : int
        The signal's initialization timestamp.

    """

    instrument_id: str
    direction: str
    strength: float
    ts_event: int
    ts_init: int


@dataclass(frozen=True)
class FillRecord:
    """
    One fill the engine reported back to the interpreter.

    Parameters
    ----------
    client_order_id : str
        The client order id the fill is against, which is the id `submit_signals` returned.
    instrument_id : str
        The instrument that filled.
    decision_at : int, optional
        The event timestamp of the decision that produced the order, or nothing when the fill cannot
        be attributed to one this interpreter submitted.
    ts_event : int
        The fill's event timestamp.
    ts_init : int
        The fill's initialization timestamp.
    last_px : str
        The fill price, as the engine reported it.

    """

    client_order_id: str
    instrument_id: str
    decision_at: int | None
    ts_event: int
    ts_init: int
    last_px: str


@dataclass(frozen=True)
class DecisionRecord:
    """
    Everything one decision read, decided and submitted.

    Parameters
    ----------
    decision_at : int
        The decision bar's event timestamp.
    ts_init : int
        The decision bar's initialization timestamp.
    rebalance : bool
        Whether the bar was a rebalance bar for the declared frequency.
    exits : tuple[str, ...]
        The held instruments the exits selected.
    cross_section : tuple[CrossSectionEntry, ...]
        The values the decision read, one entry per instrument and alias.
    selection : tuple[str, ...]
        The declared selection, before the exits and the entry conditions are applied.
    targets : tuple[Target, ...]
        The targets the allocation produced, in instrument-identifier order.
    signals : tuple[SignalRecord, ...]
        The signals the decision submitted, in the order they were submitted.
    client_order_ids : tuple[str, ...]
        The client order ids `submit_signals` returned, in the order the orders were emitted.

    """

    decision_at: int
    ts_init: int
    rebalance: bool
    exits: tuple[str, ...]
    cross_section: tuple[CrossSectionEntry, ...]
    selection: tuple[str, ...]
    targets: tuple[Target, ...]
    signals: tuple[SignalRecord, ...]
    client_order_ids: tuple[str, ...]


@dataclass(frozen=True)
class InterpreterRunInputs:
    """
    The run inputs one interpreter is configured from.

    A parameter object rather than a constructor's worth of loose arguments: these are exactly the
    things one run of one compiled plan needs, and nothing here is read from a global or a default.

    Parameters
    ----------
    plan : CompiledPlan
        The compiled plan that runs: its specification, and the definitions its features resolved
        into.
    memberships : MembershipSeries
        The point-in-time membership series the universe is read from.
    instrument_ids : tuple[str, ...]
        The instrument identifiers the run subscribes to and decides over.
    venue : Venue
        The venue the run's instruments trade on, which is the one account the positions and the
        equity are read from.
    commission_rate : Decimal
        The commission rate the cost policy declares, passed through to the pipeline's sizing.
    stop_loss_bps : int
        The stop distance in basis points the pipeline derives a position size from.
    sectors : tuple[tuple[str, str], ...], default ()
        The policy's instrument to sector mapping, which the declared sector cap is applied
        through.

    """

    plan: CompiledPlan
    memberships: MembershipSeries
    instrument_ids: tuple[str, ...]
    venue: Venue
    commission_rate: Decimal
    stop_loss_bps: int
    sectors: tuple[tuple[str, str], ...] = ()


class DiscoveryInterpreterConfig(StrategyConfig):
    """
    Configure the interpreter from one run's inputs.

    Parameters
    ----------
    inputs : InterpreterRunInputs
        The compiled plan and the run inputs the interpreter executes.

    """

    def __init__(self, *, inputs: InterpreterRunInputs) -> None:
        """
        Initialize the configuration.

        The run inputs are keyword-only because the repository's other strategy configurations are
        built that way: the platform's own configuration constructor reads a positional argument as
        a strategy identity, so a configuration takes its own fields by name.

        Parameters
        ----------
        inputs : InterpreterRunInputs
            The compiled plan and the run inputs the interpreter executes.

        """
        super().__init__()
        self.inputs = inputs


class DiscoveryInterpreter(Strategy):
    """
    Execute one compiled plan: the only executor the candidate language has.

    The interpreter reads the specification it was given, drives one feature stream per instrument,
    and submits signal batches through the platform's target pipeline. It decides nothing the
    specification does not declare: the selection, the allocation, the exits and the rebalance
    schedule all come from the compiled plan, and every decision it makes is recorded so a fixture
    can read the cross-section, the selection, the targets, the signals and the fills the engine
    produced rather than an assertion about them.

    Attributes
    ----------
    decisions : list[DecisionRecord]
        One record per decision bar, appended in the order the decisions were made.
    fills : list[FillRecord]
        One record per fill the engine reported back to the interpreter.

    """

    def __init__(self, config: DiscoveryInterpreterConfig) -> None:
        """
        Initialize the interpreter from its run inputs.
        """
        super().__init__(config)
        inputs = config.inputs
        specification = inputs.plan.specification
        self._specification = specification
        self._memberships = inputs.memberships
        self._instrument_ids = tuple(inputs.instrument_ids)
        self._venue = inputs.venue
        self._commission_rate = inputs.commission_rate
        self._stop_loss_bps = inputs.stop_loss_bps
        self._sectors = dict(inputs.sectors)

        # The candidate's own position cap is the pipeline's risk budget, and the declared weight is
        # realised as a signal strength: the pipeline sizes a position from risk, so what it needs
        # from the candidate is the risk it may take, not the weight it wants.
        self._max_position_weight = specification.portfolio.max_position_weight
        self._halt_new_entries = specification.risk.runtime.halt_new_entries
        # Two blocks may declare the flatten of an ineligible holding, the exit rules and the
        # runtime controls, and either declaration flattens it.
        self._flatten_ineligible = (
            specification.exit.flatten_ineligible or specification.risk.runtime.flatten_ineligible
        )

        registry = FeatureRegistry(inputs.plan.definitions)
        declarations = specification.features.declarations
        self._streams = {
            instrument_id: FeatureStream(
                declarations,
                registry,
                inputs.memberships,
                InstrumentId.from_str(instrument_id),
            )
            for instrument_id in self._instrument_ids
        }
        self._bar_types = {
            instrument_id: _bar_type_for(instrument_id, specification.data.signal_frequency)
            for instrument_id in self._instrument_ids
        }

        # The decision period each holding was entered in, which is what the holding-period cap
        # counts. A period is the declared signal frequency's period - a UTC calendar day for the
        # first release's daily frequency - and not a bar event, because a cross-section delivers
        # one bar per instrument per period, so a cap stated in periods must not expire part-way
        # through one.
        self._period_ordinal = 0
        self._last_period_day: tuple[int, int, int] | None = None
        self._last_period: tuple[int, int] | None = None
        self._entry_period: dict[str, int] = {}
        self._last_close: dict[str, Decimal] = {}
        self._submitted_at: dict[str, int] = {}
        self.decisions: list[DecisionRecord] = []
        self.fills: list[FillRecord] = []

    def on_start(self) -> None:
        """
        Subscribe to every instrument's bars and enable the target pipeline.
        """
        for bar_type in self._bar_types.values():
            self.subscribe_bars(bar_type)

        self.enable_target_pipeline(
            TargetPipelineConfig(
                risk_per_trade=self._max_position_weight,
                stop_loss_bps=self._stop_loss_bps,
                max_weight=self._max_position_weight,
                commission_rate=self._commission_rate,
            ),
        )

    def on_bar(self, bar: Bar) -> None:
        """
        Update the bar's instrument's stream, then make the decision.

        Parameters
        ----------
        bar : Bar
            The bar that arrived.

        """
        instrument_id = str(bar.bar_type.instrument_id)
        stream = self._streams.get(instrument_id)
        if stream is None:
            return

        stream.update(bar)
        self._last_close[instrument_id] = bar.close.as_decimal()
        self._advance_period(bar.ts_event)
        self._decide(bar)

    def on_order_filled(self, event: OrderFilled) -> None:
        """
        Record the fill the engine reported, against the decision that produced its order.

        Parameters
        ----------
        event : OrderFilled
            The fill event.

        """
        client_order_id = str(event.client_order_id)
        self.fills.append(
            FillRecord(
                client_order_id=client_order_id,
                instrument_id=str(event.instrument_id),
                decision_at=self._submitted_at.get(client_order_id),
                ts_event=event.ts_event,
                ts_init=event.ts_init,
                last_px=str(event.last_px),
            ),
        )

    def _decide(self, bar: Bar) -> None:
        """
        Evaluate the exits, allocate on a rebalance bar, submit one batch and record the decision.
        """
        decision_at = bar.ts_event
        cross_section = self._cross_section(decision_at)
        exits = self._exit_instruments(cross_section)
        rebalance = self._is_rebalance_bar(decision_at)
        selection: tuple[str, ...] = ()
        targets: tuple[Target, ...] = ()
        if rebalance:
            selection, targets = self._select_and_allocate(decision_at, cross_section, exits)

        signals = self._signal_batch(decision_at, bar.ts_init, exits, targets)
        client_order_ids: list[str] = []
        if signals:
            client_order_ids = [str(item) for item in self.submit_signals(list(signals))]
            self._submitted_at.update(dict.fromkeys(client_order_ids, decision_at))
            self._mark_entries(signals, frozenset(self._held()[0]))

        self.decisions.append(
            DecisionRecord(
                decision_at=decision_at,
                ts_init=bar.ts_init,
                rebalance=rebalance,
                exits=exits,
                cross_section=cross_section,
                selection=selection,
                targets=targets,
                signals=tuple(_signal_record(signal) for signal in signals),
                client_order_ids=tuple(client_order_ids),
            ),
        )

    def _mark_entries(self, signals: Sequence[TradingSignal], held: frozenset[str]) -> None:
        """
        Record the decision bar each newly opened position was entered on.

        A long signal for an instrument that is not already held opens a position, and the holding
        period is counted from the decision that opened it; a flat signal clears the record, so a
        position that is re-entered later counts from its own entry rather than the first one.

        Parameters
        ----------
        signals : Sequence[TradingSignal]
            The signals the decision submitted.
        held : frozenset[str]
            The instruments the platform held at the decision.

        """
        for signal in signals:
            instrument_id = str(signal.instrument_id)
            if signal.direction == SignalDirection.FLAT:
                self._entry_period.pop(instrument_id, None)
            elif instrument_id not in held and instrument_id not in self._entry_period:
                self._entry_period[instrument_id] = self._period_ordinal

    def _cross_section(self, decision_at: int) -> tuple[CrossSectionEntry, ...]:
        """
        Return every instrument's values at the decision instant.
        """
        entries: list[CrossSectionEntry] = []
        for instrument_id in self._instrument_ids:
            snapshot = self._streams[instrument_id].snapshot(decision_at)
            entries.extend(_snapshot_entries(instrument_id, snapshot.values))

        return tuple(entries)

    def _selection(
        self, decision_at: int, cross_section: tuple[CrossSectionEntry, ...]
    ) -> tuple[str, ...]:
        """
        Return the declared selection of the current cross-section.
        """
        selection = self._specification.selection
        if selection.mode == SELECTION_MODE_NONE:
            # No selection mode ranks nothing, so its eligible set is the universe membership rather
            # than a ranking feature's values, which that mode's own rule forbids it from declaring.
            rows = tuple(
                CrossSectionRow(
                    instrument_id,
                    0.0,
                    bool(
                        self._memberships.is_member_at(
                            InstrumentId.from_str(instrument_id), decision_at
                        )
                    ),
                )
                for instrument_id in self._instrument_ids
            )
            return select_cross_section(rows, selection)

        if selection.feature is None:
            return ()

        return select_cross_section(_rows_for(cross_section, selection.feature), selection)

    def _select_and_allocate(
        self,
        decision_at: int,
        cross_section: tuple[CrossSectionEntry, ...],
        exits: tuple[str, ...],
    ) -> tuple[tuple[str, ...], tuple[Target, ...]]:
        """
        Select the cross-section, allocate the entry-eligible selection, and return both.
        """
        selection = self._selection(decision_at, cross_section)
        entered = tuple(
            instrument_id
            for instrument_id in selection
            if instrument_id not in exits and self._conditions_hold(instrument_id, cross_section)
        )
        held, held_weights = self._held()
        portfolio = self._specification.portfolio
        request = AllocationRequest(
            ts_event=decision_at,
            ts_init=decision_at,
            selected=entered,
            held=held,
            held_weights=held_weights,
            ineligible=self._ineligible(held, decision_at),
            sector_of=self._sectors,
            weighting=portfolio.weighting,
            position_count=portfolio.position_count,
            max_position_weight=str(portfolio.max_position_weight),
            max_sector_weight=_text_of(portfolio.max_sector_weight),
            max_gross_exposure=_text_of(self._specification.risk.pre_trade.max_gross_exposure),
            cash_policy=portfolio.cash_policy,
            flatten_ineligible=self._flatten_ineligible,
        )

        return selection, allocate_targets(request)

    def _signal_batch(
        self,
        decision_at: int,
        ts_init: int,
        exits: tuple[str, ...],
        targets: tuple[Target, ...],
    ) -> tuple[TradingSignal, ...]:
        """
        Build the batch one decision submits: the exits first, then the targets.

        A signal is submitted once per instrument, a flatten is never suppressed, and
        `risk.runtime.halt_new_entries` drops the entries of a halted run while leaving the exits in
        the batch.
        """
        signals: list[TradingSignal] = []
        submitted: set[str] = set()
        for instrument_id in exits:
            signals.append(_flat_signal(instrument_id, decision_at, ts_init))
            submitted.add(instrument_id)

        for target in targets:
            instrument_id = str(target.instrument_id)
            if instrument_id in submitted:
                continue
            submitted.add(instrument_id)
            weight = target.value.weight
            if weight is None or weight <= 0.0:
                signals.append(_flat_signal(instrument_id, decision_at, ts_init))
            elif not self._halt_new_entries:
                signals.append(
                    _long_signal(instrument_id, self._strength(weight), decision_at, ts_init),
                )

        return tuple(signals)

    def _exit_instruments(
        self,
        cross_section: tuple[CrossSectionEntry, ...],
    ) -> tuple[str, ...]:
        """
        Return the held instruments the declared exits select, before any entry is considered.

        An ineligible holding is not an exit rule: the allocation flattens it, under the flatten
        rule the specification declares, so the two paths cannot both claim it.
        """
        held = self._held()[0]

        return tuple(
            instrument_id
            for instrument_id in held
            if self._should_exit(instrument_id, cross_section)
        )

    def _should_exit(
        self,
        instrument_id: str,
        cross_section: tuple[CrossSectionEntry, ...],
    ) -> bool:
        """
        Return whether any declared exit rule selects this holding.
        """
        return (
            self._holding_period_expired(instrument_id)
            or self._rank_exit(instrument_id, cross_section)
            or self._feature_exit(instrument_id, cross_section)
            or self._stop_or_target_hit(instrument_id)
        )

    def _advance_period(self, ts_event: int) -> None:
        """
        Advance the decision period if the bar belongs to a new one.

        Every instrument's bars in one period share the period, so the ordinal advances once however
        many instruments the cross-section holds, and every decision in that period reads the same
        ordinal.

        Parameters
        ----------
        ts_event : int
            The event timestamp of the bar that arrived.

        """
        day = _decision_period_key(ts_event)
        if day != self._last_period_day:
            self._last_period_day = day
            self._period_ordinal += 1

    def _holding_period_expired(self, instrument_id: str) -> bool:
        """
        Return whether the holding-period cap is reached, counted in decision periods.
        """
        cap = self._specification.exit.holding_period_cap
        entered = self._entry_period.get(instrument_id)
        if cap is None or entered is None:
            return False

        return self._period_ordinal - entered >= cap

    def _rank_exit(self, instrument_id: str, cross_section: tuple[CrossSectionEntry, ...]) -> bool:
        """
        Return whether the holding has fallen out of the declared rank exit count.
        """
        count = self._specification.exit.rank_exit_count
        if count is None:
            return False

        ranking = self._ranking(cross_section)
        if not ranking:
            return False

        return instrument_id not in ranking[:count]

    def _ranking(self, cross_section: tuple[CrossSectionEntry, ...]) -> tuple[str, ...]:
        """
        Return the current ranking the rank exit is evaluated against.
        """
        feature = self._specification.selection.feature
        if feature is None:
            return ()

        ranked = _ranked_rows(
            _rows_for(cross_section, feature), self._specification.selection.tie_break
        )

        return tuple(row.instrument_id for row in ranked)

    def _feature_exit(
        self, instrument_id: str, cross_section: tuple[CrossSectionEntry, ...]
    ) -> bool:
        """
        Return whether the declared feature exit is satisfied for this holding.
        """
        exit_block = self._specification.exit
        if (
            exit_block.feature is None
            or exit_block.comparison is None
            or exit_block.threshold is None
        ):
            return False

        value = _entry_value(cross_section, instrument_id, exit_block.feature)
        if value is None:
            return False

        return _satisfies(value, exit_block.comparison, exit_block.threshold)

    def _stop_or_target_hit(self, instrument_id: str) -> bool:
        """
        Return whether the current close crossed the declared stop or target, measured from entry.

        The stop and the target are close-evaluated, against the close of the bar the decision is
        made on, and the entry the distance is measured from is the position's own average entry
        price rather than a price this interpreter remembered.
        """
        exit_block = self._specification.exit
        last = self._last_close.get(instrument_id)
        if last is None or (
            exit_block.stop_fraction is None and exit_block.target_fraction is None
        ):
            return False

        positions = self.cache.positions_open(
            instrument_id=InstrumentId.from_str(instrument_id),
        )
        if not positions:
            return False

        entry = Decimal(str(positions[0].avg_px_open))
        if exit_block.stop_fraction is not None and last <= entry * (1 - exit_block.stop_fraction):
            return True

        return exit_block.target_fraction is not None and last >= entry * (
            1 + exit_block.target_fraction
        )

    def _conditions_hold(
        self, instrument_id: str, cross_section: tuple[CrossSectionEntry, ...]
    ) -> bool:
        """
        Return whether the entry block's declared conditions hold for this instrument.
        """
        conditions = self._specification.entry.conditions
        if conditions is None:
            return True

        return _condition_holds(conditions, cross_section, instrument_id)

    def _is_rebalance_bar(self, decision_at: int) -> bool:
        """
        Return whether this decision bar rebalances, per the declared frequency.

        The period is read from the decision's own instant rather than from the clock, because the
        timing is declared relative to the decision time: a clock read is a different quantity, and
        in a live run the same decision bar would be keyed by whatever moment the handler ran at.

        Parameters
        ----------
        decision_at : int
            The event timestamp of the decision.

        Returns
        -------
        bool

        """
        frequency = self._specification.rebalance.frequency
        period = rebalance_period_key(decision_at, frequency)
        if period is None:
            return True

        if period == self._last_period:
            return False

        self._last_period = period

        return True

    def _held(self) -> tuple[tuple[str, ...], dict[str, float]]:
        """
        Return the held instruments and the weight each is carried at, marked at the current close.

        The weight is marked at the decision's own close rather than at the entry price, because it
        is the weight the position is carried at and not the weight it was opened at, and the entry
        price is the fallback only for a position whose instrument has published no bar yet.
        """
        equity = self._equity()
        held: list[str] = []
        weights: dict[str, float] = {}
        for position in self.cache.positions_open():
            instrument_id = str(position.instrument_id)
            held.append(instrument_id)
            if equity > 0:
                price = self._last_close.get(
                    instrument_id,
                    Decimal(str(position.avg_px_open)),
                )
                notional = Decimal(str(abs(position.signed_qty))) * price
                weights[instrument_id] = float(notional / equity)

        return tuple(sorted(held)), weights

    def _ineligible(self, held: tuple[str, ...], decision_at: int) -> tuple[str, ...]:
        """
        Return the held instruments that are no longer members of the universe.
        """
        return tuple(
            instrument_id
            for instrument_id in held
            if not self._memberships.is_member_at(InstrumentId.from_str(instrument_id), decision_at)
        )

    def _equity(self) -> Decimal:
        """
        Return the venue account's total balance, which the held weights are stated against.
        """
        account = self.cache.account_for_venue(self._venue)
        if account is None:
            return Decimal(0)

        total = account.balance_total()

        return Decimal(0) if total is None else total.as_decimal()

    def _strength(self, weight: float) -> float:
        """
        Return the candidate's weight as the signal strength the pipeline scales risk by.
        """
        cap = float(self._max_position_weight)
        if cap <= 0.0:
            return 0.0

        return min(max(weight / cap, 0.0), 1.0)


def _flat_signal(instrument_id: str, ts_event: int, ts_init: int) -> TradingSignal:
    """
    Return one reduce-to-zero signal, which needs no strength and no context.

    Parameters
    ----------
    instrument_id : str
        The instrument to flatten.
    ts_event : int
        The decision instant.
    ts_init : int
        The decision's initialization timestamp.

    Returns
    -------
    TradingSignal

    """
    return TradingSignal(
        instrument_id=InstrumentId.from_str(instrument_id),
        direction=SignalDirection.FLAT,
        strength=0.0,
        source=SIGNAL_SOURCE,
        ts_event=ts_event,
        ts_init=ts_init,
    )


def _long_signal(
    instrument_id: str,
    strength: float,
    ts_event: int,
    ts_init: int,
) -> TradingSignal:
    """
    Return one long signal, whose strength is the candidate's weight as a fraction of its cap.

    Parameters
    ----------
    instrument_id : str
        The instrument the entry is for.
    strength : float
        The strength, in the unit interval, the pipeline scales the risk it sizes from.
    ts_event : int
        The decision instant.
    ts_init : int
        The decision's initialization timestamp.

    Returns
    -------
    TradingSignal

    """
    return TradingSignal(
        instrument_id=InstrumentId.from_str(instrument_id),
        direction=SignalDirection.LONG,
        strength=strength,
        source=SIGNAL_SOURCE,
        ts_event=ts_event,
        ts_init=ts_init,
    )


def _signal_record(signal: TradingSignal) -> SignalRecord:
    """
    Return the recorded evidence of one submitted signal.

    Parameters
    ----------
    signal : TradingSignal
        The signal that was submitted.

    Returns
    -------
    SignalRecord

    """
    strength = signal.strength

    return SignalRecord(
        instrument_id=str(signal.instrument_id),
        direction=signal.direction.name,
        strength=0.0 if strength is None else strength,
        ts_event=signal.ts_event,
        ts_init=signal.ts_init,
    )


def _condition_holds(
    node: ConditionNode,
    cross_section: tuple[CrossSectionEntry, ...],
    instrument_id: str,
) -> bool:
    """
    Return whether a declared condition tree holds for one instrument's values.

    Parameters
    ----------
    node : ConditionNode
        The declared condition, which is a leaf, an all group or an any group.
    cross_section : tuple[CrossSectionEntry, ...]
        The values the decision read.
    instrument_id : str
        The instrument the condition is evaluated for.

    Returns
    -------
    bool

    """
    if isinstance(node, ConditionLeaf):
        value = _entry_value(cross_section, instrument_id, node.feature)

        return value is not None and _satisfies(value, node.comparison, node.value)

    if isinstance(node, ConditionAll):
        return all(
            _condition_holds(child, cross_section, instrument_id) for child in node.conditions
        )

    return any(_condition_holds(child, cross_section, instrument_id) for child in node.conditions)


def _entry_value(
    cross_section: tuple[CrossSectionEntry, ...],
    instrument_id: str,
    alias: str,
) -> float | None:
    """
    Return one instrument's value for one alias, or nothing when it was not available.

    Parameters
    ----------
    cross_section : tuple[CrossSectionEntry, ...]
        The values the decision read.
    instrument_id : str
        The instrument the value belongs to.
    alias : str
        The declared alias.

    Returns
    -------
    float or None

    """
    for entry in cross_section:
        if entry.instrument_id == instrument_id and entry.alias == alias:
            return entry.value

    return None


def _snapshot_entries(
    instrument_id: str,
    values: Sequence[FeatureValueAt],
) -> tuple[CrossSectionEntry, ...]:
    """
    Return the cross-section entries one instrument's snapshot carries.

    Parameters
    ----------
    instrument_id : str
        The instrument the snapshot belongs to.
    values : Sequence[FeatureValueAt]
        The snapshot's values, in the order the specification declared them.

    Returns
    -------
    tuple[CrossSectionEntry, ...]

    """
    return tuple(
        CrossSectionEntry(
            instrument_id=instrument_id,
            alias=value.alias,
            value=value.value,
            observed_at=value.observed_at,
            available_at=value.available_at,
            member=value.member,
        )
        for value in values
    )


def _rows_for(
    cross_section: tuple[CrossSectionEntry, ...],
    feature: str,
) -> tuple[CrossSectionRow, ...]:
    """
    Return the ranking rows one alias's values make, in the order the cross-section read them.

    Parameters
    ----------
    cross_section : tuple[CrossSectionEntry, ...]
        The values the decision read.
    feature : str
        The declared alias the rows are built for.

    Returns
    -------
    tuple[CrossSectionRow, ...]

    """
    return tuple(
        CrossSectionRow(entry.instrument_id, entry.value, entry.member)
        for entry in cross_section
        if entry.alias == feature
    )


def _satisfies(value: float, comparison: str, threshold: Decimal) -> bool:
    """
    Return whether a value satisfies one of the declared comparison operators.

    Parameters
    ----------
    value : float
        The value read at the decision instant.
    comparison : str
        The declared comparison operator.
    threshold : Decimal
        The declared threshold.

    Returns
    -------
    bool

    Raises
    ------
    ValueError
        If the comparison is not one this function declares.

    """
    if comparison == COMPARISON_GT:
        return value > threshold

    if comparison == COMPARISON_GE:
        return value >= threshold

    if comparison == COMPARISON_LT:
        return value < threshold

    if comparison == COMPARISON_LE:
        return value <= threshold

    raise ValueError(f"comparison must be one of {', '.join(_COMPARISONS)}, was {comparison}")


def _ranked_rows(
    rows: Sequence[CrossSectionRow],
    tie_break: str | None,
) -> tuple[CrossSectionRow, ...]:
    """
    Return the rows in the order the declared tie rule states.

    The rule is the declared order of the ranking rather than a fallback, which is why the rank and
    threshold modes of the schema require it: `feature_desc` orders by the feature's value
    descending, and resolves equal values by instrument identifier ascending, while
    `instrument_id_asc` orders by the instrument identifier alone.

    Parameters
    ----------
    rows : Sequence[CrossSectionRow]
        The eligible rows.
    tie_break : str, optional
        The declared tie rule, defaulting to the feature's descending order.

    Returns
    -------
    tuple[CrossSectionRow, ...]

    Raises
    ------
    ValueError
        If the rule is not one this function declares.

    """
    rule = TIE_BREAK_FEATURE_DESC if tie_break is None else tie_break

    if rule == TIE_BREAK_FEATURE_DESC:
        return tuple(sorted(rows, key=lambda row: (-row.value, row.instrument_id)))

    if rule == TIE_BREAK_INSTRUMENT_ID_ASC:
        return tuple(sorted(rows, key=lambda row: row.instrument_id))

    raise ValueError(f"tie_break must be one of {', '.join(_TIE_BREAKS)}, was {tie_break}")


def _threshold_rows(
    rows: tuple[CrossSectionRow, ...],
    selection: SelectionBlock,
) -> tuple[CrossSectionRow, ...]:
    """
    Return the rows that satisfy the declared threshold, ordered by value descending.

    Parameters
    ----------
    rows : tuple[CrossSectionRow, ...]
        The eligible rows.
    selection : SelectionBlock
        The declared selection, which carries the comparison and the threshold.

    Returns
    -------
    tuple[CrossSectionRow, ...]

    Raises
    ------
    ValueError
        If the comparison or the threshold is not declared.

    """
    if selection.comparison is None or selection.threshold is None:
        raise ValueError("a threshold selection requires a comparison and a threshold")

    selected = [
        row for row in rows if _satisfies(row.value, selection.comparison, selection.threshold)
    ]

    return tuple(sorted(selected, key=lambda row: (-row.value, row.instrument_id)))


def _selection_size(eligible: int, selection: SelectionBlock) -> int:
    """
    Return how many of the ranked rows the declared bound takes.

    A percentile of n gives at least one row and at most n, computed in `Decimal` and rounded up,
    and a count of zero or fewer takes nothing. The bound never stretches the eligible set to a
    portfolio size it does not have.

    Parameters
    ----------
    eligible : int
        How many rows are eligible.
    selection : SelectionBlock
        The declared selection, which carries exactly one of percentile and count.

    Returns
    -------
    int

    Raises
    ------
    ValueError
        If the rank mode declares neither bound.

    """
    if eligible == 0:
        return 0

    if selection.count is not None:
        wanted = int(selection.count.to_integral_value(rounding=ROUND_FLOOR))

        return min(max(wanted, 0), eligible)

    if selection.percentile is not None:
        share = (selection.percentile * Decimal(eligible)).to_integral_value(rounding=ROUND_CEILING)
        wanted = int(share)

        return min(max(wanted, 1), eligible)

    raise ValueError("a rank selection requires a count or a percentile")


def _bar_type_for(instrument_id: str, signal_frequency: str) -> BarType:
    """
    Return the bar type the declared signal frequency is read at, for one instrument.

    Parameters
    ----------
    instrument_id : str
        The instrument identifier.
    signal_frequency : str
        The declared signal frequency.

    Returns
    -------
    BarType

    Raises
    ------
    ValueError
        If the frequency is not one the bar specification table holds.

    """
    specification = BAR_SPECIFICATION_BY_SIGNAL_FREQUENCY.get(signal_frequency)
    if specification is None:
        declared = ", ".join(sorted(BAR_SPECIFICATION_BY_SIGNAL_FREQUENCY))

        raise ValueError(f"signal frequency must be one of {declared}, was {signal_frequency}")

    return BarType.from_str(f"{instrument_id}-{specification}")


def _text_of(value: Decimal | None) -> str | None:
    """
    Return a `Decimal` as text, or nothing.

    Parameters
    ----------
    value : Decimal, optional
        The value to render.

    Returns
    -------
    str or None

    """
    return None if value is None else str(value)


def _decimal_of_text(value: str | None) -> Decimal | None:
    """
    Return the `Decimal` a text field carries, or nothing.

    Parameters
    ----------
    value : str, optional
        The text to read.

    Returns
    -------
    Decimal or None

    """
    return None if value is None else Decimal(value)


def _investable_ceiling(gross_cap: Decimal | None) -> Decimal:
    """
    Return the total weight the declared gross exposure cap admits.

    One unit of equity is the ceiling a weighted allocation reaches, so a gross exposure cap above
    it admits the whole unit, and a cap below it is the tighter ceiling the caps are measured
    against.

    Parameters
    ----------
    gross_cap : Decimal, optional
        The declared gross exposure cap.

    Returns
    -------
    Decimal

    """
    if gross_cap is None:
        return _UNIT

    return min(_UNIT, gross_cap)


def _selected_order(selected: tuple[str, ...], position_count: int) -> tuple[str, ...]:
    """
    Return the selection the allocation weights, in the order the selection produced it.

    The portfolio size is the pre-trade constraint of design Section 9.2, so the declared count is
    the most the allocation weights, and the names beyond it are the ones the selection ranked last.
    A repeated identifier is weighted once, at its first position.

    Parameters
    ----------
    selected : tuple[str, ...]
        The selection's instrument identifiers.
    position_count : int
        The declared portfolio size.

    Returns
    -------
    tuple[str, ...]

    """
    order: list[str] = []
    for instrument_id in selected:
        if len(order) >= max(position_count, 0):
            break
        if instrument_id not in order:
            order.append(instrument_id)

    return tuple(order)


def _initial_weights(
    order: tuple[str, ...],
    weighting: str,
) -> tuple[dict[str, Decimal], dict[str, Decimal]]:
    """
    Return the declared weighting's shares, and the weights they normalise to.

    Parameters
    ----------
    order : tuple[str, ...]
        The selection the allocation weights, in its declared order.
    weighting : str
        The declared weighting method.

    Returns
    -------
    tuple[dict[str, Decimal], dict[str, Decimal]]
        The shares and the normalised weights, each keyed by instrument identifier.

    Raises
    ------
    ValueError
        If the weighting method is not one this function declares.

    """
    if not order:
        return {}, {}

    if weighting == WEIGHTING_EQUAL:
        shares = dict.fromkeys(order, _UNIT)
    elif weighting == WEIGHTING_RANK_PROPORTIONAL:
        size = Decimal(len(order))
        shares = {instrument_id: size - index for index, instrument_id in enumerate(order)}
    else:
        raise ValueError(f"weighting must be one of {', '.join(_WEIGHTINGS)}, was {weighting}")

    total = sum(shares.values(), Decimal(0))
    weights = {instrument_id: share / total for instrument_id, share in shares.items()}

    return weights, shares


def _apply_position_cap(weights: dict[str, Decimal], caps: _Caps) -> dict[str, Decimal]:
    """
    Return the weights with the declared per-position cap applied, redistributing nothing.

    Parameters
    ----------
    weights : dict[str, Decimal]
        The weights to clamp.
    caps : _Caps
        The declared caps, which carry the per-position cap.

    Returns
    -------
    dict[str, Decimal]

    """
    position = caps.position

    return {instrument_id: min(weight, position) for instrument_id, weight in weights.items()}


def _apply_sector_cap(weights: dict[str, Decimal], caps: _Caps) -> dict[str, Decimal]:
    """
    Return the weights scaled so no declared sector exceeds the declared cap.

    An instrument the mapping does not name is its own sector, so a sector of one is scaled to the
    cap rather than left above it.

    Parameters
    ----------
    weights : dict[str, Decimal]
        The weights to scale.
    caps : _Caps
        The declared caps and the mapping the sector cap is applied through.

    Returns
    -------
    dict[str, Decimal]

    """
    cap = caps.sector
    totals = caps.sector_totals(weights)
    scaled: dict[str, Decimal] = {}
    for instrument_id, weight in weights.items():
        total = totals[caps.sector_of(instrument_id)]
        scaled[instrument_id] = weight if cap is None or total <= cap else weight * cap / total

    return scaled


def _apply_gross_cap(weights: dict[str, Decimal], caps: _Caps) -> dict[str, Decimal]:
    """
    Return the weights scaled down proportionally when their total exceeds the ceiling.

    Parameters
    ----------
    weights : dict[str, Decimal]
        The weights to scale.
    caps : _Caps
        The declared caps, which carry the ceiling the total is measured against.

    Returns
    -------
    dict[str, Decimal]

    """
    total = sum(weights.values(), Decimal(0))
    if total <= caps.ceiling or total <= 0:
        return weights

    scale = caps.ceiling / total

    return {instrument_id: weight * scale for instrument_id, weight in weights.items()}


def _reallocate(
    weights: dict[str, Decimal],
    shares: dict[str, Decimal],
    caps: _Caps,
) -> dict[str, Decimal]:
    """
    Return the weights with the residual the caps left redistributed over the instruments below it.

    The residual is offered in the declared share proportions, capped by each instrument's remaining
    headroom, and the loop is bounded by the number of instruments: a pass that adds nothing ends
    it, so no instrument can be pushed above a cap and the allocation always terminates.

    Parameters
    ----------
    weights : dict[str, Decimal]
        The weights the caps produced, which this function owns and mutates.
    shares : dict[str, Decimal]
        The declared shares, which the offer is proportional to.
    caps : _Caps
        The declared caps and the mapping the sector cap is applied through.

    Returns
    -------
    dict[str, Decimal]

    """
    for _ in range(len(weights)):
        residual = caps.ceiling - sum(weights.values(), Decimal(0))
        if residual <= 0:
            break

        headroom = caps.headroom(weights)
        candidates = [instrument_id for instrument_id, room in headroom.items() if room > 0]
        offered = sum((shares[instrument_id] for instrument_id in candidates), Decimal(0))
        if not candidates or offered <= 0:
            break

        added = Decimal(0)
        for instrument_id in candidates:
            share = residual * shares[instrument_id] / offered
            gain = min(share, headroom[instrument_id])
            weights[instrument_id] += gain
            added += gain

        if added <= 0:
            break

    return weights


def _emitted_weights(
    weights: dict[str, Decimal],
    request: AllocationRequest,
) -> tuple[tuple[str, Decimal], ...]:
    """
    Return the weights the allocation emits, with the flattened instruments, in identifier order.

    An instrument the allocation does not weight and does not hold is not emitted at all, a held
    instrument the selection no longer names is flattened, and an ineligible holding is flattened
    only when the declared rule says so: otherwise it is left untouched, with no target at all.

    Parameters
    ----------
    weights : dict[str, Decimal]
        The weights the allocation computed for the selected instruments.
    request : AllocationRequest
        The request, which carries the positions and the declared flatten rule.

    Returns
    -------
    tuple[tuple[str, Decimal], ...]

    """
    held = frozenset(request.held)
    ineligible = frozenset(request.ineligible)
    flat = (ineligible & held) if request.flatten_ineligible else frozenset()
    flat |= (held - frozenset(weights)) - ineligible
    emitted = {
        instrument_id: weight
        for instrument_id, weight in weights.items()
        if weight > 0 and instrument_id not in ineligible
    }
    for instrument_id in flat - frozenset(emitted):
        emitted[instrument_id] = Decimal(0)

    return tuple(sorted(emitted.items()))
