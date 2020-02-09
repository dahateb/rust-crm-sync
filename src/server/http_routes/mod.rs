use crate::server::response;
use std::str;
use warp::Filter;

pub fn build_routes() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let index = warp::get()
        .and(warp::path::end())
        .map(|| str::from_utf8(response::INDEX).unwrap());
    index
}
