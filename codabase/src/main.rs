use clap::Parser;
use codabase::args::Args;

fn main() {
    let args = Args::parse();
    args.execute();
}
