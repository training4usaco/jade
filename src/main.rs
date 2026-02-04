use console::style;
use std::{io, process};
use std::io::Write;
use std::process::Command;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use dialoguer::{theme::ColorfulTheme, Confirm};

const SYSTEM_PROMPT: &str = include_str!("prompts/system_prompt.txt");
const GEMINI_MODEL: &str = "gemini-3-flash-preview";

#[derive(Serialize, Deserialize, Clone, Debug)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(rename = "systemInstruction")]
    system_instruction: Option<GeminiContent>,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
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

async fn get_llm_response(
    client: &Client,
    api_key: &str,
    user_input: &str,
    git_status: &str,
    history: &mut Vec<GeminiContent>,
) -> Result<String, Box<dyn std::error::Error>> {
    let system_instruction = GeminiContent {
        role: "user".to_string(),
        parts: vec![GeminiPart { text: format!("{}\n\nGIT STATUS:\n{}", SYSTEM_PROMPT, git_status) }],
    };

    history.push(GeminiContent {
        role: "user".to_string(),
        parts: vec![GeminiPart { text: user_input.to_string() }],
    });

    let request_body = GeminiRequest {
        contents: history.clone(),
        system_instruction: Some(system_instruction),
    };

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        GEMINI_MODEL, api_key
    );

    let res = client.post(&url).json(&request_body).send().await?;

    if !res.status().is_success() {
        let error_text = res.text().await?;
        return Err(format!("API Error: {}", error_text).into());
    }

    let response_json: GeminiResponse = res.json().await?;

    let response_text = response_json
        .candidates
        .as_ref()
        .and_then(|c| c.first())
        .and_then(|c| c.content.parts.first())
        .map(|p| p.text.clone())
        .unwrap_or_else(|| "No response.".to_string());

    let cleaned_text = response_text.replace("`", "").trim().to_string();

    history.push(GeminiContent {
        role: "model".to_string(),
        parts: vec![GeminiPart { text: cleaned_text.clone() }],
    });

    if history.len() > 20 { history.drain(0..2); }

    Ok(cleaned_text)
}

fn handle_execution(command: &str) -> Result<Option<(String, String)>, Box<dyn std::error::Error>> {
    println!("{} {}", style("Suggestion:").bold().blue(), command);

    if command.contains("reset --hard") || command.contains("rm -rf") {
        println!("{}", style("ABORTING: Destructive command detected.").bold().red());
        return Ok(None);
    }

    if Confirm::with_theme(&ColorfulTheme::default()).with_prompt("Execute?").default(true).interact()? {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd").args(["/C", command]).output()?
        } else {
            Command::new("sh").arg("-c").arg(command).output()?
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            println!("{}", style("✔ Success").green());
            if !stdout.is_empty() { println!("{}", style(&stdout).dim()); }
        } else {
            println!("{}", style("✖ Failed").red());
            if !stderr.is_empty() { println!("{}", style(&stderr).red()); }
        }

        return Ok(Some((stdout, stderr)));
    }
    Ok(None)
}

async fn repl_step(
    client: &Client,
    api_key: &str,
    history: &mut Vec<GeminiContent>
) -> Result<(), Box<dyn std::error::Error>> {
    let user_input: String = read_user_input();
    let git_status = get_git_status();

    let mut attempts: i8 = 0;

    loop {
        if attempts > 20 {
            println!("{}", style(format!("ABORTING: took too many attempts ({})", attempts)).bold().red());
            break;
        }

        let response = get_llm_response(client, api_key, &user_input, &git_status, history).await?;
        println!("LLM RESPONSE: {}", response);
        let mut valid: bool = false;

        let (maybe_command, maybe_final_message) =
            if let Some((before, after)) = response.split_once("FINAL:") {
                (before, Some(after.trim()))
            } else {
                (response.as_str(), None)
            };

        if let Some((_tag, command)) = maybe_command.split_once("EXECUTE:") {
            let clean_command = command.trim();

            if !clean_command.is_empty() {
                valid = true;
                if let Some((output, error)) = handle_execution(clean_command)? {
                    let feedback = format!("Output of `{}`:\n{}\n{}", clean_command, output, error);
                    history.push(GeminiContent {
                        role: "user".to_string(),
                        parts: vec![GeminiPart { text: feedback }],
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

#[tokio::main]
async fn main() {
    print_welcome();

    dotenv().ok();
    let api_key = std::env::var("GEMINI_API_KEY").expect("KEY NOT FOUND");

    let client = Client::new();
    let mut history: Vec<GeminiContent> = Vec::new();

    loop {
        if let Err(e) = repl_step(&client, &api_key, &mut history).await {
            println!("{}", style(format!("Critical Error: {}", e)).red().bold());
        }
    }

}
