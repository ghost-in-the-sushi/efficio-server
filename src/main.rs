use std::collections::HashMap;

use failure::{self, Fail};
use warp::{self, path, Filter, Rejection, Reply};

mod aisle;
mod consts;
mod db;
mod error;
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

fn change_sort_weight(auth: String, obj: HashMap<String, String>) -> error::Result<()> {
    Ok(())
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
        .and_then(|user: user::User| {
            user::create_user(&user)
                .and_then(|token| Ok(warp::reply::json(&token)))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // POST /login
    let login = warp::path("login")
        .and(warp::path::end())
        .and(warp::body::json())
        .and_then(|auth_info: session::AuthInfo| {
            session::login(&auth_info)
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
        .and_then(|auth, data: types::NameData| {
            store::create_store(auth, &data)
                .and_then(|store_id| Ok(warp::reply::json(&store_id)))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // PUT /store/{id}
    let edit_store = path!("store" / u32)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and_then(|id, auth, data: types::NameData| {
            store::edit_store(auth, id, &data)
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // POST /store/<id>/aisle
    let create_aisle = path!("store" / u32 / "aisle")
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and_then(|store_id, auth, data: types::NameData| {
            aisle::create_aisle(auth, store_id, &data)
                .and_then(|aisle| Ok(warp::reply::json(&aisle)))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // PUT /aisle/<id>
    let edit_aisle = path!("aisle" / u32)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and_then(|aisle_id, auth, data: types::NameData| {
            aisle::rename_aisle(auth, aisle_id, &data)
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // POST /aisle/<id>/product
    let create_product = path!("aisle" / u32 / "product")
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and_then(|aisle_id, auth, data: types::NameData| {
            product::create_product(auth, aisle_id, &data)
                .and_then(|product| Ok(warp::reply::json(&product)))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // PUT /product/<id>
    let edit_product = path!("product" / u32)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and_then(|product_id, auth, data: db::products::EditProduct| {
            product::edit_product(auth, product_id, &data)
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // GET /store
    let get_all_stores = warp::path("store")
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and_then(|auth| {
            store::list_stores(auth)
                .and_then(|stores| Ok(warp::reply::json(&stores)))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // GET /store/<id>
    let list_store = path!("store" / u32)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and_then(|store_id, auth| {
            store::list_store(auth, store_id)
                .and_then(|store| Ok(warp::reply::json(&store)))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // DELETE /product/<id>
    let delete_product = path!("product" / u32)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and_then(|product_id, auth| {
            product::delete_product(auth, product_id)
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // DELETE /aisle/<id>
    let delete_aisle = path!("aisle" / u32)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and_then(|aisle_id, auth| {
            aisle::delete_aisle(auth, aisle_id)
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // DELETE /store/<id>
    let delete_store = path!("store" / u32)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and_then(|store_id, auth| {
            store::delete_store(auth, store_id)
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
        .and(edit_store.or(edit_aisle).or(edit_product))
        .recover(customize_error);

    let get_routes = warp::get2()
        .and(get_all_stores.or(list_store))
        .recover(customize_error);

    let del_routes = warp::delete2()
        .and(
            delete_product
                .or(delete_aisle)
                .or(delete_store)
                .or(delete_user),
        )
        .recover(customize_error);

    println!("Efficio's ready for requests...");
    warp::serve(get_routes.or(post_routes).or(put_routes).or(del_routes))
        .run(([127, 0, 0, 1], 3030));
}

fn customize_error(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(server_error) = err.find_cause::<failure::Compat<error::ServerError>>() {
        let server_error = server_error.get_ref();
        Ok(warp::reply::with_status(
            server_error.msg.to_owned(),
            server_error.status,
        ))
    } else {
        Err(err)
    }
}
