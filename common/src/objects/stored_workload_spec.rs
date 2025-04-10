// Copyright (c) 2024 Elektrobit Automotive GmbH
//
// This program and the accompanying materials are made available under the
// terms of the Apache License, Version 2.0 which is available at
// https://www.apache.org/licenses/LICENSE-2.0.
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
// License for the specific language governing permissions and limitations
// under the License.
//
// SPDX-License-Identifier: Apache-2.0

use regex::Regex;
use std::collections::HashMap;

use api::ank_base;
use serde::{Deserialize, Serialize};

use crate::helpers::serialize_to_ordered_map;

use super::{
    control_interface_access::ControlInterfaceAccess, file::File, AddCondition, RestartPolicy, Tag,
    WorkloadInstanceName, WorkloadSpec,
};

pub const STR_RE_CONFIG_REFERENCES: &str = r"^[a-zA-Z0-9_-]*$";

#[derive(Debug, Serialize, Default, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoredWorkloadSpec {
    pub agent: String,
    #[serde(default)]
    pub tags: Vec<Tag>,
    #[serde(default, serialize_with = "serialize_to_ordered_map")]
    pub dependencies: HashMap<String, AddCondition>,
    #[serde(default)]
    pub restart_policy: RestartPolicy,
    pub runtime: String,
    pub runtime_config: String,
    #[serde(default)]
    pub control_interface_access: ControlInterfaceAccess,
    #[serde(default, serialize_with = "serialize_to_ordered_map")]
    pub configs: HashMap<String, String>,
    #[serde(default)]
    pub files: Vec<File>,
}

impl StoredWorkloadSpec {
    // [impl->swdd~common-config-aliases-and-config-reference-keys-naming-convention~1]
    pub fn verify_config_reference_format(
        config_references: &HashMap<String, String>,
    ) -> Result<(), String> {
        let re_config_references = Regex::new(STR_RE_CONFIG_REFERENCES).unwrap();
        for (config_alias, referenced_config) in config_references {
            if !re_config_references.is_match(config_alias) {
                return Err(format!(
                    "Unsupported config alias. Received '{}', expected to have characters in {}",
                    config_alias, STR_RE_CONFIG_REFERENCES
                ));
            }

            if !re_config_references.is_match(referenced_config) {
                return Err(format!(
                    "Unsupported config reference key. Received '{}', expected to have characters in {}",
                    referenced_config, STR_RE_CONFIG_REFERENCES
                ));
            }
        }
        Ok(())
    }
}

impl TryFrom<ank_base::Workload> for StoredWorkloadSpec {
    type Error = String;

    fn try_from(value: ank_base::Workload) -> Result<Self, String> {
        Ok(StoredWorkloadSpec {
            agent: value.agent.ok_or("Missing field agent")?,
            tags: value
                .tags
                .unwrap_or_default()
                .tags
                .into_iter()
                .map(|x| x.into())
                .collect(),
            dependencies: value
                .dependencies
                .unwrap_or_default()
                .dependencies
                .into_iter()
                .map(|(k, v)| Ok((k, v.try_into()?)))
                .collect::<Result<HashMap<String, AddCondition>, String>>()?,
            restart_policy: value.restart_policy.unwrap_or_default().try_into()?,
            runtime: value.runtime.ok_or("Missing field runtime")?,
            runtime_config: value.runtime_config.ok_or("Missing field runtimeConfig")?,
            control_interface_access: value
                .control_interface_access
                .unwrap_or_default()
                .try_into()?,
            configs: value.configs.unwrap_or_default().configs,
            files: value
                .files
                .unwrap_or_default()
                .files
                .into_iter()
                .map(|file| file.try_into())
                .collect::<Result<Vec<File>, String>>()?,
        })
    }
}

impl From<StoredWorkloadSpec> for ank_base::Workload {
    fn from(workload: StoredWorkloadSpec) -> Self {
        ank_base::Workload {
            agent: workload.agent.into(),
            dependencies: Some(ank_base::Dependencies {
                dependencies: workload
                    .dependencies
                    .into_iter()
                    .map(|(k, v)| (k, v as i32))
                    .collect(),
            }),
            restart_policy: (workload.restart_policy as i32).into(),
            runtime: workload.runtime.into(),
            runtime_config: workload.runtime_config.into(),
            tags: Some(ank_base::Tags {
                tags: workload.tags.into_iter().map(Into::into).collect(),
            }),
            control_interface_access: workload.control_interface_access.into(),
            configs: Some(ank_base::ConfigMappings {
                configs: workload.configs,
            }),
            files: Some(ank_base::Files {
                files: workload.files.into_iter().map(Into::into).collect(),
            }),
        }
    }
}

