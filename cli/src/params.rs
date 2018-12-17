
use structopt::StructOpt;
use cli::CoreParams;

/// Extend params for Node
#[derive(Debug, StructOpt)]
pub struct Params {
    /// Should run as a GRANDPA authority node
    #[structopt(long = "grandpa-authority", help = "Run Node as a GRANDPA authority, implies --validator")]
    grandpa_authority: bool,

    /// Should run as a GRANDPA authority node only
    #[structopt(long = "grandpa-authority-only", help = "Run Node as a GRANDPA authority only, don't as a usual validator, implies --grandpa-authority")]
    grandpa_authority_only: bool,

    #[structopt(flatten)]
    core: CoreParams
}
