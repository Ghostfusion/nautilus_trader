# %% [markdown]
# # Strategy discovery
#
# Write a candidate strategy as data, compile it against a frozen policy, run it through the
# backtest engine, and read the evidence back out of an append-only ledger.
#
# [View source on GitHub](https://github.com/Ghostfusion/nautilus_trader/blob/develop/docs/tutorials/strategy_discovery.py).

# %% [markdown]
# ## What the package guarantees
#
# A discovery campaign tests many candidates, and the ones that rank best are also the ones most
# likely to be an artefact of how they were written down and chosen.
# `nautilus_trader.discovery` keeps that auditable with three restrictions:
#
# - **A candidate is data.** A specification is a mapping of blocks, and no part of it is callable.
# - **The language is closed.** Every value resolves against the features a campaign registered, the
#   policy's own enumerations, and the run's data. A field named for a Python module, a
#   `module:attribute` pair or an expression is refused before anything runs.
# - **There is one executor.** `DiscoveryInterpreter` is the only thing that runs a candidate, so the
#   artifact that ran is the artifact that was hashed.
#
# What comes out is a chain of identities rather than a claim: the specification digest (what was
# submitted), the behavioural digest (what ran, including the resolved feature definitions, the
# policy, the declared execution assumptions and the engine version), the engine's own canonical
# result digest, and the ledger entry that records the run.

# %% [markdown]
# ## Prerequisites
#
# - NautilusTrader 2.x, built from this repository.
# - Nothing else. The run writes its own instruments, bars and quotes into a temporary Parquet
#   catalog, so this page needs no data files, no credentials and no network access.

# %%
from __future__ import annotations

import tempfile
from copy import deepcopy
from decimal import Decimal
from pathlib import Path

from nautilus_trader.backtest import BacktestDataConfig
from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.backtest import BacktestVenueConfig
from nautilus_trader.discovery import SPECIFICATION_SCHEMA_VERSION
from nautilus_trader.discovery import DiscoveryInterpreter
from nautilus_trader.discovery import DiscoveryInterpreterConfig
from nautilus_trader.discovery import DiscoveryLedger
from nautilus_trader.discovery import DiscoveryPolicy
from nautilus_trader.discovery import FeatureDefinition
from nautilus_trader.discovery import FeatureRegistry
from nautilus_trader.discovery import InterpreterRunInputs
from nautilus_trader.discovery import Reference
from nautilus_trader.discovery import ReferenceKind
from nautilus_trader.discovery import RunRequest
from nautilus_trader.discovery import build_memberships
from nautilus_trader.discovery import compile_specification
from nautilus_trader.discovery import experiment_for
from nautilus_trader.discovery import parse_specification
from nautilus_trader.discovery import run_experiment
from nautilus_trader.discovery import validate_specification
from nautilus_trader.discovery.strategy import BAR_SPECIFICATION_BY_SIGNAL_FREQUENCY
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import Bar
from nautilus_trader.model import BarType
from nautilus_trader.model import Currency
from nautilus_trader.model import Equity
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import NautilusDataType
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import QuoteTick
from nautilus_trader.model import Symbol
from nautilus_trader.model import Venue
from nautilus_trader.persistence import ParquetDataCatalog

# %%
# The run's own scratch space: a catalog to replay and a ledger to write. A temporary directory
# keeps the page re-runnable, because the ledger refuses to overwrite an entry it already holds.
RUN_DIR = Path(tempfile.mkdtemp(prefix="nautilus-discovery-"))
CATALOG_DIR = RUN_DIR / "catalog"
LEDGER_DIR = RUN_DIR / "ledger"
print(f"run directory: {RUN_DIR}")

