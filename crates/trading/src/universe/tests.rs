// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2026 Nautech Systems Pty Ltd. All rights reserved.
//  https://nautechsystems.io
//
//  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
//  You may not use this file except in compliance with the License.
//  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
// -------------------------------------------------------------------------------------------------

//! Tests for the universe component.

use std::{cell::RefCell, collections::BTreeMap, rc::Rc, sync::Arc};

use nautilus_common::{
    actor::{DataActor, registry::get_actor_unchecked},
    cache::Cache,
    clock::{Clock, VirtualClock},
    component::Component,
    messages::data::{DataCommand, SubscribeCommand, UnsubscribeCommand},
    msgbus::{
        self, MessageBus,
        stubs::{TypedIntoMessageSavingHandler, get_typed_into_message_saving_handler},
        switchboard::MessagingSwitchboard,
    },
    runner::{SyncDataCommandSender, set_data_cmd_sender},
};
use nautilus_core::UnixNanos;
use nautilus_model::{
    data::{BarSpecification, BarType},
    enums::{AggregationSource, BarAggregation, OmsType, PositionSide, PriceType},
    identifiers::{InstrumentId, TraderId, Venue},
    instruments::{
        InstrumentAny,
        stubs::{audusd_sim, gbpusd_sim},
    },
    position::Position,
    stubs::stub_position_long,
};
use rstest::rstest;
use ustr::Ustr;

use crate::universe::{
    ScheduledUniverseRule, StaticUniverseRule, Universe, UniverseBarSpec, UniverseDefinition,
    UniverseMembershipState, UniverseRemovalPolicy, UniverseSubscription, rule::SharedUniverseRule,
};

const AUDUSD: &str = "AUD/USD.SIM";
const GBPUSD: &str = "GBP/USD.SIM";
const NOW_NS: u64 = 1_700_000_000_000_000_000;
const T1_NS: u64 = NOW_NS + 1_000_000_000;
const T2_NS: u64 = NOW_NS + 2_000_000_000;

fn instrument_id(value: &str) -> InstrumentId {
    InstrumentId::from(value)
}

fn ts(value: u64) -> UnixNanos {
    UnixNanos::from(value)
}

fn static_rule(symbols: &[&str]) -> SharedUniverseRule {
    let instruments = symbols.iter().map(|value| instrument_id(value)).collect();
    Rc::new(RefCell::new(
        StaticUniverseRule::new("static", instruments).unwrap(),
    ))
}

fn scheduled_rule(sets: &[(u64, &[&str])]) -> SharedUniverseRule {
    let sets = sets.iter().map(|(effective_ns, symbols)| {
        (
            ts(*effective_ns),
            symbols.iter().map(|value| instrument_id(value)).collect(),
        )
    });

    Rc::new(RefCell::new(
        ScheduledUniverseRule::from_schedule("scheduled", sets).unwrap(),
    ))
}

fn static_definition(name: &str, symbols: &[&str]) -> UniverseDefinition {
    UniverseDefinition::new(name, Venue::from("SIM"), static_rule(symbols)).unwrap()
}

/// A registered universe with a captured data command stream.
struct Setup {
    clock: Rc<RefCell<VirtualClock>>,
    cache: Rc<RefCell<Cache>>,
    actor_id: Ustr,
    commands: TypedIntoMessageSavingHandler<DataCommand>,
}

impl Setup {
    fn new(definition: UniverseDefinition) -> Self {
        set_data_cmd_sender(Arc::new(SyncDataCommandSender));

        let bus = msgbus::get_message_bus();
        *bus.borrow_mut() = MessageBus::default();

        let (handler, commands) = get_typed_into_message_saving_handler::<DataCommand>(None);
        msgbus::register_data_command_endpoint(
            MessagingSwitchboard::data_engine_queue_execute(),
            handler,
        );

        let clock = Rc::new(RefCell::new(VirtualClock::new()));
        clock.borrow_mut().set_time(ts(NOW_NS));
        let cache = Rc::new(RefCell::new(Cache::default()));

        let mut universe = Universe::new(definition);
        let actor_id = universe.actor_id().inner();
        Component::register(
            &mut universe,
            TraderId::from("TESTER-001"),
            clock.clone(),
            cache.clone(),
        )
        .unwrap();
        nautilus_common::actor::registry::register_actor(universe);

        Self {
            clock,
            cache,
            actor_id,
            commands,
        }
    }

