use clap::{Parser, Subcommand};
use treeclocks::{IdTree, EventTree, ItcMap, ParseEventTreeError};

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
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