# %%
# The campaign's fixed inputs. A real campaign derives these from its own data and its own
# conventions; here they are declared once and named, so the rest of the page reads as the
# decisions it makes.
VENUE = "XNAS"
INSTRUMENTS = ("AAPL.XNAS", "MSFT.XNAS", "NVDA.XNAS", "AMZN.XNAS")
MEMBERSHIP_SOURCE = "tutorial.synthetic"
FEATURE_ALIAS = "trend"
FEATURE_ID = "sma_close_3"
FEATURE_LOOKBACK = 3
SNAPSHOT = "tutorial.synthetic-2024-01"
SEED = 1
SHARPE = "Sharpe Ratio (simple, sample, 252 days)"
DRAWDOWN = "Max Drawdown (simple)"
USD = Currency.from_str("USD")
IDENTIFIERS = [InstrumentId.from_str(instrument_id) for instrument_id in INSTRUMENTS]
BAR_SPECIFICATION = BAR_SPECIFICATION_BY_SIGNAL_FREQUENCY["daily"]
SECOND_NS = 1_000_000_000
DAY_NS = 86_400 * SECOND_NS
TS_START = 1_704_067_200_000_000_000  # 2024-01-01T00:00:00Z
STARTING_BALANCE = "1000000 USD"
MAX_POSITION_WEIGHT = Decimal("0.25")
STOP_LOSS_BPS = 100
BENCHMARK_DIGEST = "sha256:" + "0" * 64
PRICES = {
    INSTRUMENTS[0]: (
        "100.00",
        "101.00",
        "102.00",
        "103.00",
        "104.00",
        "105.00",
        "106.00",
        "107.00",
    ),
    INSTRUMENTS[1]: (
        "200.00",
        "199.00",
        "198.00",
        "197.00",
        "196.00",
        "195.00",
        "194.00",
        "193.00",
    ),
    INSTRUMENTS[2]: ("50.00", "60.00", "70.00", "80.00", "90.00", "100.00", "110.00", "120.00"),
    INSTRUMENTS[3]: ("80.00", "80.00", "80.00", "80.00", "80.00", "80.00", "80.00", "80.00"),
}

# %% [markdown]
# ## Declare the candidate
#
# A specification is one mapping with a fixed set of blocks. Each block answers one question and only
# that question, which is what makes a candidate comparable with the next one.
#
# | Block        | Answers                                                                |
# | ------------ | ---------------------------------------------------------------------- |
# | `identity`   | What is claimed, and what would falsify it.                            |
# | `universe`   | Which instruments are eligible, and on what membership data.            |
# | `data`       | At what frequency decisions are taken, and when they execute.           |
# | `features`   | Which registered features the candidate reads, under which names.       |
# | `selection`  | How the cross-section is ranked and how many names are held.            |
# | `entry`      | In which direction the selected names are taken.                        |
# | `exit`       | What happens to a name that leaves the selection or the universe.       |
# | `portfolio`  | How the selected names are weighted, and what to do with idle cash.     |
# | `rebalance`  | How often the target set is recomputed, and when it takes effect.       |
# | `risk`       | The pre-trade, runtime and post-run bounds the run enforces.            |
# | `evaluation` | Which frozen references the result is judged against.                   |
# | `provenance` | Who generated the candidate, with which seed, from which parents.       |
#
# The `features` block declares aliases: a candidate reads `trend`, and the alias resolves to a
# registered feature id with a declared lookback and skip. The compiler checks the declaration
# against the registry rather than trusting the name.

# %%
SPECIFICATION = {
    "version": SPECIFICATION_SCHEMA_VERSION,
    "identity": {
        "family": "cross_sectional_rank",
        "hypothesis": "the strongest trend in the cross-section persists over the next session",
        "mechanism": "rank the cross-section by a short moving average and hold the leaders",
        "falsification": "the ranked leaders earn no more than the cross-section median",
    },
    "universe": {
        "asset_classes": ["EQUITY"],
        "membership_policy": "point_in_time",
        "source": MEMBERSHIP_SOURCE,
        "min_price": "1",
        "min_dollar_volume": "1000000",
        "exclusions": [],
    },
    "data": {
        "signal_frequency": "daily",
        "decision_time": "session_close",
        "execution_eligibility": "next_session_open",
        "corporate_action_treatment": "raw",
    },
    "features": {
        "declared": {
            FEATURE_ALIAS: {
                "feature": FEATURE_ID,
                "lookback": FEATURE_LOOKBACK,
                "skip": 0,
            },
        },
    },
    "selection": {
        "mode": "rank",
        "feature": FEATURE_ALIAS,
        "count": "3",
        "tie_break": "feature_desc",
    },
    "entry": {"direction": "long"},
    "exit": {"flatten_ineligible": True},
    "portfolio": {
        "position_count": 3,
        "weighting": "rank_proportional",
        "max_position_weight": str(MAX_POSITION_WEIGHT),
        "cash_policy": "leave_idle",
    },
    "rebalance": {"frequency": "daily", "timing": "next_session_open"},
    "risk": {
        "pre_trade": {},
        "runtime": {"halt_new_entries": False, "flatten_ineligible": False},
        "post_run": {},
    },
    "evaluation": {
        "benchmark": "tutorial-benchmark",
        "cost_model": "tutorial-cost-model",
        "validation_policy": "tutorial-validation",
    },
    "provenance": {"generator": "tutorial", "seed": SEED, "parents": []},
}