    fn universe(&self) -> nautilus_common::actor::registry::ActorRef<Universe> {
        get_actor_unchecked::<Universe>(&self.actor_id)
    }

    fn commands(&self) -> Vec<DataCommand> {
        self.commands.get_messages()
    }

    fn clear_commands(&self) {
        self.commands.clear();
    }

    fn add_instrument(&self, instrument: InstrumentAny) {
        self.cache.borrow_mut().add_instrument(instrument).unwrap();
    }

    /// Adds an open position for `instrument`, as an execution event would.
    fn add_open_position(&self, instrument_id: &InstrumentId) {
        let instrument = if *instrument_id == audusd_sim().id {
            audusd_sim()
        } else {
            gbpusd_sim()
        };

        let position = stub_position_long(instrument.clone());
        self.add_instrument(InstrumentAny::CurrencyPair(instrument));
        self.cache
            .borrow_mut()
            .add_position(&position, OmsType::Netting)
            .unwrap();
    }

    /// Closes the open positions of `instrument`, as an execution event would.
    fn close_positions(&self, instrument_id: &InstrumentId) {
        let positions: Vec<Position> = self
            .cache
            .borrow()
            .positions_open(None, Some(instrument_id), None, None, None)
            .iter()
            .map(nautilus_common::cache::PositionRef::cloned)
            .collect();

        for position in positions {
            let closed = Position {
                side: PositionSide::Flat,
                ts_closed: Some(ts(T2_NS)),
                ..position
            };

            self.cache.borrow_mut().update_position(&closed).unwrap();
        }
    }
}

/// Returns the claim key of a subscribe or unsubscribe command.
fn claim_key(command: &DataCommand) -> Option<String> {
    let (sign, key) = match command {
        DataCommand::Subscribe(command) => ("+", subscribe_key(command)),
        DataCommand::Unsubscribe(command) => ("-", unsubscribe_key(command)),
        _ => return None,
    };

    Some(format!("{sign}{}", key?))
}

fn subscribe_key(command: &SubscribeCommand) -> Option<String> {
    Some(match command {
        SubscribeCommand::Instrument(command) => {
            format!("INSTRUMENT:{}", command.instrument_id)
        }
        SubscribeCommand::InstrumentStatus(command) => {
            format!("STATUS:{}", command.instrument_id)
        }
        SubscribeCommand::Quotes(command) => format!("QUOTES:{}", command.instrument_id),
        SubscribeCommand::Trades(command) => format!("TRADES:{}", command.instrument_id),
        SubscribeCommand::Bars(command) => format!("BARS:{}", command.bar_type),
        _ => return None,
    })
}

fn unsubscribe_key(command: &UnsubscribeCommand) -> Option<String> {
    Some(match command {
        UnsubscribeCommand::Instrument(command) => {
            format!("INSTRUMENT:{}", command.instrument_id)
        }
        UnsubscribeCommand::InstrumentStatus(command) => {
            format!("STATUS:{}", command.instrument_id)
        }
        UnsubscribeCommand::Quotes(command) => format!("QUOTES:{}", command.instrument_id),
        UnsubscribeCommand::Trades(command) => format!("TRADES:{}", command.instrument_id),
        UnsubscribeCommand::Bars(command) => format!("BARS:{}", command.bar_type),
        _ => return None,
    })
}

/// Returns the claim keys of `commands` in command order.
fn claims(commands: &[DataCommand]) -> Vec<String> {
    commands.iter().filter_map(claim_key).collect()
}

/// Asserts that every subscription claim is released exactly once.
fn assert_claims_balanced(claim_keys: &[String]) {
    let mut balance: BTreeMap<String, i64> = BTreeMap::new();

    for key in claim_keys {
        let (sign, key) = key.split_at(1);
        let entry = balance.entry(key.to_string()).or_default();
        *entry += if sign == "+" { 1 } else { -1 };
    }

    let unbalanced: Vec<_> = balance.iter().filter(|(_, count)| **count != 0).collect();
    assert!(unbalanced.is_empty(), "unbalanced claims: {unbalanced:?}");
}

