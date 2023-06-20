use anyhow::Result;
use clap::Parser;
use std::collections::HashSet;

fn main() -> Result<()> {
    let opt = Opt::parse();
    let _guard = sentry::init((
        opt.client_key,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            session_mode: sentry::SessionMode::Request,
            ..Default::default()
        },
    ));
    sentry::configure_scope(|scope| scope.set_tag("bot-name", &opt.bot_name));
    let http_client = reqwest::blocking::Client::new();
    let mut states: HashSet<(String, String)> = HashSet::new();

    loop {
        match http_client
            .get(&opt.url)
            .header("accept", "application/json")
            .send()
        {
            Err(e) => {
                eprintln!("======= {e:?}");
                sentry::capture_message(
                    &format!("Fetching status page error: {e:?}"),
                    sentry::Level::Error,
                );
            }
            Ok(response) => {
                let statuses = response.json::<serde_json::Value>()?;
                let statuses = statuses
                    .as_object()
                    .expect("Wrong JSON format 1")
                    .get("statuses")
                    .expect("Wrong JSON format 2")
                    .as_array()
                    .expect("Wrong JSON format 3");

                let errors: HashSet<(String, String)> = statuses
                    .iter()
                    .filter_map(|i| {
                        let i = i.as_object().expect("Wrong JSON format 4");
                        let short = i
                            .get("short")
                            .expect("Wrong JSON format 13")
                            .as_str()
                            .expect("Wrong JSON format 14");
                        if short == "error" || short == "out-of-date" {
                            let label = i
                                .get("label")
                                .expect("Wrong JSON format 5")
                                .as_str()
                                .expect("Wrong JSON format 6");
                            let last_result = i
                                .get("status")
                                .expect("Wrong JSON format 7")
                                .as_object()
                                .expect("Wrong JSON format 8")
                                .get("last-result")
                                .expect("Wrong JSON format 9")
                                .as_object()
                                .expect("Wrong JSON format 10")
                                .get("value")
                                .expect("Wrong JSON format 11")
                                .as_object()
                                .expect("Wrong JSON format 12");
                            match last_result.get("Err") {
                                // with `short` judged ahead, this should always be `Err`, right?
                                None => {
                                    // The Result is Ok
                                    // Do nothing
                                    None
                                }
                                Some(err) => {
                                    let err = err.as_str().expect("Wrong JSON format 13");
                                    Some((label.to_string(), err.to_string()))
                                }
                            }
                        } else {
                            None
                        }
                    })
                    .collect();

                // New errors
                for (title, msg) in errors.difference(&states) {
                    eprintln!("======= {title}: {msg}");
                    sentry::with_scope(
                        |scope| scope.set_tag("part-name", title),
                        || {
                            sentry::capture_message(
                                &format!("{title}: {msg}"),
                                sentry::Level::Error,
                            )
                        },
                    );
                }
                // Gone errors
                for (title, msg) in states.difference(&errors) {
                    eprintln!("======= {title} Recoverred: {msg}");
                    sentry::with_scope(
                        |scope| scope.set_tag("part-name", title),
                        || {
                            sentry::capture_message(
                                &format!("{title} Recoverred: {msg}"),
                                sentry::Level::Info,
                            )
                        },
                    );
                }

                eprintln!("####### {}", states.len());
                states = errors;
                eprintln!("####### {}", states.len());
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(opt.interval));
    }
}

#[derive(Parser, Debug)]
struct Opt {
    /// Sentry client key
    #[arg(short, long, env = "SENTRY_KEY")]
    client_key: String,
    /// Watch interval (ms)
    #[arg(short, long)]
    interval: u64,
    /// Status endpoint
    #[arg(short, long)]
    url: String,
    /// Bot name
    #[arg(short, long)]
    bot_name: String,
}