# %% [markdown]
# ## Register the feature and freeze the policy
#
# The registry holds the definitions a candidate may resolve. The policy holds everything the search
# must not choose for itself: the admissible families, frequencies, weighting and selection modes, the
# feature allowlist, the frozen references a candidate may cite, and the sector map the run's risk
# bounds use. Both are inputs to the campaign rather than products of it, and both are hashed into the
# behavioural identity.
#
# `DiscoveryPolicy.first_release` returns the policy of the declared first-release scope. The
# allowlist, the references and the sectors are campaign inputs, so the factory takes them rather than
# inventing them.

# %%
FEATURE = FeatureDefinition(
    feature_id=FEATURE_ID,
    operator="sma",
    input_field="close",
    lookback=FEATURE_LOOKBACK,
)
REGISTRY = FeatureRegistry((FEATURE,))
REFERENCES = (
    Reference("tutorial-benchmark", ReferenceKind.BENCHMARK, BENCHMARK_DIGEST),
    Reference("tutorial-cost-model", ReferenceKind.COST_MODEL, BENCHMARK_DIGEST),
    Reference("tutorial-validation", ReferenceKind.VALIDATION_POLICY, BENCHMARK_DIGEST),
)
SECTORS = (
    (INSTRUMENTS[0], "mega"),
    (INSTRUMENTS[1], "mega"),
    (INSTRUMENTS[2], "chips"),
)
POLICY = DiscoveryPolicy.first_release(
    feature_allowlist=(FEATURE_ID,),
    references=REFERENCES,
    sectors=SECTORS,
)
print(f"policy digest: {POLICY.digest()}")

# %% [markdown]
# ## Compile the candidate
#
# Compilation is five stages over one payload: parse the blocks, check the values against the policy,
# resolve the feature declarations against the registry, check the counts and the frequency ordering,
# and compose the plan. Nothing raises. A refused payload comes back as a `CompilationResult` holding
# the refusals, because a refusal is an auditable outcome rather than an error to be retried in a
# different shape.
#
# `parse_specification` and `validate_specification` are the first two stages on their own, and they
# are what a campaign calls when it wants the reasons without a plan.

# %%
parsed = parse_specification(SPECIFICATION, POLICY)
validation = validate_specification(SPECIFICATION, POLICY, REGISTRY)
print(f"parsed without refusal: {not validation.errors}")

compiled = compile_specification(SPECIFICATION, POLICY, REGISTRY)
print(f"accepted: {compiled.accepted}")

plan = compiled.plan
assert plan is not None, compiled.errors

# %% [markdown]
# The plan carries the identities the rest of the run is keyed by. Two of them matter here: the
# specification digest is over what was submitted, and the behavioural digest is over what ran. Change
# the policy and the specification digest holds still while the behavioural digest moves, because the
# policy is part of what ran and not part of what was written down.

# %%
ROTATED_POLICY = DiscoveryPolicy.first_release(
    feature_allowlist=(FEATURE_ID,),
    references=REFERENCES,
    sectors=(*SECTORS, (INSTRUMENTS[3], "retail")),
)
rotated = compile_specification(SPECIFICATION, ROTATED_POLICY, REGISTRY)
assert rotated.plan is not None, rotated.errors

