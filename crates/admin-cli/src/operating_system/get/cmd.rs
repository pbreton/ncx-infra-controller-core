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

use super::args::Args;
use crate::operating_system::common::{str_to_rpc_uuid, SerializableOs};
use crate::rpc::ApiClient;

pub async fn get(
    opts: Args,
    format: OutputFormat,
    api_client: &ApiClient,
) -> CarbideCliResult<()> {
    let id = str_to_rpc_uuid(&opts.id)?;

    let os = match api_client.0.get_operating_system(id).await {
        Ok(os) => os,
        Err(status) if status.code() == tonic::Code::NotFound => {
            return Err(CarbideCliError::GenericError(format!(
                "Operating system not found: {}",
                opts.id
            )));
        }
        Err(err) => return Err(CarbideCliError::from(err)),
    };

    if format == OutputFormat::Json {
        let serializable: SerializableOs = os.into();
        println!(
            "{}",
            serde_json::to_string_pretty(&serializable).map_err(CarbideCliError::JsonError)?
        );
        return Ok(());
    }

    println!("ID:                  {}", os.id);
    println!("Name:                {}", os.name);
    println!("Org:                 {}", os.org);
    println!("Type:                {}", os.r#type);
    println!("Status:              {}", os.status);
    println!("Active:              {}", os.is_active);
    println!("Allow Override:      {}", os.allow_override);
    println!("Phone Home Enabled:  {}", os.phone_home_enabled);
    println!("Created:             {}", os.created);
    println!("Updated:             {}", os.updated);

    if let Some(desc) = &os.description {
        println!("Description:         {desc}");
    }
    if let Some(user_data) = &os.user_data {
        println!("User Data:           {user_data}");
    }

    if let Some(script) = &os.ipxe_script {
        println!("\niPXE Script:\n---\n{script}\n---");
    }
    if let Some(tmpl) = &os.ipxe_template_name {
        println!("iPXE Template:       {tmpl}");
    }
    if let Some(img_id) = &os.os_image_id {
        println!("OS Image ID:         {img_id}");
    }

    if !os.ipxe_parameters.is_empty() {
        println!("\niPXE Parameters:");
        for p in &os.ipxe_parameters {
            println!("  {}={}", p.name, p.value);
        }
    }
    if !os.ipxe_artifacts.is_empty() {
        println!("\niPXE Artifacts:");
        for a in &os.ipxe_artifacts {
            println!("  {} -> {}", a.name, a.url);
        }
    }

    Ok(())
}
