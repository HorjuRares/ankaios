// Copyright (c) 2023 Elektrobit Automotive GmbH
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

// [impl->swdd~common-object-representation~1]

// [impl->swdd~common-conversions-between-ankaios-and-proto~1]

mod state;
pub use state::State;

mod external_state;
pub use external_state::ExternalState;

mod workload_state;
#[cfg(any(feature = "test_utils", test))]
pub use workload_state::{
    generate_test_workload_state, generate_test_workload_state_with_agent,
    generate_test_workload_state_with_workload_spec,
};
pub use workload_state::{ExecutionState, ExecutionStateEnum, WorkloadState};

mod workload_spec;
pub use workload_spec::{
    get_workloads_per_agent, AddCondition, DeleteCondition, DeletedWorkload,
    DeletedWorkloadCollection, FulfilledBy, UpdateStrategy, WorkloadCollection, WorkloadSpec,
};

mod cronjob;
pub use cronjob::{Cronjob, Interval};

mod tag;
pub use tag::Tag;

mod access_rights;
pub use access_rights::{AccessRights, AccessRightsRule, PatchOperation};

mod workload_instance_name;
pub use workload_instance_name::{ConfigHash, WorkloadInstanceName, WorkloadInstanceNameBuilder};

mod agent_name;
pub use agent_name::AgentName;
