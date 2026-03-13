/*
 * SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use ::rpc::admin_cli::{CarbideCliError, CarbideCliResult, OutputFormat};
use ::rpc::forge::ListOperatingSystemsRequest;
use prettytable::{Cell, Row, Table};

use super::args::Args;
use crate::operating_system::common::SerializableOs;
use crate::rpc::ApiClient;

pub async fn list(
    opts: Args,
    format: OutputFormat,
    api_client: &ApiClient,
) -> CarbideCliResult<()> {
    let result = api_client
        .0
        .list_operating_systems(ListOperatingSystemsRequest { org: opts.org })
        .await?;

    let operating_systems = result.operating_systems;

    if format == OutputFormat::Json {
        let serializable: Vec<SerializableOs> =
            operating_systems.into_iter().map(SerializableOs::from).collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serializable).map_err(CarbideCliError::JsonError)?
        );
        return Ok(());
    }

    if operating_systems.is_empty() {
        println!("No operating system definitions found.");
        return Ok(());
    }

    let mut table = Table::new();
    table.set_titles(Row::new(vec![
        Cell::new("ID"),
        Cell::new("Name"),
        Cell::new("Org"),
        Cell::new("Type"),
        Cell::new("Status"),
        Cell::new("Active"),
    ]));

    for os in &operating_systems {
        table.add_row(Row::new(vec![
            Cell::new(&os.id),
            Cell::new(&os.name),
            Cell::new(&os.org),
            Cell::new(&os.r#type),
            Cell::new(&os.status),
            Cell::new(if os.is_active { "yes" } else { "no" }),
        ]));
    }

    table.printstd();
    Ok(())
}
