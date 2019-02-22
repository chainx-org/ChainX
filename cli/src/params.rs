

use cli::{AugmentClap, GetLogFilter};
use structopt::{StructOpt, clap::{arg_enum, _clap_count_exprs, App, AppSettings, SubCommand}};

#[derive(Clone, StructOpt, Debug)]
pub struct ChainXParams {
    #[structopt(long = "validator_name", value_name = "NAME")]
    /// registered validator name, when give the node `--key`, must provide matching validator's unique name
    pub validator_name: Option<String>,
}

impl AugmentClap for ChainXParams {
    fn augment_clap<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
        ChainXParams::augment_clap(app)
    }
}