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
FX-M3-2: no candidate can execute generated code.

Design criterion 3 requires that the interpreter is the only executor, and that a specification field
which resolves to a module path, a code artefact or an expression to be evaluated is refused by the
validator with the refusal recorded. The design's decision SD3 is why: a candidate is data, so the
artifact that runs is the artifact that was hashed, and nothing a candidate supplies is ever imported,
evaluated or called.

Two halves are tested here, and the second is the one that would otherwise be an assertion about
intent rather than a measurement:

- every shape of code a specification could carry is refused, at the field it was put in, with the
  refusal returned as a structured outcome rather than raised as an error;
- a legal specification compiles while every dynamic import in the process is unavailable, and the
  compiled plan is data: it is an instance of a type this package defines, it carries no code object,
  and nothing in its plain-data form is callable.
"""

from __future__ import annotations

import builtins
import importlib
import sys
from typing import TYPE_CHECKING
from typing import Any

from nautilus_trader.discovery.compile import compile_specification
from nautilus_trader.discovery.schema import RefusalCode
from tests.unit.discovery import fixtures


if TYPE_CHECKING:
    from collections.abc import Iterator

    import pytest


MODULE_PATH = "strategies.ema_cross:EMACross"
IMPORTABLE_PATH = "nautilus_trader.adapters.binance"
EXPRESSION = "eval(open('.env').read())"
IMPORT_EXPRESSION = "__import__('os').getcwd()"
SHELL_EXPRESSION = "subprocess.check_output(['ls'])"


def refused_code(payload: dict[str, Any]) -> tuple[RefusalCode, str]:
    """
    Compile the payload and return the first refusal's code and path.
    """
    result = compile_specification(payload, fixtures.fixture_policy(), fixtures.fixture_registry())

    assert result.plan is None
    assert result.errors

    return result.errors[0].code, result.errors[0].path


def test_a_specification_naming_a_module_is_refused_at_that_field() -> None:
    """
    Test that a field asking for a module path is refused by name.

    The refusal names the field rather than the value, because the intent is what makes it code: a
    field called `module_path` is a request to import, wherever it is put and whatever it holds.
    """
    payload = fixtures.changed(
        fixtures.legal_payload(),
        {"provenance.module_path": MODULE_PATH},
    )

    code, path = refused_code(payload)

    assert code is RefusalCode.CODE_REFERENCE_REFUSED
    assert path == "provenance.module_path"


def test_a_value_in_the_module_attribute_form_is_refused() -> None:
    """
    Test that the `module:attribute` form this repository uses for strategy paths is refused.

    The form is unambiguous as a value: `strategies.ema_cross:EMACross` is the shape an importable
    strategy is named by, so a specification carrying it is asking for an import wherever it is put.
    """
    payload = fixtures.changed(
        fixtures.legal_payload(),
        {"provenance.generator": MODULE_PATH},
    )

    code, path = refused_code(payload)

    assert code is RefusalCode.CODE_REFERENCE_REFUSED
    assert path == "provenance.generator"


def test_a_bare_dotted_string_is_data_because_nothing_resolves_it() -> None:
    """
    Test where the boundary actually is: on resolution, not on spelling.

    A dotted string is text. The guarantee criterion 3 needs is that nothing a candidate supplies is
    imported, evaluated or called, and this fixture tests that guarantee directly rather than by
    pattern-matching a value that could equally be an instrument identifier. A value that names an
    importable path is therefore admissible, inert, and carried through into the plan as the string
    it was, while the field name that asks for a module is refused.
    """
    payload = fixtures.changed(
        fixtures.legal_payload(),
        {"provenance.generator": IMPORTABLE_PATH},
    )
    result = compile_specification(
        payload,
        fixtures.fixture_policy(),
        fixtures.fixture_registry(),
    )

    assert result.accepted
    assert result.plan is not None
    assert IMPORTABLE_PATH in list(_leaves(result.plan.to_dict()))


def test_an_expression_a_specification_carries_is_refused() -> None:
    """
    Test that an expression to be evaluated is refused wherever it appears.

    The three shapes are the ones a generated candidate would reach for: an evaluation, an import
    inside a value, and a shell call.
    """
    for expression in (EXPRESSION, IMPORT_EXPRESSION, SHELL_EXPRESSION):
        payload = fixtures.changed(fixtures.legal_payload(), {"identity.mechanism": expression})

        code, path = refused_code(payload)

        assert code is RefusalCode.CODE_REFERENCE_REFUSED
        assert path == "identity.mechanism"


def test_a_code_artefact_a_specification_carries_is_refused() -> None:
    """
    Test that a value which is not data at all is refused with the field it was put in.

    A callable, a type and a module are the three artefacts a candidate could smuggle in beside the
    values a field is supposed to hold.
    """
    for artefact in (lambda: None, dict, sys):
        payload = fixtures.changed(fixtures.legal_payload(), {"identity.falsification": artefact})

        code, path = refused_code(payload)

        assert code is RefusalCode.CODE_REFERENCE_REFUSED
        assert path == "identity.falsification"


def test_a_refused_candidate_carries_the_refusal_rather_than_raising() -> None:
    """
    Test that a refusal is an outcome, not an error to be retried in a different shape.

    The compilation returns the refusal with its code, path and message, and the message survives a
    round trip through plain data, which is what makes a refused candidate auditable.
    """
    payload = fixtures.changed(
        fixtures.legal_payload(),
        {"provenance.module_path": MODULE_PATH},
    )
    first = compile_specification(payload, fixtures.fixture_policy(), fixtures.fixture_registry())
    second = compile_specification(payload, fixtures.fixture_policy(), fixtures.fixture_registry())

    assert first.plan is None
    assert [refusal.to_dict() for refusal in first.errors] == [
        refusal.to_dict() for refusal in second.errors
    ]
    assert all(refusal.message for refusal in first.errors)
    assert all(isinstance(refusal.path, str) for refusal in first.errors)


def test_a_legal_specification_compiles_while_every_dynamic_import_is_unavailable(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    """
    Test that the compiled artefact is constructed in process and never imported.

    Every dynamic import is recorded for the duration of one compilation, and the recording must be
    empty: a compiler that reached for a module named by, or derived from, a candidate would show up
    here rather than pass. The recording delegates to the real import so the test cannot disturb the
    interpreter it runs in.
    """
    real_import = builtins.__import__
    real_import_module = importlib.import_module
    imported: list[str] = []

    def _record(name: str, *args: object, **kwargs: object) -> object:
        imported.append(name)
        return real_import(name, *args, **kwargs)

    def _record_module(name: str, *args: object, **kwargs: object) -> object:
        imported.append(name)
        return real_import_module(name, *args, **kwargs)

    monkeypatch.setattr(builtins, "__import__", _record)
    monkeypatch.setattr(importlib, "import_module", _record_module)

    imported.clear()
    result = compile_specification(
        fixtures.legal_payload(),
        fixtures.fixture_policy(),
        fixtures.fixture_registry(),
    )
    during_compilation = list(imported)

    monkeypatch.undo()

    assert during_compilation == []

    assert result.accepted
    assert result.plan is not None
    plan = result.plan

    assert type(plan).__module__ == "nautilus_trader.discovery.compile"
    assert not hasattr(plan, "__code__")
    assert not callable(plan)
    assert not isinstance(plan, type(sys))
    assert not any(callable(value) for value in _leaves(plan.to_dict()))
    assert plan.specification_digest
    assert plan.behavioural_digest


def _leaves(payload: object) -> Iterator[object]:
    """
    Yield every leaf of a nested plain-data payload.
    """
    if isinstance(payload, dict):
        for value in payload.values():
            yield from _leaves(value)
        return
    if isinstance(payload, list | tuple):
        for item in payload:
            yield from _leaves(item)
        return

    yield payload


def test_the_interpreter_is_the_only_executor_the_plan_names() -> None:
    """
    Test that the plan names no executor at all.

    The plan is data: it holds the resolved feature definitions and the identities a run is named by,
    and the only thing that turns it into behaviour is the interpreter this package defines. A plan
    that named a module, a class or a callable would be a second execution path.
    """
    result = compile_specification(
        fixtures.legal_payload(),
        fixtures.fixture_policy(),
        fixtures.fixture_registry(),
    )

    assert result.plan is not None
    payload = result.plan.to_dict()
    code_names = {"module", "module_path", "entry_point", "strategy_path", "code", "callable"}

    assert code_names & _keys(payload) == set()
    assert all(not callable(leaf) for leaf in _leaves(payload))
    assert all(isinstance(leaf, str | int | float | bool | type(None)) for leaf in _leaves(payload))


def _keys(payload: object) -> set[str]:
    """
    Return every key of a nested plain-data payload.
    """
    if isinstance(payload, dict):
        found: set[str] = set()
        for key, value in payload.items():
            found.add(str(key))
            found |= _keys(value)
        return found
    if isinstance(payload, list | tuple):
        nested: set[str] = set()
        for item in payload:
            nested |= _keys(item)
        return nested

    return set()
