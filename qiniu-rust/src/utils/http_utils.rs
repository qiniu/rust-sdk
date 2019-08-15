pub(crate) fn head<URL: AsRef<str>>(url: URL) -> reqwest::Result<reqwest::Response> {
    reqwest::Client::builder().build().unwrap().head(url.as_ref()).send()
}
