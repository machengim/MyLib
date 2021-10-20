pub const DEFAULT_APP_NAME: &'static str = "Oasis";
pub const DEFAULT_UPDATE_FREQ: &'static str = "monthly";
pub const DEFAULT_LANGUAGE: &'static str = "en";
pub const VERSION: &'static str = "0.1";
#[allow(dead_code)]
pub const FRONTEND_DIR_DEBUG: &'static str = "../frontend/public/";
#[allow(dead_code)]
pub const FRONTEND_DIR_RELEASE: &'static str = "public";
pub const ACCESS_TOKEN: &'static str = "oa_access";
pub const ACCESS_TOKEN_MINS: i64 = 20;
pub const REFRESH_TOKEN: &'static str = "oa_refresh";
pub const REFRESH_TOKEN_DAYS: i64 = 7;
pub const CACHE_MAX_AGE: i64 = 24 * 60 * 60;
pub const APP_VERSION_URL: &'static str =
    "https://raw.githubusercontent.com/machengim/oasis/main/version.txt";
#[allow(dead_code)]
pub const CACHE_FILE_EXTS: [&'static str; 3] = ["html", "js", "css"];
