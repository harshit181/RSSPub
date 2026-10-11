use reqwest::Client;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::LazyLock;
use std::time::Duration;
use crate::models::GeneralConfig;

pub const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0";

pub static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(|| build_client(None));

static CONFIGURED_CLIENT: AtomicPtr<Client> = AtomicPtr::new(std::ptr::null_mut());

pub fn resolve_user_agent<'a>(user_agent: Option<&'a str>) -> &'a str {
    match user_agent {
        Some(ua) if !ua.trim().is_empty() => ua.trim(),
        _ => DEFAULT_USER_AGENT,
    }
}

pub fn build_client(user_agent: Option<&str>) -> Client {
    let ua = resolve_user_agent(user_agent);
    Client::builder()
        .user_agent(ua)
        .timeout(Duration::from_secs(45))
        .cookie_store(true)
        .tcp_keepalive(Duration::from_secs(60))
        .pool_idle_timeout(Duration::from_secs(90))
        .pool_max_idle_per_host(30)
        .build()
        .expect("Failed to initialize shared HTTP client")
}

pub fn build_client_from_config(config: &GeneralConfig) -> Client {
    build_client(config.user_agent.as_deref())
}

pub fn init_client_from_config(config: &GeneralConfig) {
    update_client_from_config(config);
}

pub fn update_client_from_config(config: &GeneralConfig) {
    let client = Box::leak(Box::new(build_client_from_config(config)));
    let _old = CONFIGURED_CLIENT.swap(client, Ordering::AcqRel);
}

pub fn get_client() -> &'static Client {
    let ptr = CONFIGURED_CLIENT.load(Ordering::Acquire);
    if ptr.is_null() {
        &HTTP_CLIENT
    } else {
        unsafe { &*ptr }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    fn reset_configured_client() {
        CONFIGURED_CLIENT.store(std::ptr::null_mut(), Ordering::Release);
    }

    #[test]
    fn test_client_is_created_once_and_reusable() {
        let _lock = TEST_MUTEX.lock().unwrap();
        reset_configured_client();
        let client1 = get_client();
        let client2 = get_client();
        assert!(std::ptr::eq(client1, client2));
        let _cloned = client1.clone();
    }

    #[test]
    fn test_resolve_user_agent() {
        assert_eq!(resolve_user_agent(None), DEFAULT_USER_AGENT);
        assert_eq!(resolve_user_agent(Some("")), DEFAULT_USER_AGENT);
        assert_eq!(resolve_user_agent(Some("   ")), DEFAULT_USER_AGENT);
        assert_eq!(resolve_user_agent(Some("CustomBot/1.0")), "CustomBot/1.0");
        assert_eq!(resolve_user_agent(Some("  CustomBot/1.0  ")), "CustomBot/1.0");
    }

    #[test]
    fn test_build_client_from_config() {
        let default_config = GeneralConfig::default();
        let _c1 = build_client_from_config(&default_config);

        let custom_config = GeneralConfig {
            user_agent: Some("MyAgent/2.0".to_string()),
            ..Default::default()
        };
        let _c2 = build_client_from_config(&custom_config);
    }

    #[test]
    fn test_update_client_from_config() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let custom_config = GeneralConfig {
            user_agent: Some("TestAgent/1.0".to_string()),
            ..Default::default()
        };
        update_client_from_config(&custom_config);
        let client = get_client();
        let client2 = get_client();
        assert!(std::ptr::eq(client, client2));
        reset_configured_client();
    }
}
