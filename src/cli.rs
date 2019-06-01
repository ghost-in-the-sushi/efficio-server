use structopt::StructOpt;

/// Efficio's backend
#[derive(StructOpt, Debug)]
pub struct Opt {
    /// Database host
    #[structopt(long = "db_host")]
    pub db_host: Option<String>,
    /// Database port
    #[structopt(long = "db_port")]
    pub db_port: Option<u32>,
}
