use console::style;
use std::{env, io, process};
use std::io::Write;
use std::process::Command;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use reqwest::Client;

const SYSTEM_PROMPT: &str = include_str!("prompts/system_prompt.txt");

const MODEL_NAME: &str = "moonshotai/kimi-k2.5";
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize, Debug)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
    temperature: f32,
    max_tokens: usize,
}

#[derive(Deserialize, Debug)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize, Debug)]
struct Choice {
    message: Message,
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

fn add_llm_correction(command: &str, correction_message: &str, history: &mut Vec<Message>) {
    println!("{}", style(format!("LLM correction message: {}", correction_message)).yellow().dim());

    history.push(Message {
        role: "user".to_string(),
        content: format!("ERROR: {} command is invalid. {}", command, correction_message),
    });
}

fn get_git_status() -> String {
    let output = Command::new("git").arg("status").output();
    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        Ok(o) => {
            let error_msg = String::from_utf8_lossy(&o.stderr).trim().to_string();
            if error_msg.is_empty() { "Git command failed, no error message.".to_string() } else { error_msg }
        },
        Err(e) => format!("Critical Error: Could not execute 'git'. Details: {}", e),
    }
}

async fn get_llm_response(
    client: &Client,
    api_key: &str,
    user_input: &str,
    git_status: &str,
    history: &mut Vec<Message>,
) -> Result<String, Box<dyn std::error::Error>> {
    let system_msg = Message {
        role: "system".to_string(),
        content: format!("{}\n\nGIT STATUS:\n{}", SYSTEM_PROMPT, git_status),
    };

    if !user_input.trim().is_empty() {
        history.push(Message {
            role: "user".to_string(),
            content: user_input.to_string(),
        });
    }

    let mut request_messages = vec![system_msg];
    request_messages.extend(history.clone());

    let request_body = ChatRequest {
        model: MODEL_NAME.to_string(),
        messages: request_messages,
        stream: false,
        temperature: 0.3,
        max_tokens: 4096,
    };

    let res = client.post("https://integrate.api.nvidia.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;

    if !res.status().is_success() {
        let error_text = res.text().await?;
        return Err(format!("NVIDIA API Error: {}", error_text).into());
    }

    println!("{}", style("Thinking...").dim());

    let response_json: ChatResponse = res.json().await?;
    let raw_text = response_json.choices[0].message.content.clone();

    let cleaned_text = raw_text.replace("`", "").trim().to_string();

    history.push(Message {
        role: "assistant".to_string(),
        content: cleaned_text.clone(),
    });

    if history.len() > 20 {
        history.drain(0..2);
    }

    Ok(cleaned_text)
}

fn handle_execution(command: &str) -> Result<Option<(String, String, bool)>, Box<dyn std::error::Error>> {
    if command.contains("reset --hard") || command.contains("rm -rf") {
        return Ok(Some(("Do NOT try to execute any destructive commands".to_string(), "".to_string(), false)));
    }

    if command.contains("EXECUTE:") {
        return Ok(Some((
            "Each EXECUTE command must be on its own line. Format:\n".to_string() +
            "EXECUTE: <command>\n" +
            "...\n" +
            "EXECUTE: <command>", "".to_string(), false)));
    }

    println!("{}", style(format!("Executing command: {}", command)).dim());

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", command]).output()?
    } else {
        Command::new("sh").arg("-c").arg(command).output()?
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        println!("{}", style("✔ Success").green());
    } else {
        println!("{}", style("✖ Failed").red());
        if !stderr.is_empty() { println!("{}", style(&stderr).red()); }
    }

    Ok(Some((stdout, stderr, true)))
}

async fn repl_step(
    client: &Client,
    api_key: &str,
    history: &mut Vec<Message>
) -> Result<(), Box<dyn std::error::Error>> {
    let mut current_input = read_user_input();
    let git_status = get_git_status();
    let mut attempts: i8 = 0;

    println!("{}", style("Processing...").dim());

    loop {
        if attempts > 10 {
            println!("{}", style("ABORTING: Too many attempts").bold().red());
            break;
        }

        let response = get_llm_response(client, api_key, &current_input, &git_status, history).await?;

        println!("LLM RESPONSE: {}", response);

        current_input = String::new();

        if response.contains("FINAL:") && response.contains("EXECUTE:") {
            add_llm_correction(&response, "EXECUTE lines must contain ONLY the command. \
            Remove all explanations and commentary. Format: `EXECUTE: <command>`.", history);
        }

        if let Some((_, final_msg)) = response.split_once("FINAL:") {
            let clean_msg = final_msg.trim();
            if !clean_msg.is_empty() {
                println!("{}: {}", style("Jade").green().bold(), clean_msg);
            }
            break;
        }

        let mut executed_something = false;
        let mut feedback_buffer = String::new();

        for command in response.lines() {
            println!("COMMAND: {}", command);
            if let Some((_, command_cleaned)) = command.trim().split_once("EXECUTE:") {
                if !command_cleaned.is_empty() {
                    if let Some((output, error, executed_command)) = handle_execution(command_cleaned)? {
                        executed_something |= executed_command;
                        if !executed_command {
                            add_llm_correction(command_cleaned, &output, history);
                        } else {
                            feedback_buffer.push_str(&format!("Output of `{}`:\n{}\n", command_cleaned, output));
                            if !error.is_empty() {
                                feedback_buffer.push_str(&format!("ERROR: {}\n", error));
                            }
                        }
                    }
                }
            }
            else {
                add_llm_correction(command.trim(), "Command should start with `EXECUTE`.", history);
                continue;
            }
        }

        if executed_something {
            history.push(Message {
                role: "user".to_string(),
                content: feedback_buffer
            });
        }
        else {
            add_llm_correction(&response, "Command should start with either `FINAL:` or `EXECUTE`.", history);
        }

        attempts += 1;
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    print_welcome();
    let client = Client::new();

    dotenv().ok();
    let api_key = env::var("NVIDIA_API_KEY").expect("NVIDIA_API_KEY must be set in .env file");

    let mut history: Vec<Message> = Vec::new();

    loop {
        if let Err(e) = repl_step(&client, &api_key, &mut history).await {
            println!("{}", style(format!("Critical Error: {}", e)).red().bold());
        }
    }
}