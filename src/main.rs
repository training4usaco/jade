use console::style;
use std::{io, process, thread};
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;
use serde::{Deserialize, Serialize};
use reqwest::Client;

const SYSTEM_PROMPT: &str = include_str!("prompts/system_prompt.txt");
const MODEL_NAME: &str = "gpt-oss:20b-cloud";

#[derive(Serialize, Deserialize, Debug, Clone)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Serialize, Debug)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    options: Option<OllamaOptions>,
}

#[derive(Serialize, Debug)]
struct OllamaOptions {
    num_ctx: usize,
    temperature: f32,
}

#[derive(Deserialize, Debug)]
struct OllamaResponse {
    message: OllamaMessage,
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

fn get_git_status() -> String {
    let output = Command::new("git").arg("status").output();

    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),

        Ok(o) => {
            let error_msg = String::from_utf8_lossy(&o.stderr).trim().to_string();
            if error_msg.is_empty() {
                "Git command failed, but returned no error message.".to_string()
            } else {
                error_msg
            }
        },

        Err(e) => format!("Critical Error: Could not execute 'git'. Is it installed? Details: {}", e),
    }
}

fn remove_think_tags(text: &str) -> String {
    let mut clean = String::new();
    let mut remainder = text;
    while let Some(start) = remainder.find("<think>") {
        clean.push_str(&remainder[..start]);
        let after_start = &remainder[start + 7..];
        if let Some(end_offset) = after_start.find("</think>") {
            remainder = &after_start[end_offset + 8..];
        } else {
            return clean;
        }
    }
    clean.push_str(remainder);
    clean
}

async fn get_llm_response(
    client: &Client,
    user_input: &str,
    git_status: &str,
    history: &mut Vec<OllamaMessage>,
) -> Result<String, Box<dyn std::error::Error>> {

    let system_msg = OllamaMessage {
        role: "system".to_string(),
        content: format!("{}\n\nGIT STATUS:\n{}", SYSTEM_PROMPT, git_status),
    };

    history.push(OllamaMessage {
        role: "user".to_string(),
        content: user_input.to_string(),
    });

    let mut request_messages = vec![system_msg];
    request_messages.extend(history.clone());

    let request_body = OllamaRequest {
        model: MODEL_NAME.to_string(),
        messages: request_messages,
        stream: false,
        options: Some(OllamaOptions { num_ctx: 32000, temperature: 0.1 }),
    };

    let url = "http://localhost:11434/api/chat";

    let res = client.post(url)
        .json(&request_body)
        .send()
        .await?;

    if !res.status().is_success() {
        let error_text = res.text().await?;
        return Err(format!("Ollama API Error: {}. (Did you run 'ollama signin'?)", error_text).into());
    }

    let response_json: OllamaResponse = res.json().await?;

    let response_text = response_json.message.content;

    let without_thinking = remove_think_tags(&response_text);
    let cleaned_text = without_thinking.replace("`", "").trim().to_string();

    history.push(OllamaMessage {
        role: "assistant".to_string(),
        content: cleaned_text.clone(),
    });

    if history.len() > 20 {
        history.drain(0..2);
    }

    Ok(cleaned_text)
}

fn handle_execution(command: &str) -> Result<Option<(String, String)>, Box<dyn std::error::Error>> {
    // println!("{} {}", style("Suggestion:").bold().blue(), command);

    if command.contains("reset --hard") || command.contains("rm -rf") {
        println!("{}", style("ABORTING: Destructive command detected.").bold().red());
        return Ok(None);
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

    Ok(Some((stdout, stderr)))
}

async fn repl_step(
    client: &Client,
    history: &mut Vec<OllamaMessage>
) -> Result<(), Box<dyn std::error::Error>> {
    let user_input: String = read_user_input();
    let git_status = get_git_status();

    let mut attempts: i8 = 0;

    loop {
        if attempts > 20 {
            println!("{}", style(format!("ABORTING: took too many attempts ({})", attempts)).bold().red());
            break;
        }

        let response = get_llm_response(client, &user_input, &git_status, history).await?;
        let mut valid: bool = false;

        let (maybe_command, maybe_final_message) =
            if let Some((before, after)) = response.split_once("FINAL:") {
                (before, Some(after.trim()))
            } else {
                (response.as_str(), None)
            };

        if let Some((_tag, raw_text)) = maybe_command.split_once("EXECUTE:") {
            let command = raw_text.lines().next().unwrap_or("").trim();

            if !command.is_empty() {
                valid = true;
                if let Some((output, error)) = handle_execution(command)? {
                    let feedback = if error != ""  {
                        format!("Output of `{}`:\n{}\nIMPORTANT! FIX THIS ERROR: {}", command, output, error)
                    }
                    else {
                        format!("Output of `{}`:\n{}", command, output)
                    };

                    history.push(OllamaMessage {
                        role: "user".to_string(),
                        content: feedback,
                    });
                }
            }
        }

        if let Some(msg) = maybe_final_message {
            if !msg.is_empty() {
                println!("{}: {}", style("Jade").green().bold(), msg);
                break;
            }
        }

        if !valid {
            println!("{}", style("ABORTING: Error parsing response").bold().red());
            break;
        }

        attempts += 1;
    }

    Ok(())
}

async fn start_ollama(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    let health_url = "http://localhost:11434";

    if client.get(health_url).send().await.is_ok() {
        return Ok(());
    }

    println!("{} Ollama server not found. Starting it up...", style("⚡").yellow());

    let _child = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", "ollama serve"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
    } else {
        Command::new("ollama")
            .arg("serve")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
    };

    match _child {
        Ok(_) => {
            let pb = indicatif::ProgressBar::new_spinner();
            pb.set_message("Waiting for Ollama to initialize...");
            pb.enable_steady_tick(Duration::from_millis(100));

            for _ in 0..50 {
                if client.get(health_url).send().await.is_ok() {
                    pb.finish_with_message("Ollama is ready.");
                    return Ok(());
                }
                thread::sleep(Duration::from_millis(100));
            }

            Err("Ollama started, but didn't respond in time.".into())
        }
        Err(e) => {
            Err(format!("Failed to start 'ollama server'. Is it installed? Error: {}", e).into())
        }
    }
}

#[tokio::main]
async fn main() {
    print_welcome();

    let client = Client::new();
    if let Err(e) = start_ollama(&client).await {
        println!("{}", style(format!("Error: {}", e)).red());
        return;
    }

    let mut history: Vec<OllamaMessage> = Vec::new();

    loop {
        if let Err(e) = repl_step(&client, &mut history).await {
            println!("{}", style(format!("Critical Error: {}", e)).red().bold());
        }
    }

}
