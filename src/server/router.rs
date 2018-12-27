use hyper::{Body, Method, Request, Response, StatusCode};
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
            (&Method::GET, "/setup/list") => {
                let print_func = |obj: (u32, &String, bool)| {
                    let row = json!({
                        "num":  obj.0,
                        "name":  obj.1,
                        "creatable":  obj.2
                    });
                    row.to_string()
                };

                let res = self
                    .setup
                    .list_salesforce_objects(print_func)
                    .unwrap()
                    .join(",");
                let body = Body::from(format!("[{}]", res));
                Response::builder()
                    .header("Content-Type", "application/json")
                    .body(body)
                    .unwrap()
            }
            (&Method::GET, "/setup/available") => {
                let print_func = |obj: (u32, &String, u32)| {
                    let row = json!({
                        "num":  obj.0,
                        "name":  obj.1,
                        "count":  obj.2
                    });
                    row.to_string()
                };
                let res = self.setup.list_db_objects(print_func)
                        .unwrap().join(",");
                let body = Body::from(format!("[{}]", res));
                Response::builder()
                    .header("Content-Type", "application/json")
                    .body(body)
                    .unwrap()
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
