use std::path::PathBuf;
use structopt::{clap::App, StructOpt};

#[derive(Clone, StructOpt, Debug)]
pub struct ChainXParams {
    #[structopt(long = "validator-name", value_name = "NAME")]
    /// registered validator name, when give the node `--key`, must provide matching validator's unique name
    pub validator_name: Option<String>,

    // This option is actually unused and only for the auto generated help, which could be refined later.
    #[structopt(long = "config", value_name = "CONFIG_JSON_PATH", parse(from_os_str))]
    /// pass [FLAGS] or [OPTIONS] via a JSON file, you can override them from the command line.
    pub config: Option<PathBuf>,
}

impl cli::AugmentClap for ChainXParams {
    fn augment_clap<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
        ChainXParams::augment_clap(app)
    }
}
