use failure::{self, Fail};
use lazy_static::lazy_static;
use warp::{self, path, Filter, Rejection, Reply};

use crate::db;
use crate::endpoints::*;
use crate::error;
use crate::types::*;

const HEADER_AUTH: &str = "x-auth-token";
#[cfg(debug_assertions)]
const SERVER_ADDR: &str = "redis://127.0.0.1:6379/0";

#[cfg(not(debug_assertions))]
const SERVER_ADDR: &str = "redis://127.0.0.1:6379/8";

lazy_static! {
    static ref DB_CLIENT: redis::Client = db::get_client(SERVER_ADDR);
}

pub fn init_routes() {
    // POST /nuke
    let nuke = warp::path("nuke")
        .and(warp::path::end())
        .and_then(|| misc::nuke(&DB_CLIENT));

    // POST /user
    let create_user = warp::path("user")
        .and(warp::path::end())
        .and(warp::body::json())
        .and_then(|user: User| {
            user::create_user(&user, &DB_CLIENT)
                .and_then(|token| Ok(warp::reply::json(&token)))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // POST /login
    let login = warp::path("login")
        .and(warp::path::end())
        .and(warp::body::json())
        .and_then(move |auth_info: AuthInfo| {
            session::login(&auth_info, &DB_CLIENT)
                .and_then(|token| Ok(warp::reply::json(&token)))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // POST /logout
    let logout = warp::path("logout")
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and_then(move |auth| {
            session::logout(auth, &DB_CLIENT)
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // DELETE /user
    let delete_user = warp::path("user")
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and_then(move |auth| {
            user::delete_user(auth, &DB_CLIENT)
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // POST /store
    let create_store = warp::path("store")
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and_then(move |auth, data: NameData| {
            store::create_store(auth, &data, &DB_CLIENT)
                .and_then(|store_id| Ok(warp::reply::json(&store_id)))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // PUT /store/{id}
    let edit_store = path!("store" / u32)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and_then(move |id, auth, data: NameData| {
            store::edit_store(auth, id, &data, &DB_CLIENT)
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // POST /store/<id>/aisle
    let create_aisle = path!("store" / u32 / "aisle")
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and_then(move |store_id, auth, data: NameData| {
            aisle::create_aisle(auth, store_id, &data, &DB_CLIENT)
                .and_then(|aisle| Ok(warp::reply::json(&aisle)))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // PUT /aisle/<id>
    let edit_aisle = path!("aisle" / u32)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and_then(move |aisle_id, auth, data: NameData| {
            aisle::rename_aisle(auth, aisle_id, &data, &DB_CLIENT)
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // POST /aisle/<id>/product
    let create_product = path!("aisle" / u32 / "product")
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and_then(move |aisle_id, auth, data: NameData| {
            product::create_product(auth, aisle_id, &data, &DB_CLIENT)
                .and_then(|product| Ok(warp::reply::json(&product)))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // PUT /product/<id>
    let edit_product = path!("product" / u32)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and_then(move |product_id, auth, data: EditProduct| {
            product::edit_product(auth, product_id, &data, &DB_CLIENT)
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // GET /store
    let get_all_stores = warp::path("store")
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and_then(move |auth| {
            store::list_stores(auth, &DB_CLIENT)
                .and_then(|stores| Ok(warp::reply::json(&stores)))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // GET /store/<id>
    let list_store = path!("store" / u32)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and_then(move |store_id, auth| {
            store::list_store(auth, store_id, &DB_CLIENT)
                .and_then(|store| Ok(warp::reply::json(&store)))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // DELETE /product/<id>
    let delete_product = path!("product" / u32)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and_then(move |product_id, auth| {
            product::delete_product(auth, product_id, &DB_CLIENT)
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // DELETE /aisle/<id>
    let delete_aisle = path!("aisle" / u32)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and_then(move |aisle_id, auth| {
            aisle::delete_aisle(auth, aisle_id, &DB_CLIENT)
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // DELETE /store/<id>
    let delete_store = path!("store" / u32)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and_then(move |store_id, auth| {
            store::delete_store(auth, store_id, &DB_CLIENT)
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // PUT /sort_weight
    let change_sort_weight = warp::path("sort_weight")
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and_then(move |auth, data: EditWeight| {
            misc::change_sort_weight(auth, &data, &DB_CLIENT)
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    let post_routes = warp::post2()
        .and(
            create_product
                .or(create_aisle)
                .or(create_store)
                .or(login)
                .or(create_user)
                .or(logout)
                .or(nuke),
        )
        .recover(customize_error);

    let put_routes = warp::put2()
        .and(
            change_sort_weight
                .or(edit_product)
                .or(edit_aisle)
                .or(edit_store),
        )
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

    let routes = get_routes.or(post_routes).or(put_routes).or(del_routes);
    println!("Efficio's ready for requests...");
    warp::serve(routes).run(([127, 0, 0, 1], 3030));
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
