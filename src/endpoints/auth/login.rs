use actix_web::{post, HttpResponse, web};
use mysql::prelude::Queryable;
use mysql::{Row, Params, params};
use crate::appdata::AppData;

struct LoginForm {
    username_base64: String,
    password_base64: String
}

#[post("/auth/login")]
pub async fn post_login(data: web::Data<AppData>, form: web::Form<LoginForm>) -> HttpResponse {

}