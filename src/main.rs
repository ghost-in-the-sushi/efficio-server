mod db;
#[cfg(not(test))]
mod endpoints;
mod error;
mod types;

#[cfg(not(test))]
use endpoints::routes::init_routes;

#[cfg(not(test))]
fn main() {
    init_routes();
}
