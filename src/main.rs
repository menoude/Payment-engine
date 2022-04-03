use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    // This is an optional argument that can be written in short (-d)
    // or long form (--debug)
    #[clap(short, long)]
    debug: bool,
    file_path: std::path::PathBuf,
}

fn main() {
    let args = Args::parse();
    let file = std::fs::File::open(args.file_path).expect("Cannot open file for this path");
    let mut accounts = payment_engine::ClientAccounts::new();
    let mut operations_register = payment_engine::MoneyOperationsRegister::new();
    payment_engine::read_transactions_file(
        file,
        &mut accounts,
        &mut operations_register,
        args.debug,
    );
    accounts
        .print_to(&mut std::io::stdout())
        .expect("Failed to print the account summary");
}
