use argh::FromArgs;

#[derive(FromArgs)]
/// Efficio's backend
pub struct Opt {
    /// database host
    #[argh(option, short = 'h')]
    pub db_host: Option<String>,
    /// database port
    #[argh(option, short = 'p')]
    pub db_port: Option<u32>,
}
