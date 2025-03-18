use crate::request::full;
use bytes::Bytes;
use http::{Response, StatusCode};
use http_body_util::combinators::BoxBody;
use komrad_ast::prelude::{Number, Value};

/// Converts a final 4-element Komrad response to an HTTP Response:
/// Expected format: [status, headers, cookies, body]
pub fn build_hyper_response_from_komrad(terms: &[Value]) -> Response<BoxBody<Bytes, hyper::Error>> {
    if terms.is_empty() {
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header("Content-Type", "text/plain")
            .body(full("Empty response"))
            .unwrap();
    }

    match &terms[0] {
        Value::List(list_of_4) if list_of_4.len() == 4 => {
            let status_code = match &list_of_4[0] {
                Value::Number(n) => match n {
                    Number::Int(i) => *i as u16,
                    Number::UInt(i) => *i as u16,
                    Number::Float(f) => *f as u16,
                },
                _ => 200,
            };

            let mut builder = Response::builder().status(status_code);
            if let Value::List(header_list) = &list_of_4[1] {
                for hpair in header_list {
                    if let Value::List(pair) = hpair {
                        if pair.len() == 2 {
                            if let (Value::String(k), Value::String(v)) = (&pair[0], &pair[1]) {
                                builder = builder.header(k.as_str(), v.as_str());
                            }
                        }
                    }
                }
            }

            if let Value::List(cookie_list) = &list_of_4[2] {
                for cpair in cookie_list {
                    if let Value::List(pair) = cpair {
                        if pair.len() == 2 {
                            if let (Value::String(k), Value::String(v)) = (&pair[0], &pair[1]) {
                                builder = builder.header("Set-Cookie", format!("{}={}", k, v));
                            }
                        }
                    }
                }
            }

            let body_bytes = match &list_of_4[3] {
                Value::Bytes(b) => b.clone(),
                Value::String(s) => s.as_bytes().to_vec(),
                other => format!("{:?}", other).into_bytes(),
            };
            builder
                .body(full(Bytes::from(body_bytes)))
                .unwrap_or_else(|_| {
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(full("Error building response"))
                        .unwrap()
                })
        }
        other => {
            let text = match other {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                _ => format!("Unsupported response type: {:?}", other),
            };
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/plain")
                .body(full(Bytes::from(text)))
                .unwrap()
        }
    }
}
