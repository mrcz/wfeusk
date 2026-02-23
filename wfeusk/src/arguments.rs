use clap::Parser;

#[derive(Parser)]
#[command(name = "Wordfeusk", about = "Scrabble computer player")]
pub(crate) struct Args {
    /// Filename of the wordlist to load
    #[arg(short, long, default_value = "dict-sv.txt")]
    pub wordlist: String,

    /// Set the letters in the rack
    #[arg(short, long)]
    pub rack: Option<String>,

    /// Prints basic statistics
    #[arg(short, long)]
    pub stats: bool,

    /// Prints debug info
    #[arg(short, long)]
    pub debug: bool,

    /// Sleeps for the given number of milliseconds after loading the dictionary
    #[arg(short = 'z', long)]
    pub sleep: Option<u64>,
}

pub(crate) fn get_arguments() -> Args {
    Args::parse()
}
