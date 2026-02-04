use console::style;
use std::{io, process};
use std::io::Write;

fn read_user_input() -> String {
    let mut user_input = String::new();

    print!("> ");
    io::stdout().flush().unwrap();

    io::stdin()
        .read_line(&mut user_input)
        .expect("Failed to read line");

    let user_input: String = user_input.trim().to_string();

    if user_input == "quit" {
        process::exit(0);
    }

    return user_input.trim().to_string();
}

fn print_welcome() {
    println!("{}", style("╭──────────────────────────────────────────────────────────────────╮").dim());

    println!("{}                                                                  {}", style("│").dim(), style("│").dim());

    println!(
        "{}                     {}                     {}",
        style("│").dim(),
        style(r#"       __          __   "#).bold().green(),
        style("│").dim()
    );
    println!(
        "{}                     {}                     {}",
        style("│").dim(),
        style(r#"      / /___ _____/ /__ "#).bold().green(),
        style("│").dim()
    );
    println!(
        "{}                     {}                     {}",
        style("│").dim(),
        style(r#" __  / / __ `/ __  / _ \"#).bold().green(),
        style("│").dim()
    );
    println!(
        "{}                     {}                     {}",
        style("│").dim(),
        style(r#"/ /_/ / /_/ / /_/ /  __/"#).bold().green(),
        style("│").dim()
    );
    println!(
        "{}                     {}                     {}",
        style("│").dim(),
        style(r#"\____/\__,_/\__,_/\___/ "#).bold().green(),
        style("│").dim()
    );

    println!("{}                                                                  {}", style("│").dim(), style("│").dim());

    println!(
        "{}                         {}                         {}",
        style("│").dim(),
        style("AI Git Companion").white(),
        style("│").dim()
    );

    println!("{}                                                                  {}", style("│").dim(), style("│").dim());

    println!("{}", style("╰──────────────────────────────────────────────────────────────────╯").dim());
}

fn main() {
    print_welcome();

    loop {
        let user_input: String = read_user_input();
    }
}
