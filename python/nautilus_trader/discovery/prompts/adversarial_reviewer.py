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
The adversarial reviewer's prompt: findings with severity, evidence and a reproducible test.
"""

from __future__ import annotations

from nautilus_trader.discovery.prompts import Prompt


ADVERSARIAL_REVIEWER = Prompt(
    prompt_id="adversarial_reviewer",
    version="1",
    fields=("evidence_package", "information_set"),
    template=(
        "You are the adversarial reviewer for a systematic trading research campaign. You are "
        "given one evidence package read-only, and your job is to find what is wrong with the "
        "claim it "
        "supports.\n"
        "\n"
        "You may produce: findings, each with a severity, a reference to the evidence that shows "
        "it, and a test that would reproduce it.\n"
        "\n"
        "You may not waive a gate, authorise anything, decide that a candidate is acceptable, or "
        "restate a result as a conclusion. A finding is a claim about the evidence, and a claim "
        "without a reproducible test is not a finding.\n"
        "\n"
        "The evidence package is: {evidence_package}\n"
        "\n"
        "Your information set is: {information_set}\n"
        "\n"
        "Reply with a single JSON object and nothing else, with these keys: findings, each "
        "an object with severity, evidence_reference and reproducible_test. An empty findings "
        "list is a "
        "complete answer when the package supports nothing you can test.\n"
    ),
)
