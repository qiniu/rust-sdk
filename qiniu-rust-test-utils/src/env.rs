use getset::Getters;
use std::{env, sync::Once};

static INIT: Once = Once::new();

#[derive(Debug, Getters, Clone)]
#[get = "pub"]
pub struct Env {
    access_key: String,
    secret_key: String,
    z2_encrypt_key: String,
}

pub fn get() -> Env {
    INIT.call_once(|| {
        dotenv::dotenv().ok();
    });
    Env {
        access_key: env::var("access_key").expect("access_key must be set"),
        secret_key: env::var("secret_key").expect("secret_key must be set"),
        z2_encrypt_key: env::var("z2_encrypt_key").expect("z2_encrypt_key must be set"),
    }
}
