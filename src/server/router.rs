use hyper::{Body, Request, Response, StatusCode, Method};
use sync::setup::Setup;

static NOTFOUND: &[u8] = b"Not Found";
static INDEX: &[u8] = b"<h4>===> SYNC API <===</h4>";

pub struct Router {
    pub setup: Setup,
}

impl Router {
    pub fn handle(&self, req: Request<Body>) -> Response<Body> {
        match (req.method(), req.uri().path()) {
            (&Method::GET, "/") | (&Method::GET, "/index.html") => {
                let body = Body::from(INDEX);
                Response::new(body)
            }
            _ => {
                // Return 404 not found response.
                let body = Body::from(NOTFOUND);
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(body)
                    .unwrap()
            }
        }
    }
}
