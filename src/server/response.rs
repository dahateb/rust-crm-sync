use crossbeam_channel::Sender;
use futures::{future, Future, Stream};
use hyper::http::response::Builder;
use hyper::{Body, Response, StatusCode};
use std::collections::HashMap;
use sync::setup::Setup;
use url::form_urlencoded;

pub type BoxFut = Box<Future<Item = Response<Body>, Error = hyper::Error> + Send>;

pub static NOTFOUND: &[u8] = b"Not Found";
pub static INDEX: &[u8] = b"<h4>===> SYNC API <===</h4>";
pub static MISSING: &[u8] = b"Missing field";
pub static NOTNUMERIC: &[u8] = b"Number field is not numeric";
pub static OBJECT_EXIST: &[u8] = b"Object exists already";
pub static OBJECT_NOT_EXIST: &[u8] = b"Object doesn't exist";

pub fn default_response() -> Builder {
    let mut builder = Response::builder();
    builder.header("Access-Control-Allow-Origin", "*");
    builder
}

pub fn build_json_response(res: String) -> BoxFut {
    let body = Body::from(format!("[{}]", res));
    Box::new(future::ok(
        default_response()
            .header("Content-Type", "application/json")
            .body(body)
            .unwrap(),
    ))
}

pub fn response_unprocessable(body: &'static [u8]) -> Response<Body> {
    default_response()
        .status(StatusCode::UNPROCESSABLE_ENTITY)
        .body(body.into())
        .unwrap()
}

pub fn response_notify(
    body: Body,
    route: String,
    status: StatusCode,
    sender: Sender<(String, usize)>,
    setup: Setup,
) -> BoxFut {
    Box::new(body.concat2().map(move |chunk| {
        let params = form_urlencoded::parse(chunk.as_ref())
            .into_owned()
            .collect::<HashMap<String, String>>();
        let number = if let Some(n) = params.get("number") {
            if let Ok(v) = n.parse::<usize>() {
                v
            } else {
                return response_unprocessable(NOTNUMERIC);
            }
        } else {
            return response_unprocessable(MISSING);
        };
        let exists = setup.sf_object_exists(number);
        if exists && route == "/setup/new" {
            return response_unprocessable(OBJECT_EXIST);
        }
        let _ = sender.send((route, number));

        default_response().status(status).body("OK".into()).unwrap()
    }))
}
