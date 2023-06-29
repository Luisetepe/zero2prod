use actix_web::{HttpResponse, web};

#[derive(serde::Deserialize)]
pub struct SubscribeRequest {
    name: String,
    email: String,
}

pub async fn subscribe(form: web::Form<SubscribeRequest>) -> HttpResponse {
    HttpResponse::Ok().finish()
}