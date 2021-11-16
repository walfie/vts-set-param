use anyhow::{Context, Result};
use figment::providers::{Format, Json, Serialized};
use figment::Figment;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;
use vtubestudio::data::*;
use vtubestudio::Client;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let config_path = xdg::BaseDirectories::new()?
        .place_config_file("vts-set-param.json")
        .context("Failed to find config path")?;

    let mut args: Args = Figment::new()
        .merge(Json::file(&config_path))
        .join(Serialized::defaults(Args::from_args()))
        .extract()?;

    let conf = &mut args.config;

    let (mut client, mut new_tokens) = Client::builder()
        .auth_token(conf.token.clone())
        .authentication(
            conf.plugin_name.clone(),
            conf.plugin_developer.clone(),
            None,
        )
        .build_tungstenite();

    client
        .send(&ParameterCreationRequest {
            parameter_name: args.param_id.clone(),
            explanation: args.explanation.clone(),
            min: args.min,
            max: args.max,
            default_value: args.default,
        })
        .await?;

    client
        .send(&InjectParameterDataRequest {
            parameter_values: vec![ParameterValue {
                id: args.param_id.clone(),
                value: args.value.unwrap_or(args.default),
                weight: None,
            }],
        })
        .await?;

    drop(client);

    if let Some(new_token) = new_tokens.next().await {
        conf.token = Some(new_token);
        if let Err(e) = std::fs::write(&config_path, serde_json::to_string_pretty(&conf)?) {
            eprintln!("Failed to write config file: {:?} {}", &config_path, e);
        } else {
            eprintln!("Wrote config file: {:?}", &config_path);
        }
    }

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
