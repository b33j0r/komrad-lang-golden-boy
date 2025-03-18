use bytes::Bytes;
use http::{HeaderMap, Request};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full};
use hyper::body::Body;
use komrad_ast::prelude::{Channel, Message, MessageBuilder, Value};

pub struct KomradRequest {
    pub method: String,
    pub path: Vec<String>,
    pub headers: HeaderMap,
    pub body: bytes::Bytes,
    pub delegate: Option<Channel>,
}

impl KomradRequest {
    pub async fn from_request(req: Request<impl Body>) -> Self {
        let method = req.method().to_string();
        let path = req.uri().path().to_string();
        let headers = req.headers().clone();
        let body = req
            .into_body()
            .collect()
            .await
            .map(|collected| collected.to_bytes())
            .unwrap_or_else(|_| Bytes::new());

        KomradRequest {
            method,
            path: path
                .split('/')
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            headers,
            body,
            delegate: None,
        }
    }

    pub fn with_delegate(mut self, delegate: Channel) -> Self {
        self.delegate = Some(delegate);
        self
    }
}

impl Into<Message> for KomradRequest {
    fn into(self) -> Message {
        Message::default()
            .with_terms(vec![
                Value::Word("http".to_string()),
                Value::Word(self.method),
            ])
            .with_terms(self.path.into_iter().map(|s| Value::String(s)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::{Method, Request};
    use http_body_util::Full;
    use hyper::header::{HeaderName, HeaderValue};

    #[tokio::test]
    async fn test_komrad_request() {
        let req = Request::builder()
            .method(Method::GET)
            .uri("http://localhost:3000/test/path")
            .header(
                HeaderName::from_static("x-test"),
                HeaderValue::from_static("value"),
            )
            .body(Full::new(Bytes::new()))
            .unwrap();

        let komrad_req = KomradRequest::from_request(req).await;

        assert_eq!(komrad_req.method, "GET");
        assert_eq!(komrad_req.path, vec!["test", "path"]);
        assert_eq!(komrad_req.headers.get("x-test").unwrap(), "value");
    }

    #[tokio::test]
    async fn test_komrad_request_to_message() {
        let req = Request::builder()
            .method(Method::POST)
            .uri("http://localhost:3000/test/path")
            .header(
                HeaderName::from_static("x-test"),
                HeaderValue::from_static("value"),
            )
            .body(Full::new(Bytes::new()))
            .unwrap();

        let komrad_req = KomradRequest::from_request(req).await;
        let message: Message = komrad_req.into();

        assert_eq!(message.first_word(), Some("http".to_string()));
        assert_eq!(
            message.rest(),
            vec![
                Value::Word("POST".to_string()),
                Value::String("test".to_string()),
                Value::String("path".to_string()),
            ]
        );
    }
}

// We create some utility functions to make Empty and Full bodies
// fit our broadened Response body type.
pub fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

pub fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}
