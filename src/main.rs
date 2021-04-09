mod appdata;
mod endpoints;

use crate::appdata::{Environment, Database, AppData};

use actix_web::{HttpServer, App};
use actix_cors::Cors;
use actix_web::middleware::Logger;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting server...");

    let environment = Environment::new();
    let database = Database::new(&environment);

    println!("Checking database...");
    let check_db_result = database.check_db(&environment);
    if check_db_result.is_err() {
        eprintln!("Something went wrong checking the database (main.rs)! Exiting.");
        std::process::exit(1);
    }

    if !check_db_result.unwrap() {
        println!("Database did not pass the check. Attempting to correct...");
        let init_db_result = database.init_db(&environment);
        if init_db_result.is_err() {
            println!("Something went wrong initializing the database (main.rs)! Exiting.");
            std::process::exit(1);
        } else {
            println!("Database initialized.");
        }
    } else {
        println!("Database passed the check.");
    }

    let appdata = AppData::new(database, environment);
    println!("Startup complete. Listening on 0.0.0.0:8080");

    //Start the Actix HTTP server
    HttpServer::new(move || {
        let cors = Cors::permissive().allow_any_header().allow_any_origin().allow_any_method();

        App::new()
            .data(appdata.clone())
            .service(endpoints::auth::login::post_login)
            .service(endpoints::auth::register::post_register)
            .service(endpoints::auth::logout::post_logout)
            .service(endpoints::auth::session::post_session)
            .wrap(cors)
            .wrap(Logger::default())
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
