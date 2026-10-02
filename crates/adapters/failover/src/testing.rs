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

//! A provider that answers from a script, so that a chain can be exercised without a network.
//!
//! What a chain does is a property of the chain and not of a venue, so it should be provable
//! without one. This provider is deterministic in the only way that matters to a test: every answer
//! is written down before the demand is made, and each call takes the next one in order.

use std::collections::VecDeque;

use nautilus_model::identifiers::ClientId;

use crate::{chain::Provider, failure::Failure};

/// A provider whose answers are written down in advance.
///
/// The answers are read in the order the attempts are made, so a script of `[fail, ok]` describes one
/// failed attempt and one successful one, whether they are a retry or a hop. The demands it was
/// asked are kept, which is how a test states that a provider was *not* used: a chain that never
/// hops leaves the secondary with nothing to answer.
#[derive(Debug)]
pub struct ScriptedProvider<D, T> {
    id: ClientId,
    script: VecDeque<Result<T, Failure>>,
    demands: Vec<D>,
}

impl<D, T> ScriptedProvider<D, T> {
    /// Creates a provider that answers with `script`, in order.
    #[must_use]
    pub fn new(id: ClientId, script: Vec<Result<T, Failure>>) -> Self {
        Self {
            id,
            script: script.into(),
            demands: Vec::new(),
        }
    }

    /// Returns the demands this provider was asked, in the order it was asked them.
    #[must_use]
    pub fn demands(&self) -> &[D] {
        &self.demands
    }

    /// Returns how many answers the script has left.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.script.len()
    }
}

impl<D: Clone + 'static, T: 'static> Provider for ScriptedProvider<D, T> {
    type Demand = D;
    type Answer = T;

    fn id(&self) -> ClientId {
        self.id
    }

    /// # Errors
    ///
    /// Returns the failure the script holds for this attempt.
    ///
    /// # Panics
    ///
    /// Panics when the script has no answer left. A call that was not scripted is a test that
    /// expected something else to happen, and answering it anyway would hide that.
    fn serve(&mut self, demand: &Self::Demand) -> Result<Self::Answer, Failure> {
        self.demands.push(demand.clone());

        self.script
            .pop_front()
            .expect("the script has no answer left for this demand")
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_the_script_is_read_in_order() {
        let mut provider = ScriptedProvider::new(
            ClientId::from("PRIMARY"),
            vec![Err(Failure::Unanswered), Ok("second answer".to_string())],
        );

        assert_eq!(provider.remaining(), 2);
        assert_eq!(provider.serve(&7_u32), Err(Failure::Unanswered));
        assert_eq!(provider.serve(&8_u32), Ok("second answer".to_string()));
        assert_eq!(provider.remaining(), 0);
        assert_eq!(provider.demands(), &[7, 8]);
    }

    #[rstest]
    fn test_a_provider_that_was_never_asked_has_no_demands() {
        let provider: ScriptedProvider<u32, String> =
            ScriptedProvider::new(ClientId::from("SECONDARY"), vec![Ok("answer".to_string())]);

        assert!(provider.demands().is_empty());
    }
}
