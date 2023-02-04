// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use async_recursion::async_recursion;
use datafusion::arrow::datatypes::{Field, Schema};
use datafusion::common::TableReference;
use datafusion::datasource::object_store::ObjectStoreUrl;
use datafusion::datasource::DefaultTableSource;
use datafusion::error::{DataFusionError, Result};
use datafusion::logical_expr::LogicalPlan;
use datafusion::physical_plan::file_format::{FileScanConfig, ParquetExec};
use datafusion::physical_plan::ExecutionPlan;
use datafusion::prelude::SessionContext;
use std::collections::HashMap;
use std::sync::Arc;
use substrait::proto::{
    expression::{
        reference_segment::ReferenceType::StructField, MaskExpression, RexType,
    },
    read_rel::ReadType,
    rel::RelType,
    Expression, Plan, Rel, Type,
};

/// Convert Substrait Rel to DataFusion LogicalPlan
#[async_recursion]
pub async fn from_substrait_rel(
    _ctx: &mut SessionContext,
    rel: &Rel,
    _extensions: &HashMap<u32, &String>,
) -> Result<Arc<dyn ExecutionPlan>> {
    match &rel.rel_type {
        Some(RelType::Read(read)) => match &read.as_ref().read_type {
            Some(ReadType::NamedTable(nt)) => {
                let table_reference = match nt.names.len() {
                    0 => {
                        return Err(DataFusionError::Internal(
                            "No table name found in NamedTable".to_string(),
                        ));
                    }
                    1 => TableReference::Bare {
                        table: &nt.names[0],
                    },
                    2 => TableReference::Partial {
                        schema: &nt.names[0],
                        table: &nt.names[1],
                    },
                    _ => TableReference::Full {
                        catalog: &nt.names[0],
                        schema: &nt.names[1],
                        table: &nt.names[2],
                    },
                };
                let object_store_url = ObjectStoreUrl::parse(&table_reference.table())?;
                let mut base_config = FileScanConfig {
                    object_store_url,
                    file_schema: Arc::new(Schema::empty()),
                    file_groups: vec![],
                    statistics: Default::default(),
                    projection: None,
                    limit: None,
                    table_partition_cols: vec![],
                    output_ordering: None,
                    infinite_source: false,
                };
                match &read.projection {
                    Some(MaskExpression { select, .. }) => match &select.as_ref() {
                        Some(projection) => {
                            let column_indices: Vec<usize> = projection
                                .struct_items
                                .iter()
                                .map(|item| item.field as usize)
                                .collect();
                            base_config.projection = Some(column_indices);
                        }
                        _ => {}
                    },
                    _ => {}
                }
                Ok(Arc::new(ParquetExec::new(base_config, None, None))
                    as Arc<dyn ExecutionPlan>)
            }
            _ => Err(DataFusionError::NotImplemented(
                "Only NamedTable reads are supported".to_string(),
            )),
        },
        _ => Err(DataFusionError::NotImplemented(format!(
            "Unsupported RelType: {:?}",
            rel.rel_type
        ))),
    }
}