fn instrument_requests(commands: &[DataCommand]) -> Vec<InstrumentId> {
    commands
        .iter()
        .filter_map(|command| match command {
            DataCommand::Request(nautilus_common::messages::data::RequestCommand::Instrument(
                request,
            )) => Some(request.instrument_id),
            _ => None,
        })
        .collect()
}

#[rstest]
fn test_select_adds_members_and_requests_metadata() {
    let setup = Setup::new(static_definition("equities", &[AUDUSD, GBPUSD]));
    let mut universe = setup.universe();

    assert_eq!(universe.select(ts(T1_NS)).unwrap(), 2);
    assert_eq!(
        universe.members(),
        vec![instrument_id(AUDUSD), instrument_id(GBPUSD)]
    );
    assert_eq!(
        universe.state(&instrument_id(AUDUSD)),
        Some(UniverseMembershipState::Added)
    );
    assert!(universe.is_member(&instrument_id(AUDUSD)));
    assert!(!universe.is_active(&instrument_id(AUDUSD)));

    let commands = setup.commands();
    assert_eq!(
        claims(&commands),
        vec![
            format!("+INSTRUMENT:{AUDUSD}"),
            format!("+QUOTES:{AUDUSD}"),
            format!("+TRADES:{AUDUSD}"),
            format!("+INSTRUMENT:{GBPUSD}"),
            format!("+QUOTES:{GBPUSD}"),
            format!("+TRADES:{GBPUSD}"),
        ]
    );
    assert_eq!(
        instrument_requests(&commands),
        vec![instrument_id(AUDUSD), instrument_id(GBPUSD)]
    );

    setup.clear_commands();
    assert_eq!(universe.select(ts(T2_NS)).unwrap(), 0);
    assert!(setup.commands().is_empty());
}

#[rstest]
fn test_select_is_deterministic_for_an_unsorted_rule() {
    let rule: SharedUniverseRule = Rc::new(RefCell::new(
        StaticUniverseRule::new(
            "unsorted",
            vec![
                instrument_id(GBPUSD),
                instrument_id(AUDUSD),
                instrument_id(AUDUSD),
            ],
        )
        .unwrap(),
    ));
    let definition = UniverseDefinition::new("equities", Venue::from("SIM"), rule).unwrap();
    let setup = Setup::new(definition);
    let mut universe = setup.universe();

    assert_eq!(universe.select(ts(T1_NS)).unwrap(), 2);
    assert_eq!(
        universe.members(),
        vec![instrument_id(AUDUSD), instrument_id(GBPUSD)]
    );
    assert_eq!(
        claims(&setup.commands()),
        vec![
            format!("+INSTRUMENT:{AUDUSD}"),
            format!("+QUOTES:{AUDUSD}"),
            format!("+TRADES:{AUDUSD}"),
            format!("+INSTRUMENT:{GBPUSD}"),
            format!("+QUOTES:{GBPUSD}"),
            format!("+TRADES:{GBPUSD}"),
        ]
    );
}

#[rstest]
fn test_member_promotes_to_active_when_the_definition_is_known() {
    let setup = Setup::new(static_definition("equities", &[AUDUSD]));
    let mut universe = setup.universe();

    universe.select(ts(T1_NS)).unwrap();
    setup.add_instrument(InstrumentAny::CurrencyPair(audusd_sim()));

    assert_eq!(universe.select(ts(T2_NS)).unwrap(), 1);
    assert!(universe.is_active(&instrument_id(AUDUSD)));
    assert_eq!(
        universe
            .membership(&instrument_id(AUDUSD))
            .unwrap()
            .added_ns,
        ts(T1_NS)
    );
    assert_eq!(
        universe
            .membership(&instrument_id(AUDUSD))
            .unwrap()
            .updated_ns,
        ts(T2_NS)
    );
}

#[rstest]
fn test_member_promotes_on_the_instrument_callback() {
    let setup = Setup::new(static_definition("equities", &[AUDUSD]));
    let mut universe = setup.universe();

    universe.select(ts(T1_NS)).unwrap();
    DataActor::on_instrument(&mut *universe, &InstrumentAny::CurrencyPair(audusd_sim())).unwrap();

    assert!(universe.is_active(&instrument_id(AUDUSD)));
}

