use failure::{self, Fail};
use log::*;

use crate::cli::*;
use crate::endpoints::*;
use crate::error;
use crate::types::*;
use r2d2_redis::RedisConnectionManager;
use warp::{self, path, Filter, Rejection, Reply};

const HEADER_AUTH: &str = "x-auth-token";
const DEFAULT_DB_PORT: u32 = 6379;
const DEFAULT_DB_HOST: &str = "redis://127.0.0.1";

type PooledConnection = r2d2::PooledConnection<r2d2_redis::RedisConnectionManager>;

pub fn start_server(opt: &Opt) -> error::Result<()> {
    let db_host = if let Some(ref host) = opt.db_host {
        host
    } else {
        DEFAULT_DB_HOST
    };
    let db_port = if let Some(port) = opt.db_port {
        port
    } else {
        DEFAULT_DB_PORT
    };
    let db_num = if cfg!(debug_assertions) { 0 } else { 1 };
    let redis_addr = format!("{}:{}/{}", db_host, db_port, db_num);

    info!("DB address: {}", redis_addr);
    let manager = RedisConnectionManager::new(redis_addr.as_str())?;
    debug!("Creating db connection pool");
    let pool = r2d2::Pool::builder().max_size(15).build(manager)?;

    let get_connection = warp::any()
        .and_then(move || match pool.get() {
            Ok(c) => Ok(c),
            Err(e) => Err(warp::reject::custom(e)),
        })
        .boxed();
    let get_connection = move || get_connection.clone();

    // POST /nuke
    let nuke = warp::path("nuke")
        .and(warp::path::end())
        .and(get_connection())
        .and_then(move |c: PooledConnection| misc::nuke(&*c));

    // POST /user
    let create_user = warp::path("user")
        .and(warp::path::end())
        .and(warp::body::json())
        .and(get_connection())
        .and_then(move |user: User, c: PooledConnection| {
            user::create_user(&user, &*c)
                .and_then(|token| Ok(warp::reply::json(&token)))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // POST /login
    let login = warp::path("login")
        .and(warp::path::end())
        .and(warp::body::json())
        .and(get_connection())
        .and_then(move |auth_info: AuthInfo, c: PooledConnection| {
            session::login(&auth_info, &*c)
                .and_then(|token| Ok(warp::reply::json(&token)))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // POST /logout
    let logout = path!("logout" / String)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(get_connection())
        .and_then(move |id: String, auth: String, c: PooledConnection| {
            session::logout(&auth, &id, &*c)
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // DELETE /user
    let delete_user = path!("user" / String)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(get_connection())
        .and_then(move |id: String, auth: String, c: PooledConnection| {
            user::delete_user(&auth, &id, &*c)
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // POST /store
    let create_store = warp::path("store")
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and(get_connection())
        .and_then(move |auth, data: NameData, c: PooledConnection| {
            store::create_store(auth, &data, &*c)
                .and_then(|store_id| Ok(warp::reply::json(&store_id)))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // PUT /store/{id}
    let edit_store = path!("store" / String)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and(get_connection())
        .and_then(move |id, auth, data: NameData, c: PooledConnection| {
            store::edit_store(auth, id, &data, &*c)
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // POST /store/<id>/aisle
    let create_aisle = path!("store" / String / "aisle")
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and(get_connection())
        .and_then(move |store_id, auth, data: NameData, c: PooledConnection| {
            aisle::create_aisle(auth, store_id, &data, &*c)
                .and_then(|aisle| Ok(warp::reply::json(&aisle)))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // PUT /aisle/<id>
    let edit_aisle = path!("aisle" / String)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and(get_connection())
        .and_then(move |aisle_id, auth, data: NameData, c: PooledConnection| {
            aisle::rename_aisle(auth, aisle_id, &data, &*c)
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // POST /aisle/<id>/product
    let create_product = path!("aisle" / String / "product")
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and(get_connection())
        .and_then(move |aisle_id, auth, data: NameData, c: PooledConnection| {
            product::create_product(auth, aisle_id, &data, &*c)
                .and_then(|product| Ok(warp::reply::json(&product)))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // PUT /product/<id>
    let edit_product = path!("product" / String)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and(get_connection())
        .and_then(
            move |product_id, auth, data: EditProduct, c: PooledConnection| {
                product::edit_product(auth, product_id, &data, &*c)
                    .and_then(|()| Ok(warp::reply()))
                    .or_else(|e| Err(warp::reject::custom(e.compat())))
            },
        );

    // GET /store
    let get_all_stores = warp::path("store")
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(get_connection())
        .and_then(move |auth, c: PooledConnection| {
            store::list_stores(auth, &*c)
                .and_then(|stores| Ok(warp::reply::json(&stores)))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // GET /store/<id>
    let list_store = path!("store" / String)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(get_connection())
        .and_then(move |store_id, auth, c: PooledConnection| {
            store::list_store(auth, store_id, &*c)
                .and_then(|store| Ok(warp::reply::json(&store)))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // DELETE /product/<id>
    let delete_product = path!("product" / String)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(get_connection())
        .and_then(move |product_id, auth, c: PooledConnection| {
            product::delete_product(auth, product_id, &*c)
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // DELETE /aisle/<id>
    let delete_aisle = path!("aisle" / String)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(get_connection())
        .and_then(move |aisle_id, auth, c: PooledConnection| {
            aisle::delete_aisle(auth, aisle_id, &*c)
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // DELETE /store/<id>
    let delete_store = path!("store" / String)
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(get_connection())
        .and_then(move |store_id, auth, c: PooledConnection| {
            store::delete_store(auth, store_id, &*c)
                .and_then(|()| Ok(warp::reply()))
                .or_else(|e| Err(warp::reject::custom(e.compat())))
        });

    // PUT /sort_weight
    let change_sort_weight = warp::path("sort_weight")
        .and(warp::path::end())
        .and(warp::header::<String>(HEADER_AUTH))
        .and(warp::body::json())
        .and(get_connection())
        .and_then(move |auth, data: EditWeight, c: PooledConnection| {
            misc::change_sort_weight(auth, &data, &*c)
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
    info!("Efficio's ready for requests...");
    warp::serve(routes).run(([127, 0, 0, 1], 3030));
    Ok(())
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
