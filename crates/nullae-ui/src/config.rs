pub struct Config {
    pub api_base_url: String,
}

impl Config {
    pub fn new() -> Self {
        let api_base_url = if cfg!(debug_assertions) {
            // Development environment
            web_sys::console::log_1(
                &"Config: Using development API URL http://localhost:3000".into(),
            );
            "http://localhost:3000".to_string()
        } else {
            // Production environment
            web_sys::console::log_1(
                &"Config: Using production API URL https://api.lab.0ae.ru".into(),
            );
            "https://api.lab.0ae.ru".to_string()
        };

        Self { api_base_url }
    }

    pub fn api_url(&self, endpoint: &str) -> String {
        let url = format!("{}{}", self.api_base_url, endpoint);
        web_sys::console::log_1(&format!("Config: Full API URL: {}", url).into());
        url
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}
