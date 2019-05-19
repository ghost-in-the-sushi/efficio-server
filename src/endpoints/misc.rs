use crate::db;
use crate::endpoints::INVALID_PARAMS;
use crate::error;
use crate::types::*;

#[cfg(not(test))]
use redis::Client;

#[cfg(test)]
use fake_redis::FakeCient as Client;

pub fn change_sort_weight(
    auth: String,
    data: &EditWeight,
    db_client: &Client,
) -> error::Result<()> {
    if !data.has_at_least_a_field() {
        Err(error::ServerError::new(
            INVALID_PARAMS,
            "At least a field must be present",
        ))
    } else {
        let auth = Auth(&auth);
        let c = db_client.get_connection()?;
        let mut pipe = redis::pipe();
        pipe.atomic();
        if let Some(ref aisles) = data.aisles {
            aisles
                .iter()
                .try_for_each(|w| db::aisles::edit_aisle_sort_weight(&c, &mut pipe, &auth, &w))?;
        }
        if let Some(ref products) = data.products {
            products.iter().try_for_each(|w| {
                db::products::edit_product_sort_weight(&c, &mut pipe, &auth, &w)
            })?;
        }
        pipe.query(&c)?;
        Ok(())
    }
}

// Reset the DB, only available in debug compilation
#[cfg(not(test))]
pub fn nuke(db_client: &Client) -> Result<impl warp::reply::Reply, warp::reject::Rejection> {
    if cfg!(debug_assertions) {
        let c = db_client.get_connection().expect("should have connection");
        let _: () = redis::cmd("FLUSHDB").query(&c).expect("error on flush");
        Ok(warp::reply())
    } else {
        Err(warp::reject::not_found())
    }
}

#[cfg(test)]
pub fn nuke(_: &Client) -> Result<impl warp::reply::Reply, warp::reject::Rejection> {
    if false {
        Ok(warp::reply())
    } else {
        Err(warp::reject::not_found())
    }
}
