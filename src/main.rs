mod appdata;
mod endpoints;

use crate::appdata::{Environment, Database, AppData};

use actix_web::{HttpServer, App};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting server...");

    let environment = Environment::new();
    let database = Database::new(&environment);
    let appdata = AppData::new(database, environment);

    println!("Startup complete. Listening on 0.0.0.0:8080");

    //Start the Actix HTTP server
    HttpServer::new(move || {
        App::new()
            .data(appdata.clone())
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
