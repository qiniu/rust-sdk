use lazy_static;

struct Config {
    use_https: bool,
}

lazy_static! {
    static ref DEFAULT: Config = {
        use_https: false,
    }
}