#[rstest]
fn test_deselected_member_is_removed_and_releases_its_subscriptions() {
    let definition = UniverseDefinition::new(
        "equities",
        Venue::from("SIM"),
        scheduled_rule(&[(T1_NS, &[AUDUSD, GBPUSD]), (T2_NS, &[AUDUSD])]),
    )
    .unwrap();
    let setup = Setup::new(definition);
    let mut universe = setup.universe();

    assert_eq!(universe.select(ts(T1_NS)).unwrap(), 2);
    setup.clear_commands();

    assert_eq!(universe.select(ts(T2_NS)).unwrap(), 2);
    assert_eq!(
        universe.state(&instrument_id(GBPUSD)),
        Some(UniverseMembershipState::Removed)
    );
    assert!(!universe.is_member(&instrument_id(GBPUSD)));
    assert!(universe.is_member(&instrument_id(AUDUSD)));
    assert_eq!(
        claims(&setup.commands()),
        vec![
            format!("-INSTRUMENT:{GBPUSD}"),
            format!("-QUOTES:{GBPUSD}"),
            format!("-TRADES:{GBPUSD}"),
        ]
    );
}

#[rstest]
fn test_removal_is_held_while_a_position_is_open() {
    let definition = UniverseDefinition::new(
        "equities",
        Venue::from("SIM"),
        scheduled_rule(&[(T1_NS, &[AUDUSD, GBPUSD]), (T2_NS, &[AUDUSD])]),
    )
    .unwrap();
    let setup = Setup::new(definition);
    let mut universe = setup.universe();

    universe.select(ts(T1_NS)).unwrap();
    setup.add_open_position(&instrument_id(GBPUSD));
    setup.clear_commands();

    assert_eq!(universe.select(ts(T2_NS)).unwrap(), 1);
    assert_eq!(
        universe.state(&instrument_id(GBPUSD)),
        Some(UniverseMembershipState::Removing)
    );
    assert!(universe.removal_blocked(&instrument_id(GBPUSD)));
    assert!(universe.is_member(&instrument_id(GBPUSD)));
    assert!(setup.commands().is_empty());

    assert_eq!(universe.check_removals(ts(T2_NS + 1)).unwrap(), 0);
    assert_eq!(
        universe.state(&instrument_id(GBPUSD)),
        Some(UniverseMembershipState::Removing)
    );

    setup.close_positions(&instrument_id(GBPUSD));
    assert_eq!(universe.check_removals(ts(T2_NS + 2)).unwrap(), 1);
    assert_eq!(
        universe.state(&instrument_id(GBPUSD)),
        Some(UniverseMembershipState::Removed)
    );
    assert!(!universe.removal_blocked(&instrument_id(GBPUSD)));
    assert_eq!(
        claims(&setup.commands()),
        vec![
            format!("-INSTRUMENT:{GBPUSD}"),
            format!("-QUOTES:{GBPUSD}"),
            format!("-TRADES:{GBPUSD}"),
        ]
    );
}

#[rstest]
fn test_removal_releases_regardless_of_position_under_its_policy() {
    let definition = UniverseDefinition::new(
        "equities",
        Venue::from("SIM"),
        scheduled_rule(&[(T1_NS, &[AUDUSD, GBPUSD]), (T2_NS, &[AUDUSD])]),
    )
    .unwrap()
    .with_removal_policy(UniverseRemovalPolicy::ReleaseRegardless);
    let setup = Setup::new(definition);
    let mut universe = setup.universe();

    universe.select(ts(T1_NS)).unwrap();
    setup.add_open_position(&instrument_id(GBPUSD));
    setup.clear_commands();

    assert_eq!(universe.select(ts(T2_NS)).unwrap(), 2);
    assert_eq!(
        universe.state(&instrument_id(GBPUSD)),
        Some(UniverseMembershipState::Removed)
    );
    assert_eq!(claims(&setup.commands()).len(), 3);
}

