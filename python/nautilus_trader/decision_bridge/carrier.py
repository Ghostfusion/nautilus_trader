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
The carriage of a raw research decision across the engine's custom-data boundary.

The design's D1 settles carriage: the bridge does not invent a transport, it uses the one the
platform already has. The artifact is a JSON document the research half drops on disk; this class
is the vehicle that lets that document travel the existing custom-data path unchanged
(registration, catalog write, catalog query, `add_data(sort=True)`, `on_data` delivery).

The carrier is deliberately opaque. It carries the raw document as written and the identity fields
the ledger keys on, and it interprets none of them: projecting, validating and resolving
actionability are the jobs of the boundary reader, which runs after delivery. Keeping the carrier
dumb is what lets it round-trip through `to_json_bytes` and `CustomData.from_json_bytes` without a
lossy re-encoding of a document it does not understand.
"""

from __future__ import annotations

from nautilus_trader.model.custom import customdataclass


@customdataclass()
class ResearchDecisionCarrier:
    """
    One research decision artifact, carried as custom data.

    Register the class with `register_custom_data_class` before wrapping instances in `CustomData`
    and writing them to a catalog or adding them to an engine.

    Parameters
    ----------
    decision_id : str
        The identity of the decision, as the boundary reader derives it. It is the row identity of
        the admission ledger and the identity a fill is traced back to.
    artifact_sha256 : str
        The digest of the raw document, recomputed by the reader before admission.
    idempotency_key : str
        The artifact's own key, which distinguishes a repeated delivery of one artifact from a new
        artifact for the same reference date.
    produced_at : str
        The instant the artifact existed, as written in the document. Carried verbatim rather than
        parsed, so the carrier interprets no timestamp.
    effective_date : str
        The reference date of the analysis, as written in the document.
    document : str
        The raw JSON document, exactly as the research half wrote it.

    """

    decision_id: str
    artifact_sha256: str
    idempotency_key: str
    produced_at: str
    effective_date: str
    document: str
