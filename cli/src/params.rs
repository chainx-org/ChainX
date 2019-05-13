use structopt::{clap::App, StructOpt};

#[derive(Clone, StructOpt, Debug)]
pub struct ChainXParams {
    #[structopt(long = "validator-name", value_name = "NAME")]
    /// registered validator name, when give the node `--key`, must provide matching validator's unique name
    pub validator_name: Option<String>,
}

impl cli::AugmentClap for ChainXParams {
    fn augment_clap<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
        ChainXParams::augment_clap(app)
    }
}
