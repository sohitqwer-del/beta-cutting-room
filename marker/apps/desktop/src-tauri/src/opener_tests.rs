//! Inspect the real capability with the plugin's matcher. Never call an OS opener.
use serde_json::Value;

fn capability() -> Value {
    serde_json::from_str(include_str!("../capabilities/main.json")).unwrap()
}

#[test]
fn feedback_opener_grants_only_two_url_scopes_and_no_path_or_custom_app() {
    let capability = capability();
    assert_eq!(capability["windows"], serde_json::json!(["main"]));
    let permissions = capability["permissions"].as_array().unwrap();
    let opener: Vec<_> = permissions
        .iter()
        .filter(|p| {
            p.as_str()
                .or_else(|| p["identifier"].as_str())
                .is_some_and(|id| id.starts_with("opener:"))
        })
        .collect();
    assert_eq!(opener.len(), 1);
    assert_eq!(opener[0]["identifier"], "opener:allow-open-url");
    assert_eq!(
        opener[0]["allow"],
        serde_json::json!([
            {"url":"https://github.com/sohitqwer-del/beta-cutting-room/**"},
            {"url":"mailto:sohitqwer@gmail.com*"}
        ])
    );
}

#[test]
fn feedback_scope_matches_official_globs_without_opening_anything() {
    let capability = capability();
    let permission = capability["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["identifier"] == "opener:allow-open-url")
        .unwrap();
    let patterns: Vec<_> = permission["allow"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| glob::Pattern::new(p["url"].as_str().unwrap()).unwrap())
        .collect();
    let allows = |url: &str| patterns.iter().any(|p| p.matches(url));
    for url in [
        "https://github.com/sohitqwer-del/beta-cutting-room/issues/new?title=Test",
        "https://github.com/sohitqwer-del/beta-cutting-room/discussions",
        "mailto:sohitqwer@gmail.com?subject=Test&body=Original",
    ] {
        assert!(allows(url), "Expected allowed: {url}");
    }
    for url in [
        "file:///C:/data/pattern.dxf",
        "C:\\data\\pattern.dxf",
        "/tmp/pattern",
        "http://github.com/sohitqwer-del/beta-cutting-room/issues",
        "https://github.com/sohitqwer-del/other/issues",
        "https://github.com.evil/sohitqwer-del/beta-cutting-room/issues",
        "https://example.com",
        "mailto:other@example.com",
        "javascript:alert(1)",
    ] {
        assert!(!allows(url), "Unexpected permission: {url}");
    }
}
