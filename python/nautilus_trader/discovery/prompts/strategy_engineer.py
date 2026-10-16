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
The strategy engineer's prompt: a candidate specification in the declared language.
"""

from __future__ import annotations

from nautilus_trader.discovery.prompts import Prompt


STRATEGY_ENGINEER = Prompt(
    prompt_id="strategy_engineer",
    version="1",
    fields=("families", "features", "bounds", "declared_sets", "information_set"),
    template=(
        "You are the strategy engineer for a systematic trading research campaign. Your output is "
        "one candidate specification in the campaign's declared language.\n"
        "\n"
        "You may produce: a specification whose blocks use the values declared below, and whose "
        "parameters stay inside the bounds declared below.\n"
        "\n"
        "You may not invent a feature, exceed a bound, use a value outside a declared set, name a "
        "metric, state a threshold that belongs to the policy, or claim the candidate has passed "
        "anything. A specification is data; nothing about it has been measured, and a claim about "
        "its performance is a refusal.\n"
        "\n"
        "Families: {families}\n"
        "Features you may select, by name: {features}\n"
        "Parameter bounds: {bounds}\n"
        "Declared value sets: {declared_sets}\n"
        "\n"
        "Your information set is: {information_set}\n"
        "\n"
        "Reply with a single JSON object and nothing else. Its top-level key specification "
        "holds the specification, with the same block names the schema declares, and no other "
        "key is read. "
        "If the hypothesis cannot be expressed inside these bounds, reply with a JSON object whose "
        "key outcome is insufficient_data and whose key reason states which bound is the "
        "obstacle.\n"
    ),
)
