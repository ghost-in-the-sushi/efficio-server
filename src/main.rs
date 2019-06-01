#[cfg(not(test))]
mod cli;
mod db;
#[cfg(not(test))]
mod endpoints;
mod error;
mod types;

#[cfg(not(test))]
fn main() -> error::Result<()> {
    use log::*;
    use structopt::StructOpt;

    pretty_env_logger::init_timed();

    info!("Starting Efficio…");
    let opt = cli::Opt::from_args();
    endpoints::routes::start_server(&opt)
}