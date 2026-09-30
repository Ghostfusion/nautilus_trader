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
Process-level fan-out, limited by memory rather than CPU.

Backtest execution is single-threaded inside the kernel and Python holds the GIL, so runs fan out
to separate processes. Each worker loads the extension, a catalog slice, and an engine, so the
constraint is memory, not CPU: the default worker count is the available memory budget divided by a
per-run footprint estimate, not a processor count.

The per-run estimate is measured, not guessed. A fresh worker holds roughly 71 MiB before a run,
peaks near 160 MiB of working set, and commits near 472 MiB of pagefile for the sample run in the
measurement. `DEFAULT_PER_RUN_BYTES` is rounded up to 512 MiB to cover the committed footprint with
headroom for larger catalogs and engines.
"""

from __future__ import annotations

import ctypes
import multiprocessing
import os
import sys
from concurrent.futures import ProcessPoolExecutor
from dataclasses import dataclass
from typing import TYPE_CHECKING


try:
    import psutil as _psutil
except ImportError:  # pragma: no cover - psutil is optional
    _psutil = None


if TYPE_CHECKING:
    from collections.abc import Sequence

    from nautilus_trader.optimization.runner import BacktestRunner
    from nautilus_trader.optimization.runner import RunOutcome
    from nautilus_trader.optimization.space import Experiment


DEFAULT_PER_RUN_BYTES = 512 * 1024 * 1024
MEMORY_FRACTION = 0.75

# `ProcessPoolExecutor` refuses more than 61 workers on Windows. This is a platform limit, not a
# CPU limit; the memory-derived count is clamped to it so the policy always yields a usable number.
_PLATFORM_MAX_WORKERS = 61 if sys.platform == "win32" else None


class _MemoryStatusEx(ctypes.Structure):
    """
    The Windows `MEMORYSTATUSEX` structure for `GlobalMemoryStatusEx`.
    """

    _fields_ = [
        ("dwLength", ctypes.c_ulong),
        ("dwMemoryLoad", ctypes.c_ulong),
        ("ullTotalPhys", ctypes.c_ulonglong),
        ("ullAvailPhys", ctypes.c_ulonglong),
        ("ullTotalPageFile", ctypes.c_ulonglong),
        ("ullAvailPageFile", ctypes.c_ulonglong),
        ("ullTotalVirtual", ctypes.c_ulonglong),
        ("ullAvailVirtual", ctypes.c_ulonglong),
        ("ullAvailExtendedVirtual", ctypes.c_ulonglong),
    ]


def available_memory_bytes() -> int | None:
    """
    Return the available physical memory in bytes, or None when it cannot be read.

    Uses `psutil` when it is installed, otherwise the platform API: `GlobalMemoryStatusEx` on
    Windows and `sysconf` on POSIX.

    Returns
    -------
    int | None

    """
    if _psutil is not None:
        return int(_psutil.virtual_memory().available)
    if sys.platform == "win32":
        return _windows_available_memory()
    return _posix_available_memory()


def _windows_available_memory() -> int | None:
    """
    Return the available physical memory from the Windows system API.
    """
    status = _MemoryStatusEx()
    status.dwLength = ctypes.sizeof(_MemoryStatusEx)
    if not ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(status)):
        return None
    return int(status.ullAvailPhys)


def _posix_available_memory() -> int | None:
    """
    Return the available physical memory from `sysconf`.
    """
    try:
        pages = os.sysconf("SC_AVPHYS_PAGES")
        page_size = os.sysconf("SC_PAGE_SIZE")
    except (ValueError, OSError, AttributeError):
        return None
    return int(pages) * int(page_size)


@dataclass(frozen=True)
class ConcurrencyPolicy:
    """
    The memory-driven concurrency limit for a sweep.

    The worker count is the available memory budget divided by `per_run_bytes`. Available memory is
    a fraction of physical memory, and `max_workers` caps the result when set. A count of one runs
    the sweep in-process, sequentially.

    Parameters
    ----------
    per_run_bytes : int, default DEFAULT_PER_RUN_BYTES
        The estimated footprint of one worker process.
    memory_fraction : float, default MEMORY_FRACTION
        The fraction of available memory the sweep may use.
    max_workers : int | None, default None
        An explicit cap, or None to use the memory-derived count.

    """

    per_run_bytes: int = DEFAULT_PER_RUN_BYTES
    memory_fraction: float = MEMORY_FRACTION
    max_workers: int | None = None

    def __post_init__(self) -> None:
        """
        Validate the policy.
        """
        if self.per_run_bytes <= 0:
            raise ValueError("per_run_bytes must be positive")
        if not 0.0 < self.memory_fraction <= 1.0:
            raise ValueError("memory_fraction must be in (0, 1]")
        if self.max_workers is not None and self.max_workers < 1:
            raise ValueError("max_workers must be at least 1 when set")

    @classmethod
    def sequential(cls) -> ConcurrencyPolicy:
        """
        Return a policy that runs the sweep in-process, sequentially.
        """
        return cls(max_workers=1)

    def worker_count(self) -> int:
        """
        Return the number of worker processes the policy permits, at least one.

        The count is `max(1, available_memory * memory_fraction // per_run_bytes)`, capped by
        `max_workers` when set and by the platform process limit on Windows, which is 61.

        Returns
        -------
        int

        """
        if self.max_workers is not None and self.max_workers <= 1:
            return 1

        available = available_memory_bytes()
        derived = 1
        if available is not None:
            derived = memory_budget_fraction(available, self.per_run_bytes, self.memory_fraction)
        if self.max_workers is not None:
            derived = min(derived, self.max_workers)
        if _PLATFORM_MAX_WORKERS is not None:
            derived = min(derived, _PLATFORM_MAX_WORKERS)
        return max(1, derived)


def execute_experiments(
    runner: BacktestRunner,
    experiments: Sequence[Experiment],
    metrics: frozenset[str] | None,
    policy: ConcurrencyPolicy,
) -> list[RunOutcome]:
    """
    Execute the experiments sequentially or through a process pool.

    When the policy permits more than one worker and there is more than one experiment, the work is
    fanned out to a spawn-context `ProcessPoolExecutor` that runs the same runner in each child.
    The returned outcomes are in the input order either way, so the result does not depend on the
    execution path.

    Parameters
    ----------
    runner : BacktestRunner
        The runner that executes one experiment.
    experiments : Sequence[Experiment]
        The experiments to execute, in sweep order.
    metrics : frozenset[str] | None
        The metric names to compute from each run.
    policy : ConcurrencyPolicy
        The concurrency limit.

    Returns
    -------
    list[RunOutcome]

    """
    workers = policy.worker_count()
    if workers <= 1 or len(experiments) <= 1:
        return [runner.run(experiment, metrics) for experiment in experiments]

    context = multiprocessing.get_context("spawn")
    payloads = [(runner, experiment, metrics) for experiment in experiments]
    with ProcessPoolExecutor(max_workers=workers, mp_context=context) as pool:
        return list(pool.map(_run_worker, payloads))


def _run_worker(payload: tuple[BacktestRunner, Experiment, frozenset[str] | None]) -> RunOutcome:
    """
    Execute one experiment in a worker process.
    """
    runner, experiment, metrics = payload
    return runner.run(experiment, metrics)


def memory_budget_fraction(available: int, per_run_bytes: int, fraction: float) -> int:
    """
    Return the worker count for the given memory figures.

    This is the pure form of the policy's derivation, exposed so the number can be checked against
    the memory it was computed from.

    Parameters
    ----------
    available : int
        The available physical memory in bytes.
    per_run_bytes : int
        The estimated footprint of one worker process.
    fraction : float
        The fraction of available memory the sweep may use.

    Returns
    -------
    int

    """
    return max(1, int(available * fraction) // per_run_bytes)
