use futures::{future, Future};
use hyper::{Body, Response, StatusCode};

pub type BoxFut = Box<Future<Item = Response<Body>, Error = hyper::Error> + Send>;

pub static NOTFOUND: &[u8] = b"Not Found";
pub static INDEX: &[u8] = b"<h4>===> SYNC API <===</h4>";
pub static MISSING: &[u8] = b"Missing field";
pub static NOTNUMERIC: &[u8] = b"Number field is not numeric";

pub fn build_json_response(res: String) -> BoxFut {
    let body = Body::from(format!("[{}]", res));
    Box::new(future::ok(
        Response::builder()
            .header("Content-Type", "application/json")
            .body(body)
            .unwrap(),
    ))
}

pub fn response_unprocessable(body: &'static [u8]) -> Response<Body> {
    Response::builder()
        .status(StatusCode::UNPROCESSABLE_ENTITY)
        .body(body.into())
        .unwrap()
}
