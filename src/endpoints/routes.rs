use std::convert::Infallible;
// use std::sync::Arc;

use log::*;
use r2d2_redis::RedisConnectionManager;
use warp::{self, path, Filter, Rejection, Reply};

use crate::cli::*;
use crate::endpoints::*;
use crate::error;
use crate::types::*;

const HEADER_AUTH: &str = "x-auth-token";
const DEFAULT_DB_PORT: u32 = 6379;
const DEFAULT_DB_HOST: &str = "redis://127.0.0.1";

type PooledConnection = r2d2::PooledConnection<r2d2_redis::RedisConnectionManager>;

pub async fn start_server(opt: &Opt) -> error::Result<()> {
    let db_host = match opt.db_host {
        Some(ref host) => host,
        _ => DEFAULT_DB_HOST,
    };
    let db_port = match opt.db_port {
        Some(port) => port,
        _ => DEFAULT_DB_PORT,
    };
    let db_num: u32 = if cfg!(debug_assertions) { 0 } else { 1 };
    let redis_addr = format!("{}:{}/{}", db_host, db_port, db_num);

    info!("DB address: {}", redis_addr);
    let manager = RedisConnectionManager::new(redis_addr.as_str())?;
    debug!("Creating db connection pool");
    let pool = r2d2::Pool::builder().max_size(15).build(manager)?;

    let get_connection = warp::any()
        .and_then(move || {
            let pool = pool.clone();
            async move {
                match pool.get() {
                    Ok(c) => Ok(c),
                    Err(e) => Err(warp::reject::custom(error::ServerError::from(e))),
                }
            }
        })
        .boxed();
    let get_connection = move || get_connection.clone();

    // POST /nuke
    let nuke = warp::path("nuke")
        .and(warp::path::end())
        .and(get_connection())
        .and_then(move |mut c: PooledConnection| async move { misc::nuke(&mut *c).await });

    // POST /user
    let create_user = warp::path("user")
        .and(warp::path::end())
        .and(warp::body::json())
        .and(get_connection())
        .and_then(move |user: User, mut c: PooledConnection| async move {
            user::create_user(&user, &mut *c)
                .await
                .and_then(|token| Ok(warp::reply::json(&token)))
                .or_else(|e| Err(warp::reject::custom(e)))
        });

    // POST /login
    let login = warp::path("login")
        .and(warp::path::end())
        .and(warp::body::json())
        .and(get_connection())
        .and_then(
            move |auth_info: AuthInfo, mut c: PooledConnection| async move {
                session::login(&auth_info, &mut *c)
                    .await
                    .and_then(|token| Ok(warp::reply::json(&token)))
                    .or_else(|e| Err(warp::reject::custom(e)))
            },
        );

    // POST /logout
    let logout = path!("logout" / String)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(get_connection())
        .and_then(
            move |id: String, auth: String, mut c: PooledConnection| async move {
                session::logout(&auth, &id, &mut *c)
                    .await
                    .and_then(|()| Ok(warp::reply()))
                    .or_else(|e| Err(warp::reject::custom(e)))
            },
        );

    // DELETE /user
    let delete_user = path!("user" / String)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(get_connection())
        .and_then(
            move |id: String, auth: String, mut c: PooledConnection| async move {
                user::delete_user(&auth, &id, &mut *c)
                    .await
                    .and_then(|()| Ok(warp::reply()))
                    .or_else(|e| Err(warp::reject::custom(e)))
            },
        );

    // POST /store
    let create_store = warp::path("store")
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and(get_connection())
        .and_then(
            move |auth, data: NameData, mut c: PooledConnection| async move {
                store::create_store(auth, &data, &mut *c)
                    .await
                    .and_then(|store_id| Ok(warp::reply::json(&store_id)))
                    .or_else(|e| Err(warp::reject::custom(e)))
            },
        );

    // PUT /store/{id}
    let edit_store = path!("store" / String)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and(get_connection())
        .and_then(
            move |id, auth, data: NameData, mut c: PooledConnection| async move {
                store::edit_store(auth, id, &data, &mut *c)
                    .await
                    .and_then(|()| Ok(warp::reply()))
                    .or_else(|e| Err(warp::reject::custom(e)))
            },
        );

    // POST /store/<id>/aisle
    let create_aisle = path!("store" / String / "aisle")
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and(get_connection())
        .and_then(
            move |store_id, auth, data: NameData, mut c: PooledConnection| async move {
                aisle::create_aisle(auth, store_id, &data, &mut *c)
                    .await
                    .and_then(|aisle| Ok(warp::reply::json(&aisle)))
                    .or_else(|e| Err(warp::reject::custom(e)))
            },
        );

    // PUT /aisle/<id>
    let edit_aisle = path!("aisle" / String)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and(get_connection())
        .and_then(
            move |aisle_id, auth, data: NameData, mut c: PooledConnection| async move {
                aisle::rename_aisle(auth, aisle_id, &data, &mut *c)
                    .await
                    .and_then(|()| Ok(warp::reply()))
                    .or_else(|e| Err(warp::reject::custom(e)))
            },
        );

    // POST /aisle/<id>/product
    let create_product = path!("aisle" / String / "product")
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and(get_connection())
        .and_then(
            move |aisle_id, auth, data: NameData, mut c: PooledConnection| async move {
                product::create_product(auth, aisle_id, &data, &mut *c)
                    .await
                    .and_then(|product| Ok(warp::reply::json(&product)))
                    .or_else(|e| Err(warp::reject::custom(e)))
            },
        );

    // PUT /product/<id>
    let edit_product = path!("product" / String)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and(get_connection())
        .and_then(
            move |product_id, auth, data: EditProduct, mut c: PooledConnection| async move {
                product::edit_product(auth, product_id, &data, &mut *c)
                    .await
                    .and_then(|()| Ok(warp::reply()))
                    .or_else(|e| Err(warp::reject::custom(e)))
            },
        );

    // GET /store
    let get_all_stores = warp::path("store")
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(get_connection())
        .and_then(move |auth, mut c: PooledConnection| async move {
            store::list_stores(auth, &mut *c)
                .await
                .and_then(|stores| Ok(warp::reply::json(&stores)))
                .or_else(|e| Err(warp::reject::custom(e)))
        });

    // GET /store/<id>
    let list_store = path!("store" / String)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(get_connection())
        .and_then(move |store_id, auth, mut c: PooledConnection| async move {
            store::list_store(auth, store_id, &mut *c)
                .await
                .and_then(|store| Ok(warp::reply::json(&store)))
                .or_else(|e| Err(warp::reject::custom(e)))
        });

    // DELETE /product/<id>
    let delete_product = path!("product" / String)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(get_connection())
        .and_then(
            move |product_id, auth, mut c: PooledConnection| async move {
                product::delete_product(auth, product_id, &mut *c)
                    .await
                    .and_then(|()| Ok(warp::reply()))
                    .or_else(|e| Err(warp::reject::custom(e)))
            },
        );

    // DELETE /aisle/<id>
    let delete_aisle = path!("aisle" / String)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(get_connection())
        .and_then(move |aisle_id, auth, mut c: PooledConnection| async move {
            aisle::delete_aisle(auth, aisle_id, &mut *c)
                .await
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e)))
        });

    // DELETE /store/<id>
    let delete_store = path!("store" / String)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(get_connection())
        .and_then(move |store_id, auth, mut c: PooledConnection| async move {
            store::delete_store(auth, store_id, &mut *c)
                .await
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e)))
        });

    // PUT /sort_weight
    let change_sort_weight = warp::path("sort_weight")
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and(get_connection())
        .and_then(
            move |auth, data: EditWeight, mut c: PooledConnection| async move {
                misc::change_sort_weight(auth, &data, &mut *c)
                    .await
                    .and_then(|()| Ok(warp::reply()))
                    .or_else(|e| Err(warp::reject::custom(e)))
            },
        );

    let post_routes = warp::post().and(
        create_product
            .or(create_aisle)
            .or(create_store)
            .or(login)
            .or(create_user)
            .or(logout)
            .or(nuke),
    );

    let put_routes = warp::put().and(
        change_sort_weight
            .or(edit_product)
            .or(edit_aisle)
            .or(edit_store),
    );

    let get_routes = warp::get().and(get_all_stores.or(list_store));

    let del_routes = warp::delete().and(
        delete_product
            .or(delete_aisle)
            .or(delete_store)
            .or(delete_user),
    );

    let routes = warp::path("api").and(get_routes
        .or(post_routes)
        .or(put_routes)
        .or(del_routes))
        .recover(customize_error);
    info!("Efficio's ready for requests...");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
    Ok(())
}

async fn customize_error(err: Rejection) -> Result<impl Reply, Infallible> {
    let (code, message) = match err.find::<error::ServerError>() {
        Some(server_error) => (server_error.status, server_error.msg.to_owned()),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "UNHANDLED REJECTION".to_string(),
        ),
    };
    Ok(warp::reply::with_status(message, code))
}
