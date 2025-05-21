use clap::Parser;
use codas_kit::args::Args;

fn main() {
    let args = Args::parse();
    args.execute();
}
