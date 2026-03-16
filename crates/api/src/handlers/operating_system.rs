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

use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::api::Api;
use crate::api::rpc;

fn parameters_to_json(params: &[rpc::IpxeOsParameter]) -> serde_json::Value {
    serde_json::Value::Array(
        params
            .iter()
            .map(|p| {
                serde_json::json!({
                    "name": p.name,
                    "value": p.value,
                })
            })
            .collect(),
    )
}

fn artifacts_to_json(artifacts: &[rpc::IpxeOsArtifact]) -> serde_json::Value {
    serde_json::Value::Array(
        artifacts
            .iter()
            .map(|a| {
                serde_json::json!({
                    "name": a.name,
                    "url": a.url,
                    "sha": a.sha,
                    "auth_type": a.auth_type,
                    "auth_token": a.auth_token,
                    "cache_strategy": a.cache_strategy,
                    "local_url": a.local_url,
                })
            })
            .collect(),
    )
}

pub async fn create_operating_system(
    api: &Api,
    request: Request<rpc::CreateOperatingSystemRequest>,
) -> Result<Response<rpc::OperatingSystemDefinition>, Status> {
    let mut txn = api.txn_begin().await?;
    let req = request.into_inner();

    let (type_, ipxe_script, os_image_id, ipxe_template_name, ipxe_parameters, ipxe_artifacts) =
        if let Some(ref script) = req.ipxe_script {
            (
                "ipxe".to_string(),
                Some(script.clone()),
                None,
                None,
                None,
                None,
            )
        } else if let Some(ref img_id) = req.os_image_id {
            let id = Uuid::try_from(img_id.clone())
                .map_err(|e| Status::invalid_argument(format!("invalid os_image_id: {e}")))?;
            ("image".to_string(), None, Some(id), None, None, None)
        } else if let Some(ref tmpl) = req.ipxe_template_name {
            let params = if req.ipxe_parameters.is_empty() {
                None
            } else {
                Some(parameters_to_json(&req.ipxe_parameters))
            };
            let arts = if req.ipxe_artifacts.is_empty() {
                None
            } else {
                Some(artifacts_to_json(&req.ipxe_artifacts))
            };
            (
                "ipxe_os_definition".to_string(),
                None,
                None,
                Some(tmpl.clone()),
                params,
                arts,
            )
        } else {
            return Err(Status::invalid_argument(
                "exactly one OS variant must be specified: ipxe_script, os_image_id, or ipxe_template_name",
            ));
        };

    if req.name.is_empty() {
        return Err(Status::invalid_argument("name is required"));
    }
    if req.org.is_empty() {
        return Err(Status::invalid_argument("org is required"));
    }

    let id = req
        .id
        .as_ref()
        .map(|u| Uuid::try_from(u.clone()))
        .transpose()
        .map_err(|e| Status::invalid_argument(format!("invalid id: {e}")))?;

    let input = db::operating_system::CreateOperatingSystem {
        id,
        name: req.name,
        description: req.description,
        org: req.org,
        type_,
        is_active: req.is_active,
        allow_override: req.allow_override,
        phone_home_enabled: req.phone_home_enabled,
        user_data: req.user_data,
        ipxe_script,
        os_image_id,
        ipxe_template_name,
        ipxe_parameters,
        ipxe_artifacts,
        ipxe_definition_hash: None,
    };

    let row = db::operating_system::create(&mut txn, &input)
        .await
        .map_err(|e| Status::internal(e.to_string()))?;
    txn.commit()
        .await
        .map_err(|e| Status::internal(e.to_string()))?;

    let def: rpc::OperatingSystemDefinition =
        model::operating_system_definition::OperatingSystemDefinition::from(&row).into();
    Ok(Response::new(def))
}

pub async fn get_operating_system(
    api: &Api,
    request: Request<::rpc::Uuid>,
) -> Result<Response<rpc::OperatingSystemDefinition>, Status> {
    let mut txn = api.txn_begin().await?;
    let id = Uuid::try_from(request.into_inner())
        .map_err(|e| Status::invalid_argument(e.to_string()))?;

    let row = db::operating_system::get(&mut txn, id).await.map_err(|e| {
        if e.is_not_found() {
            Status::not_found(format!("operating system {id} not found"))
        } else {
            Status::internal(e.to_string())
        }
    })?;
    txn.commit()
        .await
        .map_err(|e| Status::internal(e.to_string()))?;

    let def: rpc::OperatingSystemDefinition =
        model::operating_system_definition::OperatingSystemDefinition::from(&row).into();
    Ok(Response::new(def))
}