impl From<(String, StoredWorkloadSpec)> for WorkloadSpec {
    fn from((name, spec): (String, StoredWorkloadSpec)) -> Self {
        WorkloadSpec {
            instance_name: WorkloadInstanceName::builder()
                .workload_name(name)
                .agent_name(spec.agent)
                .config(&spec.runtime_config)
                .build(),
            tags: spec.tags,
            dependencies: spec.dependencies,
            restart_policy: spec.restart_policy,
            runtime: spec.runtime,
            runtime_config: spec.runtime_config,
            files: spec.files,
            control_interface_access: spec.control_interface_access,
        }
    }
}

impl From<WorkloadSpec> for StoredWorkloadSpec {
    fn from(value: WorkloadSpec) -> Self {
        StoredWorkloadSpec {
            runtime: value.runtime,
            agent: value.instance_name.agent_name().to_owned(),
            restart_policy: value.restart_policy,
            dependencies: value.dependencies,
            tags: value.tags,
            runtime_config: value.runtime_config,
            control_interface_access: value.control_interface_access,
            configs: Default::default(),
            files: value.files,
        }
    }
}

//////////////////////////////////////////////////////////////////////////////
//                 ########  #######    #########  #########                //
//                    ##     ##        ##             ##                    //
//                    ##     #####     #########      ##                    //
//                    ##     ##                ##     ##                    //
//                    ##     #######   #########      ##                    //
//////////////////////////////////////////////////////////////////////////////

#[cfg(any(feature = "test_utils", test))]
pub fn generate_test_stored_workload_spec_with_config(
    agent: impl Into<String>,
    runtime_name: impl Into<String>,
    runtime_config: impl Into<String>,
) -> crate::objects::StoredWorkloadSpec {
    StoredWorkloadSpec {
        agent: agent.into(),
        dependencies: HashMap::from([
            (String::from("workload_A"), AddCondition::AddCondRunning),
            (String::from("workload_C"), AddCondition::AddCondSucceeded),
        ]),
        restart_policy: RestartPolicy::Always,
        runtime: runtime_name.into(),
        tags: vec![Tag {
            key: "key".into(),
            value: "value".into(),
        }],
        runtime_config: runtime_config.into(),
        control_interface_access: Default::default(),
        configs: [
            ("ref1".into(), "config_1".into()),
            ("ref2".into(), "config_2".into()),
        ]
        .into(),
        files: vec![],
    }
}

#[cfg(any(feature = "test_utils", test))]
pub fn generate_test_stored_workload_spec_with_files(
    agent: impl Into<String>,
    runtime_name: impl Into<String>,
    files: Vec<File>,
) -> crate::objects::StoredWorkloadSpec {
    let mut stored_spec = generate_test_stored_workload_spec(agent, runtime_name);
    stored_spec.files = files;
    stored_spec
}

#[cfg(any(feature = "test_utils", test))]
pub fn generate_test_stored_workload_spec(
    agent: impl Into<String>,
    runtime_name: impl Into<String>,
) -> crate::objects::StoredWorkloadSpec {
    generate_test_stored_workload_spec_with_config(
        agent,
        runtime_name,
        "generalOptions: [\"--version\"]\ncommandOptions: [\"--network=host\"]\nimage: alpine:latest\ncommandArgs: [\"bash\"]\n"
        .to_owned()
    )
}

// [utest->swdd~common-object-serialization~1]
#[cfg(test)]
mod tests {
    use super::StoredWorkloadSpec;
    use std::collections::HashMap;

    // one test for a failing case, other cases are tested on the caller side to not repeat test code
    // [utest->swdd~common-config-aliases-and-config-reference-keys-naming-convention~1]
    #[test]
    fn utest_verify_config_reference_format_invalid_config_reference_key() {
        let invalid_config_reference_key = "invalid%key";
        let mut configs = HashMap::new();
        configs.insert(
            "config_alias_1".to_owned(),
            invalid_config_reference_key.to_owned(),
        );
        assert_eq!(
            StoredWorkloadSpec::verify_config_reference_format(&configs),
            Err(format!(
                "Unsupported config reference key. Received '{}', expected to have characters in {}",
                invalid_config_reference_key,
                super::STR_RE_CONFIG_REFERENCES
            ))
        );
    }
}
