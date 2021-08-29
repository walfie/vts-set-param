use anyhow::{bail, Context, Result};
use figment::providers::{Format, Json, Serialized};
use figment::Figment;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::path::PathBuf;
use structopt::StructOpt;
use tungstenite::{Message, WebSocket};

fn main() -> Result<()> {
    let config_path = xdg::BaseDirectories::new()?
        .place_config_file("vts-set-param.json")
        .context("Failed to find config path")?;

    let mut args: Args = Figment::new()
        .merge(Json::file(&config_path))
        .join(Serialized::defaults(Args::from_args()))
        .extract()?;

    let conf = &mut args.config;

    let url = format!("ws://{}:{}", conf.host, conf.port);
    let (mut socket, _resp) = tungstenite::connect(&url)
        .with_context(|| format!("Failed to connect to address {}", url))?;

    if conf.token.is_none() {
        set_auth_token(&mut socket, conf, &config_path)?
    };

    if !authenticate(&mut socket, &conf)?.authenticated {
        set_auth_token(&mut socket, conf, &config_path)?;
        let resp = authenticate(&mut socket, &conf)?;

        if !resp.authenticated {
            bail!("Failed to authenticate: {}", resp.reason);
        }
    }

    register_param(&mut socket, &args)?;
    inject_param(&mut socket, &args)?;

    Ok(())
}

#[derive(Serialize, Deserialize, StructOpt)]
struct Args {
    #[serde(flatten)]
    #[structopt(flatten)]
    config: Config,
    #[structopt(long)]
    param_id: String,
    #[structopt(long, default_value = "0")]
    default: f64,
    #[structopt(long)]
    value: Option<f64>,
    #[structopt(long, default_value = "0")]
    min: f64,
    #[structopt(long, default_value = "100")]
    max: f64,
    #[structopt(long)]
    explanation: Option<String>,
}

#[derive(Serialize, Deserialize, StructOpt)]
struct Config {
    #[structopt(short, long, default_value = "localhost")]
    host: String,
    #[structopt(short, long, default_value = "8001")]
    port: u16,
    #[structopt(long, env, hide_env_values = true)]
    token: Option<String>,
    #[structopt(long, default_value = "vts-set-param")]
    plugin_name: String,
    #[structopt(long, default_value = "Walfie")]
    plugin_developer: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Request<T> {
    api_name: &'static str,
    api_version: &'static str,
    #[serde(rename = "requestID")]
    request_id: &'static str,
    message_type: &'static str,
    data: T,
}

impl<T> Request<T> {
    fn new(request_id: &'static str, message_type: &'static str, data: T) -> Self {
        Self {
            api_name: "VTubeStudioPublicAPI",
            api_version: "1.0",
            request_id,
            message_type,
            data,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    #[serde(rename = "requestID")]
    request_id: String,
    message_type: String,
    data: Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthenticationTokenResponse {
    authentication_token: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthenticationResponse {
    authenticated: bool,
    reason: String,
}

fn wait_for_resp<T: Read + Write>(
    socket: &mut WebSocket<T>,
    id: &'static str,
    message_type: &'static str,
) -> Result<Value> {
    loop {
        if let Message::Text(json) = socket
            .read_message()
            .context("Failed to read from socket")?
        {
            let resp = serde_json::from_str::<Response>(&json)
                .with_context(|| format!("Failed to parse JSON: {}", &json))?;

            if resp.request_id == id {
                if resp.message_type == message_type {
                    return Ok(resp.data);
                } else {
                    anyhow::bail!("Received unexpected message: {}", &json);
                }
            }
        }
    }
}

fn set_auth_token<T: Read + Write>(
    socket: &mut WebSocket<T>,
    conf: &mut Config,
    config_path: &PathBuf,
) -> Result<()> {
    let req = Request::new(
        "0",
        "AuthenticationTokenRequest",
        json!({
            "pluginName": conf.plugin_name,
            "pluginDeveloper": conf.plugin_developer,
        }),
    );

    eprintln!("Requesting authentication token (check VTube Studio window)");

    socket
        .write_message(Message::Text(serde_json::to_string(&req)?))
        .context("Failed to get authentication token")?;

    let resp = wait_for_resp(socket, "0", "AuthenticationTokenResponse")?;
    let token = serde_json::from_value::<AuthenticationTokenResponse>(resp)
        .context("Failed to read AuthenticationTokenResponse")?
        .authentication_token;

    conf.token = Some(token);
    if let Err(e) = std::fs::write(&config_path, serde_json::to_string_pretty(&conf)?) {
        eprintln!("Failed to write config file: {:?} {}", &config_path, e);
    } else {
        eprintln!("Wrote config file: {:?}", &config_path);
    }

    Ok(())
}

fn authenticate<T: Read + Write>(
    socket: &mut WebSocket<T>,
    conf: &Config,
) -> Result<AuthenticationResponse> {
    let req = Request::new(
        "1",
        "AuthenticationRequest",
        json!({
            "pluginName": conf.plugin_name,
            "pluginDeveloper": conf.plugin_developer,
            "authenticationToken": conf.token,
        }),
    );

    socket
        .write_message(Message::Text(serde_json::to_string(&req)?))
        .context("Failed to send AuthenticationRequest")?;

    let resp = wait_for_resp(socket, "1", "AuthenticationResponse")?;

    Ok(serde_json::from_value::<AuthenticationResponse>(resp)
        .context("Failed to read AuthenticationResponse")?)
}

fn register_param<T: Read + Write>(socket: &mut WebSocket<T>, args: &Args) -> Result<()> {
    let req = Request::new(
        "2",
        "ParameterCreationRequest",
        json!({
            "parameterName": args.param_id,
            "explanation": args.explanation,
            "min": args.min,
            "max": args.max,
            "defaultValue": args.default,
        }),
    );

    socket
        .write_message(Message::Text(serde_json::to_string(&req)?))
        .context("Failed to send ParameterCreationRequest")?;

    let _ = wait_for_resp(socket, "2", "ParameterCreationResponse")?;
    Ok(())
}

fn inject_param<T: Read + Write>(socket: &mut WebSocket<T>, args: &Args) -> Result<()> {
    let req = Request::new(
        "3",
        "InjectParameterDataRequest",
        json!({
            "parameterValues": [{
                "id": args.param_id,
                "value": args.value.unwrap_or(args.default),
            }]
        }),
    );

    socket
        .write_message(Message::Text(serde_json::to_string(&req)?))
        .context("Failed to send InjectParameterDataRequest")?;

    let _ = wait_for_resp(socket, "3", "InjectParameterDataResponse")?;
    Ok(())
}
