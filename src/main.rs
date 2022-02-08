use structopt::StructOpt;

use hektor::editor::*;

fn main() {
    Hektor::new(Options::from_args())
        .run();
}

