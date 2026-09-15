use clap::{Parser, Subcommand};
use treeclocks::{EventTree, IdTree, ItcMap};

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Get {
        #[arg(short, long)]
        event_tree: EventTree,

        #[arg(short, long)]
        id_tree: IdTree,
    },
}

fn main() {
    println!("Hello, world!");
}
