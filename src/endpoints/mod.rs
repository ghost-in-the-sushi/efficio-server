use warp::http::StatusCode;

pub mod aisle;
pub mod misc;
pub mod product;
pub mod routes;
pub mod session;
pub mod store;
pub mod user;

const INVALID_PARAMS: StatusCode = StatusCode::PRECONDITION_FAILED;
