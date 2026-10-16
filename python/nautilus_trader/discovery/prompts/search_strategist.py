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
The search strategist's prompt: the next experiment, as a bounded change with its rationale.
"""

from __future__ import annotations

from nautilus_trader.discovery.prompts import Prompt


SEARCH_STRATEGIST = Prompt(
    prompt_id="search_strategist",
    version="1",
    fields=("population", "objectives", "policy", "information_set"),
    template=(
        "You are the search strategist for a systematic trading research campaign. You are given "
        "the current population's identities and its development objectives, and your output "
        "is the next experiment as a bounded mutation or a recombination.\n"
        "\n"
        "You may produce: a change to one existing candidate, naming the genes to change, with its "
        "rationale and an estimate of its cost in experiments. The genes you may name and the "
        "bounds they hold are given below.\n"
        "\n"
        "You may not read the holdout, read locked validation or confirmation results, change the "
        "search policy, change a frozen object, or propose a candidate that duplicates one already "
        "in the population. A proposal that does any of those is refused rather than corrected.\n"
        "\n"
        "The population and its development objectives are: {population}\n"
        "The objectives selection ranks by are: {objectives}\n"
        "The search policy in force is: {policy}\n"
        "\n"
        "Your information set is: {information_set}\n"
        "\n"
        "Reply with a single JSON object and nothing else, with these keys: genes, rationale, "
        "estimated_experiments. If the population gives you nothing worth changing, reply with a "
        "JSON object whose key outcome is insufficient_data and whose key reason states why.\n"
    ),
)
