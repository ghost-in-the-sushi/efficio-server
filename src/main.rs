use failure::{self, Fail};
use warp::{self, http::StatusCode, Filter, Rejection, Reply};

mod db;
mod error;
mod token;
mod user;

// Reset the DB, only available in debug compilation
fn nuke() -> Result<impl warp::reply::Reply, warp::reject::Rejection> {
    if cfg!(debug_assertions) {
        let c = db::get_connection().expect("should have connection");
        let _: () = redis::cmd("FLUSHDB").query(&c).expect("error on flush");
        Ok(warp::reply())
    } else {
        Err(warp::reject::not_found())
    }
}

fn main() {
    // GET /nuke
    let nuke = warp::path("nuke").and_then(|| nuke());

    // POST /user
    let create_user =
        warp::path("user")
            .and(warp::body::json())
            .and_then(|obj| match user::create_user(obj) {
                Ok(token) => Ok(warp::reply::json(&token)),
                Err(e) => Err(warp::reject::custom(e.compat())),
            });

    let post_routes = warp::post2().and(create_user).recover(customize_error);
    let get_routes = warp::get2().and(nuke);

    println!("Efficio's ready for requests...");
    warp::serve(post_routes.or(get_routes)).run(([127, 0, 0, 1], 3030));
}

fn customize_error(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(server_error) = err.find_cause::<failure::Compat<error::ServerError>>() {
        let json = warp::reply::json(&server_error.get_ref().clone());
        Ok(warp::reply::with_status(json, StatusCode::OK))
    } else {
        Err(err)
    }
}
