// Copyright (c) 2025 Elektrobit Automotive GmbH
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

use crate::cli::AnkCli;
use common::std_extensions::UnreachableOption;
use common::DEFAULT_SERVER_ADDRESS;
use grpc::security::read_pem_file;

use serde::de::value::MapDeserializer;
use serde::de::{Deserializer, Error, MapAccess, Visitor};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt;
use std::fs::read_to_string;
use std::path::PathBuf;
use toml::{de, from_str, Value};

const SUPPORTED_CONFIG_VARIANTS: usize = 1;

pub const CONFIG_VERSION: &str = "v1";
pub const DEFAULT_CONFIG: &str = "default";
pub const DEFAULT_RESPONSE_TIMEOUT: u64 = 3000;

#[cfg(not(test))]
pub const DEFAULT_ANK_CONFIG_FILE_PATH: &str = "$HOME/.config/ankaios/ank.conf";

#[cfg(test)]
pub const DEFAULT_ANK_CONFIG_FILE_PATH: &str = "/tmp/ankaios/ank.conf";

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ConversionErrors {
    WrongVersion(String),
    ConflictingCertificates(String),
    InvalidAnkConfig(String),
    InvalidCertificate(String),
}

impl fmt::Display for ConversionErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConversionErrors::WrongVersion(msg) => {
                write!(f, "Wrong version: {}", msg)
            }
            ConversionErrors::ConflictingCertificates(msg) => {
                write!(f, "Conflicting certificates: {}", msg)
            }
            ConversionErrors::InvalidAnkConfig(msg) => {
                write!(f, "Ank Config could not have been parsed due to: {}", msg)
            }
            ConversionErrors::InvalidCertificate(msg) => {
                write!(f, "Certificate could not have been read due to: {}", msg)
            }
        }
    }
}

fn get_default_response_timeout() -> u64 {
    DEFAULT_RESPONSE_TIMEOUT
}

fn get_default_url() -> String {
    DEFAULT_SERVER_ADDRESS.to_string()
}

#[derive(Debug, PartialEq)]
pub struct AnkConfig {
    pub version: String,
    pub response_timeout: u64,
    pub verbose: bool,
    pub quiet: bool,
    pub no_wait: bool,
    pub server_url: String,
    pub insecure: bool,
    ca_pem: Option<String>,
    crt_pem: Option<String>,
    key_pem: Option<String>,
    pub ca_pem_content: Option<String>,
    pub crt_pem_content: Option<String>,
    pub key_pem_content: Option<String>,
}

/// A helper struct that derives Deserialize. Its fields mirror those of AnkConfig.
#[derive(Deserialize)]
struct AnkConfigHelper {
    version: String,
    #[serde(default = "get_default_response_timeout")]
    response_timeout: u64,
    #[serde(default)]
    verbose: bool,
    #[serde(default)]
    quiet: bool,
    #[serde(default)]
    no_wait: bool,
    #[serde(default = "get_default_url")]
    server_url: String,
    #[serde(default)]
    insecure: bool,
    ca_pem: Option<String>,
    crt_pem: Option<String>,
    key_pem: Option<String>,
    ca_pem_content: Option<String>,
    crt_pem_content: Option<String>,
    key_pem_content: Option<String>,
}

impl From<AnkConfigHelper> for AnkConfig {
    fn from(helper: AnkConfigHelper) -> Self {
        AnkConfig {
            version: helper.version,
            response_timeout: helper.response_timeout,
            verbose: helper.verbose,
            quiet: helper.quiet,
            no_wait: helper.no_wait,
            server_url: helper.server_url,
            insecure: helper.insecure,
            ca_pem: helper.ca_pem,
            crt_pem: helper.crt_pem,
            key_pem: helper.key_pem,
            ca_pem_content: helper.ca_pem_content,
            crt_pem_content: helper.crt_pem_content,
            key_pem_content: helper.key_pem_content,
        }
    }
}

