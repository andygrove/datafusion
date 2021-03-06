// Copyright 2018 Grove Enterprises LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Execution of a filter (predicate)

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use arrow::array::{Array, ArrayRef, BinaryArray, BooleanArray, Float64Array};
use arrow::datatypes::{DataType, Schema};
use arrow::record_batch::RecordBatch;

use super::error::{ExecutionError, Result};
use super::expression::RuntimeExpr;
use super::relation::Relation;

pub struct FilterRelation {
    schema: Arc<Schema>,
    input: Rc<RefCell<Relation>>,
    expr: RuntimeExpr,
}

impl FilterRelation {
    pub fn new(input: Rc<RefCell<Relation>>, expr: RuntimeExpr, schema: Arc<Schema>) -> Self {
        Self {
            schema,
            input,
            expr,
        }
    }
}

impl Relation for FilterRelation {
    fn next(&mut self) -> Result<Option<RecordBatch>> {
        match self.input.borrow_mut().next()? {
            Some(batch) => {
                // evaluate the filter expression against the batch
                match self.expr.get_func()(&batch)?
                    .as_any()
                    .downcast_ref::<BooleanArray>()
                {
                    Some(filter_bools) => {
                        let filtered_columns: Result<Vec<ArrayRef>> = (0..batch.num_columns())
                            .map(|i| filter(batch.column(i), &filter_bools))
                            .collect();

                        let filtered_batch: RecordBatch =
                            RecordBatch::new(Arc::new(Schema::empty()), filtered_columns?);

                        Ok(Some(filtered_batch))
                    }
                    _ => Err(ExecutionError::ExecutionError(
                        "Filter expression did not evaluate to boolean".to_string(),
                    )),
                }
            }
            None => Ok(None),
        }
    }

    fn schema(&self) -> &Arc<Schema> {
        &self.schema
    }
}

//TODO: move into Arrow array_ops
fn filter(array: &Arc<Array>, filter: &BooleanArray) -> Result<ArrayRef> {
    let a = array.as_ref();

    match a.data_type() {
        DataType::Float64 => {
            let b = a.as_any().downcast_ref::<Float64Array>().unwrap();
            let mut builder = Float64Array::builder(b.len());
            for i in 0..b.len() {
                if filter.value(i) {
                    builder.append_value(b.value(i))?;
                }
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::Utf8 => {
            //TODO: this is inefficient and we should improve the Arrow impl to help make this more concise
            let b = a.as_any().downcast_ref::<BinaryArray>().unwrap();
            let mut values: Vec<String> = Vec::with_capacity(b.len());
            for i in 0..b.len() {
                if filter.value(i) {
                    values.push(b.get_string(i));
                }
            }
            let tmp: Vec<&str> = values.iter().map(|s| s.as_str()).collect();
            Ok(Arc::new(BinaryArray::from(tmp)))
        }
        other => Err(ExecutionError::ExecutionError(format!(
            "filter not supported for {:?}",
            other
        ))),
    }
}
