use std::env;
use std::process::exit;
use vts::yosys::Netlist;

fn usage() {
    println!("usage: vts [JSON_NETLIST]");
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Some(filename) = args.iter().last() {
        match Netlist::from_file(filename) {
            Ok(net) => {
                println!("netlist OK");
                let pretty = serde_json::to_string_pretty(&net).unwrap();
                println!("{}", pretty);
            }
            Err(err) => {
                println!("netlist ERROR");
                println!("{}", err);
            }
        }
    } else {
        usage();
        exit(1);
    }
}