struct AnkConfigVisitor {
    table_key: &'static str,
}

impl<'de> Visitor<'de> for AnkConfigVisitor {
    type Value = AnkConfig;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a TOML map for AnkConfig with merged nested table fields")
    }

    fn visit_map<V>(self, mut map: V) -> Result<AnkConfig, V::Error>
    where
        V: MapAccess<'de>,
    {
        let mut merged: BTreeMap<String, Value> = BTreeMap::new();

        while let Some(key) = map.next_key::<String>()? {
            let value: Value = map.next_value()?;
            if key == self.table_key {
                if let Value::Table(inner) = value {
                    for (k, v) in inner.into_iter() {
                        merged.insert(k, v);
                    }
                } else {
                    return Err(V::Error::custom(format!(
                        "Expected '{}' to be a table",
                        self.table_key
                    )));
                }
            } else {
                merged.insert(key, value);
            }
        }

        let deserializer = MapDeserializer::new(merged.into_iter());
        let helper = AnkConfigHelper::deserialize(deserializer).map_err(V::Error::custom)?;
        Ok(helper.into())
    }
}

impl<'de> Deserialize<'de> for AnkConfig {
    fn deserialize<D>(deserializer: D) -> Result<AnkConfig, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(AnkConfigVisitor {
            table_key: DEFAULT_CONFIG,
        })
    }
}

impl Default for AnkConfig {
    fn default() -> AnkConfig {
        AnkConfig {
            version: CONFIG_VERSION.to_string(),
            response_timeout: get_default_response_timeout(),
            verbose: bool::default(),
            quiet: bool::default(),
            no_wait: bool::default(),
            server_url: get_default_url(),
            insecure: bool::default(),
            ca_pem: None,
            crt_pem: None,
            key_pem: None,
            ca_pem_content: None,
            crt_pem_content: None,
            key_pem_content: None,
        }
    }
}

impl AnkConfig {
    pub fn from_file(file_path: PathBuf) -> Result<AnkConfig, ConversionErrors> {
        let ank_config_content = read_to_string(file_path.to_str().unwrap_or_unreachable())
            .map_err(|err| ConversionErrors::InvalidAnkConfig(err.to_string()))?;
        let mut ank_config: AnkConfig = from_str(&ank_config_content)
            .map_err(|err| ConversionErrors::InvalidAnkConfig(err.to_string()))?;

        if ank_config.version != CONFIG_VERSION {
            return Err(ConversionErrors::WrongVersion(ank_config.version));
        }

        // let default_context = ank_config
        //     .config_variant
        //     .get_mut(DEFAULT_CONFIG)
        //     .ok_or(ConversionErrors::DefaultContextNotFound())?;

        // if Self::has_conflicting_certificates(default_context) {
        //     return Err(ConversionErrors::ConflictingCertificates(
        //         "Certificate paths and certificate content are both set".to_string(),
        //     ));
        // }

        // Self::read_pem_files(default_context)?;

        Ok(ank_config)
    }

    // fn has_conflicting_certificates(config: &ConfigVariant) -> bool {
    //     config.ca_pem.is_some() && config.ca_pem_content.is_some()
    //         || config.crt_pem.is_some() && config.crt_pem_content.is_some()
    //         || config.key_pem.is_some() && config.key_pem_content.is_some()
    // }

    // fn read_pem_files(config: &mut ConfigVariant) -> Result<(), ConversionErrors> {
    //     if let Some(ca_pem_path) = &config.ca_pem {
    //         config.ca_pem_content = Some(
    //             read_pem_file(ca_pem_path, false)
    //                 .map_err(|err| ConversionErrors::InvalidCertificate(err.to_string()))?,
    //         );
    //     }
    //     if let Some(crt_pem_path) = &config.crt_pem {
    //         config.crt_pem_content = Some(
    //             read_pem_file(crt_pem_path, false)
    //                 .map_err(|err| ConversionErrors::InvalidCertificate(err.to_string()))?,
    //         );
    //     }
    //     if let Some(key_pem_path) = &config.key_pem {
    //         config.key_pem_content = Some(
    //             read_pem_file(key_pem_path, false)
    //                 .map_err(|err| ConversionErrors::InvalidCertificate(err.to_string()))?,
    //         );
    //     }
    //     Ok(())
    // }

