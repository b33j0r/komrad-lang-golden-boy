use bytes::Bytes;
use http::StatusCode;
use http_body_util::combinators::BoxBody;
use hyper::body::Body;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use komrad_web::request;
use komrad_web::request::KomradRequest;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::select;
use tokio_util::sync::CancellationToken;

async fn echo(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let komrad_req = KomradRequest::from_request(req).await;
    let path = komrad_req.path.join("/");

    match komrad_req.method.as_str() {
        "GET" => {
            // Check for a delegate channel
            if komrad_req.delegate.is_none() {
                return Ok(Response::new(request::full("No delegate channel found")));
            }
            Ok(Response::new(request::full(format!("Hello, {}!", path))))
        }
        "POST" => {
            //
            Ok(Response::new(request::full(komrad_req.body)))
        }
        // Return 404 Not Found for other routes.
        _ => {
            let mut not_found = Response::new(request::empty());
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let graceful = CancellationToken::new();
    // We create a TcpListener and bind it to 127.0.0.1:3000
    let listener = TcpListener::bind(addr).await?;

    // We start a loop to continuously accept incoming connections
    loop {
        select! {
            accept_result = listener.accept() => {
                if let Ok((stream, _)) = accept_result {
                    // Use an adapter to access something implementing `tokio::io` traits as if they implement
                    // `hyper::rt` IO traits.
                    let io = TokioIo::new(stream);

                    // Spawn a tokio task to serve multiple connections concurrently
                    tokio::task::spawn(async move {
                        // Finally, we bind the incoming connection to our `hello` service
                        if let Err(err) = http1::Builder::new()
                            // `service_fn` converts our function in a `Service`
                            .serve_connection(io, service_fn(echo))
                            .await
                        {
                            eprintln!("Error serving connection: {:?}", err);
                        }
                    });
                } else {
                    eprintln!("Failed to accept connection: {:?}", accept_result);
                }
            }
            // Catch ctrl-c to trigger graceful shutdown
            _ = tokio::signal::ctrl_c() => {
                println!("Ctrl-C received, shutting down...");
                graceful.cancel();
            }
            // If the graceful shutdown token is triggered, we break the loop
            _ = graceful.cancelled() => {
                println!("Shutting down gracefully...");
                break;
            }
        }
    }
    Ok(())
}
