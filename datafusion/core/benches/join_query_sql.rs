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

use arrow::{
    datatypes::{DataType, Field, Schema},
    record_batch::RecordBatch,
};
use arrow_array::{Int32Array, Int64Array};
use criterion::{criterion_group, criterion_main, Criterion};
use datafusion::prelude::SessionContext;
use datafusion::{datasource::MemTable, error::Result};
use futures::executor::block_on;
use std::sync::Arc;
use tokio::runtime::Runtime;

async fn query(ctx: &SessionContext, sql: &str) {
    let rt = Runtime::new().unwrap();

    // execute the query
    let df = rt.block_on(ctx.sql(sql)).unwrap();
    criterion::black_box(rt.block_on(df.collect()).unwrap());
}

fn create_context(array_len: usize, batch_size: usize) -> Result<SessionContext> {
    // define a schema.
    let schema = Arc::new(Schema::new(vec![
        Field::new("i32", DataType::Int32, false),
        Field::new("i64", DataType::Int64, false),
    ]));

    // define data.
    let batches = (0..array_len / batch_size)
        .map(|i| {
            RecordBatch::try_new(
                schema.clone(),
                vec![
                    Arc::new(Int32Array::from(vec![i as i32; batch_size])),
                    Arc::new(Int64Array::from(vec![i as i64; batch_size])),
                ],
            )
            .unwrap()
        })
        .collect::<Vec<_>>();

    let ctx = SessionContext::new();

    // declare a table in memory. In spark API, this corresponds to createDataFrame(...).
    let provider1 = MemTable::try_new(schema.clone(), vec![batches.clone()])?;
    let provider2 = MemTable::try_new(schema, vec![batches])?;
    ctx.register_table("t1", Arc::new(provider1))?;
    ctx.register_table("t2", Arc::new(provider2))?;

    Ok(ctx)
}

fn criterion_benchmark(c: &mut Criterion) {
    let array_len = 2048;
    let batch_size = 512;

    c.bench_function("join_inner", |b| {
        let ctx = create_context(array_len, batch_size).unwrap();
        b.iter(|| {
            block_on(query(
                &ctx,
                "select t1.*, t2.* from t1 join t2 on t1.i32 = t2.i32",
            ))
        })
    });
    c.bench_function("join_left_outer", |b| {
        let ctx = create_context(array_len, batch_size).unwrap();
        b.iter(|| {
            block_on(query(
                &ctx,
                "select t1.*, t2.* from t1 left outer join t2 on t1.i32 = t2.i32",
            ))
        })
    });
    c.bench_function("join_right_outer", |b| {
        let ctx = create_context(array_len, batch_size).unwrap();
        b.iter(|| {
            block_on(query(
                &ctx,
                "select t1.*, t2.* from t1 right outer join t2 on t1.i32 = t2.i32",
            ))
        })
    });
    c.bench_function("join_full_outer", |b| {
        let ctx = create_context(array_len, batch_size).unwrap();
        b.iter(|| {
            block_on(query(
                &ctx,
                "select t1.*, t2.* from t1 full outer join t2 on t1.i32 = t2.i32",
            ))
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
