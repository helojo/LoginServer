use crate::appdata::AppData;

use actix_web::{post, HttpResponse, web};
use mysql::prelude::Queryable;
use mysql::{Row, Params, params};
use sha2::{Sha512Trunc256, Digest};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct LoginForm {
    email_base64:       String,
    password_base64:    String
}

#[derive(Serialize)]
pub struct LoginResponse {
    status:     i16,
    message:    Option<String>,
    session_id: Option<String>,
    expiry:     Option<i64>
}

#[post("/auth/login")]
pub async fn post_login(data: web::Data<AppData>, form: web::Form<LoginForm>) -> HttpResponse {

    let email_wrapped = base64::decode(form.email_base64.clone().as_bytes());
    if email_wrapped.is_err() {
        return HttpResponse::BadRequest().body(email_wrapped.err().unwrap().to_string());
    }

    let password_wrapped = base64::decode(form.password_base64.clone().as_bytes());
    if password_wrapped.is_err() {
        return HttpResponse::BadRequest().body(password_wrapped.err().unwrap().to_string());
    }

    let email = String::from_utf8(email_wrapped.unwrap()).unwrap();
    let password = String::from_utf8(password_wrapped.unwrap()).unwrap();

    let conn_wrapped = data.database.pool.get_conn();
    if conn_wrapped.is_err() {
        eprintln!("An error occurred (login.rs): {:?}", conn_wrapped.err().unwrap());
        return HttpResponse::InternalServerError().finish();
    }
    let mut conn = conn_wrapped.unwrap();

    let sql_fetch_user_wrapped = conn.exec::<Row, &str, Params>("SELECT password, salt, user_id FROM users WHERE email = :email", params! {
        "email" => email.clone()
    });

    if sql_fetch_user_wrapped.is_err() {
        eprintln!("An error occurred (login.rs): {:?}", sql_fetch_user_wrapped.err().unwrap());
        return HttpResponse::InternalServerError().finish();
    }

    let sql_fetch_user = sql_fetch_user_wrapped.unwrap();
    let row_count = sql_fetch_user.len();

    if row_count == 0 {
        let response = LoginResponse { status: 401, message: Some("E-mail and password combination is invalid, or the account does not exist.".to_string()), session_id: None, expiry: None };
        return HttpResponse::Ok().json(&response);
    }

    if row_count > 1 {
        eprintln!("Database returned more than one Row (login.rs)!");
        return HttpResponse::InternalServerError().finish();
    }

    let (password_from_db, salt, user_id) = {
        let row = sql_fetch_user.get(0).unwrap();
        let password = row.get::<String, &str>("password").unwrap();
        let salt = row.get::<String, &str>("salt").unwrap();
        let user_id = row.get::<String, &str>("user_id").unwrap();

        (password, salt, user_id)
    };

    let mut hasher = Sha512Trunc256::new();
    hasher.update(&password);
    hasher.update(&salt);
    hasher.update(&data.environment.password_pepper);

    let password_hash = base64::encode(hasher.finalize());
    let password_bcrypt = bcrypt::hash_with_salt(&password_hash, 10, &salt.as_bytes()).unwrap();

    let password_finalized = password_bcrypt.format_for_version(bcrypt::Version::TwoY);

    if password_finalized != password_from_db {
        let response = LoginResponse { status: 401, message: Some("E-mail and password combination is invalid, or the account does not exist.".to_string()), session_id: None, expiry: None };
        return HttpResponse::Ok().json(response);
    }

    let session_id: String = rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(64).map(char::from).collect();
    let expiry = (chrono::Utc::now() + chrono::Duration::days(30)).timestamp();

    let sql_write_session_id = conn.exec::<usize, &str, Params>("INSERT INTO sessions (session_id, user_id, expiry) VALUES (:session_id, :user_id, :expiry)", params! {
        "session_id" => session_id.clone(),
        "user_id" => user_id,
        "expiry" => expiry.clone()
    });

    if sql_write_session_id.is_err() {
        eprintln!("An error occurred (login.rs): {:?}", sql_write_session_id.err().unwrap());
        return HttpResponse::InternalServerError().finish();
    }

    let response = LoginResponse { status: 200, message: None, session_id: Some(session_id), expiry: Some(expiry) };
    return HttpResponse::Ok().json(&response);
}