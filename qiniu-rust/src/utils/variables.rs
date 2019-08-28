use getset::Getters;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use std::io::BufReader;

// TODO: MOVE THIS MOD TO qiniu-rust-http-test-utils
#[derive(Debug, Getters, Clone)]
#[get = "pub"]
pub(crate) struct Variables {
    access_key: String,
    secret_key: String,
    z2_encrypt_key: String,
}

#[derive(Serialize, Deserialize)]
struct OptionalVariables {
    access_key: Option<String>,
    secret_key: Option<String>,
    z2_encrypt_key: Option<String>,
}

pub(crate) fn load_variables() -> Variables {
    let mut variables = OptionalVariables {
        access_key: env::var("access_key").ok(),
        secret_key: env::var("secret_key").ok(),
        z2_encrypt_key: env::var("z2_encrypt_key").ok(),
    };

    if let Ok(file) = File::open("variables.yml") {
        let deserialized_variables: OptionalVariables = serde_yaml::from_reader(BufReader::new(file)).unwrap();
        if variables.access_key.is_none() && deserialized_variables.access_key.is_some() {
            variables.access_key = deserialized_variables.access_key;
        }
        if variables.secret_key.is_none() && deserialized_variables.secret_key.is_some() {
            variables.secret_key = deserialized_variables.secret_key;
        }
        if variables.z2_encrypt_key.is_none() && deserialized_variables.z2_encrypt_key.is_some() {
            variables.z2_encrypt_key = deserialized_variables.z2_encrypt_key;
        }
    }
    Variables {
        access_key: variables.access_key.expect("access_key must be set"),
        secret_key: variables.secret_key.expect("secret_key must be set"),
        z2_encrypt_key: variables.z2_encrypt_key.expect("z2_encrypt_key must be set"),
    }
}
