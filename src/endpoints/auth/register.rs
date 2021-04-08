use crate::appdata::AppData;

use actix_web::{web, post, HttpResponse};
use mysql::prelude::Queryable;
use mysql::{Row, Params, params};
use rand::Rng;
use sha2::{Sha512Trunc256, Digest};
use serde::{Serialize, Deserialize};
use regex::Regex;
use lazy_static::lazy_static;

#[derive(Deserialize)]
pub struct RegisterForm {
    email_base64:       String,
    password_base64:    String
}

#[derive(Serialize)]
pub struct RegisterResponse {
    status:     i16,
    message:    Option<String>,
    session_id: Option<String>,
    expiry:     Option<i64>
}

#[post("/auth/register")]
pub async fn post_register(data: web::Data<AppData>, form: web::Form<RegisterForm>) -> HttpResponse {
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
        eprintln!("An error occurred: {:?}", conn_wrapped.err().unwrap());
        return HttpResponse::InternalServerError().finish();
    }

    //Lazy static so the regular expression only gets compiled once
    //Since compiling can take up to a couple milliseconds, we don't want it to happen on every request
    lazy_static! {
        static ref EMAIL_REGEX: Regex = Regex::new(r#"(([^<>()\[\]\\.,;:\s@"]+(\.[^<>()\[\]\\.,;:\s@"]+)*)|(".+"))@((\[[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}])|(([a-zA-Z\-0-9]+\.)+[a-zA-Z]{2,}))"#).unwrap();
    }
    let email_regex_captures = EMAIL_REGEX.captures(&email);

    if email_regex_captures.is_none() {
        let response = RegisterResponse { status: 400, message: Some("Invalid E-mail address.".to_string()), session_id: None, expiry: None };
        return HttpResponse::Ok().json(&response);
    }

    let mut conn = conn_wrapped.unwrap();

    let sql_check_email_wrapped = conn.exec::<Row, &str, Params>("SELECT COUNT(1) FROM users WHERE email = :email", params! {
        "email" => email.clone()
    });

    if sql_check_email_wrapped.is_err() {
        eprintln!("An error occurred: {:?}", sql_check_email_wrapped.err().unwrap());
        return HttpResponse::InternalServerError().finish();
    }

    if sql_check_email_wrapped.unwrap().len() != 0 {
        let response = RegisterResponse { status: 409, message: Some("Account already exists.".to_string()), session_id: None, expiry: None };
        return HttpResponse::Ok().json(response);
    }

    let salt: String = rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(16).map(char::from).collect();

    let mut hasher = Sha512Trunc256::new();
    hasher.update(&password);
    hasher.update(&salt);
    hasher.update(&data.environment.password_pepper);

    let password_hash = base64::encode(hasher.finalize());
    let password_bcrypt = bcrypt::hash_with_salt(&password_hash, 10, &salt.as_bytes()).unwrap();

    let password_finalized = password_bcrypt.format_for_version(bcrypt::Version::TwoY);

    let session_id: String = rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(64).map(char::from).collect();
    let user_id: String = rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(64).map(char::from).collect();

    let sql_insert_user = conn.exec::<usize, &str, Params>("INSERT INTO users (user_id, email, password, salt) VALUES (:user_id, :email, :password, :salt", params! {
        "user_id" => user_id.clone(),
        "email" => email,
        "password" => password_finalized,
        "salt" => salt
    });

    if sql_insert_user.is_err() {
        eprintln!("An error occurred: {:?}", sql_insert_user.err().unwrap());
        return HttpResponse::InternalServerError().finish();
    }

    let expiry = (chrono::Utc::now() + chrono::Duration::days(30)).timestamp();

    let sql_insert_session_id = conn.exec::<usize, &str, Params>("INSERT INTO sessions (session_id, user_id, expiry) VALUES (:session_id, :user_id, :expiry)", params! {
        "session_id" => session_id.clone(),
        "user_id" => user_id,
        "expiry" => expiry.clone()
    });

    if sql_insert_session_id.is_err() {
        eprintln!("An error occurred: {:?}", sql_insert_session_id.err().unwrap());
        return HttpResponse::InternalServerError().finish();
    }

    let response = RegisterResponse { status: 200, message: None, session_id: Some(session_id), expiry: Some(expiry)};
    return HttpResponse::Ok().json(&response);
}