    pub fn update_with_args(&mut self, args: &AnkCli) {
        if let Some(response_timeout) = args.response_timeout_ms {
            self.response_timeout = response_timeout;
        }

        self.verbose = args.verbose;
        self.quiet = args.quiet;
        self.no_wait = args.no_wait;

        // if let Some(default_config) = self.config_variant.get_mut(DEFAULT_CONFIG) {
        //     default_config.insecure = args.insecure;

        //     if let Some(ca_pem_path) = &args.ca_pem {
        //         default_config.ca_pem = Some(ca_pem_path.to_owned());
        //         let ca_pem_content = read_pem_file(ca_pem_path, false).unwrap_or_default();
        //         default_config.ca_pem_content = Some(ca_pem_content);
        //     }
        //     if let Some(crt_pem_path) = &args.crt_pem {
        //         default_config.crt_pem = Some(crt_pem_path.to_owned());
        //         let crt_pem_content = read_pem_file(crt_pem_path, false).unwrap_or_default();
        //         default_config.crt_pem_content = Some(crt_pem_content);
        //     }
        //     if let Some(key_pem_path) = &args.key_pem {
        //         default_config.key_pem = Some(key_pem_path.to_owned());
        //         let key_pem_content = read_pem_file(key_pem_path, false).unwrap_or_default();
        //         default_config.key_pem_content = Some(key_pem_content);
        //     }
        // }
    }
}

// //////////////////////////////////////////////////////////////////////////////
// //                 ########  #######    #########  #########                //
// //                    ##     ##        ##             ##                    //
// //                    ##     #####     #########      ##                    //
// //                    ##     ##                ##     ##                    //
// //                    ##     #######   #########      ##                    //
// //////////////////////////////////////////////////////////////////////////////

// #[cfg(test)]
// mod tests {
//     use std::io::Write;
//     use std::path::PathBuf;
//     use tempfile::NamedTempFile;

//     use common::DEFAULT_SERVER_ADDRESS;

//     use crate::ank_config::ConfigVariant;
//     use crate::{
//         ank_config::{get_default_response_timeout, ConversionErrors, DEFAULT_CONFIG},
//         cli::{AnkCli, Commands, GetArgs, GetCommands},
//     };

//     use super::{AnkConfig, DEFAULT_ANK_CONFIG_FILE_PATH};

//     const CA_PEM_PATH: &str = "some_path_to_ca_pem/ca.pem";
//     const CRT_PEM_PATH: &str = "some_path_to_crt_pem/crt.pem";
//     const KEY_PEM_PATH: &str = "some_path_to_key_pem/key.pem";
//     const CA_PEM_CONTENT: &str = r"the content of the
//         ca.pem file is stored in here";
//     const CRT_PEM_CONTENT: &str = r"the content of the
//         crt.pem file is stored in here";
//     const KEY_PEM_CONTENT: &str = r"the content of the
//         key.pem file is stored in here";

//     #[test]
//     fn utest_default_ank_config() {
//         let default_ank_config = AnkConfig::default();

//         assert_eq!(
//             default_ank_config.response_timeout,
//             get_default_response_timeout()
//         );
//         assert!(!default_ank_config.verbose);
//         assert!(!default_ank_config.quiet);
//         assert!(!default_ank_config.no_wait);
//         assert_eq!(
//             default_ank_config.config_variant[DEFAULT_CONFIG],
//             ConfigVariant::default()
//         );
//     }

//     #[test]
//     fn utest_ank_config_wrong_version() {
//         let ank_config_content: &str = r"#
//         version = 'v2'
//         #";

