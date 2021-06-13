use crate::appdata::AppData;

use actix_web::{web, post, HttpResponse};
use mysql::prelude::Queryable;
use mysql::{Row, Params, params};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct SessionRequest {
    session_id: String,
}

#[derive(Serialize)]
pub struct SessionResponse {
    status:         i16,
    user_id:        Option<String>,
    email:          Option<String>,
    message:        Option<&'static str>
}

#[post("/auth/session")]
pub async fn post_session(data: web::Data<AppData>, form: web::Form<SessionRequest>) -> HttpResponse {
    //Database connection
    let conn_wrapped = data.database.pool.get_conn();
    if conn_wrapped.is_err() {
        eprintln!("An error occurred (session.rs): {:?}", conn_wrapped.err());
        return  HttpResponse::InternalServerError().finish();
    }
    let mut conn = conn_wrapped.unwrap();

    //Verify the session_id
    let sql_verify_session_id_wrapped = conn.exec::<Row, &str, Params>("SELECT user_id, expiry FROM sessions WHERE session_id = :session_id", params! {
        "session_id" => form.session_id.clone()
    });

    if sql_verify_session_id_wrapped.is_err() {
        eprintln!("An error occurred (session.rs): {:?}", sql_verify_session_id_wrapped.err());
        return  HttpResponse::InternalServerError().finish();
    }

    let sql_verify_session_id = sql_verify_session_id_wrapped.unwrap();
    if sql_verify_session_id.len() == 0 {
        let response = SessionResponse { status: 401, user_id: None, email: None, message: Some("Session ID not found.") };
        return HttpResponse::Ok().json(&response);
    }

    let (user_id, expiry) = {
        let row = sql_verify_session_id.get(0).unwrap();
        let user_id = row.get::<String, &str>("user_id").unwrap();
        let expiry = row.get::<i64, &str>("expiry").unwrap();

        (user_id, expiry)
    };

    //Verify the expiry
    if chrono::Utc::now().timestamp() >= expiry {
        let response = SessionResponse { status: 401, user_id: None, email: None, message: Some("Session expired") };
        return HttpResponse::Ok().json(&response);
    }

    //Get the E-mail address
    let sql_get_email_wrapped = conn.exec::<Row, &str, Params>("SELECT email FROM users WHERE user_id = :user_id", params! {
        "user_id" => user_id.clone()
    });

    if sql_get_email_wrapped.is_err() {
        eprintln!("An error occurred (session.rs): {:?}", sql_get_email_wrapped.err());
        return  HttpResponse::InternalServerError().finish();
    }

    let sql_get_email = sql_get_email_wrapped.unwrap();
    let email = {
        let row = sql_get_email.get(0).unwrap();
        let email = row.get::<String, &str>("email").unwrap();

        email
    };

    let response = SessionResponse { status: 200, user_id: Some(user_id), email: Some(email), message: None };
    HttpResponse::Ok().json(&response)
}