print(f"engine version:            {plan.engine_version}")
print(f"specification digest:      {plan.specification_digest}")
print(f"behavioural digest:        {plan.behavioural_digest}")
print(f"policy digest:             {plan.policy_digest}")
print(f"registry digest:           {plan.registry_digest}")
print(f"rotated policy digest:     {rotated.plan.policy_digest}")
print(f"rotated specification:     {rotated.plan.specification_digest}")
print(f"rotated behavioural:       {rotated.plan.behavioural_digest}")

# %% [markdown]
# ## Watch it refuse
#
# The language's guarantee is about resolution rather than spelling. A value in the `module:attribute`
# form is the shape an importable strategy is named by, and an expression is the shape a generated
# candidate reaches for. Both are refused, at the field that carried them, before anything runs.

# %%
REFUSED_CODE = deepcopy(SPECIFICATION)
REFUSED_CODE["provenance"]["generator"] = "strategies.ema_cross:EMACross"
REFUSED_CODE["identity"]["mechanism"] = "eval(open('.env').read())"

refused_code = compile_specification(REFUSED_CODE, POLICY, REGISTRY)
print(f"accepted: {refused_code.accepted}")
for refusal in refused_code.errors:
    print(f"  [{refusal.code.value}] {refusal.path}: {refusal.message}")

# %% [markdown]
# A refusal is an outcome rather than an error to be retried in a different shape, and a campaign can
# group and count refusals without reading their messages. The second half of the guarantee is the
# policy: a candidate that asks for a feature no policy registered is refused by the same route.

# %%
REFUSED_FEATURE = deepcopy(SPECIFICATION)
REFUSED_FEATURE["features"]["declared"][FEATURE_ALIAS]["feature"] = "sma_close_5"

refused_feature = compile_specification(REFUSED_FEATURE, POLICY, REGISTRY)
print(f"accepted: {refused_feature.accepted}")
for refusal in refused_feature.errors:
    print(f"  [{refusal.code.value}] {refusal.path}: {refusal.message}")

# %% [markdown]
# The boundary is where a value would have to resolve, not where it looks like code. A bare dotted
# string names nothing the candidate can call, so it is carried through as the text it is: the field
# that asked for a module is refused by name, while the value that merely resembles a path is data.

# %%
ADMITTED = deepcopy(SPECIFICATION)
ADMITTED["provenance"]["generator"] = "nautilus_trader.adapters.binance"

admitted = compile_specification(ADMITTED, POLICY, REGISTRY)
print(f"a bare dotted string is admitted: {admitted.accepted}")
print(
    f"and it has a plan and a digest of its own: {admitted.plan.behavioural_digest != plan.behavioural_digest}"
)

# %% [markdown]
# ## Prepare the run's data
#
# The interpreter subscribes to the bars the declared signal frequency maps to, and the target
# pipeline resolves an entry price from the cache. Both come from the run's data rather than from the
# candidate, so a candidate cannot reach for an instrument or a series it did not declare.
#
# This page supplies its own data: four equities, eight daily bars each, and one quote one second
# before each bar so an entry price always resolves. A real campaign points the same two
# `BacktestDataConfig` objects at a `ParquetDataCatalog` of its own.


# %%
def instrument(instrument_id: str) -> Equity:
    """
    Return one equity on the run's venue.
    """
    return Equity(
        instrument_id=InstrumentId.from_str(instrument_id),
        raw_symbol=Symbol(instrument_id.split(".", maxsplit=1)[0]),
        isin=None,
        currency=USD,
        price_precision=2,
        price_increment=Price.from_str("0.01"),
        lot_size=Quantity.from_int(100),
        ts_event=0,
        ts_init=0,
    )


def bar(instrument_id: str, close: str, ts_event: int) -> Bar:
    """
    Return one daily bar with a spread around the given close.
    """
    close_price = Price.from_str(close)

    return Bar(
        BarType.from_str(f"{instrument_id}-{BAR_SPECIFICATION}"),
        close_price,
        Price.from_str(f"{close_price.as_double() + 1.0:.2f}"),
        Price.from_str(f"{close_price.as_double() - 1.0:.2f}"),
        close_price,
        Quantity.from_int(1_000_000),
        ts_event,
        ts_event,
    )


