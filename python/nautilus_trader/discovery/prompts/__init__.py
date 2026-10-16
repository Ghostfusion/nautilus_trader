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
The prompt versions the five roles are asked with.

A prompt is data with an identity: its identifier, its version and its template, digested together,
so an invocation record can say which instructions produced a response. The templates carry the
authority limits of the role that receives them, and the information set the role is given is
limited to the fields the template names, which is what keeps a prompt from becoming the place a
gate condition is smuggled into.
"""

from __future__ import annotations

from dataclasses import dataclass
from dataclasses import field

from nautilus_trader.optimization.space import digest_of


PROMPTS_ID = "discovery.prompts"


@dataclass(frozen=True)
class Prompt:
    """
    One versioned set of instructions.

    Parameters
    ----------
    prompt_id : str
        The prompt's identifier, which a reader of an invocation record looks up.
    version : str
        The version, which changes whenever the template changes.
    template : str
        The instructions, with named fields in braces.
    fields : tuple[str, ...]
        The fields the template expects.

    """

    prompt_id: str
    version: str
    template: str
    fields: tuple[str, ...] = field(default_factory=tuple)

    def render(self, **values: object) -> str:
        """
        Return the instructions with their fields filled in.

        Parameters
        ----------
        **values : object
            The field values, one per declared field.

        Returns
        -------
        str

        Raises
        ------
        ValueError
            If a declared field is missing or an undeclared field is supplied, because a prompt
            assembled from the wrong information set is not the prompt the version names.

        """
        missing = sorted(name for name in self.fields if name not in values)
        if missing:
            raise ValueError(f"{self.prompt_id} {self.version} is missing {', '.join(missing)}")

        extra = sorted(name for name in values if name not in self.fields)
        if extra:
            raise ValueError(f"{self.prompt_id} {self.version} was given {', '.join(extra)}")

        return self.template.format(**values)

    def digest(self) -> str:
        """
        Return the prompt's canonical digest, which is what an invocation record carries.

        Returns
        -------
        str

        """
        return digest_of(
            {
                "prompt_id": self.prompt_id,
                "version": self.version,
                "fields": ",".join(self.fields),
                "template": self.template,
            },
        )
