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
Test EODHD factories behavior.
"""

import pytest

from nautilus_trader.adapters.eodhd import EodhdDataClientConfig
from nautilus_trader.adapters.eodhd import EodhdDataClientFactory
from nautilus_trader.common import Environment
from nautilus_trader.live import LiveNode
from nautilus_trader.model import TraderId


EODHD = "EODHD"


def test_eodhd_data_factory_exposes_python_name() -> None:
    """
    Test EODHD data factory exposes python name.
    """
    assert EodhdDataClientFactory().name() == EODHD


def test_eodhd_config_defaults() -> None:
    """
    Test EODHD config defaults.
    """
    config = EodhdDataClientConfig()

    assert config.exchange == "US"
    assert config.poll_interval_secs == 60
    assert config.backfill_days == 5
    assert config.price_precision == 2
    assert config.load_instruments is True
    assert config.has_api_key is False


def test_eodhd_config_accepts_overrides() -> None:
    """
    Test EODHD config accepts overrides.
    """
    config = EodhdDataClientConfig(
        api_key="test-token",
        exchange="LSE",
        poll_interval_secs=30,
        backfill_days=2,
        price_precision=4,
        currency="GBP",
        load_instruments=False,
    )

    assert config.has_api_key is True
    assert config.exchange == "LSE"
    assert config.poll_interval_secs == 30
    assert config.backfill_days == 2
    assert config.price_precision == 4
    assert config.load_instruments is False


def test_eodhd_config_repr_redacts_the_api_key() -> None:
    """
    Test the config representation does not disclose the API key.
    """
    config = EodhdDataClientConfig(api_key="super-secret-token")

    assert "super-secret-token" not in repr(config)
    assert "EodhdDataClientConfig" in repr(config)


def test_eodhd_config_rejects_an_unknown_argument() -> None:
    """
    Test EODHD config rejects an unknown argument.
    """
    with pytest.raises(TypeError):
        EodhdDataClientConfig(nonsense=1)  # type: ignore[call-arg]


def test_live_node_builder_accepts_eodhd_data_factory() -> None:
    """
    Test live node builder accepts the EODHD data factory.
    """
    trader_id = TraderId.from_str("TESTER-001")

    node = (
        LiveNode.builder("EODHD-DATA-PYTEST-001", trader_id, Environment.SANDBOX)
        .add_data_client(
            None,
            EodhdDataClientFactory(),
            EodhdDataClientConfig(api_key="test-token"),
        )
        .build()
    )

    assert node.trader_id == trader_id
    assert node.environment == Environment.SANDBOX