#[rstest]
fn test_reselection_cancels_a_held_removal() {
    let definition = UniverseDefinition::new(
        "equities",
        Venue::from("SIM"),
        scheduled_rule(&[
            (T1_NS, &[AUDUSD, GBPUSD]),
            (T2_NS, &[AUDUSD]),
            (T2_NS + 1, &[AUDUSD, GBPUSD]),
        ]),
    )
    .unwrap();
    let setup = Setup::new(definition);
    let mut universe = setup.universe();

    universe.select(ts(T1_NS)).unwrap();
    setup.add_open_position(&instrument_id(GBPUSD));

    assert_eq!(universe.select(ts(T2_NS)).unwrap(), 1);
    assert!(universe.removal_blocked(&instrument_id(GBPUSD)));
    setup.clear_commands();

    assert_eq!(universe.select(ts(T2_NS + 1)).unwrap(), 1);
    assert!(universe.is_active(&instrument_id(GBPUSD)));
    assert!(!universe.removal_blocked(&instrument_id(GBPUSD)));
    assert!(setup.commands().is_empty());
}

#[rstest]
fn test_removed_member_can_rejoin() {
    let definition = UniverseDefinition::new(
        "equities",
        Venue::from("SIM"),
        scheduled_rule(&[(T1_NS, &[AUDUSD]), (T2_NS, &[]), (T2_NS + 1, &[AUDUSD])]),
    )
    .unwrap();
    let setup = Setup::new(definition);
    let mut universe = setup.universe();

    universe.select(ts(T1_NS)).unwrap();
    assert_eq!(universe.select(ts(T2_NS)).unwrap(), 2);
    assert_eq!(
        universe.state(&instrument_id(AUDUSD)),
        Some(UniverseMembershipState::Removed)
    );
    setup.clear_commands();

    assert_eq!(universe.select(ts(T2_NS + 1)).unwrap(), 1);
    assert_eq!(
        universe.state(&instrument_id(AUDUSD)),
        Some(UniverseMembershipState::Added)
    );
    assert_eq!(
        universe
            .membership(&instrument_id(AUDUSD))
            .unwrap()
            .added_ns,
        ts(T2_NS + 1)
    );
    assert_eq!(claims(&setup.commands()).len(), 3);
}

#[rstest]
fn test_explicit_add_and_remove() {
    let setup = Setup::new(static_definition("equities", &[]));
    let mut universe = setup.universe();
    let audusd = instrument_id(AUDUSD);

    universe.add(audusd, ts(T1_NS)).unwrap();
    assert_eq!(universe.members(), vec![audusd]);
    assert_eq!(
        universe.state(&audusd),
        Some(UniverseMembershipState::Added)
    );

    assert_eq!(universe.select(ts(T2_NS)).unwrap(), 2);
    assert_eq!(
        universe.state(&audusd),
        Some(UniverseMembershipState::Removed)
    );
    assert_eq!(universe.member_count(), 1);

    universe.add(audusd, ts(T2_NS + 1)).unwrap();
    assert_eq!(
        universe.state(&audusd),
        Some(UniverseMembershipState::Added)
    );

    universe.remove(audusd, ts(T2_NS + 2)).unwrap();
    assert_eq!(
        universe.state(&audusd),
        Some(UniverseMembershipState::Removed)
    );
    assert_claims_balanced(&claims(&setup.commands()));
}

#[rstest]
fn test_explicit_remove_is_held_while_a_position_is_open() {
    let setup = Setup::new(static_definition("equities", &[AUDUSD]));
    let mut universe = setup.universe();
    let audusd = instrument_id(AUDUSD);

    universe.select(ts(T1_NS)).unwrap();
    setup.add_open_position(&audusd);
    setup.clear_commands();

    universe.remove(audusd, ts(T2_NS)).unwrap();
    assert_eq!(
        universe.state(&audusd),
        Some(UniverseMembershipState::Removing)
    );
    assert!(universe.removal_blocked(&audusd));
    assert!(setup.commands().is_empty());
}