def bars(instrument_id: str) -> list[Bar]:
    """
    Return one instrument's bars, in ascending time order.
    """
    return [
        bar(instrument_id, close, TS_START + index * DAY_NS)
        for index, close in enumerate(PRICES[instrument_id])
    ]


def quotes(instrument_id: str) -> list[QuoteTick]:
    """
    Return one quote per bar, one second before it, so the pipeline can resolve an entry price.
    """
    identifier = InstrumentId.from_str(instrument_id)

    return [
        QuoteTick(
            identifier,
            Price.from_str(close),
            Price.from_str(close),
            Quantity.from_int(1_000),
            Quantity.from_int(1_000),
            TS_START + index * DAY_NS - SECOND_NS,
            TS_START + index * DAY_NS - SECOND_NS,
        )
        for index, close in enumerate(PRICES[instrument_id])
    ]


# %%
CATALOG_DIR.mkdir(parents=True, exist_ok=True)
written = ParquetDataCatalog(str(CATALOG_DIR))
written.write_instruments([instrument(instrument_id) for instrument_id in INSTRUMENTS])
for item in INSTRUMENTS:
    written.write_bars(bars(item))
    written.write_quote_ticks(quotes(item))

print(f"catalog written: {CATALOG_DIR}")

# %% [markdown]
# ## Run the candidate
#
# The interpreter is configured from `InterpreterRunInputs`: the compiled plan, the membership
# intervals, the instrument set, the venue, the commission rate, the stop-loss bound and the sector
# map. Those inputs are the plan's own execution assumptions, and they are as much a part of the
# behavioural identity as the policy is.
#
# The run itself is an ordinary backtest: a `RunRequest` carries the plan, the strategy, the venue,
# the data configurations, the engine configuration, the analysis statistics to compute, and the seed
# and snapshot the run declares.


# %%
def venue() -> BacktestVenueConfig:
    """
    Return the venue the run trades through, with a zero-cost fee model.
    """
    return BacktestVenueConfig(
        name=VENUE,
        oms_type="NETTING",
        account_type="CASH",
        starting_balances=[STARTING_BALANCE],
        base_currency=USD,
        book_type="L1_MBP",
        fee_model=MakerTakerFeeModel(
            maker_rate=Decimal(0),
            taker_rate=Decimal(0),
        ),
    )


def data_configs() -> list[BacktestDataConfig]:
    """
    Return the bar and quote configurations the run replays the catalog through.
    """
    return [
        BacktestDataConfig(
            data_type=NautilusDataType.Bar,
            catalog_path=str(CATALOG_DIR),
            instrument_ids=IDENTIFIERS,
            bar_types=[f"{instrument_id}-{BAR_SPECIFICATION}" for instrument_id in INSTRUMENTS],
        ),
        BacktestDataConfig(
            data_type=NautilusDataType.QuoteTick,
            catalog_path=str(CATALOG_DIR),
            instrument_ids=IDENTIFIERS,
        ),
    ]


# %%
memberships = build_memberships(
    "tutorial-universe",
    MEMBERSHIP_SOURCE,
    [(instrument_id, 0, None) for instrument_id in INSTRUMENTS],
)
strategy = DiscoveryInterpreter(
    DiscoveryInterpreterConfig(
        inputs=InterpreterRunInputs(
            plan=plan,
            memberships=memberships,
            instrument_ids=INSTRUMENTS,
            venue=Venue(VENUE),
            commission_rate=Decimal(0),
            stop_loss_bps=STOP_LOSS_BPS,
            sectors=POLICY.sectors,
        ),
    ),
)
outcome = run_experiment(
    RunRequest(
        plan=plan,
        strategy=strategy,
        venues=[venue()],
        data=data_configs(),
        engine=BacktestEngineConfig(bypass_logging=True, run_analysis=False),
        metrics=[SHARPE, DRAWDOWN],
        seed=SEED,
        snapshot=SNAPSHOT,
    ),
)

# %%
# The experiment is the plan's own behavioural identity, so it is recomputable from the plan rather
# than read back out of the run.
print(f"experiment digest:         {outcome.experiment.digest}")
print(f"recomputed from the plan:  {experiment_for(plan).digest}")
print(f"canonical result digest:   {outcome.run.canonical_digest}")
print(f"metric values:             {dict(outcome.run.metric_values)}")

