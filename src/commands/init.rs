//! `start` subcommand - example of how to write a subcommand

/// App-local prelude includes `app_reader()`/`app_writer()`/`app_config()`
/// accessors along with logging macros. Customize as you see fit.
use crate::{prelude::*, utility};

use crate::config::SolanaTestInitializerConfig;
use abscissa_core::{config, Command, FrameworkError, Runnable};
use clap::Parser;
use std::fs;
use std::io::Cursor;
use std::primitive;
use std::{path::PathBuf, process::exit};
use toml_edit::{value, Document};

/// `start` subcommand
///
/// The `Parser` proc macro generates an option parser based on the struct
/// definition, and is defined in the `clap` crate. See their documentation
/// for a more comprehensive example:
///
/// <https://docs.rs/clap/>
#[derive(Command, Debug, Parser)]
pub struct InitCmd {
    /// Path to tested project
    #[clap(long = "path", help = "Path to tested tested project.")]
    path: Option<PathBuf>,

    /// Path for framework download
    #[clap(
        long = "framework_path",
        help = "Path where poc-framework will be downloaded."
    )]
    poc_framework_output_path: Option<PathBuf>,

    /// Framework version
    #[clap(
        long = "framework_url",
        help = "URL to poc-framework to download (must be in zip format)."
    )]
    poc_framework_repo_url: Option<String>,

    /// Path to test file
    #[clap(long = "test_file_path", help = "Path to create test file.")]
    test_file_path: Option<PathBuf>,
}

impl Runnable for InitCmd {
    /// Start the application.
    fn run(&self) {
        let config = APP.config();
        if !config.init.poc_framework_output_path.exists() {
            status_err!("couldn't download poc framework repository");
            exit(1);
        }
        let path_to_framework_dir = config
            .init
            .poc_framework_output_path
            .to_str()
            .expect("Cannot parse POC Framework path");

        let response = abscissa_tokio::run(&APP, async {
            utility::download_poc_framework(
                path_to_framework_dir,
                config.init.poc_framework_repo_url.as_str(),
            )
            .await
            .unwrap_or_else(|_| {
                status_err!("couldn't download poc framework repository");
                exit(1);
            });
        });

        let path_to_framework = utility::get_path_to_framework(
            path_to_framework_dir,
            config.init.framework_name.as_str(),
        )
        .unwrap_or_else(|e| {
            status_err!(e);
            exit(1);
        });

        //@TODO determine path to framework
        let path_to_framework_toml = PathBuf::new()
            .join(path_to_framework.clone())
            .join("Cargo.toml");
        let path_to_project_toml = PathBuf::new()
            .join(config.init.path.clone())
            .join("Cargo.toml");

        if !path_to_framework_toml.exists() {
            status_err!("Could not find poc framework Cargo.toml");
            exit(1);
        }

        if !path_to_project_toml.exists() {
            status_err!("Could not find project Cargo.toml");
            exit(1);
        }

        let project_toml = fs::read_to_string(
            path_to_project_toml
                .to_str()
                .expect("Cannot convert path to str"),
        )
        .expect("Something went wrong reading the file");
        let poc_toml = fs::read_to_string(
            path_to_framework_toml
                .to_str()
                .expect("Cannot convert path to str"),
        )
        .expect("Something went wrong reading the file");

        //@TODO add error handling
        let mut project_toml_parsed = project_toml.parse::<Document>().unwrap();
        let mut poc_toml_parsed = poc_toml.parse::<Document>().unwrap();
        poc_toml_parsed =
            utility::set_anchor_for_framework(&mut project_toml_parsed, poc_toml_parsed.clone());
        project_toml_parsed = utility::add_test_bpf_feature(project_toml_parsed.clone());
        let framework_version = utility::get_framework_version(&poc_toml_parsed);
        project_toml_parsed = utility::add_framework_as_dev_dependency(
            project_toml_parsed.clone(),
            path_to_framework
                .to_str()
                .expect("Cannot convert to str from path"),
            &framework_version,
            &config.init.framework_name,
        );

        utility::save_toml(
            project_toml_parsed,
            path_to_project_toml
                .to_str()
                .expect("Cannot convert path to str"),
        );
        utility::save_toml(
            poc_toml_parsed,
            path_to_framework_toml
                .to_str()
                .expect("Cannot convert path to str"),
        );
    }
}

impl config::Override<SolanaTestInitializerConfig> for InitCmd {
    // Process the given command line options, overriding settings from
    // a configuration file using explicit flags taken from command-line
    // arguments.
    fn override_config(
        &self,
        mut config: SolanaTestInitializerConfig,
    ) -> Result<SolanaTestInitializerConfig, FrameworkError> {
        if self.path.is_some() {
            if self.path.clone().unwrap().exists() {
                config.init.path = self.path.clone().unwrap();
            } else {
                status_err!(
                    "Incorrect path to tested project: {}",
                    self.path.clone().unwrap().to_str().unwrap()
                );
                exit(1);
            }
        }

        if self.poc_framework_output_path.is_some() {
            if self.poc_framework_output_path.clone().unwrap().exists() {
                config.init.poc_framework_output_path =
                    self.poc_framework_output_path.clone().unwrap();
            } else {
                status_err!(
                    "Path where poc-framework will be downloaded: {}",
                    self.poc_framework_output_path
                        .clone()
                        .unwrap()
                        .to_str()
                        .unwrap()
                );
                exit(1);
            }
        }

        if self.poc_framework_repo_url.is_some() {
            config.init.poc_framework_repo_url = self.poc_framework_repo_url.clone().unwrap();
        }

        if self.test_file_path.is_some() {
            if self.test_file_path.clone().unwrap().exists() {
                config.init.test_file_path = self.test_file_path.clone().unwrap();
            } else {
                status_err!(
                    "Incorrect Path to create test file: {}",
                    self.test_file_path.clone().unwrap().to_str().unwrap()
                );
                exit(1);
            }
        }

        Ok(config)
    }
}
