use warp::Filter;

fn build_routes() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let index = warp::path!("").and(warp::get()).map(|| "TEST");
    index
}
