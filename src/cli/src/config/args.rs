#[derive(Debug, Clone, clap::Parser)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Values {
  /// Set log level to trace
  #[arg(short, long)]
  pub(crate) trace: bool,

  /// Set log level to debug
  #[arg(short, long)]
  pub(crate) debug: bool,

  /// Alternative configuration location
  #[arg(short, long)]
  pub(crate) config: Option<String>,
}

pub(crate) fn parse() -> Values {
  clap::Parser::parse()
}
