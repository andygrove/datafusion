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
    extension_info: &mut (
        Vec<extensions::SimpleExtensionDeclaration>,
        HashMap<String, u32>,
    ),
) -> Result<Box<Rel>> {
    if let Some(scan) = plan.as_any().downcast_ref::<ParquetExec>() {
        /*
                /// Override for `Self::with_pushdown_filters`. If None, uses
        /// values from base_config
        pushdown_filters: Option<bool>,
        /// Override for `Self::with_reorder_filters`. If None, uses
        /// values from base_config
        reorder_filters: Option<bool>,
        /// Override for `Self::with_enable_page_index`. If None, uses
        /// values from base_config
        enable_page_index: Option<bool>,
        /// Base configuraton for this scan
        base_config: FileScanConfig,
        projected_statistics: Statistics,
        projected_schema: SchemaRef,
        /// Execution metrics
        metrics: ExecutionPlanMetricsSet,
        /// Optional predicate for row filtering during parquet scan
        predicate: Option<Arc<Expr>>,
        /// Optional predicate for pruning row groups
        pruning_predicate: Option<Arc<PruningPredicate>>,
        /// Optional predicate for pruning pages
        page_pruning_predicate: Option<Arc<PagePruningPredicate>>,
        /// Optional hint for the size of the parquet metadata
        metadata_size_hint: Option<usize>,
        /// Optional user defined parquet file reader factory
        parquet_file_reader_factory: Option<Arc<dyn ParquetFileReaderFactory>>,




            /// Object store URL, used to get an [`ObjectStore`] instance from
        /// [`RuntimeEnv::object_store`]
        pub object_store_url: ObjectStoreUrl,
        /// Schema before `projection` is applied. It contains the all columns that may
        /// appear in the files. It does not include table partition columns
        /// that may be added.
        pub file_schema: SchemaRef,
        /// List of files to be processed, grouped into partitions
        ///
        /// Each file must have a schema of `file_schema` or a subset. If
        /// a particular file has a subset, the missing columns are
        /// padded with with NULLs.
        ///
        /// DataFusion may attempt to read each partition of files
        /// concurrently, however files *within* a partition will be read
        /// sequentially, one after the next.
        pub file_groups: Vec<Vec<PartitionedFile>>,
        /// Estimated overall statistics of the files, taking `filters` into account.
        pub statistics: Statistics,
        /// Columns on which to project the data. Indexes that are higher than the
        /// number of columns of `file_schema` refer to `table_partition_cols`.
        pub projection: Option<Vec<usize>>,
        /// The maximum number of records to read from this plan. If None,
        /// all records after filtering are returned.
        pub limit: Option<usize>,
        /// The partitioning columns
        pub table_partition_cols: Vec<(String, DataType)>,
        /// The order in which the data is sorted, if known.
        pub output_ordering: Option<Vec<PhysicalSortExpr>>,
        /// Indicates whether this plan may produce an infinite stream of records.
        pub infinite_source: bool,
             */

        let base_config = scan.base_config();

        let projection = base_config.projection.as_ref().map(|p| {
            p.iter()
                .map(|i| StructItem {
                    field: *i as i32,
                    child: None,
                })
                .collect()
        });

        if let Some(struct_items) = projection {
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
                        select: Some(StructSelect { struct_items }),
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
                "TableScan without projection is not supported".to_string(),
            ))
        }
    } else {
        Err(DataFusionError::NotImplemented(
            "Plan is not supported".to_string(),
        ))
    }
}