//         let mut tmp_config_file = NamedTempFile::new().unwrap();
//         write!(tmp_config_file, "{}", ank_config_content).unwrap();

//         let ank_config = AnkConfig::from_file(PathBuf::from(tmp_config_file.path()));

//         assert_eq!(
//             ank_config,
//             Err(ConversionErrors::WrongVersion("v2".to_string()))
//         );
//     }

//     #[test]
//     fn utest_ank_config_conflicting_certificates() {
//         let ank_config_content = format!(
//             r"#
//         version = 'v1'
//         [default]
//         ca_pem = '''{}'''
//         ca_pem_content = '''{}'''
//         #",
//             CA_PEM_PATH, CRT_PEM_CONTENT
//         );

//         let mut tmp_config_file = NamedTempFile::new().unwrap();
//         write!(tmp_config_file, "{}", ank_config_content).unwrap();

//         let ank_config = AnkConfig::from_file(PathBuf::from(tmp_config_file.path()));

//         assert_eq!(
//             ank_config,
//             Err(ConversionErrors::ConflictingCertificates(
//                 "Certificate paths and certificate content are both set".to_string()
//             ))
//         );
//     }

//     #[test]
//     fn utest_ank_config_update_with_args() {
//         let mut ank_config = AnkConfig::default();
//         let args = AnkCli {
//             command: Commands::Get(GetArgs {
//                 command: Some(GetCommands::State {
//                     output_format: crate::cli::OutputFormat::Yaml,
//                     object_field_mask: Vec::new(),
//                 }),
//             }),
//             server_url: Some(DEFAULT_SERVER_ADDRESS.to_string()),
//             config_path: Some(DEFAULT_ANK_CONFIG_FILE_PATH.to_string()),
//             response_timeout_ms: Some(5000),
//             insecure: false,
//             verbose: true,
//             quiet: true,
//             no_wait: true,
//             ca_pem: Some(CA_PEM_PATH.to_string()),
//             crt_pem: Some(CRT_PEM_PATH.to_string()),
//             key_pem: Some(KEY_PEM_PATH.to_string()),
//         };

//         ank_config.update_with_args(&args);

//         assert_eq!(ank_config.response_timeout, 5000);
//         assert!(ank_config.verbose);
//         assert!(ank_config.quiet);
//         assert!(ank_config.no_wait);
//         assert!(!ank_config.config_variant[DEFAULT_CONFIG].insecure);
//         assert_eq!(
//             ank_config.config_variant[DEFAULT_CONFIG].ca_pem,
//             Some(CA_PEM_PATH.to_string())
//         );
//         assert_eq!(
//             ank_config.config_variant[DEFAULT_CONFIG].crt_pem,
//             Some(CRT_PEM_PATH.to_string())
//         );
//         assert_eq!(
//             ank_config.config_variant[DEFAULT_CONFIG].key_pem,
//             Some(KEY_PEM_PATH.to_string())
//         );
//     }

//     #[test]
//     fn utest_ank_config_update_with_args_certificates_content() {
//         let ank_config_content = format!(
//             r"#
//         version = 'v1'
//         [default]
//         ca_pem_content = '''{}'''
//         crt_pem_content = '''{}'''
//         key_pem_content = '''{}'''
//         #",
//             CA_PEM_CONTENT, CRT_PEM_CONTENT, KEY_PEM_CONTENT
//         );

//         let mut tmp_config_file = NamedTempFile::new().unwrap();
//         write!(tmp_config_file, "{}", ank_config_content).unwrap();

//         let mut ank_config = AnkConfig::from_file(PathBuf::from(tmp_config_file.path())).unwrap();
//         let args = AnkCli {
//             command: Commands::Get(GetArgs {
//                 command: Some(GetCommands::State {
//                     output_format: crate::cli::OutputFormat::Yaml,
//                     object_field_mask: Vec::new(),
//                 }),
//             }),
//             server_url: Some(DEFAULT_SERVER_ADDRESS.to_string()),
//             config_path: Some(DEFAULT_ANK_CONFIG_FILE_PATH.to_string()),
//             response_timeout_ms: Some(5000),
//             insecure: false,
//             verbose: true,
//             quiet: true,
//             no_wait: true,
//             ca_pem: None,
//             crt_pem: None,
//             key_pem: None,
//         };

