use clap::Parser;
use config::{Args, Config};
use smppclient::SmppConnection;
use std::{io::Write, sync::Arc};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use yapp::{PasswordReader, Yapp};

mod config;
mod smppclient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if std::env::var_os("SMSCLI_LOG").is_none() {
        std::env::set_var("SMSCLI_LOG", "warning");
    }

    tracing_subscriber::fmt()
        .with_ansi(!args.disable_ansii)
        .with_env_filter(tracing_subscriber::EnvFilter::from_env("SMSCLI_LOG"))
        .init();

    let config = Config::load();

    let mut login: Option<String> = args
        .login
        .or_else(|| config.as_ref().and_then(|c| c.login.clone()));

    let mut password = args
        .password
        .or_else(|| config.as_ref().and_then(|c| c.password.clone()));

    let mut server = args
        .server
        .or_else(|| config.as_ref().and_then(|c| c.smsc_host.clone()));

    let mut source_addr = args
        .source_addr
        .or_else(|| config.as_ref().and_then(|c| c.source_addr.clone()));

    if server.is_none() {
        server = Some(String::from("192.168.254.36:3600"));
    }

    if source_addr.is_none() {
        source_addr = Some(String::from("GPP IT"));
    }

    // Validation
    if args.message.is_empty()
        || args.message.chars().all(|c| c.is_ascii()) != true
        || args.message.len() > 140
    {
        return Err("Invalid message. Message content length must be <=140 characters. Only ascii characters are allowed.".into());
    }

    if args.phone_number.is_empty()
        || !args.phone_number.chars().all(|c| c.is_ascii_digit())
        || args.phone_number.len() > 15
    {
        return Err("Invalid phone number.".into());
    }

    // Ask for credentials if not yet provided
    if login.is_none() {
        println!("Please provide credentials for SMSC service.");
        let mut input = String::new();
        print!("Login: ");
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_line(&mut input).unwrap();
        login = Some(input.trim().to_string());
    }

    if password.is_none() {
        let mut yapp = Yapp::new().with_echo_symbol('*');
        password = Some(yapp.read_password_with_prompt("Password: ").unwrap());
    }

    match send_sms(
        args.phone_number,
        args.message,
        server.unwrap_or_default(),
        login.unwrap_or_default(),
        password.unwrap_or_default(),
        source_addr.unwrap_or_default(),
    )
    .await
    {
        Ok(_) => println!("Successfully sent a message."),
        Err(e) => return Err(e),
    }

    Ok(())
}

// TODO: Add support for messages longer than 140 characters.
async fn send_sms(
    phone: String,
    message: String,
    server: String,
    system_id: String,
    password: String,
    sender_addr: String,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::debug!(
        "send_sms called: (phone: {}, message: {}, server: {}, system_id: {}, sender_addr: {}",
        phone,
        message,
        server,
        system_id,
        sender_addr
    );
    let stream = match TcpStream::connect(&server).await {
        Ok(it) => it,
        Err(e) => return Err(format!("Failed to connect to server. Details: {:?}", e).into()),
    };

    let (reader, writer) = stream.into_split();

    let smpp_conn = Arc::new(Mutex::new(SmppConnection::new(reader, writer)));

    let mut c = smpp_conn.lock().await;

    if let Err(e) = c.bind_transceiver(&system_id, &password).await {
        return Err(format!("Failed to bind. Details: {:?}", e).into());
    }

    if let Err(e) = c.submit_sm(&phone, &message, &sender_addr).await {
        return Err(format!("Sending submit_sm failed. Details: {:?}", e).into());
    };

    c.unbind().await
}
