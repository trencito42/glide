use url::Url;

use crate::model::NEW_TAB_URL;

pub const SEARCH_BASE: &str = "https://duckduckgo.com/";

pub fn resolve_omnibox(input: &str, fixture_port: u16) -> Result<String, String> {
    let raw = input.trim();
    if raw.is_empty() {
        return Err("Enter an address or a search.".into());
    }
    if raw.len() > 8_000 {
        return Err("That address is too long.".into());
    }
    if raw.eq_ignore_ascii_case(NEW_TAB_URL) {
        return Ok(NEW_TAB_URL.into());
    }
    if raw.contains("://") || raw.contains(':') && raw.split(':').next().is_some_and(|s| s.eq_ignore_ascii_case("javascript") || s.eq_ignore_ascii_case("data") || s.eq_ignore_ascii_case("file")) {
        let url = Url::parse(raw).map_err(|error| error.to_string())?;
        ensure_user_navigation(&url, fixture_port)?;
        return Ok(url.to_string());
    }
    if looks_like_host(raw) {
        let url = Url::parse(&format!("https://{raw}")).map_err(|error| error.to_string())?;
        ensure_user_navigation(&url, fixture_port)?;
        return Ok(url.to_string());
    }
    let mut search = Url::parse(SEARCH_BASE).expect("search base");
    search.query_pairs_mut().append_pair("q", raw);
    Ok(search.to_string())
}

pub fn ensure_user_navigation(url: &Url, fixture_port: u16) -> Result<(), String> {
    if !page_navigation_allowed(url, fixture_port) {
        return Err(format!("Blocked {}", url.scheme()));
    }
    if is_privileged_shell(url) {
        return Err("That address is reserved for the browser shell.".into());
    }
    Ok(())
}

pub fn page_navigation_allowed(url: &Url, fixture_port: u16) -> bool {
    match url.scheme() {
        "https" => true,
        "http" => is_public_http(url) || is_local_dev(url) || is_fixture(url, fixture_port),
        "tauri" | "asset" => true,
        _ => false,
    }
}

pub fn is_shell_document(url: &Url) -> bool {
    is_privileged_shell(url) || is_local_dev(url)
}

pub fn shell_navigation_allowed(url: &Url) -> bool {
    match url.scheme() {
        "tauri" | "asset" => true,
        "http" | "https" => is_privileged_shell(url) || is_local_dev(url),
        _ => false,
    }
}

fn is_privileged_shell(url: &Url) -> bool {
    matches!(
        url.host_str(),
        Some("tauri.localhost") | Some("asset.localhost")
    )
}

fn is_local_dev(url: &Url) -> bool {
    matches!(url.host_str(), Some("localhost") | Some("127.0.0.1"))
        && url.port() == Some(1420)
}

fn is_fixture(url: &Url, fixture_port: u16) -> bool {
    matches!(url.host_str(), Some("localhost") | Some("127.0.0.1"))
        && url.port() == Some(fixture_port)
}

fn is_public_http(url: &Url) -> bool {
    !matches!(
        url.host_str(),
        Some("localhost") | Some("127.0.0.1") | Some("tauri.localhost") | Some("asset.localhost") | None
    )
}

fn looks_like_host(raw: &str) -> bool {
    if raw.contains(' ') || raw.contains('@') || raw.contains('\\') {
        return false;
    }
    let host = raw.split(['/', '?', '#']).next().unwrap_or(raw);
    if host.eq_ignore_ascii_case("localhost") || host.to_ascii_lowercase().starts_with("localhost:") {
        return true;
    }
    let bare = host.split(':').next().unwrap_or(host);
    if bare.parse::<std::net::Ipv4Addr>().is_ok() {
        return true;
    }
    bare.contains('.') && !bare.starts_with('.') && !bare.ends_with('.') && !bare.contains("..")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn searches_words_and_upgrades_hosts() {
        let search = resolve_omnibox("weather in oslo", 9).unwrap();
        assert!(search.starts_with("https://duckduckgo.com/?q=weather"));
        assert_eq!(
            resolve_omnibox("example.com/docs", 9).unwrap(),
            "https://example.com/docs"
        );
    }

    #[test]
    fn blocks_scriptable_and_file_schemes() {
        assert!(resolve_omnibox("javascript:alert(1)", 9).is_err());
        assert!(resolve_omnibox("file:///C:/Windows", 9).is_err());
        assert!(resolve_omnibox("data:text/html,hi", 9).is_err());
    }

    #[test]
    fn allows_the_fixture_server_and_blocks_other_loopback_from_the_omnibox() {
        assert!(resolve_omnibox("http://127.0.0.1:4321/persist.html?n=1", 4321).is_ok());
        assert!(resolve_omnibox("http://127.0.0.1:3000/", 4321).is_err());
    }

    #[test]
    fn page_policy_rejects_non_http_schemes() {
        let js = Url::parse("javascript:alert(1)").unwrap();
        assert!(!page_navigation_allowed(&js, 1));
        let https = Url::parse("https://example.com").unwrap();
        assert!(page_navigation_allowed(&https, 1));
    }

    #[test]
    fn shell_policy_rejects_public_sites() {
        let site = Url::parse("https://example.com").unwrap();
        assert!(!shell_navigation_allowed(&site));
        let dev = Url::parse("http://127.0.0.1:1420/").unwrap();
        assert!(shell_navigation_allowed(&dev));
    }
}
