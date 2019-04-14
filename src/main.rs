use failure::{self, Fail};
use warp::{self, http::StatusCode, path, Filter, Rejection, Reply};

mod aisle;
mod consts;
mod db;
mod error;
mod helpers;
mod product;
mod session;
mod store;
mod token;
mod types;
mod user;

const HEADER_AUTH: &str = "x-auth-token";

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
    // POST /nuke
    let nuke = warp::path("nuke")
        .and(warp::path::end())
        .and_then(|| nuke());

    // POST /user
    let create_user = warp::path("user")
        .and(warp::path::end())
        .and(warp::body::json())
        .and_then(|obj| {
            user::create_user(obj)
                .and_then(|token| Ok(warp::reply::json(&token)))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // POST /login
    let login = warp::path("login")
        .and(warp::path::end())
        .and(warp::body::json())
        .and_then(|obj| {
            session::login(obj)
                .and_then(|token| Ok(warp::reply::json(&token)))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // POST /logout
    let logout = warp::path("logout")
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and_then(|auth| {
            session::logout(auth)
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // DELETE /user
    let delete_user = warp::path("user")
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and_then(|auth| {
            user::delete_user(auth)
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // POST /store
    let create_store = warp::path("store")
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and_then(|auth, obj| {
            store::create_store(auth, obj)
                .and_then(|store_id| Ok(warp::reply::json(&store_id)))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // PUT /store/{id}
    let edit_store = path!("store" / u32)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and_then(|id, auth, obj| {
            store::edit_store(auth, id, obj)
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // POST /store/<id>/aisle
    let create_aisle = path!("store" / u32 / "aisle")
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and_then(|store_id, auth, obj| {
            aisle::create_aisle(auth, store_id, obj)
                .and_then(|aisle| Ok(warp::reply::json(&aisle)))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // PUT /aisle/<id>
    let edit_aisle = path!("aisle" / u32)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and_then(|aisle_id, auth, obj| {
            aisle::rename_aisle(auth, aisle_id, obj)
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // POST /aisle/<id>/product
    let create_product = path!("aisle" / u32 / "product")
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and_then(|aisle_id, auth, obj| {
            product::create_product(auth, aisle_id, obj)
                .and_then(|product| Ok(warp::reply::json(&product)))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // PUT /aisle/<id>
    let rename_product = path!("product" / u32)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and_then(|product_id, auth, obj| {
            product::rename_product(auth, product_id, obj)
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    let post_routes = warp::post2()
        .and(
            create_user
                .or(login)
                .or(logout)
                .or(create_store)
                .or(create_aisle)
                .or(create_product)
                .or(nuke),
        )
        .recover(customize_error);

    let put_routes = warp::put2()
        .and(edit_store.or(edit_aisle).or(rename_product))
        .recover(customize_error);

    let del_routes = warp::delete2().and(delete_user).recover(customize_error);

    println!("Efficio's ready for requests...");
    warp::serve(post_routes.or(put_routes).or(del_routes)).run(([127, 0, 0, 1], 3030));
}

fn customize_error(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(server_error) = err.find_cause::<failure::Compat<error::ServerError>>() {
        let json = warp::reply::json(&server_error.get_ref().clone());
        Ok(warp::reply::with_status(json, StatusCode::OK))
    } else {
        Err(err)
    }
}
