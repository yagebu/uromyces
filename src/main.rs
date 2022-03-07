use std::env;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let filename = args.get(1);

    let path = if let Some(path) = filename {
        path
    } else {
        "tests/ledgers/example.beancount"
    };

    let absolute_path = env::current_dir()?.join(path);
    let ledger = uromyces::load(&absolute_path.try_into().unwrap());

    // Log some infos.
    println!(
        "\nLoaded Beancount ledger ({} entries; {} errors): {}",
        ledger.entries.len(),
        ledger.errors.len(),
        ledger.filename,
    );

    if !ledger.errors.is_empty() {
        println!("{:?}", ledger.errors);
    }
    Ok(())
}
