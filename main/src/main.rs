use clap::Parser;
use std::error::Error;

use gamepie_app::Gamepie;

#[derive(clap::Parser)]
#[clap(name = "GamePIE")]
#[clap(author = "Alex Beharrell")]
#[clap(version = "0.0")]
#[clap(about = "Raspberry PI Emulator", long_about = "...")]
struct Context {
    /// Verbosity
    #[clap(short, long)]
    verbose: bool,
    /// Trace level verbosity
    #[clap(short, long)]
    trace: bool,
    /// System directory
    #[clap(short, long, default_value_t = String::from("./system"))]
    system: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Context::parse();
    let level = if args.verbose || args.trace {
        if args.trace {
            log::LevelFilter::Trace
        } else {
            log::LevelFilter::Debug
        }
    } else {
        log::LevelFilter::Info
    };
    simple_logger::SimpleLogger::new()
        .with_level(level)
        .env()
        .init()
        .unwrap();

    let gamepie = Gamepie::new(args.system.as_ref())?;

    gamepie.run()?;
    Ok(())
}
