mod api;
mod db;
mod entity;
mod error;
mod filesystem;
mod utils;
use actix_files::{self, Files};
use actix_web::{rt::System, web, App, HttpServer};
use entity::{AppState, Site};
use sqlx::{Pool, Sqlite};
use std::sync::Mutex;

#[macro_use]
extern crate log;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::Builder::from_env("LOG_LEVEL").init();

    // let rt  = Runtime::new().unwrap();
    // let state = web::Data::new(rt.block_on(init_app_state()));
    let state = web::Data::new(init_app_state().await);
    debug!("app state: {:?}", &state);
    let react_dir = std::env::var("REACT_DIR").expect("Cannot get frontend dir from env");
    let sys = System::new();
    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .service(web::resource("/").route(web::get().to(api::index)))
            .configure(api::config)
        // .service(Files::new("/", react_dir.clone()).index_file("index.html"))
    })
    .bind("127.0.0.1:3000")?
    .run();

    sys.run()
}

async fn init_app_state() -> AppState {
    let pool = db::get_db_conn().await;
    let site = read_site(&pool).await;
    let first_run = site.first_run == 1;

    AppState {
        first_run: Mutex::new(first_run),
        pool: Mutex::new(pool),
        storage: Mutex::new(site.storage),
    }
}

async fn read_site(pool: &Pool<Sqlite>) -> Site {
    let sql = "SELECT * FROM site";
    let args = vec![];

    match db::fetch_single::<Site>(sql, args, pool).await {
        Ok(site) => site,
        Err(e) => panic!("Cannot read configuration: {}", e),
    }
}
