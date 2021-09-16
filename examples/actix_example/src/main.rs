// use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};

use actix_files as fs;
use actix_http::{body::Body, Response};
use actix_web::dev::ServiceResponse;
use actix_web::http::StatusCode;
use actix_web::middleware::errhandlers::{ErrorHandlerResponse, ErrorHandlers};
use actix_web::{
    delete, error, get, guard, http, middleware, post, web, App, Error, HttpRequest, HttpResponse,
    HttpServer, Result,
};
use listenfd::ListenFd;
use sea_orm::entity::*;
use sea_orm::query::*;
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use std::env;
use tera::Tera;

mod post;
pub use post::Entity as Post;
mod setup;

const DEFAULT_POSTS_PER_PAGE: usize = 25;

struct AppState {
    db_url: String,
    templates: tera::Tera,
}

#[derive(Debug, Deserialize)]
pub struct Params {
    page: Option<usize>,
    posts_per_page: Option<usize>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct FlashData {
    kind: String,
    message: String,
}

#[get("/")]
async fn list(
    req: HttpRequest,
    data: web::Data<AppState>,
    opt_flash: Option<actix_flash::Message<FlashData>>,
) -> Result<HttpResponse, Error> {
    let template = &data.templates;
    let conn = sea_orm::Database::connect(&data.db_url).await.unwrap();

    // get params
    let params = web::Query::<Params>::from_query(req.query_string()).unwrap();

    let page = params.page.unwrap_or(0);
    let posts_per_page = params.posts_per_page.unwrap_or(DEFAULT_POSTS_PER_PAGE);
    let paginator = Post::find().paginate(&conn, posts_per_page);
    let num_pages = paginator.num_pages().await.ok().unwrap();

    let mut flash_message = String::new();
    let mut flash_kind = String::new();

    if let Some(flash) = opt_flash {
        let flash_inner = flash.into_inner();
        flash_message = flash_inner.message.to_owned();
        flash_kind = flash_inner.kind.to_owned();
    }

    let posts = paginator
        .fetch_page(page)
        .await
        .expect("could not retrieve posts");
    let mut ctx = tera::Context::new();
    ctx.insert("posts", &posts);
    ctx.insert("page", &page);
    ctx.insert("posts_per_page", &posts_per_page);
    ctx.insert("num_pages", &num_pages);
    ctx.insert("flash_message", &flash_message);
    ctx.insert("flash_kind", &flash_kind);

    let body = template
        .render("index.html.tera", &ctx)
        .map_err(|_| error::ErrorInternalServerError("Template error"))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(body))
}

#[get("/new")]
async fn new(data: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let template = &data.templates;
    let ctx = tera::Context::new();
    let body = template
        .render("new.html.tera", &ctx)
        .map_err(|_| error::ErrorInternalServerError("Template error"))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(body))
}

#[post("/")]
async fn create(
    data: web::Data<AppState>,
    post_form: web::Form<post::Model>,
) -> actix_flash::Response<HttpResponse, FlashData> {
    let conn = sea_orm::Database::connect(&data.db_url).await.unwrap();

    let form = post_form.into_inner();

    post::ActiveModel {
        title: Set(form.title.to_owned()),
        text: Set(form.text.to_owned()),
        ..Default::default()
    }
    .save(&conn)
    .await
    .expect("could not insert post");

    let flash = FlashData {
        kind: "success".to_owned(),
        message: "Post successfully added.".to_owned(),
    };

    actix_flash::Response::with_redirect(flash.clone(), "/")
}

#[get("/{id}")]
async fn edit(data: web::Data<AppState>, id: web::Path<i32>) -> Result<HttpResponse, Error> {
    let conn = sea_orm::Database::connect(&data.db_url).await.unwrap();
    let template = &data.templates;

    let post: post::Model = Post::find_by_id(id.into_inner())
        .one(&conn)
        .await
        .expect("could not find post")
        .unwrap();

    let mut ctx = tera::Context::new();
    ctx.insert("post", &post);

    let body = template
        .render("edit.html.tera", &ctx)
        .map_err(|_| error::ErrorInternalServerError("Template error"))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(body))
}

#[post("/{id}")]
async fn update(
    data: web::Data<AppState>,
    id: web::Path<i32>,
    post_form: web::Form<post::Model>,
) -> actix_flash::Response<HttpResponse, FlashData> {
    let conn = sea_orm::Database::connect(&data.db_url).await.unwrap();
    let form = post_form.into_inner();

    post::ActiveModel {
        id: Set(Some(id.into_inner())),
        title: Set(form.title.to_owned()),
        text: Set(form.text.to_owned()),
    }
    .save(&conn)
    .await
    .expect("could not edit post");

    let flash = FlashData {
        kind: "success".to_owned(),
        message: "Post successfully updated.".to_owned(),
    };

    actix_flash::Response::with_redirect(flash.clone(), "/")
}

#[post("/delete/{id}")]
async fn delete(
    data: web::Data<AppState>,
    id: web::Path<i32>,
) -> actix_flash::Response<HttpResponse, FlashData> {
    let conn = sea_orm::Database::connect(&data.db_url).await.unwrap();

    let post: post::ActiveModel = Post::find_by_id(id.into_inner())
        .one(&conn)
        .await
        .unwrap()
        .unwrap()
        .into();

    post.delete(&conn).await.unwrap();

    let flash = FlashData {
        kind: "success".to_owned(),
        message: "Post successfully deleted.".to_owned(),
    };

    actix_flash::Response::with_redirect(flash.clone(), "/")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=debug");
    env_logger::init();

    // get env vars
    dotenv::dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
    let host = env::var("HOST").expect("HOST is not set in .env file");
    let port = env::var("PORT").expect("PORT is not set in .env file");
    let server_url = format!("{}:{}", host, port);

    // create post table if not exists
    let conn = sea_orm::Database::connect(&db_url).await.unwrap();
    let _ = setup::create_post_table(&conn).await;

    let mut listenfd = ListenFd::from_env();
    let mut server = HttpServer::new(move || {
        let templates = Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*")).unwrap();
        App::new()
            .data(AppState {
                db_url: db_url.to_owned(),
                templates: templates,
            })
            .wrap(middleware::Logger::default()) // enable logger
            .wrap(actix_flash::Flash::default())
            .configure(init)
            .service(fs::Files::new("/static", "./static").show_files_listing())
    });

    server = match listenfd.take_tcp_listener(0)? {
        Some(listener) => server.listen(listener)?,
        None => server.bind(&server_url)?,
    };

    println!("Starting server at {}", server_url);
    server.run().await?;

    Ok(())
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(list);
    cfg.service(new);
    cfg.service(create);
    cfg.service(edit);
    cfg.service(update);
    cfg.service(delete);
}