#[rstest]
fn test_stop_releases_every_member_and_cancels_selection() {
    let definition = static_definition("equities", &[AUDUSD, GBPUSD])
        .with_selection_interval(nautilus_core::DurationNanos::from_mins(1))
        .unwrap();
    let setup = Setup::new(definition);
    let mut universe = setup.universe();

    DataActor::on_start(&mut *universe).unwrap();

    let timer_name = Ustr::from(universe.selection_timer_name().as_str());
    assert!(setup.clock.borrow().timer_exists(&timer_name));
    assert_eq!(universe.member_count(), 2);

    DataActor::on_stop(&mut *universe).unwrap();

    assert!(!setup.clock.borrow().timer_exists(&timer_name));
    assert_eq!(universe.active_members(), Vec::new());
    assert!(!universe.is_member(&instrument_id(AUDUSD)));
    assert!(!universe.is_member(&instrument_id(GBPUSD)));
    assert_claims_balanced(&claims(&setup.commands()));
}

#[rstest]
fn test_stop_releases_a_held_removal() {
    let setup = Setup::new(static_definition("equities", &[AUDUSD, GBPUSD]));
    let mut universe = setup.universe();

    universe.select(ts(T1_NS)).unwrap();
    setup.add_open_position(&instrument_id(GBPUSD));
    universe.remove(instrument_id(GBPUSD), ts(T2_NS)).unwrap();
    assert!(universe.removal_blocked(&instrument_id(GBPUSD)));

    DataActor::on_stop(&mut *universe).unwrap();
    assert_eq!(
        universe.state(&instrument_id(GBPUSD)),
        Some(UniverseMembershipState::Removed)
    );
    assert_claims_balanced(&claims(&setup.commands()));
}

#[rstest]
fn test_start_requests_venue_instruments_and_selects() {
    let setup = Setup::new(static_definition("equities", &[AUDUSD]));
    let mut universe = setup.universe();

    DataActor::on_start(&mut *universe).unwrap();

    let venue_requests = setup
        .commands()
        .iter()
        .filter(|command| {
            matches!(
                command,
                DataCommand::Request(
                    nautilus_common::messages::data::RequestCommand::Instruments(_)
                )
            )
        })
        .count();
    assert_eq!(venue_requests, 1);
    assert!(universe.is_member(&instrument_id(AUDUSD)));
}

#[rstest]
fn test_bar_subscription_uses_the_configured_specification() {
    let bar_spec = UniverseBarSpec::new(5, BarAggregation::Minute, PriceType::Bid).unwrap();
    let definition = static_definition("equities", &[AUDUSD])
        .with_subscription(UniverseSubscription::Bars)
        .with_bar_spec(bar_spec);
    let setup = Setup::new(definition);
    let mut universe = setup.universe();

    universe.select(ts(T1_NS)).unwrap();

    let expected = BarType::new(
        instrument_id(AUDUSD),
        BarSpecification::new(5, BarAggregation::Minute, PriceType::Bid),
        AggregationSource::External,
    );
    assert_eq!(claims(&setup.commands()).len(), 4);
    assert!(claims(&setup.commands()).contains(&format!("+BARS:{expected}")));
}

#[rstest]
fn test_defined_subscription_set_is_released_in_full() {
    let definition = static_definition("equities", &[AUDUSD]).with_subscriptions(vec![
        UniverseSubscription::Instrument,
        UniverseSubscription::InstrumentStatus,
        UniverseSubscription::Quotes,
        UniverseSubscription::Trades,
        UniverseSubscription::Bars,
    ]);
    let setup = Setup::new(definition);
    let mut universe = setup.universe();

    universe.select(ts(T1_NS)).unwrap();
    let mut all_claims = claims(&setup.commands());
    assert_eq!(all_claims.len(), 5);
    setup.clear_commands();

    universe.remove(instrument_id(AUDUSD), ts(T2_NS)).unwrap();
    let released = claims(&setup.commands());
    assert_eq!(
        released,
        vec![
            format!("-INSTRUMENT:{AUDUSD}"),
            format!("-STATUS:{AUDUSD}"),
            format!("-QUOTES:{AUDUSD}"),
            format!("-TRADES:{AUDUSD}"),
            format!("-BARS:{AUDUSD}-1-MINUTE-LAST-EXTERNAL"),
        ]
    );

    all_claims.extend(released);
    assert_claims_balanced(&all_claims);
}