pub async fn update_operating_system(
    api: &Api,
    request: Request<rpc::UpdateOperatingSystemRequest>,
) -> Result<Response<rpc::OperatingSystemDefinition>, Status> {
    let mut txn = api.txn_begin().await?;
    let req = request.into_inner();

    let id_proto = req
        .id
        .ok_or_else(|| Status::invalid_argument("id is required"))?;
    let id = Uuid::try_from(id_proto)
        .map_err(|e| Status::invalid_argument(format!("invalid id: {e}")))?;

    let existing = db::operating_system::get(&mut txn, id).await.map_err(|e| {
        if e.is_not_found() {
            Status::not_found(format!("operating system {id} not found"))
        } else {
            Status::internal(e.to_string())
        }
    })?;

    let ipxe_parameters = if req.ipxe_parameters.is_empty() {
        None
    } else {
        Some(parameters_to_json(&req.ipxe_parameters))
    };
    let ipxe_artifacts = if req.ipxe_artifacts.is_empty() {
        None
    } else {
        Some(artifacts_to_json(&req.ipxe_artifacts))
    };

    let input = db::operating_system::UpdateOperatingSystem {
        id,
        name: req.name,
        description: req.description,
        is_active: req.is_active,
        allow_override: req.allow_override,
        phone_home_enabled: req.phone_home_enabled,
        user_data: req.user_data,
        ipxe_script: req.ipxe_script,
        ipxe_template_name: req.ipxe_template_name,
        ipxe_parameters,
        ipxe_artifacts,
    };

    let row = db::operating_system::update(&mut txn, &existing, &input)
        .await
        .map_err(|e| Status::internal(e.to_string()))?;
    txn.commit()
        .await
        .map_err(|e| Status::internal(e.to_string()))?;

    let def: rpc::OperatingSystemDefinition =
        model::operating_system_definition::OperatingSystemDefinition::from(&row).into();
    Ok(Response::new(def))
}

pub async fn delete_operating_system(
    api: &Api,
    request: Request<rpc::DeleteOperatingSystemRequest>,
) -> Result<Response<rpc::DeleteOperatingSystemResponse>, Status> {
    let mut txn = api.txn_begin().await?;
    let req = request.into_inner();

    let id_proto = req
        .id
        .ok_or_else(|| Status::invalid_argument("id is required"))?;
    let id = Uuid::try_from(id_proto)
        .map_err(|e| Status::invalid_argument(format!("invalid id: {e}")))?;

    db::operating_system::delete(&mut txn, id)
        .await
        .map_err(|e| {
            if e.is_not_found() {
                Status::not_found(format!("operating system {id} not found"))
            } else {
                Status::internal(e.to_string())
            }
        })?;
    txn.commit()
        .await
        .map_err(|e| Status::internal(e.to_string()))?;

    Ok(Response::new(rpc::DeleteOperatingSystemResponse {}))
}

pub async fn find_operating_system_ids(
    api: &Api,
    request: Request<rpc::OperatingSystemSearchFilter>,
) -> Result<Response<rpc::OperatingSystemIdList>, Status> {
    let mut txn = api.txn_begin().await?;
    let filter = request.into_inner();

    let ids = db::operating_system::list_ids(&mut txn, filter.org.as_deref())
        .await
        .map_err(|e| Status::internal(e.to_string()))?;
    txn.commit()
        .await
        .map_err(|e| Status::internal(e.to_string()))?;

    let ids = ids
        .into_iter()
        .map(|u| ::rpc::common::Uuid {
            value: u.to_string(),
        })
        .collect();

    Ok(Response::new(rpc::OperatingSystemIdList { ids }))
}

pub async fn find_operating_systems_by_ids(
    api: &Api,
    request: Request<rpc::OperatingSystemsByIdsRequest>,
) -> Result<Response<rpc::OperatingSystemList>, Status> {
    let mut txn = api.txn_begin().await?;
    let req = request.into_inner();

    let ids: Vec<Uuid> = req
        .ids
        .iter()
        .filter_map(|u| Uuid::parse_str(&u.value).ok())
        .collect();

    let rows = db::operating_system::get_many(&mut txn, &ids)
        .await
        .map_err(|e| Status::internal(e.to_string()))?;
    txn.commit()
        .await
        .map_err(|e| Status::internal(e.to_string()))?;

    let operating_systems: Vec<rpc::OperatingSystemDefinition> = rows
        .iter()
        .map(|row| model::operating_system_definition::OperatingSystemDefinition::from(row).into())
        .collect();

    Ok(Response::new(rpc::OperatingSystemList {
        operating_systems,
    }))
}
