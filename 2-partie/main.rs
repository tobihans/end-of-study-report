mod webrtc;
mod websocket;

use actix::Actor;
use actix_web::{middleware::Logger, web, App, HttpServer};
use dotenv::dotenv;
use mediasoup::prelude::*;
use std::{env, net};
use tracing::{info, Level};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt()
        .compact()
        .with_max_level(Level::INFO)
        .init();

    let address = env::var("ADDRESS")
        .expect("ADDRESS env var is not set")
        .parse::<net::SocketAddr>()
        .expect("ADDRESS is invalid");

    // Application State
    let redis_url = env::var("REDIS_URL")
        .expect("REDIS_URL env var is not set");
    let redis = redis::Client::open(redis_url)
        .expect("Unable to connect to Redis");

    let server = websocket::Server::new(redis.clone()).start();
    let worker_manager = WorkerManager::new();
    let registry = webrtc::registry::Registry::default();
    let transport_ips = {
        let ip = env::var("MEDIASOUP_IP")
            .expect("MEDIASOUP_IP is not set")
            .parse::<net::IpAddr>()
            .expect("MEDIASOUP_IP is invalid");
        let announced_ip = env::var("MEDIASOUP_ANNOUNCED_IP").ok().map(|ip| {
            ip.parse::<net::IpAddr>()
                .expect("MEDIASOUP_ANNOUNCED_IP is invalid")
        });

        (ip, announced_ip)
    };

    info!("Server is starting on address: {}", address);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(server.clone()))
            .app_data(web::Data::new(worker_manager.clone()))
            .app_data(web::Data::new(registry.clone()))
            .app_data(web::Data::new(redis.clone()))
            .app_data(web::Data::new(transport_ips.clone()))
            .wrap(Logger::default())
            .service(web::scope("/ws").service(websocket::room))
    })
    .bind(address)?
    .run()
    .await
}
