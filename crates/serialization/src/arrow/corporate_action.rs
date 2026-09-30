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

use std::collections::HashMap;

use arrow::{datatypes::Schema, error::ArrowError, record_batch::RecordBatch};
use nautilus_model::data::{CorporateAction, Data};

use super::{
    ArrowSchemaProvider, DecodeDataFromRecordBatch, DecodeFromRecordBatch, EncodeToRecordBatch,
    EncodingError, KEY_INSTRUMENT_ID,
    json::{
        JsonFieldSpec, decode_batch_with_metadata_fields, encode_batch_with_identifier,
        metadata_for_type, schema_for_type_with_identifier,
    },
};

const CORPORATE_ACTION_FIELDS: &[JsonFieldSpec] = &[
    JsonFieldSpec::enum_dictionary("action", false),
    JsonFieldSpec::utf8("value", false),
    JsonFieldSpec::utf8("new_symbol", true),
    JsonFieldSpec::timestamp("effective_ns", false),
    JsonFieldSpec::timestamp("ts_event", false),
    JsonFieldSpec::timestamp("ts_init", false),
];

impl ArrowSchemaProvider for CorporateAction {
    fn get_schema(metadata: Option<HashMap<String, String>>) -> Schema {
        schema_for_type_with_identifier("CorporateAction", metadata, CORPORATE_ACTION_FIELDS)
    }
}

impl EncodeToRecordBatch for CorporateAction {
    fn encode_batch<T>(
        metadata: &HashMap<String, String>,
        data: &[T],
    ) -> Result<RecordBatch, ArrowError>
    where
        T: std::borrow::Borrow<Self>,
    {
        encode_batch_with_identifier(
            "CorporateAction",
            metadata,
            data.iter().map(std::borrow::Borrow::borrow),
            CORPORATE_ACTION_FIELDS,
            data.iter()
                .map(std::borrow::Borrow::borrow)
                .map(|action| action.instrument_id),
        )
    }

    fn metadata(&self) -> HashMap<String, String> {
        let mut metadata = metadata_for_type("CorporateAction");
        metadata.extend(Self::get_metadata(&self.instrument_id));
        metadata
    }
}

impl DecodeFromRecordBatch for CorporateAction {
    fn decode_batch(
        metadata: &HashMap<String, String>,
        record_batch: RecordBatch,
    ) -> Result<Vec<Self>, EncodingError> {
        decode_batch_with_metadata_fields(
            metadata,
            &record_batch,
            CORPORATE_ACTION_FIELDS,
            &[KEY_INSTRUMENT_ID],
            Some("CorporateAction"),
        )
    }
}

impl DecodeDataFromRecordBatch for CorporateAction {
    fn decode_data_batch(
        metadata: &HashMap<String, String>,
        record_batch: RecordBatch,
    ) -> Result<Vec<Data>, EncodingError> {
        let actions: Vec<Self> = Self::decode_batch(metadata, record_batch)?;
        Ok(actions.into_iter().map(Data::from).collect())
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use nautilus_core::UnixNanos;
    use nautilus_model::{
        data::CorporateActionType,
        identifiers::{InstrumentId, Symbol},
    };
    use rstest::rstest;
    use rust_decimal::Decimal;

    use super::*;

    #[rstest]
    fn test_corporate_action_round_trip() {
        let instrument_id = InstrumentId::from("AAPL.XNYS");
        let split = CorporateAction::new(
            instrument_id,
            CorporateActionType::Split,
            Decimal::from_str("4").unwrap(),
            None,
            UnixNanos::from(1_000_000_000),
            UnixNanos::from(2_000_000_000),
            UnixNanos::from(3_000_000_000),
        );
        let symbol_change = CorporateAction::new(
            instrument_id,
            CorporateActionType::SymbolChange,
            Decimal::from_str("0.12345678").unwrap(),
            Some(Symbol::from("AAPL.NEW")),
            UnixNanos::from(4_000_000_000),
            UnixNanos::from(5_000_000_000),
            UnixNanos::from(6_000_000_000),
        );

        let original = vec![split, symbol_change];
        let metadata = split.metadata();
        let record_batch = CorporateAction::encode_batch(&metadata, &original).unwrap();
        let decoded =
            CorporateAction::decode_batch(record_batch.schema().metadata(), record_batch).unwrap();

        assert_eq!(decoded, original);
    }
}