//         ank_config.update_with_args(&args);

//         assert_eq!(
//             ank_config.config_variant[DEFAULT_CONFIG].ca_pem_content,
//             Some(CA_PEM_CONTENT.to_string())
//         );
//         assert_eq!(
//             ank_config.config_variant[DEFAULT_CONFIG].crt_pem_content,
//             Some(CRT_PEM_CONTENT.to_string())
//         );
//         assert_eq!(
//             ank_config.config_variant[DEFAULT_CONFIG].key_pem_content,
//             Some(KEY_PEM_CONTENT.to_string())
//         );
//     }

//     #[test]
//     fn utest_ank_config_default_context_not_found() {
//         let ank_config_content = r"#
//         version = 'v1'
//         #";
//         let mut tmp_config_file = NamedTempFile::new().unwrap();
//         write!(tmp_config_file, "{}", ank_config_content).unwrap();

//         let ank_config = AnkConfig::from_file(PathBuf::from(tmp_config_file.path()));

//         assert_eq!(ank_config, Err(ConversionErrors::DefaultContextNotFound()));
//     }

//     #[test]
//     fn utest_ank_config_multiple_contexts_found() {
//         let ank_config_content = r"#
//         version = 'v1'
//         [default]
//         [context]
//         #";
//         let mut tmp_config_file = NamedTempFile::new().unwrap();
//         write!(tmp_config_file, "{}", ank_config_content).unwrap();

//         let ank_config = AnkConfig::from_file(PathBuf::from(tmp_config_file.path()));

//         assert_eq!(ank_config, Err(ConversionErrors::UnsupportedContexts()));
//     }

//     #[test]
//     fn utest_ank_config_from_file_successful() {
//         let ank_config_content = format!(
//             r"#
//         version = 'v1'
//         response_timeout = 3000
//         verbose = false
//         quiet = false
//         no_wait = false
//         [default]
//         server_url = 'https://127.0.0.1:25551'
//         insecure = false
//         ca_pem_content = '''{}'''
//         crt_pem_content = '''{}'''
//         key_pem_content = '''{}'''
//         #",
//             CA_PEM_CONTENT, CRT_PEM_CONTENT, KEY_PEM_CONTENT
//         );

//         let mut tmp_config_file = NamedTempFile::new().unwrap();
//         write!(tmp_config_file, "{}", ank_config_content).unwrap();

//         let ank_config_res = AnkConfig::from_file(PathBuf::from(tmp_config_file.path()));

//         assert!(ank_config_res.is_ok());

//         let ank_config = ank_config_res.unwrap();

//         assert_eq!(ank_config.response_timeout, 3000);
//         assert!(!ank_config.verbose);
//         assert!(!ank_config.quiet);
//         assert!(!ank_config.no_wait);
//         assert_eq!(
//             ank_config.config_variant[DEFAULT_CONFIG].server_url,
//             DEFAULT_SERVER_ADDRESS.to_string()
//         );
//         assert!(!ank_config.config_variant[DEFAULT_CONFIG].insecure);
//         assert_eq!(
//             ank_config.config_variant[DEFAULT_CONFIG].ca_pem_content,
//             Some(CA_PEM_CONTENT.to_string())
//         );
//         assert_eq!(
//             ank_config.config_variant[DEFAULT_CONFIG].crt_pem_content,
//             Some(CRT_PEM_CONTENT.to_string())
//         );
//         assert_eq!(
//             ank_config.config_variant[DEFAULT_CONFIG].key_pem_content,
//             Some(KEY_PEM_CONTENT.to_string())
//         );
//     }
// }
