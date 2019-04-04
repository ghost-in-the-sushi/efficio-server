use failure::{self, Fail};
use warp::{self, http::StatusCode, Filter, Rejection, Reply};

mod db;
mod error;
mod token;
mod user;

fn main() {
    // POST /user/
    let create_user =
        warp::path("user")
            .and(warp::body::json())
            .and_then(|obj| match user::create_user(obj) {
                Ok(token) => Ok(warp::reply::json(&token)),
                Err(e) => Err(warp::reject::custom(e.compat())),
            });

    let post_routes = warp::post2().and(create_user).recover(customize_error);

    println!("Efficio's ready for requests...");
    warp::serve(post_routes).run(([127, 0, 0, 1], 3030));
}

fn customize_error(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(server_error) = err.find_cause::<failure::Compat<error::ServerError>>() {
        let json = warp::reply::json(&server_error.get_ref().clone());
        Ok(warp::reply::with_status(json, StatusCode::OK))
    } else {
        Err(err)
    }
}
