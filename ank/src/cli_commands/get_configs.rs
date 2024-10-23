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
use super::CliCommands;
use crate::cli_commands::config_table_row::ConfigTableRow;
use crate::filtered_complete_state::FilteredCompleteState;
use crate::{cli_commands::cli_table::CliTable, cli_error::CliError, output_debug};
use common::objects::ConfigItem;
const EMPTY_FILTER_MASK: [String; 0] = [];

impl CliCommands {
    pub async fn get_configs(&mut self) -> Result<String, CliError> {
        let filtered_complete_state: FilteredCompleteState = self
            .server_connection
            .get_complete_state(&EMPTY_FILTER_MASK)
            .await?;

        let configs = filtered_complete_state
            .desired_state
            .and_then(|state| state.configs)
            .unwrap_or_default()
            .into_iter();

        let config_table_rows = transform_into_table_rows(configs);

        output_debug!("Got configs: {:?}", config_table_rows);

        Ok(CliTable::new(&config_table_rows).create_default_table())
    }
}

fn transform_into_table_rows(
    configs: impl Iterator<Item = (String, ConfigItem)>,
) -> Vec<ConfigTableRow> {
    let mut config_table_rows: Vec<ConfigTableRow> = configs
        .map(|(config_str, _config_item)| ConfigTableRow { config: config_str })
        .collect();

    // sort in order to ensure consistent output
    config_table_rows.sort_by(|a, b| a.config.cmp(&b.config));
    config_table_rows
}

//////////////////////////////////////////////////////////////////////////////
//                 ########  #######    #########  #########                //
//                    ##     ##        ##             ##                    //
//                    ##     #####     #########      ##                    //
//                    ##     ##                ##     ##                    //
//                    ##     #######   #########      ##                    //
//////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use crate::cli_commands::{
        server_connection::{MockServerConnection, ServerConnectionError},
        CliCommands,
    };

    use api::ank_base;
    use common::test_utils;
    use mockall::predicate::eq;

    const RESPONSE_TIMEOUT_MS: u64 = 3000;
    const CONFIG_1: &str = "config_1";
    const CONFIG_2: &str = "config_2";

    #[tokio::test]
    async fn test_get_configs() {
        let mut mock_server_connection = MockServerConnection::default();
        mock_server_connection
            .expect_get_complete_state()
            .with(eq(vec![]))
            .return_once(|_| {
                Ok(ank_base::CompleteState::from(
                    test_utils::generate_test_complete_state_with_configs(vec![
                        CONFIG_1.to_string(),
                        CONFIG_2.to_string(),
                    ]),
                )
                .into())
            });

        let mut cmd = CliCommands {
            _response_timeout_ms: RESPONSE_TIMEOUT_MS,
            no_wait: false,
            server_connection: mock_server_connection,
        };

        let table_output_result = cmd.get_configs().await;

        let expected_table_output = ["CONFIG  ", "config_1", "config_2"].join("\n");

        assert_eq!(Ok(expected_table_output), table_output_result);
    }

    #[tokio::test]
    async fn test_get_configs_no_config_present_in_complete_state() {
        let mut mock_server_connection = MockServerConnection::default();
        mock_server_connection
            .expect_get_complete_state()
            .with(eq(vec![]))
            .return_once(|_| Ok(ank_base::CompleteState::default().into()));

        let mut cmd = CliCommands {
            _response_timeout_ms: RESPONSE_TIMEOUT_MS,
            no_wait: false,
            server_connection: mock_server_connection,
        };

        let table_output_result = cmd.get_configs().await;

        let expected_table_output = "CONFIG".to_string();

        assert_eq!(Ok(expected_table_output), table_output_result);
    }

    #[tokio::test]
    async fn test_get_configs_failed_to_get_complete_state() {
        let mut mock_server_connection = MockServerConnection::default();
        mock_server_connection
            .expect_get_complete_state()
            .with(eq(vec![]))
            .return_once(|_| {
                Err(ServerConnectionError::ExecutionError(
                    "connection error".to_string(),
                ))
            });

        let mut cmd = CliCommands {
            _response_timeout_ms: RESPONSE_TIMEOUT_MS,
            no_wait: false,
            server_connection: mock_server_connection,
        };

        let table_output_result = cmd.get_configs().await;
        assert!(table_output_result.is_err());
    }
}
