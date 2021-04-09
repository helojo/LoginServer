use crate::appdata::AppData;

use actix_web::{web, post, HttpResponse};
use mysql::prelude::Queryable;
use mysql::{Params, params, Row};
use serde::{Serialize, Deserialize};

#[derive(Deserialize)]
pub struct LogoutRequest {
    session_id:     String
}

#[derive(Serialize)]
pub struct LogoutResponse {
    status:         i16
}

#[post("/auth/logout")]
pub async fn post_logout(data: web::Data<AppData>, form: web::Form<LogoutRequest>) -> HttpResponse {
    //Database connection
    let conn_wrapped = data.database.pool.get_conn();
    if conn_wrapped.is_err() {
        eprintln!("An error occurred (logout.rs): {:?}", conn_wrapped.err());
        return HttpResponse::InternalServerError().finish();
    }
    let mut conn = conn_wrapped.unwrap();

    //Verify the session ID
    let sql_verify_session_id = conn.exec::<Row, &str, Params>("SELECT 1 FROM sessions WHERE session_id = :session_id", params! {
         "session_id" => form.session_id.clone()
    });

    if sql_verify_session_id.is_err() {
        eprintln!("An error occurred (logout.rs): {:?}", sql_verify_session_id.is_err());
        return HttpResponse::InternalServerError().finish();
    }

    if sql_verify_session_id.unwrap().len() == 0 {
        //session_id doesn't exist
        let response = LogoutResponse { status: 401 };
        return HttpResponse::Ok().json(&response);
    }

    let sql_delete_session_id = conn.exec::<usize, &str, Params>("DELETE FROM sessions WHERE session_id = :session_id", params! {
        "session_id" => form.session_id.clone()
    });

    if sql_delete_session_id.is_err() {
        eprintln!("An error occurred (logout.rs): {:?}", sql_delete_session_id.is_err());
        return HttpResponse::InternalServerError().finish();
    }

    let response = LogoutResponse { status: 200 };
    HttpResponse::Ok().json(&response)
}