# %% [markdown]
# ## Read the evidence
#
# The interpreter records what each of its decisions saw and did: the cross-section it built, the
# names it selected, the targets it allocated, the signals it submitted and the fills the engine
# produced. Every cross-section entry carries the instant its value was observed and the instant it
# became available, which is what keeps a decision from reading a value it could not have had.

# %%
decisions = strategy.decisions
print(f"decisions: {len(decisions)}")
for decision in decisions[:2] + decisions[-1:]:
    print(
        f"  at {decision.decision_at}: selection={decision.selection} "
        f"rebalance={decision.rebalance}",
    )

latest = decisions[-1]
print("the last decision's cross-section:")
for entry in latest.cross_section:
    print(
        f"  {entry.instrument_id} {entry.alias}={entry.value} observed_at={entry.observed_at} "
        f"available_at={entry.available_at}",
    )

print("the last decision's signals:")
for signal in latest.signals:
    print(f"  {signal.instrument_id} {signal.direction} strength={signal.strength:.4f}")

print("the last decision's target weights:")
for target in latest.targets:
    print(f"  {target.instrument_id}: {target.value.weight}")

print("fills:")
for fill in strategy.fills[:4]:
    print(
        f"  {fill.instrument_id} {fill.client_order_id} last_px={fill.last_px} "
        f"decision_at={fill.decision_at} ts_event={fill.ts_event}",
    )

# %% [markdown]
# The evidence package is what one experiment says about itself. It carries both identities, the
# experiment's identity, and the engine's canonical result bytes alongside their digest, so a reader
# compares an artifact rather than a claim.

# %%
package = outcome.package
print(f"candidate id:              {package.candidate_id}")
print(f"specification digest:      {package.specification_digest}")
print(f"behavioural digest:        {package.behavioural_digest}")
print(f"policy digest:             {package.policy_digest}")
print(f"experiment digest:         {package.experiment_digest}")
print(f"canonical digest:          {package.canonical_digest}")
print(f"canonical bytes:           {len(package.canonical_bytes)}")
print(f"matches the plan:          {package.behavioural_digest == plan.behavioural_digest}")

# %% [markdown]
# ## Append to the ledger
#
# The ledger is a campaign's memory: one file per candidate, named for its behavioural digest, written
# once. Two candidates that differ only in the policy they ran under are two entries, because the
# behavioural digest is what ran rather than what was written down. Appending the same package twice
# is refused, so a campaign cannot quietly replace what it already learned.

# %%
ledger = DiscoveryLedger(LEDGER_DIR)
entry_path = ledger.append(outcome.package)
entry = ledger.entries()[0]
print(f"appended: {entry_path.name}")
print(f"candidates in the ledger: {ledger.candidate_ids()}")

# %%
for key in (
    "candidate_id",
    "specification_digest",
    "behavioural_digest",
    "policy_digest",
    "registry_digest",
    "experiment_digest",
    "canonical_digest",
    "engine_version",
    "snapshot",
    "seed",
    "status",
):
    print(f"{key}: {entry[key]}")

# %%
try:
    ledger.append(outcome.package)
except FileExistsError as error:
    print(f"the ledger refused the rewrite: {error}")

# %% [markdown]
# ## Next steps
#
# - [Strategy discovery design](../design/strategy_discovery_design.md) for the language, the policy
#   and the evidence model the page exercised.
# - [Strategy discovery implementation plan](../design/strategy_discovery_implementation.md) for the
#   milestones, including the campaign-side protocols that this page does not cover.
# - `nautilus_trader.discovery` for the full surface: the feature contract, the cross-section
#   selector, the allocation function, the run and the ledger.
#
# Two things are worth knowing before this page is used for equity results. The corporate-action
# stage of the fork's backtest engine credits nothing for a cash distribution, so the accounting this
# page measures is exact for instruments whose corporate actions are known not to matter, and the
# plan's exit record therefore withholds the equity-readiness statement. And the campaign-side
# protocols, the append protocol, the trial counts, the holdout reservation and the admission
# assessment, are milestones of their own rather than part of what the discovery package does today.
