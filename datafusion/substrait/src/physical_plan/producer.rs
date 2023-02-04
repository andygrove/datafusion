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

use datafusion::error::{DataFusionError, Result};
use datafusion::physical_plan::file_format::ParquetExec;
use datafusion::physical_plan::ExecutionPlan;
use std::collections::HashMap;
use substrait::proto::expression::mask_expression::StructItem;
use substrait::proto::expression::mask_expression::StructSelect;
use substrait::proto::expression::MaskExpression;
use substrait::proto::extensions;
use substrait::proto::read_rel::NamedTable;
use substrait::proto::read_rel::ReadType;
use substrait::proto::rel::RelType;
use substrait::proto::NamedStruct;
use substrait::proto::ReadRel;
use substrait::proto::Rel;

/// Convert DataFusion ExecutionPlan to Substrait Rel
pub fn to_substrait_rel(
    plan: &dyn ExecutionPlan,
    _extension_info: &mut (
        Vec<extensions::SimpleExtensionDeclaration>,
        HashMap<String, u32>,
    ),
) -> Result<Box<Rel>> {
    if let Some(scan) = plan.as_any().downcast_ref::<ParquetExec>() {
        let base_config = scan.base_config();
        //
        // let projection = base_config.projection.as_ref().map(|p| {
        //     p.iter()
        //         .map(|i| StructItem {
        //             field: *i as i32,
        //             child: None,
        //         })
        //         .collect()
        // });

        Ok(Box::new(Rel {
            rel_type: Some(RelType::Read(Box::new(ReadRel {
                common: None,
                base_schema: Some(NamedStruct {
                    names: scan
                        .projected_schema
                        .fields()
                        .iter()
                        .map(|f| f.name().to_owned())
                        .collect(),
                    r#struct: None,
                }),
                filter: None,
                best_effort_filter: None,
                projection: Some(MaskExpression {
                    select: None, //Some(StructSelect { struct_items }),
                    maintain_singular_struct: false,
                }),
                advanced_extension: None,
                read_type: Some(ReadType::NamedTable(NamedTable {
                    names: vec![base_config.object_store_url.as_str().to_string()],
                    advanced_extension: None,
                })),
            }))),
        }))
    } else {
        Err(DataFusionError::NotImplemented(
            "Plan is not supported".to_string(),
        ))
    }
}
