use console::style;
use std::{io, process};
use std::io::Write;
use std::process::Command;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use reqwest::Client;

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

async fn repl_step(
    client: &Client,
    api_key: &str,
    history: &mut Vec<GeminiContent>
) -> Result<(), Box<dyn std::error::Error>> {
    let user_input: String = read_user_input();
    let git_status = get_git_status();

    let response_result = get_llm_response(client, api_key, &user_input, &git_status, history).await?;
    println!("{}", response_result);

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
