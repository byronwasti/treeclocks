use clap::{Parser, Subcommand};
use treeclocks::{EventTree, IdTree};

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
    let args = Args::parse();

    match args.command {
        Command::Get {
            event_tree,
            id_tree,
        } => {
            println!("{}", event_tree.get(&id_tree));
        }
    }
}
